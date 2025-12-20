use sandl::*;

fn main() -> Result<()> {
    // Define a computational layer with a method
    let compute = Layer::builder("L_compute")
        .method("M_double")
        .args::<i64>()
        .bind(|&x, _ctx| Ok(value!(x * 2)))
        .build();

    // Creates slices that call the method with some args
    let mut slices: Vec<Slice> = vec![];
    for i in 0..1000000 {
        let slice_name = format!("task_{}", i);

        let slice = Slice::builder(slice_name)
            .layer("L_compute", |m| m.call("M_double", i))
            .build();

        slices.push(slice);
    }

    let config = EngineConfig::new().batch_size(1000);

    // Build the engine
    let engine = Engine::builder()
        .add_layer(compute)
        .add_slices(&mut slices)
        .config(config)
        .build()?;

    // Run with flags of choice
    let _results = engine.run(RunFlags::default());
    Ok(())
}
