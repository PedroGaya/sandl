use sandl::*;

#[derive(Debug, Args)]
struct WorkArgsA {
    gravity: f64,
    friction: f64,
}

#[derive(Debug, Args)]
struct WorkArgsB {
    color: String,
    size: i32,
}

fn do_work_for(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

fn main() -> Result<()> {
    let layer_a = Layer::builder("Layer_A")
        .method("Work_A")
        .args::<WorkArgsA>()
        .bind(|args, _ctx| {
            do_work_for(50);
            Ok(args.to_value())
        })
        .build();

    let layer_b = Layer::builder("Layer_B")
        .method("Work_B")
        .args_with_default::<WorkArgsB>(WorkArgsB {
            color: "red".to_string(),
            size: 10,
        })
        .bind(|args, _ctx| Ok(args.to_value()))
        .method("Work_C")
        .args::<WorkArgsB>()
        .bind(|args, _ctx| Ok(args.to_value()))
        .build();

    let mut slices: Vec<Slice> = vec![];

    for i in 1..100 {
        let slice_name = format!("Slice_{}", i);
        let slice = Slice::builder(slice_name)
            .layer("Layer_A", |methods| {
                methods.call(
                    "Work_A",
                    WorkArgsA {
                        gravity: 9.8,
                        friction: 0.1,
                    },
                )
            })
            .layer("Layer_B", |methods| {
                methods.call_default("Work_B").call(
                    "Work_C",
                    WorkArgsB {
                        color: "Reusing my args type!".to_string(),
                        size: i,
                    },
                )
            })
            .build();

        slices.push(slice);
    }

    let engine = Engine::builder()
        .add_layer(layer_a)
        .add_layer(layer_b)
        .dependency("Layer_B", "Layer_A")
        .add_slices(&mut slices)
        .build()?;

    let results = engine.run(RunFlags::default());

    let summary = results.summary();
    println!("{:?}", summary);

    Ok(())
}
