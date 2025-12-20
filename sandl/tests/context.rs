use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use sandl::*;

#[test]
fn context_basic_operations() {
    let ctx = Context::new();

    ctx.set("name", Value::from("Alice"));
    assert_eq!(ctx.get("name").unwrap().as_str(), Some("Alice"));

    assert!(ctx.contains("name"));
    assert!(!ctx.contains("age"));

    let removed = ctx.remove("name");
    assert!(removed.is_some());
    assert!(!ctx.contains("name"));
}

#[test]
fn context_typed_access() {
    let ctx = Context::new();

    ctx.set_from("count", 42i64);
    let count: i64 = ctx.get_as("count").unwrap();
    assert_eq!(count, 42);

    ctx.set_from("ratio", 3.14f64);
    let ratio: f64 = ctx.get_as("ratio").unwrap();
    assert_eq!(ratio, 3.14);
}

#[test]
fn context_clone_shares_data() {
    let ctx1 = Context::new();
    ctx1.set("shared", Value::from(42));

    let ctx2 = ctx1.clone();
    assert_eq!(ctx2.get("shared").unwrap().as_i64(), Some(42));

    ctx2.set("shared", Value::from(100));
    assert_eq!(ctx1.get("shared").unwrap().as_i64(), Some(100));
}

#[test]
fn context_basic_set_get() {
    let layer1 = quick_layer!("layer1", "set", Value, |_args, ctx| {
        ctx.set("key", Value::from(42));
        Ok(value!({}))
    });

    let layer2 = quick_layer!("layer2", "get", Value, |_args, ctx| {
        let value = ctx.get("key").unwrap();
        assert_eq!(value.as_i64().unwrap(), 42);
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer1", |m| m.call_default("set"))
        .layer("layer2", |m| m.call_default("get"))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), layer1, layer2),
        "layer2" => ["layer1"]
    )
    .add_slice(slice)
    .build()
    .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_missing_key() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        let value = ctx.get("nonexistent");
        assert!(value.is_none());
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_contains() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        assert!(!ctx.contains("key"));

        ctx.set("key", Value::from(42));
        assert!(ctx.contains("key"));

        ctx.remove("key");
        assert!(!ctx.contains("key"));

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_overwrite() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        ctx.set("key", Value::from(10));
        assert_eq!(ctx.get("key").unwrap().as_i64().unwrap(), 10);

        ctx.set("key", Value::from(20));
        assert_eq!(ctx.get("key").unwrap().as_i64().unwrap(), 20);

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_type_safe_access() {
    let layer1 = quick_layer!("layer1", "set", Value, |_args, ctx| {
        ctx.set_from("int", 42i64);
        ctx.set_from("float", 3.14f64);
        ctx.set_from("string", "hello".to_string());
        ctx.set_from("bool", true);
        ctx.set_from("vec", vec![1i64, 2, 3]);
        Ok(value!({}))
    });

    let layer2 = quick_layer!("layer2", "get", Value, |_args, ctx| {
        let int_val: i64 = ctx.get_as("int").unwrap();
        let float_val: f64 = ctx.get_as("float").unwrap();
        let string_val: String = ctx.get_as("string").unwrap();
        let bool_val: bool = ctx.get_as("bool").unwrap();
        let vec_val: Vec<i64> = ctx.get_as("vec").unwrap();

        assert_eq!(int_val, 42);
        assert_eq!(float_val, 3.14);
        assert_eq!(string_val, "hello");
        assert_eq!(bool_val, true);
        assert_eq!(vec_val, vec![1, 2, 3]);

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer1", |m| m.call_default("set"))
        .layer("layer2", |m| m.call_default("get"))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), layer1, layer2),
        "layer2" => ["layer1"]
    )
    .add_slice(slice)
    .build()
    .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_type_conversion_error() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        ctx.set("key", Value::from("not a number"));

        let result: Result<i64> = ctx.get_as("key");
        assert!(result.is_err());

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_clear() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        ctx.set("a", Value::from(1));
        ctx.set("b", Value::from(2));
        ctx.set("c", Value::from(3));

        assert_eq!(ctx.len(), 3);
        assert!(!ctx.is_empty());

        ctx.clear();

        assert_eq!(ctx.len(), 0);
        assert!(ctx.is_empty());

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_keys() {
    let layer = quick_layer!("layer", "work", Value, |_args, ctx| {
        ctx.set("alpha", Value::from(1));
        ctx.set("beta", Value::from(2));
        ctx.set("gamma", Value::from(3));

        let keys = ctx.keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"alpha".to_string()));
        assert!(keys.contains(&"beta".to_string()));
        assert!(keys.contains(&"gamma".to_string()));

        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn context_isolation_between_slices() {
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    let layer = quick_layer!("layer", "work", Value, move |args, ctx| {
        let id = args.get("id").unwrap().as_i64().unwrap();

        assert!(ctx.get("shared").is_none());

        ctx.set("my_id", Value::from(id));
        ctx.set("shared", Value::from(id));

        let my_id = ctx.get("my_id").unwrap().as_i64().unwrap();
        assert_eq!(my_id, id);

        c.fetch_add(1, Ordering::SeqCst);

        Ok(value!({}))
    });

    let s1 = Slice::builder("s1")
        .layer("layer", |m| m.call("work", value!({ "id": 1 })))
        .build();

    let s2 = Slice::builder("s2")
        .layer("layer", |m| m.call("work", value!({ "id": 2 })))
        .build();

    let s3 = Slice::builder("s3")
        .layer("layer", |m| m.call("work", value!({ "id": 3 })))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(s1)
        .add_slice(s2)
        .add_slice(s3)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert_eq!(counter.load(Ordering::SeqCst), 3);
}
#[test]
fn context_shared_across_layers() {
    let layer1 = Layer::builder("layer1")
        .method("set_a")
        .args::<Value>()
        .bind(|_args, ctx| {
            ctx.set("a", Value::from(10));
            Ok(value!({}))
        })
        .method("set_b")
        .args::<Value>()
        .bind(|_args, ctx| {
            ctx.set("b", Value::from(20));
            Ok(value!({}))
        })
        .build();

    let layer2 = quick_layer!("layer2", "verify", Value, |_args, ctx| {
        let a = ctx.get("a").unwrap().as_i64().unwrap();
        let b = ctx.get("b").unwrap().as_i64().unwrap();
        assert_eq!(a, 10);
        assert_eq!(b, 20);
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("layer1", |m| m.call_default("set_a").call_default("set_b"))
        .layer("layer2", |m| m.call_default("verify"))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), layer1, layer2),
        "layer2" => ["layer1"]
    )
    .add_slice(slice)
    .build()
    .unwrap();

    engine.run(RunFlags::SILENT);
}
