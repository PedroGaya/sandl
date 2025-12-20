use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SliceStart {
        slice: String,
    },
    SliceComplete {
        slice: String,
        duration: Duration,
    },
    SliceFailed {
        slice: String,
        error: String,
    },

    MethodStart {
        slice: String,
        layer: String,
        method: String,
    },
    MethodComplete {
        slice: String,
        layer: String,
        method: String,
        duration: Duration,
    },
    MethodFailed {
        slice: String,
        layer: String,
        method: String,
        error: String,
    },
}

pub type EventCallback = Arc<dyn Fn(&EngineEvent) + Send + Sync>;

#[derive(Clone)]
pub struct Observer {
    callbacks: Vec<EventCallback>,
}

impl Observer {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(&EngineEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Arc::new(callback));
    }

    pub fn emit(&self, event: EngineEvent) {
        for callback in &self.callbacks {
            callback(&event);
        }
    }
}

impl Default for Observer {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer {
    pub fn on_slice_start<F>(&mut self, f: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_event(move |event| {
            if let EngineEvent::SliceStart { slice } = event {
                f(slice);
            }
        });
    }

    pub fn on_slice_complete<F>(&mut self, f: F)
    where
        F: Fn(&str, Duration) + Send + Sync + 'static,
    {
        self.on_event(move |event| {
            if let EngineEvent::SliceComplete { slice, duration } = event {
                f(slice, *duration);
            }
        });
    }

    pub fn on_method_start<F>(&mut self, f: F)
    where
        F: Fn(&str, &str, &str) + Send + Sync + 'static,
    {
        self.on_event(move |event| {
            if let EngineEvent::MethodStart {
                slice,
                layer,
                method,
            } = event
            {
                f(slice, layer, method);
            }
        });
    }

    pub fn on_method_complete<F>(&mut self, f: F)
    where
        F: Fn(&str, &str, &str, Duration) + Send + Sync + 'static,
    {
        self.on_event(move |event| {
            if let EngineEvent::MethodComplete {
                slice,
                layer,
                method,
                duration,
            } = event
            {
                f(slice, layer, method, *duration);
            }
        });
    }

    pub fn on_method_failed<F>(&mut self, f: F)
    where
        F: Fn(&str, &str, &str, &str) + Send + Sync + 'static,
    {
        self.on_event(move |event| {
            if let EngineEvent::MethodFailed {
                slice,
                layer,
                method,
                error,
            } = event
            {
                f(slice, layer, method, error);
            }
        });
    }
}
