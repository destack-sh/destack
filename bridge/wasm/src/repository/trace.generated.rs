// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

/// One bridge trace report.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceReport {
    total_micros: f64,
    workers: u32,
    spans: Vec<TraceSpan>,
    counters: Vec<TraceCounter>,
    stages: Vec<TraceStage>,
    times: Vec<TraceTime>,
    artifacts: Vec<TraceArtifact>,
}

#[wasm_bindgen]
impl TraceReport {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        total_micros: f64,
        workers: u32,
        spans: Vec<TraceSpan>,
        counters: Vec<TraceCounter>,
        stages: Vec<TraceStage>,
        times: Vec<TraceTime>,
        artifacts: Vec<TraceArtifact>,
    ) -> Self {
        Self {
            total_micros,
            workers,
            spans,
            counters,
            stages,
            times,
            artifacts,
        }
    }

    /// The wall time of the traced operation in microseconds.
    #[wasm_bindgen(getter, js_name = "totalMicros")]
    pub fn total_micros(&self) -> f64 {
        self.total_micros
    }

    /// The number of workers that recorded attempts.
    #[wasm_bindgen(getter, js_name = "workers")]
    pub fn workers(&self) -> u32 {
        self.workers
    }

    /// Operation-level spans around artifact execution.
    #[wasm_bindgen(getter, js_name = "spans")]
    pub fn spans(&self) -> Vec<TraceSpan> {
        self.spans.clone()
    }

    /// Operation-level counters.
    #[wasm_bindgen(getter, js_name = "counters")]
    pub fn counters(&self) -> Vec<TraceCounter> {
        self.counters.clone()
    }

    /// Busy time per toolchain stage.
    #[wasm_bindgen(getter, js_name = "stages")]
    pub fn stages(&self) -> Vec<TraceStage> {
        self.stages.clone()
    }

    /// Summed time per named trace span.
    #[wasm_bindgen(getter, js_name = "times")]
    pub fn times(&self) -> Vec<TraceTime> {
        self.times.clone()
    }

    /// Detailed artifact attempts.
    #[wasm_bindgen(getter, js_name = "artifacts")]
    pub fn artifacts(&self) -> Vec<TraceArtifact> {
        self.artifacts.clone()
    }
}

impl TraceReport {
    /// Convert one bridge value into one WASM value.
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
            times: value
                .times
                .into_iter()
                .map(TraceTime::from_bridge)
                .collect(),
            artifacts: value
                .artifacts
                .into_iter()
                .map(TraceArtifact::from_bridge)
                .collect(),
        }
    }
}

/// Busy time of one toolchain stage.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceStage {
    name: String,
    micros: f64,
}

#[wasm_bindgen]
impl TraceStage {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, micros: f64) -> Self {
        Self { name, micros }
    }

    /// The stage display name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The summed attempt time in microseconds.
    #[wasm_bindgen(getter, js_name = "micros")]
    pub fn micros(&self) -> f64 {
        self.micros
    }
}

impl TraceStage {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::TraceStage) -> Self {
        Self {
            name: value.name,
            micros: value.micros as f64,
        }
    }
}

/// Summed time of one named trace span.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceTime {
    name: String,
    micros: f64,
}

#[wasm_bindgen]
impl TraceTime {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, micros: f64) -> Self {
        Self { name, micros }
    }

    /// The span name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The summed span time in microseconds.
    #[wasm_bindgen(getter, js_name = "micros")]
    pub fn micros(&self) -> f64 {
        self.micros
    }
}

impl TraceTime {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::TraceTime) -> Self {
        Self {
            name: value.name,
            micros: value.micros as f64,
        }
    }
}

/// One artifact attempt in a detailed trace report.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceArtifact {
    name: String,
    stage: String,
    label: Option<String>,
    target: Option<String>,
    worker: u32,
    start_micros: f64,
    micros: f64,
    outcome: String,
    spans: Vec<TraceSpan>,
    counters: Vec<TraceCounter>,
}

#[wasm_bindgen]
impl TraceArtifact {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: String,
        stage: String,
        label: Option<String>,
        target: Option<String>,
        worker: u32,
        start_micros: f64,
        micros: f64,
        outcome: String,
        spans: Vec<TraceSpan>,
        counters: Vec<TraceCounter>,
    ) -> Self {
        Self {
            name,
            stage,
            label,
            target,
            worker,
            start_micros,
            micros,
            outcome,
            spans,
            counters,
        }
    }

    /// The artifact kind name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The toolchain stage display name.
    #[wasm_bindgen(getter, js_name = "stage")]
    pub fn stage(&self) -> String {
        self.stage.clone()
    }

    /// The resolved artifact label.
    #[wasm_bindgen(getter, js_name = "label")]
    pub fn label(&self) -> Option<String> {
        self.label.clone()
    }

    /// The resolved target name.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> Option<String> {
        self.target.clone()
    }

    /// The worker that executed the attempt.
    #[wasm_bindgen(getter, js_name = "worker")]
    pub fn worker(&self) -> u32 {
        self.worker
    }

    /// The offset from the run start in microseconds.
    #[wasm_bindgen(getter, js_name = "startMicros")]
    pub fn start_micros(&self) -> f64 {
        self.start_micros
    }

    /// The attempt duration in microseconds.
    #[wasm_bindgen(getter, js_name = "micros")]
    pub fn micros(&self) -> f64 {
        self.micros
    }

    /// The attempt outcome name.
    #[wasm_bindgen(getter, js_name = "outcome")]
    pub fn outcome(&self) -> String {
        self.outcome.clone()
    }

    /// Interior phases recorded by the executor or provider.
    #[wasm_bindgen(getter, js_name = "spans")]
    pub fn spans(&self) -> Vec<TraceSpan> {
        self.spans.clone()
    }

    /// Counters recorded by the executor or provider.
    #[wasm_bindgen(getter, js_name = "counters")]
    pub fn counters(&self) -> Vec<TraceCounter> {
        self.counters.clone()
    }
}

impl TraceArtifact {
    /// Convert one bridge value into one WASM value.
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
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceSpan {
    name: String,
    start_micros: f64,
    micros: f64,
}

#[wasm_bindgen]
impl TraceSpan {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, start_micros: f64, micros: f64) -> Self {
        Self {
            name,
            start_micros,
            micros,
        }
    }

    /// The phase name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The offset from the run start in microseconds.
    #[wasm_bindgen(getter, js_name = "startMicros")]
    pub fn start_micros(&self) -> f64 {
        self.start_micros
    }

    /// The phase duration in microseconds.
    #[wasm_bindgen(getter, js_name = "micros")]
    pub fn micros(&self) -> f64 {
        self.micros
    }
}

impl TraceSpan {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::TraceSpan) -> Self {
        Self {
            name: value.name,
            start_micros: value.start_micros as f64,
            micros: value.micros as f64,
        }
    }
}

/// One detailed counter in a trace report.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TraceCounter {
    name: String,
    value: f64,
}

#[wasm_bindgen]
impl TraceCounter {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, value: f64) -> Self {
        Self { name, value }
    }

    /// The counter name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The counter value.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl TraceCounter {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::TraceCounter) -> Self {
        Self {
            name: value.name,
            value: value.value as f64,
        }
    }
}
