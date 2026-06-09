use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{file, source, update};

/// Register generated source classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    file::register(module)?;
    source::register(module)?;
    update::register(module)
}
