// Usage: quick_layer!(layer_name, method_name, Type, |args, ctx| { ... });
#[macro_export]
macro_rules! quick_layer {
    ($layer:expr, $method:expr, $arg_type:ty, $func:expr) => {
        $crate::Layer::builder($layer)
            .method($method)
            .args::<$arg_type>()
            .bind($func)
            .build()
    };

    ($layer:expr, $method:expr, $arg_type:ty, default = $default:expr, $func:expr) => {
        $crate::Layer::builder($layer)
            .method($method)
            .args_with_default::<$arg_type>($default)
            .bind($func)
            .build()
    };
}

// Usage: add_slices!(engine_builder, slice1, slice2, slice3);
#[macro_export]
macro_rules! add_slices {
    ($builder:expr, $($slice:expr),+ $(,)?) => {{
        let mut builder = $builder;
        $(
            builder = builder.add_slice($slice);
        )*
        builder
    }};
}

// Usage: add_layers!(engine_builder, layer1, layer2, layer3);
#[macro_export]
macro_rules! add_layers {
    ($builder:expr, $($layer:expr),+ $(,)?) => {{
        let mut builder = $builder;
        $(
            builder = builder.add_layer($layer);
        )*
        builder
    }};
}

// Usage: dependencies!(engine_builder, layer1 => [dep1, dep2], layer2 => [dep3]);
#[macro_export]
macro_rules! dependencies {
    ($builder:expr, $($layer:expr => [$($dep:expr),+ $(,)?]),+ $(,)?) => {{
        let mut builder = $builder;
        $(
            $(
                builder = builder.dependency($layer, $dep);
            )*
        )*
        builder
    }};
}

#[macro_export]
macro_rules! value {
    (null) => {
        $crate::Value::Null
    };

    (true) => {
        $crate::Value::Bool(true)
    };
    (false) => {
        $crate::Value::Bool(false)
    };

    ($num:literal) => {
        $crate::Value::from($num)
    };

    ($s:literal) => {
        $crate::Value::from($s)
    };

    ([$($item:tt),* $(,)?]) => {
        $crate::Value::Array(vec![
            $($crate::value!($item)),*
        ])
    };

    ({$($key:literal : $value:tt),* $(,)?}) => {{
        let mut map = std::collections::HashMap::new();
        $(
            let key_str = stringify!($key);
            let key = key_str.trim_matches('"').to_string();
            map.insert(key, $crate::value!($value));
        )*
        $crate::Value::Object(map)
    }};

    ({$($key:literal : $value:expr),* $(,)?}) => {{
        let mut map = std::collections::HashMap::new();
        $(
            let key_str = stringify!($key);
            let key = key_str.trim_matches('"').to_string();
            map.insert(key, $crate::value!($value));
        )*
        $crate::Value::Object(map)
    }};

    ($expr:expr) => {
        $crate::Value::from($expr)
    };
}

#[macro_export]
macro_rules! execution_error {
    ($msg:expr) => {
        $crate::Error::ExecutionError($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error::ExecutionError(format!($fmt, $($arg)*))
    };
}

#[cfg(feature = "serde_json")]
#[macro_export]
macro_rules! json_wrapper {
    // Main usage: json_wrapper!(WrapperName, InnerType);
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        struct $name($inner);

        impl $name {
            pub fn new() -> Self
            where
                $inner: std::default::Default,
            {
                Self(<$inner>::default())
            }

            pub fn from_inner(inner: $inner) -> Self {
                Self(inner)
            }

            pub fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl $crate::ToValue for $name {
            fn to_value(&self) -> $crate::Value {
                let json = serde_json::to_string(&self.0).unwrap_or_else(|_| String::new());
                $crate::Value::String(json)
            }
        }

        impl $crate::FromValue for $name {
            fn from_value(value: &$crate::Value) -> $crate::Result<Self> {
                let json = value
                    .as_str()
                    .ok_or_else(|| $crate::Error::ConfigError("Expected string".into()))?;
                let inner: $inner =
                    serde_json::from_str(json).map_err(|e| $crate::Error::ConfigError(e.to_string()))?;
                Ok($name(inner))
            }
        }

        // Auto-implement Default if inner implements Default
        impl std::default::Default for $name
        where
            $inner: std::default::Default,
        {
            fn default() -> Self {
                Self(<$inner>::default())
            }
        }
    };

    // With visibility modifier: json_wrapper!(pub WrapperName, InnerType);
    ($vis:vis $name:ident, $inner:ty) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        $vis struct $name($inner);

        impl $name {
            $vis fn new() -> Self
            where
                $inner: std::default::Default,
            {
                Self(<$inner>::default())
            }

            $vis fn from_inner(inner: $inner) -> Self {
                Self(inner)
            }

            $vis fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl $crate::ToValue for $name {
            fn to_value(&self) -> $crate::Value {
                let json = serde_json::to_string(&self.0).unwrap_or_else(|_| String::new());
                $crate::Value::String(json)
            }
        }

        impl $crate::FromValue for $name {
            fn from_value(value: &$crate::Value) -> $crate::Result<Self> {
                let json = value
                    .as_str()
                    .ok_or_else(|| $crate::Error::ConfigError("Expected string".into()))?;
                let inner: $inner =
                    serde_json::from_str(json).map_err(|e| $crate::Error::ConfigError(e.to_string()))?;
                Ok($name(inner))
            }
        }

        // Auto-implement Default if inner implements Default
        impl std::default::Default for $name
        where
            $inner: std::default::Default,
        {
            fn default() -> Self {
                Self(<$inner>::default())
            }
        }
    };
}

// Define KiB (Kibibyte = 1024 bytes)
#[macro_export]
macro_rules! KiB {
    ($x:expr) => {
        $x * 1024
    };
}

// Define MiB (Mebibyte = 1024 * 1024 bytes)
#[macro_export]
macro_rules! MiB {
    ($x:expr) => {
        $x * 1024 * 1024
    };
}

// Define GiB (Gibibyte = 1024 * 1024 * 1024 bytes)
#[macro_export]
macro_rules! GiB {
    ($x:expr) => {
        $x * 1024 * 1024 * 1024
    };
}
