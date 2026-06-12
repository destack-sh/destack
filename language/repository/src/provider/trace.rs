use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_artifact::{ArtifactKey, ArtifactStage};
use destack_source::TargetId;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Terminal outcome of one artifact build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceOutcome {
    /// The build produced its artifact.
    Ready,
    /// The build blocked on missing requirements.
    Blocked,
    /// The build failed.
    Failed,
}

impl TraceOutcome {
    /// Return this outcome's display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }
}

/// One interior phase recorded by an artifact build.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactSpan {
    /// The phase name.
    pub name: &'static str,
    /// The offset from the trace epoch.
    pub start: Duration,
    /// The phase duration.
    pub duration: Duration,
}

/// One named size counter recorded by an artifact build.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactCounter {
    /// The counter name.
    pub name: &'static str,
    /// The counted value.
    pub value: u64,
}

/// One recorded artifact build.
#[derive(Debug, Clone)]
pub struct ArtifactTrace {
    /// The built artifact key.
    pub key: ArtifactKey,
    /// The worker that executed the build.
    pub worker: usize,
    /// The offset from the trace epoch.
    pub start: Duration,
    /// The build duration.
    pub duration: Duration,
    /// The build outcome.
    pub outcome: TraceOutcome,
    /// Interior phases the provider chose to record.
    pub spans: Vec<ArtifactSpan>,
    /// Counters the provider chose to record.
    pub counters: Vec<ArtifactCounter>,
}

/// Collected artifact build traces for one provider run.
///
/// The trace pairs with the dependency edges already recorded on every
/// completed artifact: together they describe where the time went and
/// which artifacts gated which.
#[derive(Debug)]
pub struct ProviderTrace {
    /// The instant all recorded offsets measure from.
    epoch: Instant,
    /// The recorded artifact builds in completion order.
    artifacts: Mutex<Vec<ArtifactTrace>>,
}

impl ProviderTrace {
    /// Create an empty trace starting now.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            epoch: Instant::now(),
            artifacts: Mutex::new(Vec::new()),
        })
    }

    /// Begin tracing one artifact build on one worker.
    pub fn begin(self: &Arc<Self>, key: ArtifactKey, worker: usize) -> ArtifactTracer {
        ArtifactTracer {
            trace: Arc::clone(self),
            key,
            worker,
            started: Instant::now(),
            spans: Mutex::new(Vec::new()),
            counters: Mutex::new(Vec::new()),
        }
    }

    /// Return the recorded artifact builds.
    pub fn artifacts(&self) -> Vec<ArtifactTrace> {
        self.artifacts.lock().clone()
    }

    /// Render an aggregate per-artifact-kind summary table.
    pub fn render_summary(&self) -> String {
        use std::collections::BTreeMap;

        // aggregate total, count, and maximum per kind and outcome
        let mut rows: BTreeMap<(&'static str, &'static str), (Duration, usize, Duration)> =
            BTreeMap::new();
        let artifacts = self.artifacts.lock();
        for artifact in artifacts.iter() {
            let row = rows
                .entry((artifact.key.display_name(), artifact.outcome.name()))
                .or_insert((Duration::ZERO, 0, Duration::ZERO));
            row.0 += artifact.duration;
            row.1 += 1;
            row.2 = row.2.max(artifact.duration);
        }

        // order kinds by their total time
        let mut rows = rows.into_iter().collect::<Vec<_>>();
        rows.sort_by(|left, right| right.1.0.cmp(&left.1.0));

        let mut output = String::new();
        output.push_str("artifact                    outcome     total      count        max\n");
        for ((kind, outcome), (total, count, max)) in rows {
            output.push_str(&format!(
                "{kind:<27} {outcome:<8} {total:>9.2?} {count:>10} {max:>10.2?}\n"
            ));
        }

        output
    }

    /// Render this trace as Chrome trace event JSON.
    /// The output loads directly in Perfetto and chrome://tracing, with
    /// one track per session worker.
    pub fn render_chrome_trace(&self) -> String {
        let mut events = Vec::new();
        let artifacts = self.artifacts.lock();

        for artifact in artifacts.iter() {
            // counters attach as arguments on the artifact slice
            let mut arguments = serde_json::Map::new();
            arguments.insert("outcome".to_string(), json!(artifact.outcome.name()));
            for counter in &artifact.counters {
                arguments.insert(counter.name.to_string(), json!(counter.value));
            }

            events.push(json!({
                "name": format!("{:?}", artifact.key),
                "cat": artifact.key.display_name(),
                "ph": "X",
                "ts": artifact.start.as_micros() as u64,
                "dur": artifact.duration.as_micros() as u64,
                "pid": 1,
                "tid": artifact.worker,
                "args": serde_json::Value::Object(arguments),
            }));

            // interior phases nest under the artifact on the same track
            for span in &artifact.spans {
                events.push(json!({
                    "name": span.name,
                    "cat": "phase",
                    "ph": "X",
                    "ts": span.start.as_micros() as u64,
                    "dur": span.duration.as_micros() as u64,
                    "pid": 1,
                    "tid": artifact.worker,
                }));
            }
        }

        json!({ "traceEvents": events }).to_string()
    }
}

/// Wire-ready report of one provider trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceReport {
    /// The wall time of the traced run in microseconds.
    pub total_micros: u64,
    /// The number of workers that recorded builds.
    pub workers: usize,
    /// Busy time per toolchain stage, ordered by stage.
    pub stages: Vec<TraceStageReport>,
    /// Time spent on builds that blocked on requirements.
    pub blocked_micros: u64,
    /// The recorded artifact builds, present only in detailed reports.
    pub artifacts: Vec<TraceArtifactReport>,
}

/// Busy time of one toolchain stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStageReport {
    /// The stage display name.
    pub name: String,
    /// The summed build time in microseconds.
    pub micros: u64,
}

/// One artifact build in a detailed trace report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceArtifactReport {
    /// The artifact kind name.
    pub name: String,
    /// The toolchain stage display name.
    pub stage: String,
    /// The resolved artifact label, usually a module display name.
    pub label: Option<String>,
    /// The resolved target name for emitted and linked artifacts.
    #[serde(default)]
    pub target: Option<String>,
    /// The worker that executed the build.
    pub worker: usize,
    /// The offset from the run start in microseconds.
    pub start_micros: u64,
    /// The build duration in microseconds.
    pub micros: u64,
    /// The build outcome name.
    pub outcome: String,
}

impl ProviderTrace {
    /// Build the wire-ready report of this trace.
    /// Detailed reports carry every artifact build, labeled through the
    /// resolvers; summary reports carry only the stage rollup. Emit
    /// time splits per target once more than one target emitted.
    pub fn report(
        &self,
        detailed: bool,
        label: impl Fn(&ArtifactKey) -> Option<String>,
        target: impl Fn(TargetId) -> Option<String>,
    ) -> TraceReport {
        let artifacts = self.artifacts.lock();

        // roll up ready time per stage, keeping blocked time separate
        // and emit time keyed by its target
        let mut total = Duration::ZERO;
        let mut blocked = Duration::ZERO;
        let mut workers = 0usize;
        let mut stages: [(ArtifactStage, Duration); 10] = [
            (ArtifactStage::Parse, Duration::ZERO),
            (ArtifactStage::Bind, Duration::ZERO),
            (ArtifactStage::Macro, Duration::ZERO),
            (ArtifactStage::Check, Duration::ZERO),
            (ArtifactStage::Lower, Duration::ZERO),
            (ArtifactStage::Emit, Duration::ZERO),
            (ArtifactStage::Link, Duration::ZERO),
            (ArtifactStage::Lint, Duration::ZERO),
            (ArtifactStage::Query, Duration::ZERO),
            (ArtifactStage::Init, Duration::ZERO),
        ];
        let mut emits: Vec<(Option<TargetId>, Duration)> = Vec::new();
        for artifact in artifacts.iter() {
            total = total.max(artifact.start + artifact.duration);
            workers = workers.max(artifact.worker + 1);
            if artifact.outcome != TraceOutcome::Ready {
                blocked += artifact.duration;
                continue;
            }

            let stage = artifact.key.stage();
            if stage == ArtifactStage::Emit {
                let emit_target = artifact.key.target_id();
                match emits
                    .iter_mut()
                    .find(|(candidate, _)| *candidate == emit_target)
                {
                    Some(row) => row.1 += artifact.duration,
                    None => emits.push((emit_target, artifact.duration)),
                }
                continue;
            }
            let row = stages
                .iter_mut()
                .find(|(candidate, _)| *candidate == stage)
                .expect("every stage has a rollup row");
            row.1 += artifact.duration;
        }

        // a single emitted target keeps the plain stage name
        let split_emit = emits.len() > 1;
        let mut rows = Vec::new();
        for (stage, duration) in stages {
            if stage == ArtifactStage::Emit {
                for (emit_target, duration) in emits.drain(..) {
                    let name = match emit_target.and_then(&target) {
                        Some(name) if split_emit => format!("emit {name}"),
                        _ => stage.name().to_string(),
                    };
                    rows.push((name, duration));
                }
                continue;
            }
            if !duration.is_zero() {
                rows.push((stage.name().to_string(), duration));
            }
        }
        let stages = rows
            .into_iter()
            .map(|(name, duration)| TraceStageReport {
                name,
                micros: duration.as_micros() as u64,
            })
            .collect();

        // detailed reports carry the labeled artifact rows
        let artifacts = if detailed {
            artifacts
                .iter()
                .map(|artifact| TraceArtifactReport {
                    name: artifact.key.display_name().to_string(),
                    stage: artifact.key.stage().name().to_string(),
                    label: label(&artifact.key),
                    target: artifact.key.target_id().and_then(&target),
                    worker: artifact.worker,
                    start_micros: artifact.start.as_micros() as u64,
                    micros: artifact.duration.as_micros() as u64,
                    outcome: artifact.outcome.name().to_string(),
                })
                .collect()
        } else {
            Vec::new()
        };

        TraceReport {
            total_micros: total.as_micros() as u64,
            workers,
            stages,
            blocked_micros: blocked.as_micros() as u64,
            artifacts,
        }
    }
}

/// Tracer buffering one running artifact build.
#[derive(Debug)]
pub struct ArtifactTracer {
    /// The owning trace.
    trace: Arc<ProviderTrace>,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The worker executing the build.
    worker: usize,
    /// The build start.
    started: Instant,
    /// Interior phases recorded so far.
    spans: Mutex<Vec<ArtifactSpan>>,
    /// Counters recorded so far.
    counters: Mutex<Vec<ArtifactCounter>>,
}

impl ArtifactTracer {
    /// Record one interior phase that started at one instant.
    pub fn record_span(&self, name: &'static str, started: Instant) {
        let span = ArtifactSpan {
            name,
            start: started.duration_since(self.trace.epoch),
            duration: started.elapsed(),
        };

        self.spans.lock().push(span);
    }

    /// Record one named counter.
    pub fn record_counter(&self, name: &'static str, value: u64) {
        self.counters.lock().push(ArtifactCounter { name, value });
    }

    /// Finish this artifact build with its outcome.
    pub fn finish(&self, outcome: TraceOutcome) {
        let artifact = ArtifactTrace {
            key: self.key,
            worker: self.worker,
            start: self.started.duration_since(self.trace.epoch),
            duration: self.started.elapsed(),
            outcome,
            spans: std::mem::take(&mut self.spans.lock()),
            counters: std::mem::take(&mut self.counters.lock()),
        };

        self.trace.artifacts.lock().push(artifact);
    }
}
