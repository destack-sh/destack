// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{Diagnostic, DirParsed};

/// One parser output.
#[pyclass(name = "ParseOutput", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ParseOutput {
    pub(crate) value: bridge::ParseOutput,
}

#[pymethods]
impl ParseOutput {
    /// Parsed DIR artifact projection.
    #[getter]
    pub fn parsed(&self) -> DirParsed {
        DirParsed::from_bridge(self.value.parsed.clone())
    }

    /// Diagnostics emitted by parsing.
    #[getter]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.value
            .diagnostics
            .clone()
            .into_iter()
            .map(Diagnostic::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl ParseOutput {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ParseOutput) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ParseOutput>()?;
    Ok(())
}
