use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{File, FileId, FileSystem, FileType, Uri};
use serde::Deserialize;

use crate::{Destack, RepositoryError, WorkspacesField};

/// The workspace field extracted from one `package.json` file.
#[derive(Debug, Deserialize)]
struct PackageJsonWorkspaces {
    /// The optional workspace declaration.
    workspaces: Option<WorkspacesField>,
}

/// Discover one workspace root from one path.
pub(crate) fn discover_workspace_root(
    fs: &Arc<dyn FileSystem>,
    path: &Path,
) -> Result<PathBuf, RepositoryError> {
    // walk up directories looking for a workspace root
    let mut current = path.to_path_buf();
    loop {
        // check for a Destack workspace root
        if let Some(root) = check_destack_workspace(fs, &current)? {
            return Ok(root);
        }

        // check for workspaces in package.json
        if let Some(root) = check_npm_workspace(fs, &current)? {
            return Ok(root);
        }

        // check for pnpm workspace
        if let Some(root) = check_pnpm_workspace(fs, &current)? {
            return Ok(root);
        }

        // treat a plain package boundary as the workspace root
        let package_json_path = current.join("package.json");
        if fs
            .metadata(&package_json_path)
            .is_ok_and(|metadata| metadata.is_file)
        {
            return Ok(current.clone());
        }

        // move up to the parent
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }

    Ok(path.to_path_buf())
}

/// Check if one directory contains a Destack workspace root.
fn check_destack_workspace(
    fs: &Arc<dyn FileSystem>,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let Some(config) = read_workspace_destack_config(fs, directory)? else {
        return Ok(None);
    };

    if config.options.workspace.members.is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Check if one directory contains an npm or yarn workspace root.
fn check_npm_workspace(
    fs: &Arc<dyn FileSystem>,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let package_json_path = directory.join("package.json");
    let bytes = match fs.read(&package_json_path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };

    let (name, uri) = Uri::from_path_with_name(&package_json_path);
    let file_id = FileId::from_logical_path(&package_json_path);
    let file = File::from_bytes_as_json(
        file_id,
        name,
        uri,
        Some(package_json_path.clone()),
        FileType::Json,
        bytes,
    )
    .map_err(|error| RepositoryError::WorkspaceDiscovery {
        path: package_json_path.clone(),
        message: error.to_string(),
    })?;

    let destack_source::FileContent::Json { value, .. } = &file.content else {
        return Ok(None);
    };

    let package: PackageJsonWorkspaces =
        serde_json::from_value(value.clone()).map_err(|error| {
            RepositoryError::WorkspaceDiscovery {
                path: package_json_path.clone(),
                message: error.to_string(),
            }
        })?;

    let Some(workspaces) = package.workspaces else {
        return Ok(None);
    };

    if workspaces.patterns().is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Check if one directory contains a pnpm workspace root.
fn check_pnpm_workspace(
    fs: &Arc<dyn FileSystem>,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let workspace_path = directory.join("pnpm-workspace.yaml");
    let content = match fs.read_to_string(&workspace_path) {
        Ok(content) => content,
        Err(_) => return Ok(None),
    };

    let patterns = parse_pnpm_workspace_packages(&content);
    if patterns.is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Read one workspace root `destack.json` when present.
fn read_workspace_destack_config(
    fs: &Arc<dyn FileSystem>,
    root: &Path,
) -> Result<Option<Destack>, RepositoryError> {
    let path = root.join("destack.json");
    let content = match fs.read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return Ok(None),
    };

    let file = File::from_text_as_jsonc(
        FileId::from_logical_path(&path),
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "destack.json".to_string()),
        Uri::from_path(&path),
        Some(path.clone()),
        FileType::Json,
        content,
    )
    .map_err(|error| RepositoryError::WorkspaceDiscovery {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let path = path.clone();
    let file = Arc::new(file);

    Destack::parse(&file)
        .map(Some)
        .map_err(|error| RepositoryError::WorkspaceDiscovery {
            path,
            message: error.to_string(),
        })
}

/// Parse the packages field from `pnpm-workspace.yaml` content.
fn parse_pnpm_workspace_packages(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut is_in_packages = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // enter the packages section
        if trimmed.starts_with("packages:") {
            is_in_packages = true;

            // handle inline arrays
            if let Some(rest) = trimmed.strip_prefix("packages:") {
                let rest = rest.trim();
                if rest.starts_with('[') && rest.ends_with(']') {
                    let inner = &rest[1..rest.len() - 1];
                    for item in inner.split(',') {
                        let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
                        if !item.is_empty() {
                            patterns.push(item.to_string());
                        }
                    }
                    is_in_packages = false;
                }
            }

            continue;
        }

        // parse list items inside the packages section
        if is_in_packages {
            // stop at the next top level key
            if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('#') {
                is_in_packages = false;
                continue;
            }

            // parse one list item
            if let Some(item) = trimmed.strip_prefix('-') {
                let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
                if !item.is_empty() {
                    patterns.push(item.to_string());
                }
            }
        }
    }

    patterns
}
