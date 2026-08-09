use anyhow::Result;

use super::core::workspace_root;
use super::schema::Schema;
use super::typescript;

/// Generate client target bindings.
pub(crate) fn run() -> Result<()> {
    let root = workspace_root()?;
    let schema = Schema::load()?;

    typescript::generate(&root, &schema)?;
    typescript::format(&root)?;

    Ok(())
}
