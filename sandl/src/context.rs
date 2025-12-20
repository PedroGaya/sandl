use crate::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct Context {
    data: Arc<RwLock<HashMap<String, Value>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.read().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: impl Into<String>, value: Value) {
        self.data.write().unwrap().insert(key.into(), value);
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.read().unwrap().contains_key(key)
    }

    pub fn remove(&self, key: &str) -> Option<Value> {
        self.data.write().unwrap().remove(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.data.read().unwrap().keys().cloned().collect()
    }

    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }

    pub fn get_as<T>(&self, key: &str) -> crate::Result<T>
    where
        T: crate::FromValue,
    {
        let value = self.get(key).ok_or_else(|| {
            crate::Error::ConfigError(format!("Key '{}' not found in context", key))
        })?;
        T::from_value(&value)
    }

    pub fn set_from<T>(&self, key: impl Into<String>, value: T)
    where
        T: crate::ToValue,
    {
        self.set(key, value.to_value());
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
