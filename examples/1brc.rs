use sandl::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

// Arguments for processing a chunk of the file
#[derive(Debug, Args, Clone)]
struct ChunkArgs {
    file_path: String,
    start_byte: u64,
    end_byte: u64,
    chunk_id: usize,
}

// Partial statistics for one station in one chunk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StationStats {
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
}

impl StationStats {
    fn new(temp: f64) -> Self {
        Self {
            min: temp,
            max: temp,
            sum: temp,
            count: 1,
        }
    }

    fn update(&mut self, temp: f64) {
        self.min = self.min.min(temp);
        self.max = self.max.max(temp);
        self.sum += temp;
        self.count += 1;
    }

    fn merge(&mut self, other: &StationStats) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum += other.sum;
        self.count += other.count;
    }

    fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }
}

json_wrapper!(PartialResults, HashMap<String, StationStats>);

// Core processing function for each chunk
fn process_chunk(args: &ChunkArgs, _ctx: &Context) -> Result<Value> {
    let mut file =
        File::open(&args.file_path).map_err(|e| execution_error!("Failed to open file: {}", e))?;

    // Seek to our chunk's start
    file.seek(SeekFrom::Start(args.start_byte))
        .map_err(|e| execution_error!("Failed to seek: {}", e))?;

    let chunk_size = (args.end_byte - args.start_byte) as usize;
    let mut limited = file.take(chunk_size as u64);
    let mut reader = BufReader::new(&mut limited);

    let mut buffer = String::new();
    let mut stats = PartialResults::new();

    // If we're not at the file start, skip the first partial line
    if args.start_byte > 0 {
        reader
            .read_line(&mut buffer)
            .map_err(|e| execution_error!("Failed to read line: {}", e))?;
        buffer.clear();
    }

    // Process all lines in this chunk
    while reader
        .read_line(&mut buffer)
        .map_err(|e| execution_error!("Failed to read line: {}", e))?
        > 0
    {
        let line = buffer.trim();

        if let Some((station, temp_str)) = line.split_once(';') {
            if let Ok(temp) = temp_str.parse::<f64>() {
                stats
                    .entry(station.to_string())
                    .and_modify(|s| s.update(temp))
                    .or_insert_with(|| StationStats::new(temp));
            }
        }

        buffer.clear();
    }

    Ok(stats.to_value())
}

fn main() -> Result<()> {
    let file_path = "measurements.txt";

    println!("Analyzing temperature data from: {}", file_path);

    // Get file size
    let file = File::open(file_path).expect("Failed to open file");
    let file_size = file.metadata().expect("Failed to get file metadata").len();

    println!("File size: {:.2} bytes", file_size as f64);

    // Determine number of slices
    let num_slices = ((file_size / (MiB!(2))) as usize).max(100);
    let chunk_size = file_size / num_slices as u64;

    println!(
        "Splitting into {} slices of ~{:.2} MB each\n",
        num_slices,
        chunk_size as f64 / 1_048_576.0
    );

    // Create processing layer
    let process_layer = Layer::builder("process")
        .method("chunk")
        .args::<ChunkArgs>()
        .bind(process_chunk)
        .build();

    // Create slices for each chunk
    let mut slices = Vec::with_capacity(num_slices);

    for i in 0..num_slices {
        let start = i as u64 * chunk_size;
        let end = if i == num_slices - 1 {
            file_size
        } else {
            (i + 1) as u64 * chunk_size
        };

        let slice = Slice::builder(format!("chunk_{}", i))
            .layer("process", |m| {
                m.call(
                    "chunk",
                    ChunkArgs {
                        file_path: file_path.to_string(),
                        start_byte: start,
                        end_byte: end,
                        chunk_id: i,
                    },
                )
            })
            .build();

        slices.push(slice);
    }

    let config = EngineConfig::new().batch_size(500).chunk_size(10);

    // Build and run engine
    let engine = Engine::builder()
        .add_layer(process_layer)
        .add_slices(&mut slices)
        .config(config)
        .build()?;

    // Run with progress display
    let results = engine.run(RunFlags::TRACKED);

    // Merge all partial results
    let mut final_stats: HashMap<String, StationStats> = HashMap::new();

    for (_slice_name, slice_result) in results.iter() {
        if let Ok(slice_results) = slice_result {
            let key = ("process".to_string(), "chunk".to_string());
            if let Some(Ok(value)) = slice_results.method_results.get(&key) {
                let partial = PartialResults::from_value(value)?;

                for (station, stats) in partial.into_inner() {
                    final_stats
                        .entry(station)
                        .and_modify(|s| s.merge(&stats))
                        .or_insert(stats);
                }
            }
        }
    }

    // Sort stations alphabetically and print results
    let mut stations: Vec<_> = final_stats.iter().collect();
    stations.sort_by_key(|(name, _)| *name);

    println!("{}", "=".repeat(60));

    for (station, stats) in stations {
        println!(
            "{};{:.1};{:.1};{:.1}",
            station,
            stats.min,
            stats.mean(),
            stats.max
        );
    }

    Ok(())
}
