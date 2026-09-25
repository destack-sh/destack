use std::mem;
use std::sync::Arc;

use parking_lot::Mutex;
use tspp_artifact::{ArtifactDependency, ArtifactKey};

use crate::Moment;

use super::{
    ArtifactAttempt, ArtifactAttemptOutcome, Trace, TraceCounter, TraceEvent, TraceSpan,
    TraceSpanKind,
};

/// Recorder buffering one running artifact attempt.
#[derive(Debug)]
pub struct ArtifactAttemptRecorder {
    /// The owning trace.
    trace: Arc<Trace>,
    /// Whether the owning trace records timings and counters.
    records_timings: bool,
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
    /// Events recorded so far.
    events: Mutex<Vec<TraceEvent>>,
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
            records_timings: trace.records_timings(),
            trace,
            key,
            worker,
            started,
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
            dependencies: Mutex::new(None),
        }
    }

    /// Return whether this attempt records detailed provider events.
    pub fn records_events(&self) -> bool {
        self.trace.records_events()
    }

    /// Return whether this attempt records timings and counters.
    pub fn records_timings(&self) -> bool {
        self.records_timings
    }

    /// Record one timed span around a closure.
    pub fn span<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        self.timed(name, TraceSpanKind::Work, work)
    }

    /// Record one timed breakdown around a closure.
    pub fn breakdown<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        self.timed(name, TraceSpanKind::Breakdown, work)
    }

    /// Record one timed breakdown around a closure when a recorder is present.
    pub fn breakdown_maybe<T>(
        recorder: Option<&Self>,
        name: &'static str,
        work: impl FnOnce() -> T,
    ) -> T {
        match recorder {
            Some(recorder) => recorder.breakdown(name, work),
            None => work(),
        }
    }

    /// Record one timed span of one kind around a closure.
    fn timed<T>(&self, name: &'static str, kind: TraceSpanKind, work: impl FnOnce() -> T) -> T {
        if !self.records_timings {
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
        if !self.records_timings {
            return;
        }

        let span = self
            .trace
            .span_from(name, started, TraceSpanKind::Breakdown);

        self.spans.lock().push(span);
    }

    /// Record one named counter.
    pub fn record_counter(&self, name: &'static str, value: u64) {
        if !self.records_timings {
            return;
        }

        self.counters.lock().push(TraceCounter { name, value });
    }

    /// Record several named counters together.
    pub fn record_counters(&self, counters: &[(&'static str, u64)]) {
        if !self.records_timings {
            return;
        }

        let counters = counters
            .iter()
            .map(|&(name, value)| TraceCounter { name, value });
        self.counters.lock().extend(counters);
    }

    /// Record ordered events.
    pub fn record_events(&self, events: Vec<TraceEvent>) {
        if !self.records_events() {
            return;
        }

        self.events.lock().extend(events);
    }

    /// Record per-kind counters for this attempt's dependency reads.
    pub fn record_reads(&self, reads: &[ArtifactDependency]) {
        if !self.records_timings {
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

        self.record_counters(&[
            ("reads.sources", sources as u64),
            ("reads.artifacts", artifacts as u64),
            ("reads.projections", projections as u64),
        ]);
    }

    /// Record the exact artifact dependencies resolved for this attempt.
    pub fn record_dependencies(&self, dependencies: &[ArtifactDependency]) {
        if !self.records_timings {
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
        if !self.records_timings {
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
            spans: mem::take(&mut self.spans.lock()),
            counters: mem::take(&mut self.counters.lock()),
            events: mem::take(&mut self.events.lock()),
            dependencies: self.dependencies.lock().take(),
        };

        self.trace.record_attempt(attempt);
    }
}
