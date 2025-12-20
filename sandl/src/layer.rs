use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::*;

pub type LayerMethodFn = Arc<dyn Fn(&Value, &Context) -> Result<Value> + Send + Sync>;

pub struct MethodConfig {
    pub name: String,
    pub default: crate::Value,
}

pub struct Layer {
    pub name: String,
    pub methods_to_defaults: HashMap<String, crate::Value>,
    pub binds: HashMap<String, LayerMethodFn>,
}

impl Layer {
    pub fn new(layer_name: String) -> Self {
        Self {
            name: layer_name,
            methods_to_defaults: HashMap::new(),
            binds: HashMap::new(),
        }
    }

    pub fn with_method(mut self, method: MethodConfig) -> Self {
        self.methods_to_defaults.insert(method.name, method.default);
        self
    }

    pub fn bind<F>(&mut self, method_name: &str, func: F) -> crate::Result<&mut Self>
    where
        F: Fn(&Value, &Context) -> Result<Value> + Send + Sync + 'static,
    {
        if !self.methods_to_defaults.contains_key(method_name) {
            return Err(crate::Error::MethodNotFound {
                method: method_name.to_string(),
                layer: self.name.clone(),
            });
        }

        self.binds.insert(method_name.to_string(), Arc::new(func));
        Ok(self)
    }

    pub fn execute(&self, method_name: &str, args: &Value, ctx: &Context) -> crate::Result<Value> {
        let func = self.binds.get(method_name).ok_or_else(|| {
            crate::Error::MethodNotBound(method_name.to_string(), self.name.clone())
        })?;

        func(&args, ctx)
    }

    pub fn execute_with_default(&self, method_name: &str, ctx: &Context) -> crate::Result<Value> {
        let func = self.binds.get(method_name).ok_or_else(|| {
            crate::Error::MethodNotBound(method_name.to_string(), self.name.clone())
        })?;

        let args = self.get_default_args(method_name).ok_or_else(|| {
            crate::Error::ConfigError("method with no defaults called with null".to_string())
        })?;

        func(&args, ctx)
    }

    pub fn is_bound(&self, method_name: &str) -> bool {
        self.binds.contains_key(method_name)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_methods(&self) -> HashSet<&str> {
        self.methods_to_defaults
            .keys()
            .map(|k| k.as_str())
            .collect()
    }

    pub fn get_default_args(&self, method: &str) -> Option<&crate::Value> {
        self.methods_to_defaults.get(method)
    }
}
