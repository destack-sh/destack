// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// One bridge trace report.
#[pyclass(name = "TraceReport", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TraceReport {
    pub(crate) value: bridge::TraceReport,
}

#[pymethods]
impl TraceReport {
    /// The wall time of the traced operation in microseconds.
    #[getter]
    pub fn total_micros(&self) -> u64 {
        self.value.total_micros
    }

    /// The number of workers that recorded attempts.
    #[getter]
    pub fn workers(&self) -> usize {
        self.value.workers
    }

    /// Operation-level spans around artifact execution.
    #[getter]
    pub fn spans(&self) -> Vec<TraceSpan> {
        self.value
            .spans
            .clone()
            .into_iter()
            .map(TraceSpan::from_bridge)
            .collect()
    }

    /// Operation-level counters.
    #[getter]
    pub fn counters(&self) -> Vec<TraceCounter> {
        self.value
            .counters
            .clone()
            .into_iter()
            .map(TraceCounter::from_bridge)
            .collect()
    }

    /// Busy time per toolchain stage.
    #[getter]
    pub fn stages(&self) -> Vec<TraceStage> {
        self.value
            .stages
            .clone()
            .into_iter()
            .map(TraceStage::from_bridge)
            .collect()
    }

    /// Time spent on attempts that blocked on requirements.
    #[getter]
    pub fn blocked_micros(&self) -> u64 {
        self.value.blocked_micros
    }

    /// Detailed artifact attempts.
    #[getter]
    pub fn artifacts(&self) -> Vec<TraceArtifact> {
        self.value
            .artifacts
            .clone()
            .into_iter()
            .map(TraceArtifact::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl TraceReport {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TraceReport) -> Self {
        Self { value }
    }
}

/// Busy time of one toolchain stage.
#[pyclass(name = "TraceStage", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TraceStage {
    pub(crate) value: bridge::TraceStage,
}

#[pymethods]
impl TraceStage {
    /// The stage display name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The summed attempt time in microseconds.
    #[getter]
    pub fn micros(&self) -> u64 {
        self.value.micros
    }
}

#[allow(dead_code)]
impl TraceStage {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TraceStage) -> Self {
        Self { value }
    }
}

/// One artifact attempt in a detailed trace report.
#[pyclass(name = "TraceArtifact", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TraceArtifact {
    pub(crate) value: bridge::TraceArtifact,
}

#[pymethods]
impl TraceArtifact {
    /// The artifact kind name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The toolchain stage display name.
    #[getter]
    pub fn stage(&self) -> String {
        self.value.stage.clone()
    }

    /// The resolved artifact label.
    #[getter]
    pub fn label(&self) -> Option<String> {
        self.value.label.clone()
    }

    /// The resolved target name.
    #[getter]
    pub fn target(&self) -> Option<String> {
        self.value.target.clone()
    }

    /// The worker that executed the attempt.
    #[getter]
    pub fn worker(&self) -> usize {
        self.value.worker
    }

    /// The offset from the run start in microseconds.
    #[getter]
    pub fn start_micros(&self) -> u64 {
        self.value.start_micros
    }

    /// The attempt duration in microseconds.
    #[getter]
    pub fn micros(&self) -> u64 {
        self.value.micros
    }

    /// The attempt outcome name.
    #[getter]
    pub fn outcome(&self) -> String {
        self.value.outcome.clone()
    }

    /// Interior phases recorded by the executor or provider.
    #[getter]
    pub fn spans(&self) -> Vec<TraceSpan> {
        self.value
            .spans
            .clone()
            .into_iter()
            .map(TraceSpan::from_bridge)
            .collect()
    }

    /// Counters recorded by the executor or provider.
    #[getter]
    pub fn counters(&self) -> Vec<TraceCounter> {
        self.value
            .counters
            .clone()
            .into_iter()
            .map(TraceCounter::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl TraceArtifact {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TraceArtifact) -> Self {
        Self { value }
    }
}

/// One detailed span in a trace report.
#[pyclass(name = "TraceSpan", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub(crate) value: bridge::TraceSpan,
}

#[pymethods]
impl TraceSpan {
    /// The phase name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The offset from the run start in microseconds.
    #[getter]
    pub fn start_micros(&self) -> u64 {
        self.value.start_micros
    }

    /// The phase duration in microseconds.
    #[getter]
    pub fn micros(&self) -> u64 {
        self.value.micros
    }
}

#[allow(dead_code)]
impl TraceSpan {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TraceSpan) -> Self {
        Self { value }
    }
}

/// One detailed counter in a trace report.
#[pyclass(name = "TraceCounter", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TraceCounter {
    pub(crate) value: bridge::TraceCounter,
}

#[pymethods]
impl TraceCounter {
    /// The counter name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The counter value.
    #[getter]
    pub fn value(&self) -> u64 {
        self.value.value
    }
}

#[allow(dead_code)]
impl TraceCounter {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TraceCounter) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TraceReport>()?;
    module.add_class::<TraceStage>()?;
    module.add_class::<TraceArtifact>()?;
    module.add_class::<TraceSpan>()?;
    module.add_class::<TraceCounter>()?;
    Ok(())
}
