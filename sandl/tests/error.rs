use sandl::*;

#[test]
fn error_with_context() {
    let inner = Error::ExecutionError("division by zero".to_string());
    let wrapped = inner.with_context(
        "slice_1",
        "math",
        "divide",
        crate::value!({ "a": 10, "b": 0 }),
    );

    assert!(wrapped.is_execution_error());

    let (slice, layer, method, _args) = wrapped.execution_context().unwrap();
    assert_eq!(slice, "slice_1");
    assert_eq!(layer, "math");
    assert_eq!(method, "divide");
}

#[test]
fn root_cause() {
    let inner = Error::ExecutionError("root problem".to_string());
    let wrapped = inner.with_context("s1", "l1", "m1", crate::value!(null));

    let root = wrapped.root_cause();
    assert!(matches!(root, Error::ExecutionError(_)));
    assert_eq!(root.message(), "root problem");
}

#[test]
fn config_errors_not_execution() {
    let err = Error::LayerNotFound("missing".to_string());
    assert!(!err.is_execution_error());
    assert!(err.execution_context().is_none());
}

#[test]
fn error_message() {
    let inner = Error::ExecutionError("inner message".to_string());
    let wrapped = inner.with_context("s", "l", "m", crate::value!(null));

    // message() should return the inner message, not the full context
    assert_eq!(wrapped.message(), "inner message");
}

#[test]
fn execution_error_macro() {
    let err = execution_error!("test error");
    assert!(matches!(err, Error::ExecutionError(_)));

    let err = execution_error!("value: {}", 42);
    assert_eq!(err.message(), "value: 42");
}

#[test]
fn thiserror_display() {
    let err = Error::LayerNotFound("layer".to_string());
    assert_eq!(err.to_string(), "Layer 'layer' not found");

    let err = Error::MethodNotFound {
        method: "method".to_string(),
        layer: "layer".to_string(),
    };
    assert_eq!(
        err.to_string(),
        "Method 'method' not found in layer 'layer'"
    );
}

#[test]
fn error_source_chain() {
    let inner = Error::ExecutionError("inner error".to_string());
    let wrapped = inner.with_context("s", "l", "m", crate::value!(null));

    // thiserror's #[source] provides error chain traversal
    let source = std::error::Error::source(&wrapped);
    assert!(source.is_some());
}
