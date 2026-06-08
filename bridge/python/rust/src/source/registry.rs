use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::{component, file, module, package, profile, span, target};

/// Register generated source id classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    package::register(module)?;
    module::register(module)?;
    profile::register(module)?;
    component::register(module)?;
    file::register(module)?;
    span::register(module)?;
    target::register(module)
}
