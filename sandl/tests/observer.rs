use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use sandl::*;

#[test]
fn observer_callbacks() {
    let mut observer = Observer::new();
    let events = Arc::new(Mutex::new(Vec::new()));

    let e = events.clone();
    observer.on_slice_start(move |slice| {
        e.lock().unwrap().push(format!("slice_start:{}", slice));
    });

    let e = events.clone();
    observer.on_method_start(move |slice, layer, method| {
        e.lock()
            .unwrap()
            .push(format!("method_start:{}:{}:{}", slice, layer, method));
    });

    observer.emit(EngineEvent::SliceStart {
        slice: "s1".to_string(),
    });
    observer.emit(EngineEvent::MethodStart {
        slice: "s1".to_string(),
        layer: "l1".to_string(),
        method: "m1".to_string(),
    });

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], "slice_start:s1");
    assert_eq!(recorded[1], "method_start:s1:l1:m1");
}

#[test]
fn observer_method_start() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| { Ok(value!({})) });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let called = Arc::new(AtomicUsize::new(0));
    let c = called.clone();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .observe(move |observer| {
            observer.on_method_start(move |_, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        })
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert_eq!(called.load(Ordering::SeqCst), 1);
}

#[test]
fn observer_method_complete() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| { Ok(value!({})) });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let called = Arc::new(AtomicUsize::new(0));
    let c = called.clone();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .observe(move |observer| {
            observer.on_method_complete(move |_, _, _, _| {
                println!("HEREEEEEEEEEEEEEEEEEEEEEEEEEEE");
                c.fetch_add(1, Ordering::SeqCst);
            });
        })
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert_eq!(called.load(Ordering::SeqCst), 1);
}

#[test]
fn observer_method_failed() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| {
        Err(Error::ExecutionError("test error".to_string()))
    });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let errors = Arc::new(Mutex::new(Vec::new()));
    let e = errors.clone();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .observe(move |observer| {
            observer.on_method_failed(move |slice, layer, method, error| {
                e.lock()
                    .unwrap()
                    .push(format!("{}:{}:{}:{}", slice, layer, method, error));
            });
        })
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    let errors = errors.lock().unwrap();
    assert_eq!(errors.len(), 1);
}

#[test]
fn observer_slice_lifecycle() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| { Ok(value!({})) });

    let slice = Slice::builder("test")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let events = Arc::new(Mutex::new(Vec::new()));
    let e = events.clone();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(slice)
        .observe(move |observer| {
            let e1 = e.clone();
            observer.on_slice_start(move |slice| {
                e1.lock().unwrap().push(format!("start:{}", slice));
            });

            let e2 = e.clone();
            observer.on_slice_complete(move |slice, _| {
                e2.lock().unwrap().push(format!("complete:{}", slice));
            });
        })
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "start:test");
    assert_eq!(events[1], "complete:test");
}

#[test]
fn observer_multiple_slices() {
    let layer = quick_layer!("layer", "work", Value, |_args, _ctx| { Ok(value!({})) });

    let s1 = Slice::builder("s1")
        .layer("layer", |m| m.call_default("work"))
        .build();
    let s2 = Slice::builder("s2")
        .layer("layer", |m| m.call_default("work"))
        .build();

    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();

    let engine = Engine::builder()
        .add_layer(layer)
        .add_slice(s1)
        .add_slice(s2)
        .observe(move |observer| {
            observer.on_method_complete(move |_, _, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        })
        .build()
        .unwrap();

    engine.run(RunFlags::SILENT);

    assert_eq!(count.load(Ordering::SeqCst), 2);
}
