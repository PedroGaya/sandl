use rayon::ThreadPoolBuilder;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub num_threads: Option<usize>,
    pub stack_size: Option<usize>,
    pub chunk_size: usize,

    pub batch_size: Option<usize>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            num_threads: None,
            stack_size: None,
            batch_size: None, // No batching = process all at once
            chunk_size: 1,    // No chunking = one item per coordination
        }
    }
}

impl EngineConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn num_threads(mut self, threads: usize) -> Self {
        self.num_threads = Some(threads);
        self
    }

    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.max(1);
        self
    }

    pub(crate) fn build_thread_pool(&self) -> crate::Result<rayon::ThreadPool> {
        let mut builder = ThreadPoolBuilder::new();

        if let Some(threads) = self.num_threads {
            builder = builder.num_threads(threads);
        }

        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }

        builder
            .build()
            .map_err(|e| crate::Error::ConfigError(format!("Failed to build thread pool: {}", e)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunFlags {
    pub silent: bool,
    pub with_observer: bool,
}

impl RunFlags {
    pub const SILENT: Self = Self {
        silent: true,
        with_observer: true,
    };
    pub const SILENT_NO_OBSERVER: Self = Self {
        silent: true,
        with_observer: false,
    };
    pub const TRACKED: Self = Self {
        silent: false,
        with_observer: true,
    };

    pub fn new() -> Self {
        Self::TRACKED
    }
}

impl Default for RunFlags {
    fn default() -> Self {
        Self::TRACKED
    }
}
