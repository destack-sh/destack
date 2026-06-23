use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::generate::schema::{Schema, SchemaModule};

use super::package::PythonPackage;
use super::path::protocol_python_segments;

const GENERATED_HEADER: &str = "# generated bridge target, do not edit";
const FORMATTER: &str = "ruff==0.15.18";
const FORMAT_PATHS: &[&str] = &["src", "tests"];

/// Format generated Python bridge files.
pub(in crate::generate) fn format(root: &Path) -> Result<()> {
    let mut command = Command::new("uv");
    command.current_dir(root.join("bridge/python")).arg("run");
    command.arg("--no-project").arg("--with").arg(FORMATTER);
    command.arg("ruff").arg("format");

    // limit ruff to generated python package paths
    for path in FORMAT_PATHS {
        command.arg(path);
    }

    let status = command
        .status()
        .context("failed to run generated Python formatter")?;

    // reject partial formatter failures
    if !status.success() {
        bail!("generated Python formatter failed");
    }

    Ok(())
}

/// Prune generated Python output from previous generator layouts.
pub(super) fn prune_outputs(root: &Path, schema: &Schema) -> Result<()> {
    let generated_root = root.join("bridge/python/src/destack/_generated");
    if generated_root.exists() {
        fs::remove_dir_all(generated_root)?;
    }

    prune_directory(root, "bridge/python/src/destack/session")?;

    for module in &schema.modules {
        prune_file(root, &old_module_facade_path(module))?;
        prune_file(root, &old_module_stub_path(module))?;
    }

    Ok(())
}

/// Prune generated Python protocol files.
pub(super) fn prune_protocol_outputs(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            prune_protocol_outputs(&path)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension == "py" || extension == "pyi")
            && fs::read_to_string(&path)?.starts_with(GENERATED_HEADER)
        {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

/// Prune one stale generated Python directory when every file is generated.
fn prune_directory(root: &Path, path: &str) -> Result<()> {
    let path = root.join(path);
    if !path.exists() {
        return Ok(());
    }

    if is_generated_directory(&path)? {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

/// Return whether one Python directory only contains generated bridge files.
fn is_generated_directory(path: &Path) -> Result<bool> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if !is_generated_directory(&path)? {
                return Ok(false);
            }
        } else if !fs::read_to_string(&path)?.starts_with(GENERATED_HEADER) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Prune one stale generated Python file when it exists.
fn prune_file(root: &Path, path: &str) -> Result<()> {
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

/// Return one generated Python facade path.
pub(super) fn module_facade_path(module: &SchemaModule) -> String {
    format!(
        "bridge/python/src/destack/_generated/{}.py",
        module.path.slash_path()
    )
}

/// Return one generated Python facade stub path.
pub(super) fn module_stub_path(module: &SchemaModule) -> String {
    format!(
        "bridge/python/src/destack/_generated/{}.pyi",
        module.path.slash_path()
    )
}

/// Return one old generated Python facade path.
fn old_module_facade_path(module: &SchemaModule) -> String {
    format!("bridge/python/src/destack/{}.py", module.path.slash_path())
}

/// Return one old generated Python facade stub path.
fn old_module_stub_path(module: &SchemaModule) -> String {
    format!("bridge/python/src/destack/{}.pyi", module.path.slash_path())
}

/// Return one generated Python protocol facade path.
pub(super) fn protocol_module_facade_path(schema: &Schema, module: &SchemaModule) -> String {
    let path = protocol_python_segments(schema, module).join("/");

    format!("bridge/python/src/destack/_generated/{path}.py")
}

/// Return one generated Python package facade path.
pub(super) fn package_facade_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/{path}/__init__.py")
}

/// Return one generated Python package stub path.
pub(super) fn package_stub_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/{path}/__init__.pyi")
}

/// Return one generated Python package facade path.
pub(super) fn generated_package_facade_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/_generated/{path}/__init__.py")
}

/// Return one generated Python package stub path.
pub(super) fn generated_package_stub_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/_generated/{path}/__init__.pyi")
}
