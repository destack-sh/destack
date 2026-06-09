use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{checked, parsed, resolved};

/// Register generated DIR classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    parsed::register(module)?;
    resolved::register(module)?;
    checked::register(module)
}
