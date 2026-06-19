use std::sync::Arc;

use destack_artifact::ArtifactKey;
use parking_lot::Mutex;

use crate::Moment;

use super::{ArtifactAttempt, ArtifactAttemptOutcome, Trace, TraceCounter, TraceSpan};

/// Recorder buffering one running artifact attempt.
#[derive(Debug)]
pub struct ArtifactAttemptRecorder {
    /// The owning trace.
    trace: Arc<Trace>,
    /// The artifact key being attempted.
    key: ArtifactKey,
    /// The worker executing the attempt.
    worker: usize,
    /// The attempt start.
    started: Option<Moment>,
    /// Interior spans recorded so far.
    spans: Mutex<Vec<TraceSpan>>,
    /// Counters recorded so far.
    counters: Mutex<Vec<TraceCounter>>,
}

impl ArtifactAttemptRecorder {
    /// Create one recorder for a running artifact attempt.
    pub(super) fn new(
        trace: Arc<Trace>,
        key: ArtifactKey,
        worker: usize,
        started: Option<Moment>,
    ) -> Self {
        Self {
            trace,
            key,
            worker,
            started,
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
        }
    }

    /// Record one timed span around a closure.
    pub fn span<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        let started = self.trace.now();
        let value = work();
        self.record_span(name, started);

        value
    }

    /// Record one interior span that started at one clock reading.
    pub fn record_span(&self, name: &'static str, started: Option<Moment>) {
        let span = self.trace.span_from(name, started);

        self.spans.lock().push(span);
    }

    /// Record one named counter.
    pub fn record_counter(&self, name: &'static str, value: u64) {
        self.counters.lock().push(TraceCounter { name, value });
    }

    /// Finish this artifact attempt with its outcome.
    pub fn finish(&self, outcome: ArtifactAttemptOutcome) {
        let span = self.trace.span_from(self.key.display_name(), self.started);
        let attempt = ArtifactAttempt {
            key: self.key,
            worker: self.worker,
            span,
            outcome,
            spans: std::mem::take(&mut self.spans.lock()),
            counters: std::mem::take(&mut self.counters.lock()),
        };

        self.trace.record_attempt(attempt);
    }
}
