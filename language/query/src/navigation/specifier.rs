use std::path::{Path, PathBuf};

use destack_source::FileSystem;

/// One resolved target for one document link specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SpecifierLinkTarget {
    /// One filesystem path target.
    File { path: PathBuf },
    /// One url target.
    Url { url: String },
}

/// Resolve one document link target from a raw module specifier.
pub(super) fn resolve_document_link_target(
    fs: &dyn FileSystem,
    base_directory: &Path,
    specifier: &str,
) -> Option<SpecifierLinkTarget> {
    // handle urls directly
    if specifier.starts_with("http://") || specifier.starts_with("https://") {
        return Some(SpecifierLinkTarget::Url {
            url: specifier.to_string(),
        });
    }

    // handle file urls
    if let Some(path) = specifier.strip_prefix("file://") {
        return Some(SpecifierLinkTarget::File {
            path: resolve_file_target(fs, PathBuf::from(path)),
        });
    }

    // handle relative or absolute paths
    if specifier.starts_with('.') || specifier.starts_with('/') {
        let mut path = PathBuf::from(specifier);
        if path.is_relative() {
            path = base_directory.join(path);
        }

        return Some(SpecifierLinkTarget::File {
            path: resolve_file_target(fs, path),
        });
    }

    None
}

/// Resolve one file path target for a module specifier.
fn resolve_file_target(fs: &dyn FileSystem, path: PathBuf) -> PathBuf {
    let candidates = document_link_candidates(&path);

    candidates
        .into_iter()
        .find(|candidate| {
            fs.metadata(candidate)
                .map(|metadata| metadata.is_file || metadata.is_directory || metadata.is_symlink)
                .unwrap_or(false)
        })
        .unwrap_or(path)
}

/// Build candidate paths for one module specifier.
fn document_link_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(path.to_path_buf());

    if path.extension().is_some() {
        return candidates;
    }

    let extensions = ["ds", "d.ts", "ts", "tsx"];
    for extension in extensions {
        candidates.push(path.with_extension(extension));
    }

    for extension in extensions {
        candidates.push(path.join(format!("index.{extension}")));
    }

    candidates
}
