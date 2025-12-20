use sandl::*;

#[test]
fn args_default_values() {
    let layer = Layer::builder("layer")
        .method("work")
        .args_with_default(value!({ "x": 10, "y": 20 }))
        .bind(|args, _ctx| {
            let x = args.get("x").unwrap().as_i64().unwrap();
            let y = args.get("y").unwrap().as_i64().unwrap();
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            Ok(value!({ "sum": x + y }))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("layer".to_string(), "work".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(result.get("sum").unwrap().as_i64().unwrap(), 30);
}

#[test]
fn args_explicit_override() {
    let layer = Layer::builder("layer")
        .method("multiply")
        .args_with_default(value!({ "a": 2, "b": 3 }))
        .bind(|args, _ctx| {
            let a = args.get("a").unwrap().as_i64().unwrap();
            let b = args.get("b").unwrap().as_i64().unwrap();
            Ok(value!({ "result": a * b }))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call("multiply", value!({ "a": 5, "b": 7 })))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("layer".to_string(), "multiply".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(result.get("result").unwrap().as_i64().unwrap(), 35);
}

#[test]
fn args_partial_override() {
    let layer = Layer::builder("layer")
        .method("compute")
        .args_with_default(value!({ "x": 10, "y": 20, "z": 30 }))
        .bind(|args, _ctx| {
            let x = args.get("x").unwrap().as_i64().unwrap();
            let y = args.get("y").unwrap().as_i64().unwrap();
            let z = args.get("z").unwrap().as_i64().unwrap();
            Ok(value!({ "sum": x + y + z }))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call("compute", value!({ "x": 100 })))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("layer".to_string(), "compute".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(result.get("sum").unwrap().as_i64().unwrap(), 150);
}

#[test]
fn args_different_types() {
    let layer = Layer::builder("layer")
        .method("mixed")
        .args_with_default(value!({
            "count": 42,
            "name": "Alice",
            "active": true,
            "ratio": 3.14
        }))
        .bind(|args, _ctx| {
            let count = args.get("count").unwrap().as_i64().unwrap();
            let name = args.get("name").unwrap().as_str().unwrap();
            let active = args.get("active").unwrap().as_bool().unwrap();
            let ratio = args.get("ratio").unwrap().as_f64().unwrap();

            assert_eq!(count, 42);
            assert_eq!(name, "Alice");
            assert_eq!(active, true);
            assert_eq!(ratio, 3.14);

            Ok(value!({}))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("mixed"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn args_no_defaults() {
    let layer = Layer::builder("layer")
        .method("work")
        .args::<Value>()
        .bind(|args, _ctx| {
            let value = args.get("value").unwrap().as_i64().unwrap();
            Ok(value!({ "doubled": value * 2 }))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call("work", value!({ "value": 50 })))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("layer".to_string(), "work".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(result.get("doubled").unwrap().as_i64().unwrap(), 100);
}

#[test]
fn args_different_per_slice() {
    let layer = Layer::builder("layer")
        .method("process")
        .args::<Value>()
        .bind(|args, _ctx| {
            let id = args.get("id").unwrap().as_i64().unwrap();
            Ok(value!({ "processed_id": id * 10 }))
        })
        .build();

    let s1 = Slice::builder("s1")
        .layer("layer", |m| m.call("process", value!({ "id": 1 })))
        .build();

    let s2 = Slice::builder("s2")
        .layer("layer", |m| m.call("process", value!({ "id": 2 })))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(s1)
        .add_slice(s2)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);

    let s1_result = results.get("s1").unwrap().as_ref().unwrap();
    let s1_value = s1_result
        .method_results
        .get(&("layer".to_string(), "process".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(s1_value.get("processed_id").unwrap().as_i64().unwrap(), 10);

    let s2_result = results.get("s2").unwrap().as_ref().unwrap();
    let s2_value = s2_result
        .method_results
        .get(&("layer".to_string(), "process".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(s2_value.get("processed_id").unwrap().as_i64().unwrap(), 20);
}

#[test]
fn args_array_type() {
    let layer = Layer::builder("layer")
        .method("sum")
        .args_with_default(value!({ "numbers": [1, 2, 3, 4, 5] }))
        .bind(|args, _ctx| {
            let numbers = args.get("numbers").unwrap().as_array().unwrap();
            let sum: i64 = numbers.iter().filter_map(|v| v.as_i64()).sum();
            Ok(value!({ "sum": sum }))
        })
        .build();

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("sum"))
        .build();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("layer".to_string(), "sum".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(result.get("sum").unwrap().as_i64().unwrap(), 15);
}
