use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const PACKAGES: &[&str] = &["destack_napi", "destack_wasm"];

/// Format generated Rust client crates.
pub(in crate::generate) fn format(root: &Path) -> Result<()> {
    let mut command = Command::new("cargo");
    command.current_dir(root).arg("fmt");

    // limit rustfmt to crates that receive generated rust
    for package in PACKAGES {
        command.arg("-p").arg(package);
    }

    let status = command
        .status()
        .context("failed to run generated Rust formatter")?;

    // reject partial formatter failures
    if !status.success() {
        bail!("generated Rust formatter failed");
    }

    Ok(())
}
