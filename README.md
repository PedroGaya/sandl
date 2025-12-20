# sandl

A Rust framework for building parallel execution engines with dependency management, type-safe method dispatch, and event observation.

## Installation

Run the following Cargo command in your project directory:

`cargo add sandl`

Or add the following line to your Cargo.toml:

`sandl = "0.1.0"`

## Quick Start

```rust
use sandl::*;

fn main() -> Result<()> {
    // Define a computational layer with a method
    let compute = Layer::builder("L_compute")
        .method("M_double")
        .args::<i64>()
        .bind(|&x, _ctx| Ok(value!(x * 2)))
        .build();

    // Creates slices that call methods with some args
    let mut slices: Vec<Slice> = vec![];
    for i in 0..1000000 {
        let slice_name = format!("task_{}", i);

        let slice = Slice::builder(slice_name)
            .layer("L_compute", |m| m.call("M_double", i))
            .build();

        slices.push(slice);
    }

    // Configure the engine
    let config = EngineConfig::new().batch_size(1000);

    // Build it
    let engine = Engine::builder()
        .add_layer(compute)
        .add_slices(&mut slices)
        .config(config)
        .build()?;

    // Run with flags of choice
    let results = engine.run(RunFlags::default());
    Ok(())
}
```

## Overview

**sandl** enables you to define reusable computational **Layers** with typed methods, compose them into execution **Slices**, and run them in parallel with automatic dependency resolution. It's designed for scenarios where you need to:

- Execute the same operations across many similar workloads in parallel
- Manage complex dependencies between computation stages
- Maintain type safety across dynamic execution boundaries
- Observe and monitor execution progress

Some use cases:

- Data pipeline processing
- Parallel API request handling
- Batch image processing
- Monte-Carlo simulations
- Generic ETL operations

## Key Concepts

### Layers

A **Layer** is a collection of independent methods that perform related operations. Each method has:

- A name
- Type-safe arguments (via the `FromValue`/`ToValue` traits)
- Optional default arguments
- An implementation function

```rust
#[derive(Args)]
struct TransformArgs {
    value: f64
}

let process_layer = Layer::builder("process")
    .method("transform")
    // The type is optional
    .args_with_default<TransformArgs>(value!({ "value": 16 })) // Defaults are optional, just call args()
    .bind(|args, _ctx| {
        let x = args.value * 2; // Type-safe!
        Ok(value!({ "value": x })) // { "value": 32 }
    })
    .build();
```

Slices have access to a thread-safe get/set context by default. For methods that don't need it, use `bind_pure`:

```rust
let process_layer = Layer::builder("process")
    .method("transform")
    // The type is optional
    .args_with_default<TransformArgs>(value!({ "value": 16 }))
    .bind_pure(|args| { // Transform is a pure function!
        let x = args.value * 2;
        Ok(value!({ "value": x }))
    })
    .build();
```

### Slices

A **Slice** specifies which layer methods to execute and with what arguments. Slices are the units of work that get executed in parallel:

```rust
let slice = Slice::builder("slice")
    .layer("process", |methods| {
        methods.call("transform", value!({ "value": 64 })) // On the "process" layer, call "transform" with these args.
    }) // If you have more than one layer and/or method,
       // slices will only interact with the ones you configure them to.
    .build();
```

Default arguments can be overridden at the slice level with automatic merging for object types:

```rust
// Layer defines defaults
.args_with_default(value!({ "timeout": 30, "retries": 3 }))

// Slice overrides one field
.call("fetch", value!({ "retries": 5 }))

// Merged: { "timeout": 30, "retries": 5 }
```

### Engine

The **Engine** orchestrates execution:

- Registers layers and slices
- Manages layer dependencies
- Executes slices in parallel using rayon
- Performs topological sorting for proper execution order
- Provides event observation hooks

```rust
let engine = Engine::builder()
    .add_layer(process_layer)
    .add_slice(slice1)
    .add_slice(slice2)
    .build()?;

let results = engine.run(RunFlags::default());
```

**sandl** relies on **rayon** for its parallelism, whose parameters can be configured with **EngineConfig**:

```rust
let config = EngineConfig::new()
    // Rayon configuration
    .num_threads(4)
    .stack_size(MiB!(2)) // Expands to 2 * 1024 * 1024
    .chunk_size(100)
    // sandl specific. Set this to enable batching
    .batch_size(1000);

let engine = Engine::builder()
    .config(config)
    .build()?;

let results = engine.run(RunFlags::default());
```

You can also pass run flags to the engine:

```rust
    let results = engine.run(RunFlags::TRACKED); // Default. Tracks runtime data and prints to stdout.
    let results = engine.run(RunFlags::SILENT); // Prints nothing (still tracks timing data).
    let results = engine.run(RunFlags::SILENT_NO_OBSERVER); // Minimal overhead - Nothing but runtime.
    // run() still returns RunResults in all cases, so you can do your own processing *after* it's done.
```

Overhead from stdout writes and from observer hooks is minimal, but it exists.

### Context

A **Context** provides thread-safe, per-slice shared state during execution. Methods can read from and write to the context:

```rust
.bind(|args, ctx| {
    ctx.set("result", value!(42));
    let prev = ctx.get("result");
    Ok(value!(null))
})
```

**Beware the shared state**. Methods within a slice run in parallel, so all behavior is undefined by default. You can set dependencies amongst layers in the engine builder:

```rust
// Use the builder...
let engine = Engine::builder()
    .add_layer(process_layer)
    .add_layer(show_layer)
    .dependency("show", "process") // show depends on process
    .add_slice(slice1)
    .add_slice(slice2)
    .build()?;

// ...or macros
let engine = dependencies!(
        add_layers!(Engine::builder(), layer1, layer2, layer3, layer4),
        "layer2" => ["layer1"], // layer => depends_on[layers]
        "layer3" => ["layer2", "layer4"]
    )
    .add_slice(slice)
    .build()?;
```

You can also set an initialization layer - All layers will depend on it:

```rust
// Here, we're using the quick_layer! macro to create a layer with a single method and some arg type.
let init = quick_layer!("init", "setup", Value, |_args, ctx| {
    ctx.set("config", Value::from("test")); // Values set in the init layer...
    ctx.set("version", Value::from(1));
    Ok(value!({}))
});

let verify = quick_layer!("verify", "check", Value, |_args, ctx| {
    let config: String = ctx.get_as("config")?; // ...can be acessed in other layers, safely!
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
    .init_layer("init") // Every other layer now depends on "init"
    .add_slice(slice)
    .build()?;
```

### Observer

You can inspect the runtime by creating an observer:

```rust
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
// Hook into failures
observer.on_method_failed(|slice_name, layer, method, error| {
    println!("  {}.{}.{} failed: {}", slice_name, layer, method, error);
});

// Define a simple computational layer
let compute = Layer::builder("calculator")
    .method("divide")
    .args::<CalculatorArgs>() // {x: i32, y: i32}, derives Args
    .bind(|args, _ctx| {
        if args.y == 0 {
            Err(execution_error!("Division by zero")) // sandl handles failure gracefully
        } else {
            Ok(value!(args.x / args.y))
        }
    })
    .build();

let mut slices = vec![
    // Successful slices
    Slice::builder("slice_1")
        .layer("calculator", |m| {
            m.call("divide", CalculatorArgs { x: 6, y: 7 })
        })
        .build()
];

let engine = Engine::builder()
    .add_layer(compute)
    .add_slices(&mut slices)
    .observer(observer) // Set the observer
    .build()?;

 // Inspect errors with one of the many helper methods
let failures = results.get_all_method_errors();
```

## Helper Macros

### `quick_layer!`

Quickly define a single-method layer:

```rust
let init = quick_layer!("init", "setup", Value, |_args, ctx| {
        ctx.set("ready", Value::from(true));
        Ok(value!({}))
});
```

### `value!`

Construct `Value` instances easily:

```rust
let v = value!({ "name": "Alice", "age": 30, "scores": [85, 90, 95] });
```

sandl `Value` is fully compatible with `serde_json::Value`.

### `add_slices!` / `add_layers!`

Fluently add multiple items:

```rust
let builder = add_slices!(Engine::builder(), slice1, slice2, slice3);
let builder = add_layers!(Engine::builder(), layer1, layer2);
```

### `dependencies!`

Define multiple dependencies concisely:

```rust
let builder = dependencies!(
        Engine::builder(),
        "layer2" => ["layer1"]
);
```

### `json_wrapper!`

When using serde, wrap types with a sandl compatibility layer:

```rust
json_wrapper!(pub WrapperType, HashMap<String, CustomType>); // pub is optional!
```

### `execution_error!`

Inside a method, quickly return an error:

```rust
.args::<CalculatorArgs>()
.bind(|args, _ctx| {
    if args.y == 0 {
        Err(execution_error!("Division by zero"))
    } else {
        Ok(value!(args.x / args.y))
    }
})
```

This error is automatically wrapped with runtime context:

```rust
MethodExecutionFailed {
    slice: String,
    layer: String,
    method: String,
    args: Value,
    cause: Box<Error>,
},
```

### `KiB!` / `MiB!` / `GiB!`

Useful when configuring stack_size for rayon:

```rust
let two_kilobytes = KiB!(2);
let two_megabytes = MiB!(2);
let two_gigabytes = GiB!(2);
```

## Result Analysis

The `RunResults` type provides rich analysis:

```rust
let results = engine.run(RunFlags::default());

println!("Total slices: {}", results.total_slices());
println!("Successful: {}", results.successful_slices());
println!("Failed methods: {}", results.failed_methods());

// fn total_slices(&self) -> usize;
// fn successful_slices(&self) -> usize;
// fn failed_slices(&self) -> usize;
// fn total_methods(&self) -> usize;
// fn successful_methods(&self) -> usize;
// fn failed_methods(&self) -> usize;
// fn is_all_success(&self) -> bool;
// fn has_failures(&self) -> bool;
// fn summary(&self) -> String;
// fn get_slice_errors(&self) -> Vec<(&String, &Error)>;
// fn get_all_method_errors(&self) -> Vec<(&String, &String, &String, &Error)>;
// fn get_execution_errors(&self) -> Vec<(&String, &String, &String, &Error)>;
// fn from_slice(&self, slice_name: &str) -> Option<&Result<SliceResults>>;
// fn slice_names(&self) -> Vec<&String>;
// fn average_slice_duration(&self) -> Option<Duration>;
// fn min_slice_duration(&self) -> Option<Duration>;
// fn max_slice_duration(&self) -> Option<Duration>;
// fn timing_summary(&self) -> String;

if results.has_failures() {
    for (slice, layer, method, error) in results.get_execution_errors() {
        eprintln!("Error in {}.{}.{}: {}", slice, layer, method, error);
    }
}

// Get results for a specific slice
if let Some(slice_result) = results.from_slice("slice1") {
    // Process slice-specific results
}
```

Slice results contain whatever was returned from each method that was run, as well as how long it took to run the whole slice:

```rust
pub struct SliceResults {
    // HashMap<(layer, method), Result>
    pub method_results: HashMap<(String, String), Result<Value>>,
    pub duration: Duration,
}
```

## Performance

sandl adds minimal overhead over rayon. For maximum performance:

- Use `RunFlags::SILENT_NO_OBSERVER`
- Consider larger batch sizes and smaller chunks for 10ms~ workloads
- Limit stack size per worker thread

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

MIT OR Apache-2.0
