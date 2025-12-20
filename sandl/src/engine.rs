use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use crate::tracker::ProgressTracker;
use crate::*;

pub struct Engine {
    slices: Vec<Slice>,
    layers: HashMap<String, Layer>,
    dependencies: HashMap<String, Vec<String>>,
    init_layer: Option<String>,
    observer: Observer,
    pub config: EngineConfig,
    pub flags: RunFlags,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            slices: Vec::new(),
            layers: HashMap::new(),
            dependencies: HashMap::new(),
            init_layer: None,
            observer: Observer::new(),
            config: EngineConfig::new(),
            flags: RunFlags::new(),
        }
    }

    fn topological_sort(&self) -> crate::Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for layer_name in self.layers.keys() {
            in_degree.insert(layer_name.clone(), 0);
            graph.insert(layer_name.clone(), Vec::new());
        }

        for (layer, deps) in &self.dependencies {
            *in_degree.get_mut(layer).unwrap() = deps.len();
            for dep in deps {
                if !self.layers.contains_key(dep) {
                    return Err(crate::Error::LayerNotFound(dep.clone()));
                }
                graph.get_mut(dep).unwrap().push(layer.clone());
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        if result.len() != self.layers.len() {
            return Err(crate::Error::ConfigError(
                "Circular dependency detected in layers".to_string(),
            ));
        }

        if let Some(init_name) = &self.init_layer {
            result.retain(|name| name != init_name);
            result.insert(0, init_name.clone());
        }

        Ok(result)
    }

    fn compute_method_waves(
        &self,
        slice: &Slice,
        execution_order: &[String],
    ) -> crate::Result<Vec<Vec<(String, String)>>> {
        let mut waves: Vec<Vec<(String, String)>> = Vec::new();
        let mut remaining_layers: HashSet<String> = execution_order
            .iter()
            .filter(|layer| slice.has_layer(layer))
            .cloned()
            .collect();
        let mut completed_layers: HashSet<String> = HashSet::new();

        while !remaining_layers.is_empty() {
            let mut current_wave = Vec::new();

            for layer_name in &remaining_layers.clone() {
                let deps = self.dependencies.get(layer_name);
                let deps_satisfied = deps
                    .map(|d| d.iter().all(|dep| completed_layers.contains(dep)))
                    .unwrap_or(true);

                if deps_satisfied {
                    if let Ok(methods) = slice.get_layer_methods(layer_name) {
                        for method_name in methods {
                            current_wave.push((layer_name.clone(), method_name.to_string()));
                        }
                    }
                }
            }

            if current_wave.is_empty() {
                return Err(crate::Error::ConfigError(
                    "Unable to compute method waves".to_string(),
                ));
            }

            let wave_layers: HashSet<String> = current_wave
                .iter()
                .map(|(layer, _)| layer.clone())
                .collect();

            for layer in &wave_layers {
                remaining_layers.remove(layer);
                completed_layers.insert(layer.clone());
            }

            waves.push(current_wave);
        }

        Ok(waves)
    }

    fn execute_slice(
        &self,
        slice: &Slice,
        execution_order: &[String],
        use_observer: bool,
    ) -> Result<SliceResults> {
        use rayon::prelude::*;

        let slice_name = slice.get_name().to_string();
        let slice_start = Instant::now();

        if use_observer {
            self.observer.emit(EngineEvent::SliceStart {
                slice: slice_name.clone(),
            });
        }

        let waves = self.compute_method_waves(slice, execution_order)?;
        let mut results = SliceResults::new();

        let context = Context::new();

        for wave in waves {
            let wave_results: Vec<((String, String), Result<Value>)> = wave
                .par_iter()
                .map(|(layer_name, method_name)| {
                    let result = if use_observer {
                        self.observe_execute_method(slice, layer_name, method_name, &context)
                    } else {
                        self.execute_method(slice, layer_name, method_name, &context)
                    };

                    ((layer_name.clone(), method_name.clone()), result)
                })
                .collect();

            for ((layer_name, method_name), result) in wave_results {
                results.add_result(layer_name, method_name, result);
            }
        }

        if use_observer {
            let duration = slice_start.elapsed();
            results.set_duration(duration);

            self.observer.emit(EngineEvent::SliceComplete {
                slice: slice_name,
                duration: duration,
            });
        }

        Ok(results)
    }

    fn observe_execute_method(
        &self,
        slice: &Slice,
        layer_name: &str,
        method_name: &str,
        ctx: &Context,
    ) -> Result<Value> {
        let start = Instant::now();
        let slice_name = &slice.name;

        self.observer.emit(EngineEvent::MethodStart {
            slice: slice_name.to_string(),
            layer: layer_name.to_string(),
            method: method_name.to_string(),
        });

        let result = self.execute_method(slice, layer_name, method_name, &ctx);

        let result = result.map_err(|e| {
            let args = slice
                .get_method_arg(layer_name, method_name)
                .unwrap_or(&Value::Null);

            // If it's already a MethodExecutionFailed, don't double-wrap
            if e.is_execution_error() {
                e
            } else {
                e.with_context(slice_name, layer_name, method_name, args.clone())
            }
        });

        match &result {
            Ok(_) => {
                self.observer.emit(EngineEvent::MethodComplete {
                    slice: slice_name.to_string(),
                    layer: layer_name.to_string(),
                    method: method_name.to_string(),
                    duration: start.elapsed(),
                });
            }
            Err(e) => {
                self.observer.emit(EngineEvent::MethodFailed {
                    slice: slice_name.to_string(),
                    layer: layer_name.to_string(),
                    method: method_name.to_string(),
                    error: e.to_string(),
                });
            }
        }
        result
    }

    fn execute_method(
        &self,
        slice: &Slice,
        layer_name: &str,
        method_name: &str,
        ctx: &Context,
    ) -> Result<Value> {
        let layer = self
            .layers
            .get(layer_name)
            .ok_or_else(|| crate::Error::LayerNotFound(layer_name.to_string()))?;

        let slice_args = slice.get_method_arg(layer_name, method_name)?;

        if slice_args.is_null() {
            layer.execute_with_default(method_name, ctx)
        } else {
            let merged_args = if let Some(default_args) = layer.get_default_args(method_name) {
                Self::merge_args(default_args, slice_args)
            } else {
                slice_args.clone()
            };

            layer.execute(method_name, &merged_args, ctx)
        }
    }

    fn merge_args(defaults: &Value, overrides: &Value) -> Value {
        match (defaults, overrides) {
            (Value::Object(def_map), Value::Object(over_map)) => {
                let mut merged = def_map.clone();
                for (k, v) in over_map {
                    merged.insert(k.clone(), v.clone());
                }
                Value::Object(merged)
            }
            (_, Value::Null) => defaults.clone(),
            _ => overrides.clone(), // If override is not an object, just use it entirely
        }
    }

    pub fn run(&self, flags: RunFlags) -> RunResults {
        if flags.silent {
            self.run_silent(flags.with_observer)
        } else {
            self.run_with_progress(flags.with_observer)
        }
    }

    fn run_silent(&self, use_observer: bool) -> RunResults {
        let pool = self.config.build_thread_pool().ok();
        let execution_order = match self.topological_sort() {
            Ok(order) => order,
            Err(e) => panic!("Engine misconfigured: {}", e),
        };

        // Check if we need batched execution (for memory management)
        let intermediary = if let Some(batch_size) = self.config.batch_size {
            // Process in batches to prevent memory exhaustion
            let mut all_results = HashMap::new();

            for batch in self.slices.chunks(batch_size) {
                let batch_results =
                    self.execute_batch_silent(batch, &execution_order, &pool, use_observer);
                all_results.extend(batch_results);
            }

            all_results
        } else {
            // Process all slices at once
            self.execute_batch_silent(&self.slices, &execution_order, &pool, use_observer)
        };

        RunResults::from(intermediary)
    }

    fn run_with_progress(&self, use_observer: bool) -> RunResults {
        let execution_order = match self.topological_sort() {
            Ok(order) => order,
            Err(e) => panic!("Engine misconfigured: {}", e),
        };

        let pool = self.config.build_thread_pool().ok();
        let tracker = Arc::new(ProgressTracker::new(self.slices.len()));
        tracker.print_header();

        // Check if we need batched execution (for memory management)
        let intermediary = if let Some(batch_size) = self.config.batch_size {
            // Process in batches with progress tracking
            let mut all_results = HashMap::new();

            for batch in self.slices.chunks(batch_size) {
                let batch_results = self.execute_batch_with_progress(
                    batch,
                    &execution_order,
                    &pool,
                    &tracker,
                    use_observer,
                );
                all_results.extend(batch_results);
            }

            all_results
        } else {
            // Process all slices at once with progress
            self.execute_batch_with_progress(
                &self.slices,
                &execution_order,
                &pool,
                &tracker,
                use_observer,
            )
        };

        let results = RunResults::from(intermediary);
        tracker.print_summary(&results);
        results
    }

    fn execute_batch_silent(
        &self,
        slices: &[Slice],
        execution_order: &[String],
        pool: &Option<rayon::ThreadPool>,
        use_observer: bool,
    ) -> HashMap<String, Result<SliceResults>> {
        use rayon::prelude::*;

        let chunk_size = self.config.chunk_size;

        let execute = || {
            if chunk_size > 1 {
                // Use chunking to reduce thread coordination overhead
                slices
                    .par_chunks(chunk_size)
                    .flat_map(|chunk| {
                        chunk
                            .iter()
                            .map(|slice| {
                                let slice_name = slice.get_name().to_string();
                                let result =
                                    self.execute_slice(slice, execution_order, use_observer);
                                (slice_name, result)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            } else {
                // No chunking - one item per coordination
                slices
                    .par_iter()
                    .map(|slice| {
                        let slice_name = slice.get_name().to_string();
                        let result = self.execute_slice(slice, execution_order, use_observer);
                        (slice_name, result)
                    })
                    .collect()
            }
        };

        if let Some(pool) = pool {
            pool.install(execute)
        } else {
            execute()
        }
    }

    fn execute_batch_with_progress(
        &self,
        slices: &[Slice],
        execution_order: &[String],
        pool: &Option<rayon::ThreadPool>,
        tracker: &Arc<ProgressTracker>,
        use_observer: bool,
    ) -> HashMap<String, Result<SliceResults>> {
        use rayon::prelude::*;

        let chunk_size = self.config.chunk_size;

        let execute = || {
            if chunk_size > 1 {
                // Use chunking to reduce thread coordination overhead
                slices
                    .par_chunks(chunk_size)
                    .flat_map(|chunk| {
                        chunk
                            .iter()
                            .map(|slice| {
                                let slice_name = slice.get_name().to_string();
                                let result =
                                    self.execute_slice(slice, execution_order, use_observer);

                                // Update progress if observer is enabled
                                if use_observer {
                                    match &result {
                                        Ok(_) => tracker.increment_completed(),
                                        Err(_) => tracker.increment_failed(),
                                    }
                                }

                                (slice_name, result)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            } else {
                // No chunking - one item per coordination
                slices
                    .par_iter()
                    .map(|slice| {
                        let slice_name = slice.get_name().to_string();
                        let result = self.execute_slice(slice, execution_order, use_observer);

                        // Update progress if observer is enabled
                        if use_observer {
                            match &result {
                                Ok(_) => tracker.increment_completed(),
                                Err(_) => tracker.increment_failed(),
                            }
                        }

                        (slice_name, result)
                    })
                    .collect()
            }
        };

        if let Some(pool) = pool {
            pool.install(execute)
        } else {
            execute()
        }
    }

    pub fn set_observer(&mut self, observer: Observer) {
        self.observer = observer;
    }

    pub fn observer_mut(&mut self) -> &mut Observer {
        &mut self.observer
    }

    pub fn set_init_layer(&mut self, layer_name: &str) -> crate::Result<()> {
        if !self.layers.contains_key(layer_name) {
            return Err(crate::Error::LayerNotFound(layer_name.to_string()));
        }

        self.init_layer = Some(layer_name.to_string());
        Ok(())
    }

    pub fn add_dependency(&mut self, layer: &str, depends_on: &str) -> crate::Result<()> {
        self.dependencies
            .entry(layer.to_string())
            .or_insert_with(Vec::new)
            .push(depends_on.to_string());
        Ok(())
    }

    pub fn register_slice(&mut self, slice: Slice) {
        self.slices.push(slice);
    }

    pub fn register_layer(&mut self, layer: Layer) -> crate::Result<()> {
        let name = layer.get_name().to_string();
        if self.layers.contains_key(&name) {
            return Err(crate::Error::LayerAlreadyExists(name));
        }
        self.layers.insert(name, layer);
        Ok(())
    }

    pub fn get_layer_names(&self) -> Vec<String> {
        self.layers.keys().map(|s| s.to_string()).collect()
    }

    pub fn get_slice_names(&self) -> Vec<String> {
        self.slices
            .iter()
            .map(|s| s.get_name().to_string())
            .collect()
    }

    pub fn get_dependencies(&self, layer: &str) -> Option<&Vec<String>> {
        self.dependencies.get(layer)
    }
}
