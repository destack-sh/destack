use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tspp_artifact::{ArtifactKey, ArtifactStage};
use tspp_serde::Reflect;
use tspp_source::TargetId;

use crate::{Clock, Moment, Repository, RepositoryError, Revision};

use super::{ArtifactAttemptRecorder, TraceEvent};

/// Terminal outcome of one artifact executor attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactAttemptOutcome {
    /// The provider produced the artifact in this trace.
    Built,
    /// The artifact was already present in memory.
    MemoryCached,
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
            Self::Parked => "parked",
            Self::Failed => "failed",
        }
    }
}

/// One timed interval in a trace.
#[derive(Debug, Clone, Copy)]
pub struct TraceSpan {
    /// How this span contributes to timing aggregates.
    pub(super) kind: TraceSpanKind,
    /// The span name.
    pub name: &'static str,
    /// The offset from the trace epoch.
    pub start: Duration,
    /// The span duration.
    pub duration: Duration,
}

/// How one trace span contributes to timing aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TraceSpanKind {
    /// Executor or provider work owned by the artifact attempt.
    Work,
    /// A named breakdown nested inside other recorded work.
    Breakdown,
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
    /// Events recorded by the provider.
    pub events: Vec<TraceEvent>,
    /// Exact artifact dependencies, present once dependency resolution completes.
    pub dependencies: Option<Box<[ArtifactKey]>>,
}

/// Cross-run aggregate of artifact attempts by kind.
#[derive(Debug, Default)]
pub struct TraceAggregate {
    /// Aggregated attempt rows keyed by artifact kind name.
    attempts: BTreeMap<&'static str, TraceAggregateAttempts>,
    /// Aggregated interior span totals keyed by artifact kind name and span name.
    spans: BTreeMap<(&'static str, &'static str), TraceAggregateSpan>,
    /// The number of merged traces.
    runs: u64,
    /// The summed duration of merged traces.
    duration: Duration,
}

/// Aggregated attempt outcomes and work time for one artifact kind.
#[derive(Debug, Default)]
struct TraceAggregateAttempts {
    /// Attempts that executed a provider.
    built: u64,
    /// Attempts served from memory.
    cached: u64,
    /// Attempts that parked or failed.
    stalled: u64,
    /// Summed executed attempt time.
    time: Duration,
}

/// Aggregated count and time for one artifact kind's interior span.
#[derive(Debug, Default)]
struct TraceAggregateSpan {
    /// The number of recorded spans.
    count: u64,
    /// The summed span time.
    time: Duration,
}

impl TraceAggregate {
    /// Merge one finished trace into this aggregate.
    pub fn merge(&mut self, trace: &Trace) {
        self.runs += 1;
        let attempts = trace.attempts();
        self.duration += trace.duration(&trace.spans(), &attempts);
        for attempt in attempts {
            let row = self.attempts.entry(attempt.key.name()).or_default();
            match attempt.outcome {
                ArtifactAttemptOutcome::Built => {
                    row.built += 1;
                    row.time += attempt.span.duration;
                }
                ArtifactAttemptOutcome::MemoryCached => {
                    row.cached += 1;
                    row.time += attempt.span.duration;
                }
                ArtifactAttemptOutcome::Parked | ArtifactAttemptOutcome::Failed => {
                    row.stalled += 1;
                }
            }
            for span in &attempt.spans {
                let totals = self
                    .spans
                    .entry((attempt.key.name(), span.name))
                    .or_default();
                totals.count += 1;
                totals.time += span.duration;
            }
        }
    }

    /// Return the number of merged traces.
    pub fn runs(&self) -> u64 {
        self.runs
    }

    /// Render the aggregate as one aligned table, slowest kinds first.
    pub fn render(&self) -> String {
        let mut output = format!(
            "runs={} traced={:.3}s\n\n{:<24} {:>8} {:>9} {:>9} {:>10}\n",
            self.runs,
            self.duration.as_secs_f64(),
            "kind",
            "built",
            "cached",
            "stalled",
            "seconds",
        );
        let mut rows = self.attempts.iter().collect::<Vec<_>>();
        rows.sort_by_key(|(_, row)| Reverse(row.time));
        for (name, row) in rows {
            output.push_str(&format!(
                "{name:<24} {:>8} {:>9} {:>9} {:>10.3}\n",
                row.built,
                row.cached,
                row.stalled,
                row.time.as_secs_f64(),
            ));
        }

        let mut spans = self.spans.iter().collect::<Vec<_>>();
        spans.sort_by_key(|(_, totals)| Reverse(totals.time));
        output.push_str(&format!(
            "\n{:<24} {:<18} {:>9} {:>10}\n",
            "kind", "span", "count", "seconds"
        ));
        for ((kind, name), totals) in spans {
            output.push_str(&format!(
                "{kind:<24} {name:<18} {:>9} {:>10.3}\n",
                totals.count,
                totals.time.as_secs_f64(),
            ));
        }

        output
    }
}

/// Recording level for one toolchain trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceLevel {
    /// Record nothing.
    Disabled,
    /// Record timings and counters.
    Timings,
    /// Record timings, counters, and provider events.
    Events,
}

/// Trace of one public toolchain operation.
#[derive(Debug)]
pub struct Trace {
    /// The recording level.
    level: TraceLevel,
    /// The clock used for timing samples.
    clock: Clock,
    /// The executor workers available to this operation.
    workers: usize,
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
    pub fn new(clock: Clock, workers: usize, level: TraceLevel) -> Arc<Self> {
        // sample the epoch only for recorded traces
        let epoch = match level {
            TraceLevel::Disabled => None,
            TraceLevel::Timings | TraceLevel::Events => clock.now(),
        };

        Arc::new(Self {
            level,
            clock,
            workers,
            epoch,
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
            attempts: Mutex::new(Vec::new()),
            duration: Mutex::new(None),
        })
    }

    /// Return whether this trace records timings and counters.
    pub fn records_timings(&self) -> bool {
        self.level != TraceLevel::Disabled
    }

    /// Return whether artifact providers record detailed events.
    pub fn records_events(&self) -> bool {
        self.level == TraceLevel::Events
    }

    /// Record one timed operation-level span around a closure.
    pub fn span<T>(&self, name: &'static str, work: impl FnOnce() -> T) -> T {
        if !self.records_timings() {
            return work();
        }

        let started = self.clock.now();
        let value = work();
        self.record_span(name, started, TraceSpanKind::Breakdown);

        value
    }

    /// Record one timed operation-level span around a future.
    pub async fn span_async<T>(&self, name: &'static str, work: impl Future<Output = T>) -> T {
        if !self.records_timings() {
            return work.await;
        }

        let started = self.clock.now();
        let value = work.await;
        self.record_span(name, started, TraceSpanKind::Breakdown);

        value
    }

    /// Add to one operation-level counter.
    pub fn add_counter(&self, name: &'static str, value: u64) {
        if !self.records_timings() {
            return;
        }

        let mut counters = self.counters.lock();
        if let Some(counter) = counters.iter_mut().find(|counter| counter.name == name) {
            counter.value += value;
        } else {
            counters.push(TraceCounter { name, value });
        }
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
        let started = if self.records_timings() {
            self.clock.now()
        } else {
            None
        };

        ArtifactAttemptRecorder::new(self.clone(), key, worker, started)
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
    pub fn snapshot<E>(
        &self,
        view: TraceView,
        mut label: impl FnMut(&ArtifactKey) -> Result<Option<String>, E>,
        mut target: impl FnMut(TargetId) -> Result<Option<String>, E>,
    ) -> Result<TraceSnapshot, E> {
        let spans = self.spans.lock();
        let counters = self.counters.lock();
        let attempts = self.attempts.lock();

        // resolve each optional display value at most once
        let mut labels = BTreeMap::<ArtifactKey, Option<String>>::new();
        let mut artifact_label = |key: &ArtifactKey| {
            if let Some(value) = labels.get(key) {
                return Ok(value.clone());
            }

            let value = label(key)?;
            labels.insert(*key, value.clone());

            Ok(value)
        };
        let mut targets = BTreeMap::<TargetId, Option<String>>::new();
        let mut target_label = |target_id: TargetId| {
            if let Some(value) = targets.get(&target_id) {
                return Ok(value.clone());
            }

            let value = target(target_id)?;
            targets.insert(target_id, value.clone());

            Ok(value)
        };

        // roll up artifact attempt time per stage
        let mut stages = ArtifactStage::ALL.map(|stage| (stage, Duration::ZERO));
        for attempt in attempts.iter() {
            let stage = attempt.key.stage();
            let row = &mut stages[stage as usize];
            row.1 += work_duration(&attempt.spans);
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
        let attempt_snapshots = if view.includes_attempts() {
            let mut snapshots = Vec::with_capacity(attempts.len());
            for attempt in attempts.iter() {
                let target = match attempt.key.target_id() {
                    Some(target_id) => target_label(target_id)?,
                    None => None,
                };
                snapshots.push(ArtifactAttemptSnapshot {
                    name: attempt.key.display_name().to_string(),
                    stage: attempt.key.stage().name().to_string(),
                    label: artifact_label(&attempt.key)?,
                    target,
                    worker: attempt.worker,
                    start_micros: attempt.span.start.as_micros() as u64,
                    latency_micros: attempt.span.duration.as_micros() as u64,
                    work_micros: work_duration(&attempt.spans).as_micros() as u64,
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
                    events: attempt.events.clone(),
                });
            }

            snapshots
        } else {
            Vec::new()
        };
        let total = self.duration(&spans, &attempts);
        let spans = spans.iter().map(TraceSpanSnapshot::from_span).collect();
        let counters = counters
            .iter()
            .map(TraceCounterSnapshot::from_counter)
            .collect();
        let parallelism = if view.includes_attempts() {
            parallelism(&attempts, total, self.workers, &mut artifact_label)?
        } else {
            TraceParallelismSnapshot::default()
        };

        Ok(TraceSnapshot {
            total_micros: total.as_micros() as u64,
            workers: self.workers,
            stats: outcomes,
            spans,
            counters,
            stages,
            times,
            attempts: attempt_snapshots,
            parallelism,
        })
    }

    /// Record one operation-level span that started at one clock reading.
    fn record_span(&self, name: &'static str, started: Option<Moment>, kind: TraceSpanKind) {
        let span = self.span_from(name, started, kind);

        self.spans.lock().push(span);
    }

    /// Build one span from a sampled clock reading.
    pub(super) fn span_from(
        &self,
        name: &'static str,
        started: Option<Moment>,
        kind: TraceSpanKind,
    ) -> TraceSpan {
        let start = started
            .map(|started| self.clock.duration_since(started, self.epoch))
            .unwrap_or(Duration::ZERO);
        let duration = started
            .map(|started| self.clock.elapsed(started))
            .unwrap_or(Duration::ZERO);

        TraceSpan {
            kind,
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

impl Repository {
    /// Snapshot one operation trace with repository display names.
    pub fn snapshot_trace(
        &self,
        revision: Revision,
        trace: &Trace,
        view: TraceView,
    ) -> Result<TraceSnapshot, RepositoryError> {
        trace.snapshot(
            view,
            |key| self.artifact_display(revision, *key),
            |target| self.target_display(revision, target),
        )
    }

    /// Return the display label for one traced artifact.
    pub fn artifact_display(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> Result<Option<String>, RepositoryError> {
        // label module artifacts through the repository index
        let Some(module) = key.module_id() else {
            return Ok(None);
        };

        self.module_display(revision, module)
    }
}

/// Sum the non-overlapping work intervals in one artifact attempt.
fn work_duration(spans: &[TraceSpan]) -> Duration {
    let mut total = Duration::ZERO;
    let mut previous_end = Duration::ZERO;

    // sum executor intervals while enforcing exclusive ownership
    for span in spans {
        if span.kind != TraceSpanKind::Work {
            continue;
        }

        assert!(
            span.start >= previous_end,
            "artifact work spans must not overlap"
        );
        previous_end = span.start + span.duration;
        total += span.duration;
    }

    total
}

/// Derive work, span, and the attempted artifact critical path.
fn parallelism<E>(
    attempts: &[ArtifactAttempt],
    total: Duration,
    workers: usize,
    label: &mut impl FnMut(&ArtifactKey) -> Result<Option<String>, E>,
) -> Result<TraceParallelismSnapshot, E> {
    let mut indices = BTreeMap::new();

    // index attempted artifacts in stable key order
    for attempt in attempts {
        indices.insert(attempt.key, 0);
    }
    for (index, value) in indices.values_mut().enumerate() {
        *value = index;
    }
    let mut artifacts = indices
        .keys()
        .copied()
        .map(ArtifactVertex::new)
        .collect::<Vec<_>>();

    // weight graph vertices with terminal artifact work
    for attempt in attempts {
        if attempt.outcome == ArtifactAttemptOutcome::Parked {
            continue;
        }

        let index = indices[&attempt.key];
        artifacts[index].work_micros += work_duration(&attempt.spans).as_micros() as u64;
    }

    // retain direct dependencies of terminal attempts in this run
    for attempt in attempts {
        if attempt.outcome == ArtifactAttemptOutcome::Parked {
            continue;
        }

        let artifact = indices[&attempt.key];
        let Some(dependencies) = &attempt.dependencies else {
            continue;
        };
        for dependency in dependencies {
            let Some(&dependency) = indices.get(dependency) else {
                continue;
            };
            if dependency == artifact {
                continue;
            }

            artifacts[artifact].dependencies.push(dependency);
        }
    }
    for artifact in &mut artifacts {
        artifact.dependencies.sort_unstable();
        artifact.dependencies.dedup();
    }

    // invert dependency edges once for forward span propagation
    let mut dependents = vec![Vec::new(); artifacts.len()];
    for (artifact, value) in artifacts.iter().enumerate() {
        for dependency in &value.dependencies {
            dependents[*dependency].push(artifact);
        }
    }

    // propagate longest dependency spans from leaves to roots
    let mut remaining = artifacts
        .iter()
        .map(|artifact| artifact.dependencies.len())
        .collect::<Vec<_>>();
    let mut spans = artifacts
        .iter()
        .map(|artifact| artifact.work_micros)
        .collect::<Vec<_>>();
    let mut predecessors = vec![None; artifacts.len()];
    let mut ready = remaining
        .iter()
        .enumerate()
        .filter_map(|(index, remaining)| (*remaining == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut processed = 0;
    while let Some(dependency) = ready.pop_front() {
        processed += 1;

        for artifact in &dependents[dependency] {
            let candidate = spans[dependency] + artifacts[*artifact].work_micros;
            let is_better = candidate > spans[*artifact]
                || candidate == spans[*artifact]
                    && predecessors[*artifact].is_none_or(|current| dependency < current);
            if is_better {
                spans[*artifact] = candidate;
                predecessors[*artifact] = Some(dependency);
            }

            remaining[*artifact] -= 1;
            if remaining[*artifact] == 0 {
                ready.push_back(*artifact);
            }
        }
    }
    assert_eq!(
        processed,
        artifacts.len(),
        "trace artifact dependencies must form a DAG"
    );

    // select the stable root with the greatest span
    let root = spans
        .iter()
        .enumerate()
        .fold(None, |selected, (index, span)| match selected {
            Some((selected_index, selected_span)) if selected_span >= *span => {
                Some((selected_index, selected_span))
            }
            _ => Some((index, *span)),
        });

    // walk the selected dependency chain into chronological order
    let mut path = Vec::new();
    let span_micros = root.map_or(0, |(_, span)| span);
    if let Some((mut artifact, _)) = root.filter(|(_, span)| *span > 0) {
        loop {
            let value = &artifacts[artifact];
            path.push(TraceCriticalArtifactSnapshot {
                name: value.key.display_name().to_string(),
                stage: value.key.stage().name().to_string(),
                label: label(&value.key)?,
                work_micros: value.work_micros,
                cumulative_micros: spans[artifact],
                dependencies: value.dependencies.len(),
                dependents: dependents[artifact].len(),
            });

            let Some(predecessor) = predecessors[artifact] else {
                break;
            };
            artifact = predecessor;
        }
        path.reverse();
    }

    // derive the standard work and span bounds
    let total_micros = total.as_micros() as u64;
    let work_micros: u64 = attempts
        .iter()
        .map(|attempt| work_duration(&attempt.spans).as_micros() as u64)
        .sum();
    let artifact_work_micros: u64 = artifacts.iter().map(|artifact| artifact.work_micros).sum();
    let parked_work_micros = work_micros - artifact_work_micros;
    let work_bound_micros = work_micros.div_ceil(workers as u64);
    let lower_bound_micros = work_bound_micros.max(span_micros);
    assert!(
        lower_bound_micros <= total_micros,
        "trace work and span lower bound must not exceed wall time"
    );
    let bound_gap_micros = total_micros - lower_bound_micros;
    let scheduler_micros = attempts
        .iter()
        .flat_map(|attempt| &attempt.spans)
        .filter(|span| span.kind == TraceSpanKind::Work && span.name == "park")
        .map(|span| span.duration.as_micros() as u64)
        .sum();

    Ok(TraceParallelismSnapshot {
        work_micros,
        artifact_work_micros,
        parked_work_micros,
        span_micros,
        lower_bound_micros,
        bound_gap_micros,
        scheduler_micros,
        critical_path: path,
    })
}

/// One artifact vertex used while deriving work and span.
#[derive(Debug)]
struct ArtifactVertex {
    /// The artifact key.
    key: ArtifactKey,
    /// Exclusive work across terminal attempts.
    work_micros: u64,
    /// Direct attempted artifact dependencies.
    dependencies: Vec<usize>,
}

impl ArtifactVertex {
    /// Create one empty artifact graph vertex.
    fn new(key: ArtifactKey) -> Self {
        Self {
            key,
            work_micros: 0,
            dependencies: Vec::new(),
        }
    }
}

/// Serializable snapshot of one trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct TraceSnapshot {
    /// The wall time of the traced operation in microseconds.
    pub total_micros: u64,
    /// The executor workers available to this operation.
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
    pub attempts: Vec<ArtifactAttemptSnapshot>,
    /// Work, span, and the artifact dependency critical path.
    pub parallelism: TraceParallelismSnapshot,
}

/// Trace detail returned to a caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    fn includes_attempts(self) -> bool {
        matches!(self, Self::Detailed)
    }
}

/// Artifact attempt outcome counts in one trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct TraceStats {
    /// Attempts that produced an artifact.
    pub built: u64,
    /// Attempts served from memory.
    pub memory_cached: u64,
    /// Attempts parked on missing requirements.
    pub parked: u64,
    /// Attempts that failed.
    pub failed: u64,
}

impl TraceStats {
    /// Return the total artifact attempts.
    pub fn attempts(&self) -> u64 {
        self.built + self.memory_cached + self.parked + self.failed
    }

    /// Add one terminal attempt outcome.
    fn add(&mut self, outcome: ArtifactAttemptOutcome) {
        match outcome {
            ArtifactAttemptOutcome::Built => self.built += 1,
            ArtifactAttemptOutcome::MemoryCached => self.memory_cached += 1,
            ArtifactAttemptOutcome::Parked => self.parked += 1,
            ArtifactAttemptOutcome::Failed => self.failed += 1,
        }
    }
}

/// Executor and provider work for one toolchain stage.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TraceStageSnapshot {
    /// The stage display name.
    pub name: String,
    /// The summed work time in microseconds.
    pub micros: u64,
}

/// Summed time of one named trace span.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TraceSpanSnapshot {
    /// How this span contributes to timing aggregates.
    pub kind: TraceSpanKind,
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
            kind: span.kind,
            name: span.name.to_string(),
            start_micros: span.start.as_micros() as u64,
            micros: span.duration.as_micros() as u64,
        }
    }
}

/// One counter in a trace snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
    /// The inclusive request latency in microseconds.
    pub latency_micros: u64,
    /// The exclusive executor and provider work in microseconds.
    pub work_micros: u64,
    /// The attempt outcome name.
    pub outcome: String,
    /// Interior spans recorded by the executor or provider.
    #[serde(default)]
    pub spans: Vec<TraceSpanSnapshot>,
    /// Counters recorded by the executor or provider.
    #[serde(default)]
    pub counters: Vec<TraceCounterSnapshot>,
    /// Events recorded by the provider.
    #[serde(default)]
    pub events: Vec<TraceEvent>,
}

/// Work and span measurements of one artifact trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct TraceParallelismSnapshot {
    /// Aggregate exclusive work across every artifact attempt.
    pub work_micros: u64,
    /// Work performed by terminal artifact attempts.
    pub artifact_work_micros: u64,
    /// Work performed by attempts that parked on dependencies.
    pub parked_work_micros: u64,
    /// Aggregate exclusive work along the critical path.
    pub span_micros: u64,
    /// The work and span lower bound at the recorded worker count.
    pub lower_bound_micros: u64,
    /// Wall time beyond the work and span lower bound.
    pub bound_gap_micros: u64,
    /// Artifact work spent scheduling dependencies.
    pub scheduler_micros: u64,
    /// Artifacts along the path in dependency order.
    pub critical_path: Vec<TraceCriticalArtifactSnapshot>,
}

/// One artifact on a trace critical path.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TraceCriticalArtifactSnapshot {
    /// The artifact kind name.
    pub name: String,
    /// The toolchain stage display name.
    pub stage: String,
    /// The resolved artifact label.
    pub label: Option<String>,
    /// Exclusive work across this artifact's attempts.
    pub work_micros: u64,
    /// Cumulative critical work through this artifact.
    pub cumulative_micros: u64,
    /// Direct attempted artifacts this artifact depended on.
    pub dependencies: usize,
    /// Direct attempted artifacts that depended on this artifact.
    pub dependents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tspp_source::{ModuleId, PackageId, ProfileId};

    /// Build one deterministic attempt for work and span tests.
    fn attempt(
        key: ArtifactKey,
        outcome: ArtifactAttemptOutcome,
        work_micros: u64,
        dependencies: Option<Vec<ArtifactKey>>,
    ) -> ArtifactAttempt {
        let name = if outcome == ArtifactAttemptOutcome::Parked {
            "park"
        } else {
            "provide"
        };
        let duration = Duration::from_micros(work_micros);

        ArtifactAttempt {
            key,
            worker: 0,
            span: TraceSpan {
                kind: TraceSpanKind::Breakdown,
                name: key.display_name(),
                start: Duration::ZERO,
                duration,
            },
            outcome,
            spans: vec![TraceSpan {
                kind: TraceSpanKind::Work,
                name,
                start: Duration::ZERO,
                duration,
            }],
            counters: Vec::new(),
            events: Vec::new(),
            dependencies: dependencies.map(Vec::into_boxed_slice),
        }
    }

    /// Derive work and span without moving parked work behind dependencies.
    #[test]
    fn test_derive_work_and_span_from_artifact_dependencies() {
        let package = PackageId::new(1);
        let module = ModuleId::new(package, 1);
        let profile = ProfileId::new(1);
        let parse = ArtifactKey::dir_parsed(module);
        let bind = ArtifactKey::dir_bound(module, profile);
        let resolve = ArtifactKey::dir_resolved(module, profile);
        let export = ArtifactKey::dir_exported(module, profile);
        let check = ArtifactKey::dir_checked(module, profile);
        let attempts = vec![
            attempt(parse, ArtifactAttemptOutcome::Built, 2, Some(Vec::new())),
            attempt(bind, ArtifactAttemptOutcome::Built, 3, Some(vec![parse])),
            attempt(resolve, ArtifactAttemptOutcome::Built, 7, Some(vec![bind])),
            attempt(export, ArtifactAttemptOutcome::Built, 11, Some(vec![bind])),
            attempt(check, ArtifactAttemptOutcome::Parked, 4, None),
            attempt(
                check,
                ArtifactAttemptOutcome::Built,
                1,
                Some(vec![resolve, export]),
            ),
        ];

        // the terminal artifact DAG determines span while every attempt contributes work
        let mut label = |_: &ArtifactKey| Ok::<_, ()>(None);
        let report = parallelism(&attempts, Duration::from_micros(20), 2, &mut label).unwrap();
        assert_eq!(report.work_micros, 28);
        assert_eq!(report.artifact_work_micros, 24);
        assert_eq!(report.parked_work_micros, 4);
        assert_eq!(report.span_micros, 17);
        assert_eq!(report.lower_bound_micros, 17);
        assert_eq!(report.bound_gap_micros, 3);
        assert_eq!(report.scheduler_micros, 4);

        // the heavier export branch forms the complete critical path
        let path = report
            .critical_path
            .iter()
            .map(|artifact| {
                (
                    artifact.name.as_str(),
                    artifact.work_micros,
                    artifact.cumulative_micros,
                    artifact.dependencies,
                    artifact.dependents,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            path,
            vec![
                ("dir.parse", 2, 2, 0, 1),
                ("dir.bind", 3, 5, 1, 2),
                ("dir.export", 11, 16, 1, 1),
                ("dir.check", 1, 17, 2, 0),
            ]
        );
    }
}
