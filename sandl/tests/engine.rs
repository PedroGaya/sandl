use sandl::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn independent_layers_can_run_parallel() {
    let concurrent = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let c1 = concurrent.clone();
    let mc1 = max_concurrent.clone();
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c1.fetch_add(1, Ordering::SeqCst) + 1;
            mc1.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c1.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({ "l1": true }))
        })
        .build();

    let c2 = concurrent.clone();
    let mc2 = max_concurrent.clone();
    let l2 = Layer::builder("l2")
        .method("m2")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c2.fetch_add(1, Ordering::SeqCst) + 1;
            mc2.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c2.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({ "l2": true }))
        })
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| methods.call_default("m1"))
        .layer("l2", |methods| methods.call_default("m2"))
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_layer(l2)
        .add_slice(s1)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert!(
        max_concurrent.load(Ordering::SeqCst) >= 2,
        "Expected concurrent execution of independent layers"
    );
}

#[test]
fn multiple_methods_same_layer_run_parallel() {
    let concurrent = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let c1 = concurrent.clone();
    let mc1 = max_concurrent.clone();
    let c2 = concurrent.clone();
    let mc2 = max_concurrent.clone();
    let c3 = concurrent.clone();
    let mc3 = max_concurrent.clone();

    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c1.fetch_add(1, Ordering::SeqCst) + 1;
            mc1.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c1.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({}))
        })
        .method("m2")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c2.fetch_add(1, Ordering::SeqCst) + 1;
            mc2.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c2.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({}))
        })
        .method("m3")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c3.fetch_add(1, Ordering::SeqCst) + 1;
            mc3.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c3.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({}))
        })
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| {
            methods
                .call_default("m1")
                .call_default("m2")
                .call_default("m3")
        })
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert!(
        max_concurrent.load(Ordering::SeqCst) >= 3,
        "Expected 3 methods to run concurrently"
    );
}

#[test]
fn errors_returned_in_results() {
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(|_args, _ctx| Err(Error::ExecutionError("Intentional failure".to_string())))
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| methods.call_default("m1"))
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_result = results.get("s1").unwrap().as_ref().unwrap();

    let method_result = slice_result
        .method_results
        .get(&("l1".to_string(), "m1".to_string()))
        .unwrap();

    assert!(method_result.is_err());
}

#[test]
fn method_failure_does_not_stop_other_methods() {
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(|_args, _ctx| Err(Error::ExecutionError("m1 failed".to_string())))
        .method("m2")
        .args::<Value>()
        .bind(|_args, _ctx| Ok(value!({ "m2": "success" })))
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| {
            methods.call_default("m1").call_default("m2")
        })
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_result = results.get("s1").unwrap().as_ref().unwrap();

    assert!(
        slice_result
            .method_results
            .get(&("l1".to_string(), "m1".to_string()))
            .unwrap()
            .is_err()
    );

    assert!(
        slice_result
            .method_results
            .get(&("l1".to_string(), "m2".to_string()))
            .unwrap()
            .is_ok()
    );
}

#[test]
fn all_results_collected_in_runresults() {
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(|_args, _ctx| Ok(value!({ "m1": 1 })))
        .method("m2")
        .args::<Value>()
        .bind(|_args, _ctx| Ok(value!({ "m2": 2 })))
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| {
            methods.call_default("m1").call_default("m2")
        })
        .build();

    let s2 = Slice::builder("s2")
        .layer("l1", |methods| {
            methods.call_default("m1").call_default("m2")
        })
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .add_slice(s2)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);

    assert!(results.contains_key("s1"));
    assert!(results.contains_key("s2"));

    let s1_results = results.get("s1").unwrap().as_ref().unwrap();
    assert_eq!(s1_results.method_results.len(), 2);

    let s2_results = results.get("s2").unwrap().as_ref().unwrap();
    assert_eq!(s2_results.method_results.len(), 2);
}

#[test]
fn results_keyed_by_layer_and_method() {
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(|_args, _ctx| Ok(value!({ "result": "data" })))
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| methods.call_default("m1"))
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .build()
        .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_result = results.get("s1").unwrap().as_ref().unwrap();

    let key = ("l1".to_string(), "m1".to_string());
    assert!(slice_result.method_results.contains_key(&key));
}

#[test]
fn slices_run_in_parallel() {
    let concurrent = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let c = concurrent.clone();
    let mc = max_concurrent.clone();
    let l1 = Layer::builder("l1")
        .method("m1")
        .args::<Value>()
        .bind(move |_args, _ctx| {
            let current = c.fetch_add(1, Ordering::SeqCst) + 1;
            mc.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
            c.fetch_sub(1, Ordering::SeqCst);
            Ok(value!({}))
        })
        .build();

    let s1 = Slice::builder("s1")
        .layer("l1", |methods| methods.call_default("m1"))
        .build();
    let s2 = Slice::builder("s2")
        .layer("l1", |methods| methods.call_default("m1"))
        .build();
    let s3 = Slice::builder("s3")
        .layer("l1", |methods| methods.call_default("m1"))
        .build();

    let engine = Engine::builder()
        .add_layer(l1)
        .add_slice(s1)
        .add_slice(s2)
        .add_slice(s3)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert!(
        max_concurrent.load(Ordering::SeqCst) >= 2,
        "Expected concurrent slice execution"
    );
}

#[test]
fn args_and_context_together() {
    let layer1 = quick_layer!("layer1", "process", Value, |args, ctx| {
        let input = args.get("input").unwrap().as_i64().unwrap();
        let result = input * 2;
        ctx.set("doubled", Value::from(result));
        Ok(value!({ "doubled": result }))
    });

    let layer2 = quick_layer!("layer2", "add", Value, |args, ctx| {
        let doubled: i64 = ctx.get_as("doubled").unwrap();
        let add_value = args.get("add").unwrap().as_i64().unwrap();
        let result = doubled + add_value;
        Ok(value!({ "final": result }))
    });

    let slice = Slice::builder("test")
        .layer("layer1", |m| m.call("process", value!({ "input": 5 })))
        .layer("layer2", |m| m.call("add", value!({ "add": 3 })))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), layer1, layer2),
        "layer2" => ["layer1"]
    )
    .add_slice(slice)
    .build()
    .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("test").unwrap().as_ref().unwrap();

    let layer1_result = slice_results
        .method_results
        .get(&("layer1".to_string(), "process".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(layer1_result.get("doubled").unwrap().as_i64().unwrap(), 10);

    let layer2_result = slice_results
        .method_results
        .get(&("layer2".to_string(), "add".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(layer2_result.get("final").unwrap().as_i64().unwrap(), 13);
}

#[test]
fn pipeline_with_args_and_context() {
    let extract = quick_layer!("extract", "load", Value, |args, ctx| {
        let id = args.get("id").unwrap().as_i64().unwrap();
        let data = format!("data_{}", id);
        ctx.set("raw", Value::from(data));
        Ok(value!({}))
    });

    let transform = quick_layer!("transform", "process", Value, |args, ctx| {
        let raw: String = ctx.get_as("raw").unwrap();
        let uppercase = args.get("uppercase").unwrap().as_bool().unwrap();

        let transformed = if uppercase {
            raw.to_uppercase()
        } else {
            raw.to_lowercase()
        };

        ctx.set("transformed", Value::from(transformed));
        Ok(value!({}))
    });

    let load = quick_layer!("load", "save", Value, |args, ctx| {
        let data: String = ctx.get_as("transformed").unwrap();
        let prefix = args.get("prefix").unwrap().as_str().unwrap();
        let final_data = format!("{}{}", prefix, data);

        Ok(value!({ "saved": final_data }))
    });

    let slice = Slice::builder("etl")
        .layer("extract", |m| m.call("load", value!({ "id": 42 })))
        .layer("transform", |m| {
            m.call("process", value!({ "uppercase": true }))
        })
        .layer("load", |m| m.call("save", value!({ "prefix": "OUTPUT_" })))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), extract, transform, load),
        "transform" => ["extract"],
        "load" => ["transform"]
    )
    .add_slice(slice)
    .build()
    .unwrap();

    let results = engine.run(RunFlags::SILENT);
    let slice_results = results.get("etl").unwrap().as_ref().unwrap();
    let result = slice_results
        .method_results
        .get(&("load".to_string(), "save".to_string()))
        .unwrap()
        .as_ref()
        .unwrap();

    assert_eq!(
        result.get("saved").unwrap().as_str().unwrap(),
        "OUTPUT_DATA_42"
    );
}

#[test]
fn init_layer() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    let o1 = execution_order.clone();
    let init = quick_layer!("init", "setup", Value, move |_args, ctx| {
        o1.lock().unwrap().push("init");
        ctx.set("initialized", Value::from(true));
        Ok(value!({}))
    });

    let o2 = execution_order.clone();
    let work = quick_layer!("work", "process", Value, move |_args, ctx| {
        assert!(ctx.get_as::<bool>("initialized").unwrap());
        o2.lock().unwrap().push("work");
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("init", |m| m.call_default("setup"))
        .layer("work", |m| m.call_default("process"))
        .build();

    let engine = Engine::builder()
        .add_layer(init)
        .add_layer(work)
        .init_layer("init")
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    let order = execution_order.lock().unwrap();
    assert_eq!(*order, vec!["init", "work"]);
}

#[test]
fn init_layer_sets_context() {
    let init = quick_layer!("init", "setup", Value, |_args, ctx| {
        ctx.set("config", Value::from("test"));
        ctx.set("version", Value::from(1));
        Ok(value!({}))
    });

    let verify = quick_layer!("verify", "check", Value, |_args, ctx| {
        let config: String = ctx.get_as("config")?;
        let version: i64 = ctx.get_as("version")?;
        assert_eq!(config, "test");
        assert_eq!(version, 1);
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("init", |m| m.call_default("setup"))
        .layer("verify", |m| m.call_default("check"))
        .build();

    let engine = Engine::builder()
        .add_layer(init)
        .add_layer(verify)
        .init_layer("init")
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn init_layer_with_multiple_layers() {
    let init = quick_layer!("init", "setup", Value, |_args, ctx| {
        ctx.set("ready", Value::from(true));
        Ok(value!({}))
    });

    let l1 = quick_layer!("layer1", "work", Value, |_args, ctx| {
        assert!(ctx.get_as::<bool>("ready").unwrap());
        Ok(value!({}))
    });

    let l2 = quick_layer!("layer2", "work", Value, |_args, ctx| {
        assert!(ctx.get_as::<bool>("ready").unwrap());
        Ok(value!({}))
    });

    let l3 = quick_layer!("layer3", "work", Value, |_args, ctx| {
        assert!(ctx.get_as::<bool>("ready").unwrap());
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("init", |m| m.call_default("setup"))
        .layer("layer1", |m| m.call_default("work"))
        .layer("layer2", |m| m.call_default("work"))
        .layer("layer3", |m| m.call_default("work"))
        .build();

    let engine = Engine::builder()
        .add_layer(init)
        .add_layer(l1)
        .add_layer(l2)
        .add_layer(l3)
        .init_layer("init")
        .add_slice(slice)
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
fn init_layer_with_dependencies() {
    let init = quick_layer!("init", "setup", Value, |_args, ctx| {
        ctx.set("step", Value::from(1));
        Ok(value!({}))
    });

    let l1 = quick_layer!("layer1", "work", Value, |_args, ctx| {
        let step: i64 = ctx.get_as("step")?;
        ctx.set("step", Value::from(step + 1));
        Ok(value!({}))
    });

    let l2 = quick_layer!("layer2", "work", Value, |_args, ctx| {
        let step: i64 = ctx.get_as("step")?;
        assert_eq!(step, 2);
        Ok(value!({}))
    });

    let slice = Slice::builder("test")
        .layer("init", |m| m.call_default("setup"))
        .layer("layer1", |m| m.call_default("work"))
        .layer("layer2", |m| m.call_default("work"))
        .build();

    let engine = dependencies!(
        add_layers!(Engine::builder(), init, l1, l2),
        "layer2" => ["layer1"]
    )
    .init_layer("init")
    .add_slice(slice)
    .build()
    .unwrap();

    engine.run(RunFlags::SILENT);
}

#[test]
#[should_panic(expected = "LayerNotFound")]
fn init_layer_nonexistent() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| { Ok(value!({})) });

    Engine::builder()
        .add_layer(layer)
        .init_layer("nonexistent")
        .build()
        .unwrap();
}
