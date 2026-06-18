use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{file, format, lint, module, source};

/// Register generated session classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    file::register(module)?;
    format::register(module)?;
    lint::register(module)?;
    module::register(module)?;
    source::register(module)
}
