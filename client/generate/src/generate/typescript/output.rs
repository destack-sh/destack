use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::generate::schema::SchemaModule;

use super::text::GENERATED_HEADER;

const FORMAT_PATHS: &[&str] = &["client/napi", "client/wasm", "client/typescript"];
const GENERATED_ROOT: &str = "client/typescript/src/_generated";
const OLD_GENERATED_ROOT: &str = "client/typescript/src/generated";
const TYPESCRIPT_SOURCE_ROOT: &str = "client/typescript/src";

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
pub(super) fn prune_outputs(root: &Path) -> Result<()> {
    for path in [GENERATED_ROOT, OLD_GENERATED_ROOT] {
        let path = root.join(path);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    let source = root.join(TYPESCRIPT_SOURCE_ROOT);
    prune_old_generated_files(&source)?;

    Ok(())
}

/// Prune stale generated TypeScript files below one directory.
fn prune_old_generated_files(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        // descend into source directories or inspect one legacy output
        if file_type.is_dir() {
            prune_old_generated_files(&path)?;
        } else {
            let name = entry.file_name();
            let name = name.to_str().with_context(|| {
                format!("TypeScript source path is not UTF-8: {}", path.display())
            })?;

            // delete only legacy files carrying the generator ownership header
            if name.ends_with(".generated.ts") {
                let source = fs::read_to_string(&path)?;
                if source.starts_with(GENERATED_HEADER) {
                    fs::remove_file(path)?;
                }
            }
        }
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
