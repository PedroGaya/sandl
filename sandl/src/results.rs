use crate::{Error, Result, Value};
use std::{collections::HashMap, time::Duration};

#[derive(Debug)]
pub struct SliceResults {
    pub method_results: HashMap<(String, String), Result<Value>>,
    pub duration: Duration,
}

impl SliceResults {
    pub fn new() -> Self {
        Self {
            method_results: HashMap::new(),
            duration: Duration::ZERO,
        }
    }

    pub fn add_result(&mut self, layer: String, method: String, result: Result<Value>) {
        self.method_results.insert((layer, method), result);
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }
}

pub type RunResults = HashMap<String, Result<SliceResults>>;

pub trait RunResultsExt {
    fn total_slices(&self) -> usize;
    fn successful_slices(&self) -> usize;
    fn failed_slices(&self) -> usize;

    fn total_methods(&self) -> usize;
    fn successful_methods(&self) -> usize;
    fn failed_methods(&self) -> usize;

    fn is_all_success(&self) -> bool;
    fn has_failures(&self) -> bool;
    fn summary(&self) -> String;

    fn get_slice_errors(&self) -> Vec<(&String, &Error)>;
    fn get_all_method_errors(&self) -> Vec<(&String, &String, &String, &Error)>;
    fn get_execution_errors(&self) -> Vec<(&String, &String, &String, &Error)>;

    fn from_slice(&self, slice_name: &str) -> Option<&Result<SliceResults>>;
    fn slice_names(&self) -> Vec<&String>;

    fn average_slice_duration(&self) -> Option<Duration>;
    fn min_slice_duration(&self) -> Option<Duration>;
    fn max_slice_duration(&self) -> Option<Duration>;
    fn timing_summary(&self) -> String;
}

impl RunResultsExt for RunResults {
    fn total_slices(&self) -> usize {
        self.len()
    }

    fn successful_slices(&self) -> usize {
        self.values().filter(|result| result.is_ok()).count()
    }

    fn failed_slices(&self) -> usize {
        self.values().filter(|result| result.is_err()).count()
    }

    fn total_methods(&self) -> usize {
        self.values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| slice_results.method_results.len())
            .sum()
    }

    fn successful_methods(&self) -> usize {
        self.values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| {
                slice_results
                    .method_results
                    .values()
                    .filter(|result| result.is_ok())
                    .count()
            })
            .sum()
    }

    fn failed_methods(&self) -> usize {
        self.values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| {
                slice_results
                    .method_results
                    .values()
                    .filter(|result| result.is_err())
                    .count()
            })
            .sum()
    }

    fn is_all_success(&self) -> bool {
        self.successful_slices() == self.total_slices()
            && self.successful_methods() == self.total_methods()
    }

    fn has_failures(&self) -> bool {
        self.failed_slices() > 0 || self.failed_methods() > 0
    }

    fn summary(&self) -> String {
        let total_slices = self.total_slices();
        let successful_slices = self.successful_slices();
        let total_methods = self.total_methods();
        let successful_methods = self.successful_methods();

        format!(
            "Slices: {}/{} succeeded, Methods: {}/{} succeeded",
            successful_slices, total_slices, successful_methods, total_methods
        )
    }

    fn get_slice_errors(&self) -> Vec<(&String, &Error)> {
        self.iter()
            .filter_map(|(slice_name, result)| result.as_ref().err().map(|e| (slice_name, e)))
            .collect()
    }

    fn get_all_method_errors(&self) -> Vec<(&String, &String, &String, &Error)> {
        let mut errors = Vec::new();

        for (slice_name, slice_result) in self {
            if let Ok(slice_results) = slice_result {
                for ((layer, method), method_result) in &slice_results.method_results {
                    if let Err(e) = method_result {
                        errors.push((slice_name, layer, method, e));
                    }
                }
            }
        }

        errors
    }

    fn from_slice(&self, slice_name: &str) -> Option<&Result<SliceResults>> {
        self.get(slice_name)
    }

    fn slice_names(&self) -> Vec<&String> {
        self.keys().collect()
    }

    fn get_execution_errors(&self) -> Vec<(&String, &String, &String, &Error)> {
        self.get_all_method_errors()
            .into_iter()
            .filter(|(_, _, _, error)| error.is_execution_error())
            .collect()
    }

    fn average_slice_duration(&self) -> Option<Duration> {
        let durations: Vec<Duration> = self
            .values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| slice_results.duration)
            .collect();

        if durations.is_empty() {
            return None;
        }

        let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
        let avg_nanos = total_nanos / durations.len() as u128;

        Some(Duration::from_nanos(avg_nanos as u64))
    }

    fn min_slice_duration(&self) -> Option<Duration> {
        self.values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| slice_results.duration)
            .min()
    }

    fn max_slice_duration(&self) -> Option<Duration> {
        self.values()
            .filter_map(|result| result.as_ref().ok())
            .map(|slice_results| slice_results.duration)
            .max()
    }

    fn timing_summary(&self) -> String {
        let avg = self.average_slice_duration();
        let min = self.min_slice_duration();
        let max = self.max_slice_duration();

        format!(
            "Avg: {:?}, Min: {:?}, Max: {:?}",
            avg.unwrap_or(Duration::ZERO),
            min.unwrap_or(Duration::ZERO),
            max.unwrap_or(Duration::ZERO)
        )
    }
}
