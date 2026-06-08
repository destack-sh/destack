use anyhow::Result;

use super::core::{Schema, workspace_root};
use super::{napi, python, typescript, wasm};

/// Generate bridge target bindings.
pub(crate) fn run() -> Result<()> {
    let root = workspace_root()?;
    let schema = Schema::load(&root)?;

    schema.validate()?;
    napi::generate(&root, &schema)?;
    python::generate(&root, &schema)?;
    wasm::generate(&root, &schema)?;
    typescript::generate(&root, &schema)?;

    Ok(())
}
