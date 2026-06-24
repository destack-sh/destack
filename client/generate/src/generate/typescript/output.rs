use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::generate::schema::{Schema, SchemaModule};

use super::text::GENERATED_HEADER;

const FORMAT_PATHS: &[&str] = &["client/napi", "client/wasm", "client/typescript"];
const GENERATED_ROOT: &str = "client/typescript/src/_generated";
const OLD_GENERATED_ROOT: &str = "client/typescript/src/generated";

/// Format generated TypeScript client files.
pub(in crate::generate) fn format(root: &Path) -> Result<()> {
    let mut command = Command::new("bun");
    command
        .current_dir(root)
        .arg("x")
        .arg("biome")
        .arg("format")
        .arg("--write");

    // limit biome to client packages that receive generated typescript
    for path in FORMAT_PATHS {
        command.arg(path);
    }

    let status = command
        .status()
        .context("failed to run generated TypeScript formatter")?;

    // reject partial formatter failures
    if !status.success() {
        bail!("generated TypeScript formatter failed");
    }

    Ok(())
}

/// Prune generated TypeScript output from previous generator layouts.
pub(super) fn prune_outputs(root: &Path, schema: &Schema) -> Result<()> {
    for path in [GENERATED_ROOT, OLD_GENERATED_ROOT] {
        let path = root.join(path);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    for module in &schema.modules {
        prune_generated_file(root, &old_module_path(module))?;
    }

    Ok(())
}

/// Prune one stale generated TypeScript file when it exists.
pub(super) fn prune_generated_file(root: &Path, path: &str) -> Result<()> {
    let path = root.join(path);
    if !path.exists() {
        return Ok(());
    }

    let source = fs::read_to_string(&path)?;
    if source.starts_with(GENERATED_HEADER) {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Return one generated TypeScript module path.
pub(super) fn module_path(module: &SchemaModule) -> String {
    format!(
        "client/typescript/src/_generated/{}.ts",
        module.path.slash_path()
    )
}

/// Return one old generated TypeScript module path.
fn old_module_path(module: &SchemaModule) -> String {
    format!(
        "client/typescript/src/{}.generated.ts",
        module.path.slash_path()
    )
}
