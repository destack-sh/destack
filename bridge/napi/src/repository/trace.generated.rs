// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// One bridge trace report.
#[derive(Debug)]
#[napi(object, js_name = "TraceReport")]
pub struct TraceReport {
    /// The wall time of the traced operation in microseconds.
    pub total_micros: f64,
    /// The number of workers that recorded attempts.
    pub workers: u32,
    /// Operation-level spans around artifact execution.
    pub spans: Vec<TraceSpan>,
    /// Operation-level counters.
    pub counters: Vec<TraceCounter>,
    /// Busy time per toolchain stage.
    pub stages: Vec<TraceStage>,
    /// Time spent on attempts that blocked on requirements.
    pub blocked_micros: f64,
    /// Detailed artifact attempts.
    pub artifacts: Vec<TraceArtifact>,
}

impl TraceReport {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TraceReport) -> Self {
        Self {
            total_micros: value.total_micros as f64,
            workers: value.workers as u32,
            spans: value
                .spans
                .into_iter()
                .map(TraceSpan::from_bridge)
                .collect(),
            counters: value
                .counters
                .into_iter()
                .map(TraceCounter::from_bridge)
                .collect(),
            stages: value
                .stages
                .into_iter()
                .map(TraceStage::from_bridge)
                .collect(),
            blocked_micros: value.blocked_micros as f64,
            artifacts: value
                .artifacts
                .into_iter()
                .map(TraceArtifact::from_bridge)
                .collect(),
        }
    }
}

/// Busy time of one toolchain stage.
#[derive(Debug)]
#[napi(object, js_name = "TraceStage")]
pub struct TraceStage {
    /// The stage display name.
    pub name: String,
    /// The summed attempt time in microseconds.
    pub micros: f64,
}

impl TraceStage {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TraceStage) -> Self {
        Self {
            name: value.name,
            micros: value.micros as f64,
        }
    }
}

/// One artifact attempt in a detailed trace report.
#[derive(Debug)]
#[napi(object, js_name = "TraceArtifact")]
pub struct TraceArtifact {
    /// The artifact kind name.
    pub name: String,
    /// The toolchain stage display name.
    pub stage: String,
    /// The resolved artifact label.
    pub label: Option<String>,
    /// The resolved target name.
    pub target: Option<String>,
    /// The worker that executed the attempt.
    pub worker: u32,
    /// The offset from the run start in microseconds.
    pub start_micros: f64,
    /// The attempt duration in microseconds.
    pub micros: f64,
    /// The attempt outcome name.
    pub outcome: String,
    /// Interior phases recorded by the executor or provider.
    pub spans: Vec<TraceSpan>,
    /// Counters recorded by the executor or provider.
    pub counters: Vec<TraceCounter>,
}

impl TraceArtifact {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TraceArtifact) -> Self {
        Self {
            name: value.name,
            stage: value.stage,
            label: value.label,
            target: value.target,
            worker: value.worker as u32,
            start_micros: value.start_micros as f64,
            micros: value.micros as f64,
            outcome: value.outcome,
            spans: value
                .spans
                .into_iter()
                .map(TraceSpan::from_bridge)
                .collect(),
            counters: value
                .counters
                .into_iter()
                .map(TraceCounter::from_bridge)
                .collect(),
        }
    }
}

/// One detailed span in a trace report.
#[derive(Debug)]
#[napi(object, js_name = "TraceSpan")]
pub struct TraceSpan {
    /// The phase name.
    pub name: String,
    /// The offset from the run start in microseconds.
    pub start_micros: f64,
    /// The phase duration in microseconds.
    pub micros: f64,
}

impl TraceSpan {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TraceSpan) -> Self {
        Self {
            name: value.name,
            start_micros: value.start_micros as f64,
            micros: value.micros as f64,
        }
    }
}

/// One detailed counter in a trace report.
#[derive(Debug)]
#[napi(object, js_name = "TraceCounter")]
pub struct TraceCounter {
    /// The counter name.
    pub name: String,
    /// The counter value.
    pub value: f64,
}

impl TraceCounter {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::TraceCounter) -> Self {
        Self {
            name: value.name,
            value: value.value as f64,
        }
    }
}
