use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{file, snapshot, update};

/// Register generated source classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    file::register(module)?;
    snapshot::register(module)?;
    update::register(module)
}
