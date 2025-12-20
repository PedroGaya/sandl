use crate::Value;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Layer '{0}' not found")]
    LayerNotFound(String),

    #[error("Layer '{0}' already exists")]
    LayerAlreadyExists(String),

    #[error("Method '{method}' not found in layer '{layer}'")]
    MethodNotFound { method: String, layer: String },

    #[error("Method '{0}' has not been bound in layer '{1}'")]
    MethodNotBound(String, String),

    #[error("Method execution failed in slice '{slice}', layer '{layer}', method '{method}'")]
    MethodExecutionFailed {
        slice: String,
        layer: String,
        method: String,
        args: Value,
        #[source]
        cause: Box<Error>,
    },

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl Error {
    pub fn with_context(
        self,
        slice: impl Into<String>,
        layer: impl Into<String>,
        method: impl Into<String>,
        args: Value,
    ) -> Self {
        Error::MethodExecutionFailed {
            slice: slice.into(),
            layer: layer.into(),
            method: method.into(),
            args,
            cause: Box::new(self),
        }
    }

    pub fn root_cause(&self) -> &Error {
        match self {
            Error::MethodExecutionFailed { cause, .. } => cause.root_cause(),
            other => other,
        }
    }

    pub fn execution_context(&self) -> Option<(&str, &str, &str, &Value)> {
        match self {
            Error::MethodExecutionFailed {
                slice,
                layer,
                method,
                args,
                ..
            } => Some((slice, layer, method, args)),
            _ => None,
        }
    }

    pub fn is_execution_error(&self) -> bool {
        matches!(self, Error::MethodExecutionFailed { .. })
    }

    pub fn message(&self) -> String {
        match self {
            Error::MethodExecutionFailed { cause, .. } => cause.message(),
            Error::ExecutionError(msg) => msg.clone(),
            other => other.to_string(),
        }
    }
}
