use sandl::*;

fn main() -> Result<()> {
    // Define the process layer with transform method
    let process_layer: Layer = Layer::builder("process")
        .method("transform")
        .args_with_default(value!({ "value": 16 }))
        .bind(|args, _ctx| {
            let x = args.get("value").unwrap().as_i64().unwrap() * 2;
            Ok(value!({ "value": x }))
        })
        .build();

    // Define the show layer with print method
    let show_layer = Layer::builder("show")
        .method("print")
        .args::<Value>()
        .bind(|_args, _ctx| {
            println!("Print method called");
            Ok(value!(null))
        })
        .build();

    // Create slice1 using default transform args and calling print
    let slice1 = Slice::builder("slice1")
        .layer("process", |methods| methods.call_default("transform"))
        .layer("show", |methods| methods.call_default("print"))
        .build();

    // Create slice2 with explicit transform args, no show layer
    let slice2 = Slice::builder("slice2")
        .layer("process", |methods| {
            methods.call("transform", value!({ "value": 64 }))
        })
        .build();

    // Build the engine with layers, dependencies, and slices
    let engine = Engine::builder()
        .add_layer(process_layer)
        .add_layer(show_layer)
        .dependency("show", "process") // show depends on process
        .add_slice(slice1)
        .add_slice(slice2)
        .build()?;

    // Run all slices in parallel
    let results = engine.run(RunFlags::default());

    println!("{:?}", results.summary());

    Ok(())
}
