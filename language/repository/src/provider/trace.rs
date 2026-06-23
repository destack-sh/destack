use std::sync::Arc;
use std::time::Duration;

use destack_artifact::{ArtifactKey, ArtifactStage};
use destack_serde::Schema;
use destack_source::TargetId;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{Clock, Moment};

use super::ArtifactAttemptRecorder;

/// Terminal outcome of one artifact executor attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactAttemptOutcome {
    /// The provider produced the artifact in this trace.
    Built,
    /// The artifact was already present in memory.
    MemoryCached,
    /// The artifact was restored from the persistent artifact store.
    StoreCached,
    /// The attempt parked on missing requirements.
    Parked,
    /// The attempt failed.
    Failed,
}

impl ArtifactAttemptOutcome {
    /// Return this outcome's display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Built => "built",
            Self::MemoryCached => "memory_cached",
            Self::StoreCached => "store_cached",
            Self::Parked => "parked",
            Self::Failed => "failed",
        }
    }
}

/// One timed interval in a trace.
#[derive(Debug, Clone, Copy)]
pub struct TraceSpan {
    /// The span name.
    pub name: &'static str,
    /// The offset from the trace epoch.
    pub start: Duration,
    /// The span duration.
    pub duration: Duration,
}

/// One named count in a trace.
#[derive(Debug, Clone, Copy)]
pub struct TraceCounter {
    /// The counter name.
    pub name: &'static str,
    /// The counter value.
    pub value: u64,
}

/// One recorded artifact executor attempt.
#[derive(Debug, Clone)]
pub struct ArtifactAttempt {
    /// The attempted artifact key.
    pub key: ArtifactKey,
    /// The worker that executed the attempt.
    pub worker: usize,
    /// The full attempt span.
    pub span: TraceSpan,
    /// The attempt outcome.
    pub outcome: ArtifactAttemptOutcome,
    /// Interior spans recorded by the executor or provider.
    pub spans: Vec<TraceSpan>,
    /// Counters recorded by the executor or provider.
    pub counters: Vec<TraceCounter>,
}

/// Trace of one public toolchain operation.
#[derive(Debug)]
pub struct Trace {
    /// The clock used for timing samples.
    clock: Clock,
    /// The clock reading all recorded offsets measure from.
    epoch: Option<Moment>,
    /// The recorded operation-level spans.
    spans: Mutex<Vec<TraceSpan>>,
    /// The recorded operation-level counters.
    counters: Mutex<Vec<TraceCounter>>,
    /// The recorded artifact attempts in completion order.
    attempts: Mutex<Vec<ArtifactAttempt>>,
    /// The final trace duration.
    duration: Mutex<Option<Duration>>,
}

impl Trace {
    /// Create an empty trace starting now.
    pub fn new(clock: Clock) -> Arc<Self> {
        Arc::new(Self {
            clock,
            epoch: clock.now(),
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
            attempts: Mutex::new(Vec::new()),
            duration: Mutex::new(None),
        })
    }

    /// Record one timed operation-level span around a closure.
    pub fn span<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        let started = self.clock.now();
        let value = work();
        self.record_span(name, started);

        value
    }

    /// Record one operation-level counter.
    pub fn record_counter(&self, name: &'static str, value: u64) {
        self.counters.lock().push(TraceCounter { name, value });
    }

    /// Finish this trace at the current clock reading.
    pub fn finish(&self) {
        let duration = self
            .epoch
            .map(|epoch| self.clock.elapsed(epoch))
            .unwrap_or(Duration::ZERO);

        *self.duration.lock() = Some(duration);
    }

    /// Begin recording one artifact attempt on one worker.
    pub fn begin(self: &Arc<Self>, key: ArtifactKey, worker: usize) -> ArtifactAttemptRecorder {
        ArtifactAttemptRecorder::new(Arc::clone(self), key, worker, self.clock.now())
    }

    /// Return the recorded artifact attempts.
    pub fn attempts(&self) -> Vec<ArtifactAttempt> {
        self.attempts.lock().clone()
    }

    /// Return the recorded operation-level spans.
    pub fn spans(&self) -> Vec<TraceSpan> {
        self.spans.lock().clone()
    }

    /// Return the recorded operation-level counters.
    pub fn counters(&self) -> Vec<TraceCounter> {
        self.counters.lock().clone()
    }

    /// Build one serializable snapshot of this trace.
    pub fn snapshot(
        &self,
        view: TraceView,
        label: impl Fn(&ArtifactKey) -> Option<String>,
        target: impl Fn(TargetId) -> Option<String>,
    ) -> TraceSnapshot {
        let spans = self.spans.lock();
        let counters = self.counters.lock();
        let attempts = self.attempts.lock();

        // roll up artifact attempt time per stage
        let mut workers = 0usize;
        let mut stages = ArtifactStage::ALL.map(|stage| (stage, Duration::ZERO));
        for attempt in attempts.iter() {
            workers = workers.max(attempt.worker + 1);

            let stage = attempt.key.stage();
            let row = stages
                .iter_mut()
                .find(|(candidate, _)| *candidate == stage)
                .expect("every stage has a rollup row");
            row.1 += attempt.span.duration;
        }

        // keep the snapshot stage rows faithful to artifact stages
        let stages = stages
            .into_iter()
            .filter(|(_stage, duration)| !duration.is_zero())
            .map(|(stage, duration)| TraceStageSnapshot {
                name: stage.name().to_string(),
                micros: duration.as_micros() as u64,
            })
            .collect();

        // roll up named spans across operation and artifact attempts
        let mut times = Vec::<TraceTimeSnapshot>::new();
        for span in spans.iter() {
            TraceTimeSnapshot::add(&mut times, span);
        }
        for attempt in attempts.iter() {
            for span in &attempt.spans {
                TraceTimeSnapshot::add(&mut times, span);
            }
        }

        // count terminal outcomes for cheap summary consumers
        let mut outcomes = TraceStats::default();
        for attempt in attempts.iter() {
            outcomes.add(attempt.outcome);
        }

        // detailed snapshots carry the labeled artifact rows
        let artifacts = if view.includes_artifacts() {
            attempts
                .iter()
                .map(|attempt| ArtifactAttemptSnapshot {
                    name: attempt.key.display_name().to_string(),
                    stage: attempt.key.stage().name().to_string(),
                    label: label(&attempt.key),
                    target: attempt.key.target_id().and_then(&target),
                    worker: attempt.worker,
                    start_micros: attempt.span.start.as_micros() as u64,
                    micros: attempt.span.duration.as_micros() as u64,
                    outcome: attempt.outcome.name().to_string(),
                    spans: attempt
                        .spans
                        .iter()
                        .map(TraceSpanSnapshot::from_span)
                        .collect(),
                    counters: attempt
                        .counters
                        .iter()
                        .map(TraceCounterSnapshot::from_counter)
                        .collect(),
                })
                .collect()
        } else {
            Vec::new()
        };
        let total = self.duration(&spans, &attempts);
        let spans = if view.includes_artifacts() {
            spans.iter().map(TraceSpanSnapshot::from_span).collect()
        } else {
            Vec::new()
        };
        let counters = if view.includes_artifacts() {
            counters
                .iter()
                .map(TraceCounterSnapshot::from_counter)
                .collect()
        } else {
            Vec::new()
        };

        TraceSnapshot {
            total_micros: total.as_micros() as u64,
            workers,
            stats: outcomes,
            spans,
            counters,
            stages,
            times,
            artifacts,
        }
    }

    /// Record one operation-level span that started at one clock reading.
    fn record_span(&self, name: &'static str, started: Option<Moment>) {
        let span = self.span_from(name, started);

        self.spans.lock().push(span);
    }

    /// Build one span from a sampled clock reading.
    pub(super) fn span_from(&self, name: &'static str, started: Option<Moment>) -> TraceSpan {
        let start = started
            .map(|started| self.clock.duration_since(started, self.epoch))
            .unwrap_or(Duration::ZERO);
        let duration = started
            .map(|started| self.clock.elapsed(started))
            .unwrap_or(Duration::ZERO);

        TraceSpan {
            name,
            start,
            duration,
        }
    }

    /// Return one current clock reading.
    pub(super) fn now(&self) -> Option<Moment> {
        self.clock.now()
    }

    /// Push one finished artifact attempt.
    pub(super) fn record_attempt(&self, attempt: ArtifactAttempt) {
        self.attempts.lock().push(attempt);
    }

    /// Return the total trace duration.
    fn duration(&self, spans: &[TraceSpan], attempts: &[ArtifactAttempt]) -> Duration {
        if let Some(duration) = *self.duration.lock() {
            return duration;
        }

        let operation_end = spans
            .iter()
            .map(|span| span.start + span.duration)
            .max()
            .unwrap_or(Duration::ZERO);
        let attempt_end = attempts
            .iter()
            .map(|attempt| attempt.span.start + attempt.span.duration)
            .max()
            .unwrap_or(Duration::ZERO);

        operation_end.max(attempt_end)
    }
}

/// Serializable snapshot of one trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Schema)]
pub struct TraceSnapshot {
    /// The wall time of the traced operation in microseconds.
    pub total_micros: u64,
    /// The number of workers that recorded attempts.
    pub workers: usize,
    /// Artifact stats.
    pub stats: TraceStats,
    /// Operation-level spans around artifact execution.
    pub spans: Vec<TraceSpanSnapshot>,
    /// Operation-level counters.
    pub counters: Vec<TraceCounterSnapshot>,
    /// Busy time per toolchain stage, ordered by stage.
    pub stages: Vec<TraceStageSnapshot>,
    /// Summed time per named trace span.
    pub times: Vec<TraceTimeSnapshot>,
    /// The recorded artifact attempts, present only in detailed snapshots.
    pub artifacts: Vec<ArtifactAttemptSnapshot>,
}

/// Trace detail returned to a caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum TraceView {
    /// Return aggregate trace data.
    #[default]
    Summary,
    /// Return aggregate trace data and attempt rows.
    Detailed,
}

impl TraceView {
    /// Return detailed view when requested, otherwise summary view.
    pub fn detailed(is_detailed: bool) -> Self {
        if is_detailed {
            Self::Detailed
        } else {
            Self::Summary
        }
    }

    /// Return whether this view includes artifact attempt rows.
    fn includes_artifacts(self) -> bool {
        matches!(self, Self::Detailed)
    }
}

/// Artifact attempt outcome counts in one trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Schema)]
pub struct TraceStats {
    /// Attempts that produced an artifact.
    pub built: u64,
    /// Attempts served from memory.
    pub memory_cached: u64,
    /// Attempts restored from the persistent store.
    pub store_cached: u64,
    /// Attempts parked on missing requirements.
    pub parked: u64,
    /// Attempts that failed.
    pub failed: u64,
}

impl TraceStats {
    /// Add one terminal attempt outcome.
    fn add(&mut self, outcome: ArtifactAttemptOutcome) {
        match outcome {
            ArtifactAttemptOutcome::Built => self.built += 1,
            ArtifactAttemptOutcome::MemoryCached => self.memory_cached += 1,
            ArtifactAttemptOutcome::StoreCached => self.store_cached += 1,
            ArtifactAttemptOutcome::Parked => self.parked += 1,
            ArtifactAttemptOutcome::Failed => self.failed += 1,
        }
    }
}

/// Busy time of one toolchain stage.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TraceStageSnapshot {
    /// The stage display name.
    pub name: String,
    /// The summed attempt time in microseconds.
    pub micros: u64,
}

/// Summed time of one named trace span.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TraceTimeSnapshot {
    /// The span name.
    pub name: String,
    /// The summed span time in microseconds.
    pub micros: u64,
}

impl TraceTimeSnapshot {
    /// Add one span to a time rollup.
    fn add(times: &mut Vec<Self>, span: &TraceSpan) {
        if let Some(time) = times.iter_mut().find(|time| time.name == span.name) {
            time.micros += span.duration.as_micros() as u64;
        } else {
            times.push(Self {
                name: span.name.to_string(),
                micros: span.duration.as_micros() as u64,
            });
        }
    }
}

/// One span in a trace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TraceSpanSnapshot {
    /// The span name.
    pub name: String,
    /// The offset from the trace start in microseconds.
    pub start_micros: u64,
    /// The span duration in microseconds.
    pub micros: u64,
}

impl TraceSpanSnapshot {
    /// Convert one in-memory trace span into a snapshot span.
    fn from_span(span: &TraceSpan) -> Self {
        Self {
            name: span.name.to_string(),
            start_micros: span.start.as_micros() as u64,
            micros: span.duration.as_micros() as u64,
        }
    }
}

/// One counter in a trace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TraceCounterSnapshot {
    /// The counter name.
    pub name: String,
    /// The counter value.
    pub value: u64,
}

impl TraceCounterSnapshot {
    /// Convert one in-memory trace counter into a snapshot counter.
    fn from_counter(counter: &TraceCounter) -> Self {
        Self {
            name: counter.name.to_string(),
            value: counter.value,
        }
    }
}

/// One artifact attempt in a detailed trace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ArtifactAttemptSnapshot {
    /// The artifact kind name.
    pub name: String,
    /// The toolchain stage display name.
    pub stage: String,
    /// The resolved artifact label, usually a module display name.
    pub label: Option<String>,
    /// The resolved target name for emitted and linked artifacts.
    #[serde(default)]
    pub target: Option<String>,
    /// The worker that executed the attempt.
    pub worker: usize,
    /// The offset from the trace start in microseconds.
    pub start_micros: u64,
    /// The attempt duration in microseconds.
    pub micros: u64,
    /// The attempt outcome name.
    pub outcome: String,
    /// Interior spans recorded by the executor or provider.
    #[serde(default)]
    pub spans: Vec<TraceSpanSnapshot>,
    /// Counters recorded by the executor or provider.
    #[serde(default)]
    pub counters: Vec<TraceCounterSnapshot>,
}
