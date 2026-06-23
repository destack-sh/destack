use anyhow::Result;

use super::core::workspace_root;
use super::schema::Schema;
use super::{capi, python, rust, typescript};

/// Generate bridge target bindings.
pub(crate) fn run() -> Result<()> {
    let root = workspace_root()?;
    let schema = Schema::load()?;
    let protocol_schema = Schema::load_protocol()?;

    schema.validate()?;
    protocol_schema.validate()?;
    capi::generate(&root, &schema)?;
    python::generate(&root, &schema)?;
    python::generate_protocol(&root, &protocol_schema)?;
    typescript::generate(&root, &schema)?;
    typescript::generate_protocol(&root, &protocol_schema)?;
    typescript::generate_index(&root, &schema)?;
    rust::format(&root)?;
    python::format(&root)?;
    typescript::format(&root)?;

    Ok(())
}
