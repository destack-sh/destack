use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::path::GENERATED_ROOT;

/// Format generated TypeScript client files.
pub(in crate::generate) fn format(root: &Path) -> Result<()> {
    // format the generated TypeScript files
    let status = Command::new("bun")
        .current_dir(root)
        .arg("x")
        .arg("@biomejs/biome")
        .arg("format")
        .arg("--write")
        .arg(GENERATED_ROOT)
        .status()
        .context("failed to run generated TypeScript formatter")?;

    // reject partial formatter failures
    if !status.success() {
        bail!("generated TypeScript formatter failed");
    }

    Ok(())
}
