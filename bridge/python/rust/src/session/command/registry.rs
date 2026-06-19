use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{check, format, lint, parse};

/// Register generated command classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    check::register(module)?;
    format::register(module)?;
    lint::register(module)?;
    parse::register(module)
}
