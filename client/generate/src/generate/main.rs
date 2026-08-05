use anyhow::Result;

use super::core::workspace_root;
use super::schema::Schema;
use super::{rust, typescript};

/// Generate client target bindings.
pub(crate) fn run() -> Result<()> {
    let root = workspace_root()?;
    let schema = Schema::load()?;

    schema.validate()?;
    typescript::generate(&root, &schema)?;
    rust::format(&root)?;
    typescript::format(&root)?;

    Ok(())
}
