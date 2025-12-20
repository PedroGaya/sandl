use crate::*;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct LayerBuilder {
    name: String,
    methods: Vec<MethodBuilder>,
}

pub struct MethodBuilder {
    name: String,
    default_args: Value,
    func: Option<LayerMethodFn>,
}

impl Layer {
    pub fn builder(name: impl Into<String>) -> LayerBuilder {
        LayerBuilder {
            name: name.into(),
            methods: Vec::new(),
        }
    }
}

impl LayerBuilder {
    pub fn method(self, name: impl Into<String>) -> MethodBuilderArgsStep {
        let method_name = name.into();
        MethodBuilderArgsStep {
            layer_builder: self,
            method_name,
        }
    }

    pub fn build(self) -> Layer {
        let mut layer = Layer {
            name: self.name,
            methods_to_defaults: std::collections::HashMap::new(),
            binds: std::collections::HashMap::new(),
        };

        for method in self.methods {
            layer
                .methods_to_defaults
                .insert(method.name.clone(), method.default_args);
            if let Some(func) = method.func {
                layer.binds.insert(method.name, func);
            }
        }

        layer
    }
}

pub struct MethodBuilderArgsStep {
    layer_builder: LayerBuilder,
    method_name: String,
}

pub struct MethodBuilderBindStep<A> {
    layer_builder: LayerBuilder,
    method_name: String,
    default_args: Value,
    _phantom: PhantomData<A>,
}

impl MethodBuilderArgsStep {
    pub fn args_with_default<A: FromValue + ToValue>(self, default: A) -> MethodBuilderBindStep<A> {
        MethodBuilderBindStep {
            layer_builder: self.layer_builder,
            method_name: self.method_name,
            default_args: default.to_value(),
            _phantom: PhantomData,
        }
    }

    pub fn args<A: FromValue + ToValue>(self) -> MethodBuilderBindStep<A> {
        MethodBuilderBindStep {
            layer_builder: self.layer_builder,
            method_name: self.method_name,
            default_args: Value::Null,
            _phantom: PhantomData,
        }
    }
}

impl<A: FromValue + ToValue + 'static> MethodBuilderBindStep<A> {
    pub fn bind<F>(mut self, f: F) -> LayerBuilder
    where
        F: Fn(&A, &Context) -> Result<Value> + Send + Sync + 'static,
    {
        let func = Arc::new(move |args: &Value, context: &Context| {
            let typed_args = A::from_value(args)?;
            f(&typed_args, context)
        });

        self.layer_builder.methods.push(MethodBuilder {
            name: self.method_name,
            default_args: self.default_args,
            func: Some(func),
        });

        self.layer_builder
    }

    pub fn bind_pure<F>(mut self, f: F) -> LayerBuilder
    where
        F: Fn(&A) -> Result<Value> + Send + Sync + 'static,
    {
        let func = Arc::new(move |args: &Value, _context: &Context| {
            let typed_args = A::from_value(args)?;
            f(&typed_args)
        });

        self.layer_builder.methods.push(MethodBuilder {
            name: self.method_name,
            default_args: self.default_args,
            func: Some(func),
        });

        self.layer_builder
    }
}

pub struct SliceBuilder {
    name: String,
    layers: std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
}

impl Slice {
    pub fn builder(name: impl Into<String>) -> SliceBuilder {
        SliceBuilder {
            name: name.into(),
            layers: std::collections::HashMap::new(),
        }
    }
}

impl SliceBuilder {
    pub fn layer<F>(mut self, layer_name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(LayerMethodsBuilder) -> LayerMethodsBuilder,
    {
        let builder = LayerMethodsBuilder {
            methods: std::collections::HashMap::new(),
        };

        let builder = f(builder);
        self.layers.insert(layer_name.into(), builder.methods);
        self
    }

    pub fn build(self) -> Slice {
        Slice {
            name: self.name,
            methods_per_layer: self.layers,
        }
    }
}

pub struct LayerMethodsBuilder {
    methods: std::collections::HashMap<String, Value>,
}

impl LayerMethodsBuilder {
    pub fn call<A: ToValue>(mut self, method_name: impl Into<String>, args: A) -> Self {
        self.methods.insert(method_name.into(), args.to_value());
        self
    }

    pub fn call_default(mut self, method_name: impl Into<String>) -> Self {
        self.methods.insert(method_name.into(), Value::Null);
        self
    }
}

pub struct EngineBuilder {
    layers: Vec<Layer>,
    slices: Vec<Slice>,
    dependencies: std::collections::HashMap<String, Vec<String>>,
    init_layer: Option<String>,
    observer: Observer,
    config: EngineConfig,
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder {
            layers: Vec::new(),
            slices: Vec::new(),
            dependencies: std::collections::HashMap::new(),
            init_layer: None,
            observer: Observer::new(),
            config: EngineConfig::new(),
        }
    }
}

impl EngineBuilder {
    pub fn add_layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn init_layer(mut self, layer_name: impl Into<String>) -> Self {
        self.init_layer = Some(layer_name.into());
        self
    }

    pub fn add_slice(mut self, slice: Slice) -> Self {
        self.slices.push(slice);
        self
    }

    pub fn add_slices(mut self, slices: &mut Vec<Slice>) -> Self {
        self.slices.append(slices);
        self
    }

    pub fn dependency(mut self, layer: impl Into<String>, depends_on: impl Into<String>) -> Self {
        self.dependencies
            .entry(layer.into())
            .or_insert_with(Vec::new)
            .push(depends_on.into());
        self
    }

    pub fn observer(mut self, observer: Observer) -> Self {
        self.observer = observer;
        self
    }

    pub fn observe<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Observer),
    {
        f(&mut self.observer);
        self
    }

    pub fn config(mut self, config: EngineConfig) -> Self {
        self.config = config;
        self
    }

    pub fn num_threads(mut self, threads: usize) -> Self {
        self.config = self.config.num_threads(threads);
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config = self.config.batch_size(size);
        self
    }

    pub fn stack_size(mut self, size: usize) -> Self {
        self.config = self.config.stack_size(size);
        self
    }

    pub fn build(self) -> Result<Engine> {
        let mut engine = Engine::new();
        engine.config = self.config;

        for layer in self.layers {
            engine.register_layer(layer)?;
        }

        if let Some(init_name) = &self.init_layer {
            engine.set_init_layer(init_name)?;

            for layer_name in engine.get_layer_names() {
                if layer_name != *init_name {
                    engine.add_dependency(&layer_name, init_name)?;
                }
            }
        }

        for (layer, deps) in self.dependencies {
            for dep in deps {
                engine.add_dependency(&layer, &dep)?;
            }
        }

        for slice in self.slices {
            engine.register_slice(slice);
        }

        engine.set_observer(self.observer);

        Ok(engine)
    }
}
