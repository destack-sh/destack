use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactKey};
use parking_lot::Mutex;

use crate::Moment;

use super::{
    ArtifactAttempt, ArtifactAttemptOutcome, Trace, TraceCounter, TraceSpan, TraceSpanKind,
};

/// Recorder buffering one running artifact attempt.
#[derive(Debug)]
pub struct ArtifactAttemptRecorder {
    /// The owning trace.
    trace: Arc<Trace>,
    /// Whether the owning trace records this attempt.
    is_enabled: bool,
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
    /// Exact artifact dependencies resolved for this attempt.
    dependencies: Mutex<Option<Box<[ArtifactKey]>>>,
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
            is_enabled: trace.is_enabled(),
            trace,
            key,
            worker,
            started,
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
            dependencies: Mutex::new(None),
        }
    }

    /// Record one timed span around a closure.
    pub fn span<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        self.timed(name, TraceSpanKind::Work, work)
    }

    /// Record one timed breakdown around a closure.
    pub fn breakdown<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        self.timed(name, TraceSpanKind::Breakdown, work)
    }

    /// Record one timed span of one kind around a closure.
    fn timed<T>(&self, name: &'static str, kind: TraceSpanKind, work: impl FnOnce() -> T) -> T {
        if !self.is_enabled {
            return work();
        }

        let started = self.trace.now();
        let value = work();
        let span = self.trace.span_from(name, started, kind);
        self.spans.lock().push(span);

        value
    }

    /// Record one interior span that started at one clock reading.
    pub fn record_span(&self, name: &'static str, started: Option<Moment>) {
        if !self.is_enabled {
            return;
        }

        let span = self
            .trace
            .span_from(name, started, TraceSpanKind::Breakdown);

        self.spans.lock().push(span);
    }

    /// Record one named counter.
    pub fn record_counter(&self, name: &'static str, value: u64) {
        if !self.is_enabled {
            return;
        }

        self.counters.lock().push(TraceCounter { name, value });
    }

    /// Record several named counters together.
    pub fn record_counters<const N: usize>(&self, counters: [(&'static str, u64); N]) {
        if !self.is_enabled {
            return;
        }

        let counters = counters.map(|(name, value)| TraceCounter { name, value });
        self.counters.lock().extend(counters);
    }

    /// Record per-kind counters for this attempt's dependency reads.
    pub fn record_reads(&self, reads: &[ArtifactDependency]) {
        if !self.is_enabled {
            return;
        }

        // count each dependency kind once
        let sources = reads
            .iter()
            .filter(|read| matches!(read, ArtifactDependency::Source(_)))
            .count();
        let artifacts = reads
            .iter()
            .filter(|read| matches!(read, ArtifactDependency::Artifact(_)))
            .count();
        let projections = reads.len() - sources - artifacts;

        self.record_counters([
            ("reads.sources", sources as u64),
            ("reads.artifacts", artifacts as u64),
            ("reads.projections", projections as u64),
        ]);
    }

    /// Record the exact artifact dependencies resolved for this attempt.
    pub fn record_dependencies(&self, dependencies: &[ArtifactDependency]) {
        if !self.is_enabled {
            return;
        }

        let mut dependencies = dependencies
            .iter()
            .filter_map(ArtifactDependency::artifact_key)
            .collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies.dedup();

        let previous = self
            .dependencies
            .lock()
            .replace(dependencies.into_boxed_slice());
        assert!(
            previous.is_none(),
            "artifact attempt dependencies must be recorded once"
        );
    }

    /// Finish this artifact attempt with its outcome.
    pub fn finish(&self, outcome: ArtifactAttemptOutcome) {
        if !self.is_enabled {
            return;
        }

        let span = self.trace.span_from(
            self.key.display_name(),
            self.started,
            TraceSpanKind::Breakdown,
        );
        let attempt = ArtifactAttempt {
            key: self.key,
            worker: self.worker,
            span,
            outcome,
            spans: std::mem::take(&mut self.spans.lock()),
            counters: std::mem::take(&mut self.counters.lock()),
            dependencies: self.dependencies.lock().take(),
        };

        self.trace.record_attempt(attempt);
    }
}
