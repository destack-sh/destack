use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const RUST_PACKAGES: &[&str] = &[
    "destack_capi",
    "destack_napi",
    "destack_wasm",
    "destack_python",
];
const BIOME_PATHS: &[&str] = &["bridge/napi", "bridge/wasm", "bridge/typescript"];

/// Format generated bridge targets.
pub(crate) fn run(root: &Path) -> Result<()> {
    // format generated rust bindings
    format_rust(root)?;

    // format generated typescript bindings
    format_typescript(root)?;

    Ok(())
}

/// Format generated Rust bridge crates.
fn format_rust(root: &Path) -> Result<()> {
    // build one workspace cargo fmt command
    let mut command = Command::new("cargo");
    command.current_dir(root).arg("fmt");

    // limit rustfmt to crates that receive generated rust
    for package in RUST_PACKAGES {
        command.arg("-p").arg(package);
    }

    run_command(command, "cargo fmt")
}

/// Format generated TypeScript bridge targets.
fn format_typescript(root: &Path) -> Result<()> {
    // build one biome format command
    let mut command = Command::new("bun");
    command
        .current_dir(root)
        .arg("x")
        .arg("biome")
        .arg("format")
        .arg("--write");

    // limit biome to generated bridge packages
    for path in BIOME_PATHS {
        command.arg(path);
    }

    run_command(command, "biome format")
}

/// Run one formatter command.
fn run_command(mut command: Command, label: &str) -> Result<()> {
    // run the formatter directly so failures are loud
    let status = command
        .status()
        .with_context(|| format!("failed to run {label}"))?;

    // reject partial formatter failures
    if !status.success() {
        bail!("{label} failed");
    }

    Ok(())
}
