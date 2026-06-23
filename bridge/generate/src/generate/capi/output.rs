use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Prune generated C ABI output before writing the current schema.
pub(super) fn prune_outputs(root: &Path) -> Result<()> {
    let directory = root.join("bridge/capi/include/destack");

    if directory.exists() {
        prune_headers(&directory)?;
    }

    Ok(())
}

/// Prune generated C ABI headers below one directory.
fn prune_headers(directory: &Path) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read {}", directory.display()))?;
        let path = entry.path();

        // recurse into generated header subdirectories
        if path.is_dir() {
            prune_headers(&path)?;
            continue;
        }

        // remove generated headers only
        let is_generated = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".generated.h"));
        if is_generated {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }

    Ok(())
}
