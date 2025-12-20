use crate::{Error, Result, Value};
use std::collections::HashMap;

pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self>;
}

pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl FromValue for usize {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_size()
            .ok_or_else(|| Error::ConfigError("Expected u64".into()))
    }
}

impl ToValue for usize {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for u64 {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_u64()
            .ok_or_else(|| Error::ConfigError("Expected u64".into()))
    }
}

impl ToValue for u64 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for i64 {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_i64()
            .ok_or_else(|| Error::ConfigError("Expected i64".into()))
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for i32 {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_i64()
            .map(|v| v as i32)
            .ok_or_else(|| Error::ConfigError("Expected i32".into()))
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for f64 {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_f64()
            .ok_or_else(|| Error::ConfigError("Expected f64".into()))
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for f32 {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_f64()
            .map(|v| v as f32)
            .ok_or_else(|| Error::ConfigError("Expected f32".into()))
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl FromValue for bool {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_bool()
            .ok_or_else(|| Error::ConfigError("Expected bool".into()))
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self> {
        value
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::ConfigError("Expected string".into()))
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::String(self.clone())
    }
}

impl ToValue for &str {
    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl<T: FromValue, const N: usize> FromValue for [T; N] {
    fn from_value(value: &Value) -> Result<Self> {
        let arr = value
            .as_array()
            .ok_or_else(|| Error::ConfigError("Expected array".into()))?;

        if arr.len() != N {
            return Err(Error::ConfigError(format!(
                "Expected array of length {}, got {}",
                N,
                arr.len()
            )));
        }

        let vec: Vec<T> = arr
            .iter()
            .map(|v| T::from_value(v))
            .collect::<Result<Vec<T>>>()?;

        vec.try_into()
            .map_err(|_| Error::ConfigError("Array conversion failed".into()))
    }
}

impl<T: ToValue, const N: usize> ToValue for [T; N] {
    fn to_value(&self) -> Value {
        Value::Array(self.iter().map(|item| item.to_value()).collect())
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: &Value) -> Result<Self> {
        let arr = value
            .as_array()
            .ok_or_else(|| Error::ConfigError("Expected array".into()))?;

        arr.iter().map(|v| T::from_value(v)).collect()
    }
}

impl<T: ToValue> ToValue for Vec<T> {
    fn to_value(&self) -> Value {
        Value::Array(self.iter().map(|item| item.to_value()).collect())
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: &Value) -> Result<Self> {
        if value.is_null() {
            Ok(None)
        } else {
            T::from_value(value).map(Some)
        }
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::Null,
        }
    }
}

impl<T: FromValue> FromValue for HashMap<String, T> {
    fn from_value(value: &Value) -> Result<Self> {
        let obj = value
            .as_object()
            .ok_or_else(|| Error::ConfigError("Expected object".into()))?;

        obj.iter()
            .map(|(k, v)| T::from_value(v).map(|val| (k.clone(), val)))
            .collect()
    }
}

impl<T: ToValue> ToValue for HashMap<String, T> {
    fn to_value(&self) -> Value {
        Value::Object(
            self.iter()
                .map(|(k, v)| (k.clone(), v.to_value()))
                .collect(),
        )
    }
}

impl FromValue for Value {
    fn from_value(value: &Value) -> Result<Self> {
        Ok(value.clone())
    }
}

impl ToValue for Value {
    fn to_value(&self) -> Value {
        self.clone()
    }
}

impl FromValue for () {
    fn from_value(_value: &Value) -> Result<Self> {
        Ok(())
    }
}

impl ToValue for () {
    fn to_value(&self) -> Value {
        Value::Null
    }
}
