use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::revision;

/// Register generated repository classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    revision::register(module)
}
