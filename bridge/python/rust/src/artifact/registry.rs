use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{key, sidecar, version};

/// Register generated artifact classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    key::register(module)?;
    sidecar::register(module)?;
    version::register(module)
}
