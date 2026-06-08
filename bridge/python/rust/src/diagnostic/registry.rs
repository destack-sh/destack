use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{diagnostic, edit};

/// Register generated diagnostic classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    edit::register(module)?;
    diagnostic::register(module)
}
