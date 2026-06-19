use destack_repository as repository;

use crate::bridge;

/// One bridge trace report.
#[bridge]
#[derive(Debug, Clone, PartialEq)]
pub struct TraceReport {
    /// The wall time of the traced operation in microseconds.
    pub total_micros: u64,
    /// The number of workers that recorded attempts.
    pub workers: usize,
    /// Operation-level spans around artifact execution.
    pub spans: Vec<TraceSpan>,
    /// Operation-level counters.
    pub counters: Vec<TraceCounter>,
    /// Busy time per toolchain stage.
    pub stages: Vec<TraceStage>,
    /// Time spent on attempts that blocked on requirements.
    pub blocked_micros: u64,
    /// Detailed artifact attempts.
    pub artifacts: Vec<TraceArtifact>,
}

/// Busy time of one toolchain stage.
#[bridge]
#[derive(Debug, Clone, PartialEq)]
pub struct TraceStage {
    /// The stage display name.
    pub name: String,
    /// The summed attempt time in microseconds.
    pub micros: u64,
}

/// One artifact attempt in a detailed trace report.
#[bridge]
#[derive(Debug, Clone, PartialEq)]
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
    pub worker: usize,
    /// The offset from the run start in microseconds.
    pub start_micros: u64,
    /// The attempt duration in microseconds.
    pub micros: u64,
    /// The attempt outcome name.
    pub outcome: String,
    /// Interior phases recorded by the executor or provider.
    pub spans: Vec<TraceSpan>,
    /// Counters recorded by the executor or provider.
    pub counters: Vec<TraceCounter>,
}

/// One detailed span in a trace report.
#[bridge]
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSpan {
    /// The phase name.
    pub name: String,
    /// The offset from the run start in microseconds.
    pub start_micros: u64,
    /// The phase duration in microseconds.
    pub micros: u64,
}

/// One detailed counter in a trace report.
#[bridge]
#[derive(Debug, Clone, PartialEq)]
pub struct TraceCounter {
    /// The counter name.
    pub name: String,
    /// The counter value.
    pub value: u64,
}

impl TraceReport {
    /// Convert one repository trace snapshot into one bridge trace report.
    pub fn from_repository(value: repository::TraceSnapshot) -> Self {
        Self {
            total_micros: value.total_micros,
            workers: value.workers,
            spans: value
                .spans
                .into_iter()
                .map(TraceSpan::from_repository)
                .collect(),
            counters: value
                .counters
                .into_iter()
                .map(TraceCounter::from_repository)
                .collect(),
            stages: value
                .stages
                .into_iter()
                .map(TraceStage::from_repository)
                .collect(),
            blocked_micros: value.blocked_micros,
            artifacts: value
                .artifacts
                .into_iter()
                .map(TraceArtifact::from_repository)
                .collect(),
        }
    }
}

impl TraceStage {
    /// Convert one repository stage report into one bridge trace stage.
    fn from_repository(value: repository::TraceStageSnapshot) -> Self {
        Self {
            name: value.name,
            micros: value.micros,
        }
    }
}

impl TraceArtifact {
    /// Convert one repository artifact report into one bridge trace artifact.
    fn from_repository(value: repository::ArtifactAttemptSnapshot) -> Self {
        Self {
            name: value.name,
            stage: value.stage,
            label: value.label,
            target: value.target,
            worker: value.worker,
            start_micros: value.start_micros,
            micros: value.micros,
            outcome: value.outcome,
            spans: value
                .spans
                .into_iter()
                .map(TraceSpan::from_repository)
                .collect(),
            counters: value
                .counters
                .into_iter()
                .map(TraceCounter::from_repository)
                .collect(),
        }
    }
}

impl TraceSpan {
    /// Convert one repository span report into one bridge trace span.
    fn from_repository(value: repository::TraceSpanSnapshot) -> Self {
        Self {
            name: value.name,
            start_micros: value.start_micros,
            micros: value.micros,
        }
    }
}

impl TraceCounter {
    /// Convert one repository counter report into one bridge trace counter.
    fn from_repository(value: repository::TraceCounterSnapshot) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}
