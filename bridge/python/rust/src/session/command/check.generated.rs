// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{Diagnostic, DirChecked};

/// One checker output.
#[pyclass(name = "CheckOutput", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct CheckOutput {
    pub(crate) value: bridge::CheckOutput,
}

#[pymethods]
impl CheckOutput {
    /// Checked DIR artifact projection.
    #[getter]
    pub fn checked(&self) -> DirChecked {
        DirChecked::from_bridge(self.value.checked.clone())
    }

    /// Diagnostics emitted by checking.
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
impl CheckOutput {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::CheckOutput) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CheckOutput>()?;
    Ok(())
}
