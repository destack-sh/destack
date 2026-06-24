use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Return the workspace root for this generator invocation.
pub(crate) fn workspace_root() -> Result<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .context("client generator is not inside the workspace")?;

    Ok(root.to_path_buf())
}

/// Write one generated text file.
pub(crate) fn write_text(root: &Path, relative: &str, content: String) -> Result<()> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .with_context(|| format!("generated path has no parent: {}", path.display()))?;
    let content = normalize_text(content);

    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

/// Normalize generated text file endings.
fn normalize_text(mut content: String) -> String {
    while content.ends_with("\n\n") {
        content.pop();
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    content
}
