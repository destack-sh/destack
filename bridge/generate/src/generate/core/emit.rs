use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use proc_macro2::TokenStream;

const HEADER: &str = "// generated bridge target, do not edit\n\n";

/// Return the workspace root for this generator invocation.
pub(crate) fn workspace_root() -> Result<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .context("bridge generator is not inside the workspace")?;

    Ok(root.to_path_buf())
}

/// Write one generated Rust file.
pub(crate) fn write_rust(root: &Path, relative: &str, tokens: TokenStream) -> Result<()> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .with_context(|| format!("generated path has no parent: {}", path.display()))?;
    let content = render_rust(tokens)
        .with_context(|| format!("failed to parse generated Rust for {}", path.display()))?;

    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    format_rust_file(&path)?;

    Ok(())
}

/// Write one generated text file.
pub(crate) fn write_text(root: &Path, relative: &str, content: String) -> Result<()> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .with_context(|| format!("generated path has no parent: {}", path.display()))?;

    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

/// Render generated Rust tokens as source text.
fn render_rust(tokens: TokenStream) -> Result<String> {
    let file: syn::File = syn::parse2(tokens)?;
    let items = file
        .items
        .into_iter()
        .map(|item| {
            let file = syn::File {
                shebang: None,
                attrs: Vec::new(),
                items: vec![item],
            };

            prettyplease::unparse(&file).trim().to_string()
        })
        .collect::<Vec<_>>();
    let content = items.join("\n\n");
    let content = content.replace("\n    }\n    ///", "\n    }\n\n    ///");
    let content = format!("{HEADER}{content}\n");

    Ok(content)
}

/// Format one generated Rust file.
fn format_rust_file(path: &Path) -> Result<()> {
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(path)
        .status()
        .with_context(|| format!("failed to run rustfmt for {}", path.display()))?;
    if !status.success() {
        bail!("rustfmt failed for {}", path.display());
    }

    Ok(())
}
