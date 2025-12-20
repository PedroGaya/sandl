use sandl::*;

fn main() -> Result<()> {
    // Create an observer with various event handlers
    let mut observer = Observer::new();

    // Track when slices start and complete
    observer.on_slice_start(|slice_name| {
        println!("Starting slice: {}", slice_name);
    });

    observer.on_slice_complete(|slice_name, duration| {
        println!("Completed slice: {} in {:?}", slice_name, duration);
    });

    // Track when methods are called
    observer.on_method_start(|slice_name, layer, method| {
        println!("  Executing: {}.{}.{}", slice_name, layer, method);
    });

    observer.on_method_complete(|slice_name, layer, method, duration| {
        println!("  {}.{}.{} took {:?}", slice_name, layer, method, duration);
    });

    // Handle failures
    observer.on_method_failed(|slice_name, layer, method, error| {
        println!("  {}.{}.{} failed: {}", slice_name, layer, method, error);
    });

    // sandl is typesafe!
    #[derive(Args)]
    struct CalculatorArgs {
        x: i32,
        y: i32,
    }

    // Define a simple computational layer
    let compute = Layer::builder("calculator")
        .method("add")
        .args::<CalculatorArgs>()
        .bind(|args, _ctx| Ok(value!(args.x + args.y)))
        .method("subtract")
        .args::<CalculatorArgs>()
        .bind(|args, _ctx| Ok(value!(args.x - args.y)))
        .method("multiply")
        .args::<CalculatorArgs>()
        .bind(|args, _ctx| Ok(value!(args.x * args.y)))
        .method("divide")
        .args::<CalculatorArgs>()
        .bind(|args, _ctx| {
            if args.y == 0 {
                Err(execution_error!("Division by zero"))
            } else {
                Ok(value!(args.x / args.y))
            }
        })
        .build();

    // Create slices that call different different methods
    // You can also ignore layers, if you have more than one
    let mut slices = vec![
        // Successful slices
        Slice::builder("slice_1")
            .layer("calculator", |m| {
                m.call("add", CalculatorArgs { x: 6, y: 7 })
                    .call("subtract", CalculatorArgs { x: 7, y: 6 })
                    .call("multiply", CalculatorArgs { x: 7, y: 6 })
                    .call("divide", CalculatorArgs { x: 7, y: 6 })
            })
            .build(),
        Slice::builder("slice_2")
            .layer("calculator", |m| {
                m.call("multiply", CalculatorArgs { x: 12, y: 2 })
            })
            .build(),
        Slice::builder("slice_3")
            .layer("calculator", |m| {
                m.call("divide", CalculatorArgs { x: 9, y: 3 })
            })
            .build(),
        // This one will fail
        Slice::builder("slice_4")
            .layer("calculator", |m| {
                m.call("divide", CalculatorArgs { x: 6, y: 0 })
            })
            .build(),
    ];

    let engine = Engine::builder()
        .add_layer(compute)
        .add_slices(&mut slices)
        .observer(observer)
        .build()?;

    // Run with without printing progress
    let results = engine.run(RunFlags::SILENT);

    println!("\nResults summary:");
    println!("{}", results.summary());

    Ok(())
}
