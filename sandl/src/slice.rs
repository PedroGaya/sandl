use std::collections::HashMap;

use crate::Value;

pub struct LayerArgs {
    pub layer: String,
    pub methods_args: HashMap<String, Value>,
}

pub struct Slice {
    pub name: String,
    pub methods_per_layer: HashMap<String, HashMap<String, Value>>,
}

impl Slice {
    pub fn new(name: String) -> Self {
        Self {
            name,
            methods_per_layer: HashMap::new(),
        }
    }

    pub fn with_layer(mut self, layer_args: LayerArgs) -> Self {
        self.methods_per_layer
            .insert(layer_args.layer, layer_args.methods_args);
        self
    }

    pub fn has_layer(&self, layer: &str) -> bool {
        self.methods_per_layer.contains_key(layer)
    }

    pub fn get_layer_names(&self) -> crate::Result<Vec<&str>> {
        Ok(self.methods_per_layer.keys().map(|k| k.as_str()).collect())
    }

    pub fn get_layer_methods(&self, layer: &str) -> crate::Result<Vec<&str>> {
        let methods = self
            .methods_per_layer
            .get(layer)
            .ok_or_else(|| crate::Error::LayerNotFound(layer.to_string()))?;

        Ok(methods.keys().map(|k| k.as_str()).collect())
    }

    pub fn get_method_arg(&self, layer: &str, method: &str) -> crate::Result<&Value> {
        let methods = self
            .methods_per_layer
            .get(layer)
            .ok_or_else(|| crate::Error::LayerNotFound(layer.to_string()))?;

        methods
            .get(method)
            .ok_or_else(|| crate::Error::MethodNotFound {
                method: method.to_string(),
                layer: layer.to_string(),
            })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
