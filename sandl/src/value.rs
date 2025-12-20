use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    UnsignedInt(u64),
    Int(i64),
    Size(usize),
    Float(f64),
}

impl Value {
    pub fn null() -> Self {
        Value::Null
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_size(&self) -> Option<usize> {
        match self {
            Value::Number(Number::Size(i)) => Some(*i),
            Value::Number(Number::Int(i)) => Some(*i as usize),
            Value::Number(Number::UnsignedInt(i)) => Some(*i as usize),
            Value::Number(Number::Float(f)) => Some(*f as usize),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(Number::Size(i)) => Some(*i as u64),
            Value::Number(Number::Int(i)) => Some(*i as u64),
            Value::Number(Number::UnsignedInt(i)) => Some(*i),
            Value::Number(Number::Float(f)) => Some(*f as u64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(Number::Int(i)) => Some(*i),
            Value::Number(Number::Float(f)) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(Number::Float(f)) => Some(*f),
            Value::Number(Number::Int(i)) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object()?.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.as_object_mut()?.get_mut(key)
    }

    pub fn get_index(&self, index: usize) -> Option<&Value> {
        self.as_array()?.get(index)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Number(Number::Size(i))
    }
}

impl From<u64> for Value {
    fn from(i: u64) -> Self {
        Value::Number(Number::UnsignedInt(i))
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Number(Number::Int(i))
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Number(Number::Int(i as i64))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Number(Number::Float(f))
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Number(Number::Float(f as f64))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<HashMap<String, T>> for Value {
    fn from(m: HashMap<String, T>) -> Self {
        Value::Object(m.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

// serde-json compat
#[cfg(feature = "serde_json")]
impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Number(Number::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Value::Number(Number::Float(f))
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

#[cfg(feature = "serde_json")]
impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Number(Number::UnsignedInt(i)) => serde_json::Value::Number(i.into()),
            Value::Number(Number::Size(i)) => serde_json::Value::Number(i.into()),
            Value::Number(Number::Int(i)) => serde_json::Value::Number(i.into()),
            Value::Number(Number::Float(f)) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s) => serde_json::Value::String(s),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            Value::Object(obj) => serde_json::Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect(),
            ),
        }
    }
}
