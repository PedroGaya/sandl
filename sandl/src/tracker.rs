use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{RunResults, RunResultsExt};

pub struct ProgressTracker {
    total: usize,
    completed: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    start_time: Instant,
    last_print: Arc<Mutex<Instant>>,
    run_time: Duration,
}

impl ProgressTracker {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            last_print: Arc::new(Mutex::new(Instant::now())),
            run_time: Duration::ZERO,
        }
    }

    pub fn set_run_time(&mut self, duration: Duration) {
        self.run_time = duration
    }

    pub fn increment_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
        self.maybe_print_progress();
    }

    pub fn increment_failed(&self) {
        self.failed.fetch_add(1, Ordering::SeqCst);
        self.maybe_print_progress();
    }

    pub fn maybe_print_progress(&self) {
        let completed = self.completed.load(Ordering::SeqCst);
        let failed = self.failed.load(Ordering::SeqCst);
        let total_done = completed + failed;

        // Always print on completion
        if total_done == self.total {
            self.force_print_progress();
            return;
        }

        let should_print = {
            let mut last = self.last_print.lock().unwrap();
            let elapsed_since_print = last.elapsed().as_millis();

            // Print if 50ms has passed OR we've completed another 1%
            if elapsed_since_print >= 50
                || (total_done > 0 && total_done % (self.total / 100).max(1) == 0)
            {
                *last = Instant::now();
                true
            } else {
                false
            }
        };

        if should_print {
            self.force_print_progress();
        }
    }

    pub fn force_print_progress(&self) {
        let completed = self.completed.load(Ordering::SeqCst);
        let failed = self.failed.load(Ordering::SeqCst);
        let total_done = completed + failed;
        let percent = (total_done as f64 / self.total as f64 * 100.0) as usize;
        let elapsed = self.start_time.elapsed();

        // Clear line and print progress
        print!("\r\x1B[K"); // Clear current line
        print!(
            "Progress: [{}/{}] {}% | ✓ {} ✗ {} | {:?}",
            total_done, self.total, percent, completed, failed, elapsed
        );

        use std::io::Write;
        let _ = std::io::stdout().flush();

        if total_done == self.total {
            println!(); // New line when complete
        }
    }

    pub fn print_header(&self) {
        println!("Starting execution of {} slices...", self.total);
    }

    pub fn print_summary(&self, results: &RunResults) {
        let elapsed = self.start_time.elapsed();
        println!("{}", results.summary());
        println!("Total: {:?} | {}", elapsed, results.timing_summary());

        if results.has_failures() {
            println!("\nErrors occurred:");
            for (slice, layer, method, error) in results.get_all_method_errors() {
                println!("  ✗ {}.{}.{}: {}", slice, layer, method, error.message());
            }
        }
    }
}
