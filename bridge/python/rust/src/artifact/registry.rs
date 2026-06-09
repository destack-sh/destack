use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{dependency, key, record, sidecar, version};

/// Register generated artifact classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    dependency::register(module)?;
    key::register(module)?;
    record::register(module)?;
    sidecar::register(module)?;
    version::register(module)
}
