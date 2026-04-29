use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{File, FileId, FileSystem, FileType, Uri};
use destack_workspace::{
    DestackDeclaration, HostEnvironment, Ref, Repository, RepositoryError, WorkspacesField,
    parse_json_file,
};
use serde::Deserialize;

/// The workspace field extracted from one `package.json` file.
#[derive(Debug, Deserialize)]
struct PackageJsonWorkspaces {
    /// The optional workspace declaration.
    workspaces: Option<WorkspacesField>,
}

/// The packages field extracted from one `pnpm-workspace.yaml` file.
#[derive(Debug, Deserialize)]
struct PnpmWorkspacePackages {
    /// The optional workspace package globs.
    packages: Option<Vec<String>>,
}

/// Open one repository after discovering the workspace root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    fs: Arc<dyn FileSystem>,
    environment: HostEnvironment,
) -> Result<Repository, RepositoryError> {
    let root = discover_workspace_root(fs.as_ref(), &path)?;

    let repository = Repository::new(
        root.clone(),
        Arc::new(DiskCacheStore::new()),
        fs,
        environment,
    );
    let workspace_ref = Ref::for_workspace_root(&root);

    repository.current(&workspace_ref)?;

    Ok(repository)
}

/// Discover one workspace root from one input path.
fn discover_workspace_root(fs: &dyn FileSystem, path: &Path) -> Result<PathBuf, RepositoryError> {
    // normalize file inputs to their containing directory
    let input_directory = match fs.metadata(path) {
        Ok(metadata) if metadata.is_file => path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf()),
        _ => path.to_path_buf(),
    };
    let mut current = input_directory.clone();

    // walk up directories looking for a workspace root
    loop {
        // destack workspace root
        if let Some(root) = check_destack_workspace(fs, &current)? {
            return Ok(root);
        }

        // npm or yarn workspace root
        if let Some(root) = check_npm_workspace(fs, &current)? {
            return Ok(root);
        }

        // pnpm workspace root
        if let Some(root) = check_pnpm_workspace(fs, &current)? {
            return Ok(root);
        }

        // plain package boundary
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

    Ok(input_directory)
}

/// Check if one directory contains a Destack workspace root.
fn check_destack_workspace(
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let Some(config) = read_workspace_destack_config(fs, directory)? else {
        return Ok(None);
    };

    if config.workspace_options().membership.members.is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Check if one directory contains an npm or yarn workspace root.
fn check_npm_workspace(
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let package_json_path = directory.join("package.json");
    let bytes = match fs.read(&package_json_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(RepositoryError::WorkspaceRootDiscovery {
                path: package_json_path,
                message: error.to_string(),
            });
        }
    };

    let (name, uri) = Uri::from_path_with_name(&package_json_path);
    let file_id = FileId::from_logical_path(&package_json_path);
    let content =
        String::from_utf8(bytes).map_err(|error| RepositoryError::WorkspaceRootDiscovery {
            path: package_json_path.clone(),
            message: error.to_string(),
        })?;
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(package_json_path.clone()),
        FileType::Json,
        content,
    );

    let package: PackageJsonWorkspaces =
        serde_json::from_value(parse_json_file(&file).map_err(|error| {
            RepositoryError::WorkspaceRootDiscovery {
                path: package_json_path.clone(),
                message: error.to_string(),
            }
        })?)
        .map_err(|error| RepositoryError::WorkspaceRootDiscovery {
            path: package_json_path.clone(),
            message: error.to_string(),
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
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let workspace_path = directory.join("pnpm-workspace.yaml");
    let content = match fs.read_to_string(&workspace_path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(RepositoryError::WorkspaceRootDiscovery {
                path: workspace_path,
                message: error.to_string(),
            });
        }
    };

    let patterns = parse_pnpm_workspace_packages(&workspace_path, &content)?;
    if patterns.is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Read one workspace root `destack.json` when present.
fn read_workspace_destack_config(
    fs: &dyn FileSystem,
    root: &Path,
) -> Result<Option<DestackDeclaration>, RepositoryError> {
    let path = root.join("destack.json");
    let content = match fs.read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(RepositoryError::WorkspaceRootDiscovery {
                path,
                message: error.to_string(),
            });
        }
    };

    let file = File::from_text(
        FileId::from_logical_path(&path),
        "destack.json".to_string(),
        Uri::from_path(&path),
        Some(path.clone()),
        FileType::Json,
        content,
    );
    let path = path.clone();
    let file = Arc::new(file);

    DestackDeclaration::parse(&file).map(Some).map_err(|error| {
        RepositoryError::WorkspaceRootDiscovery {
            path,
            message: error.to_string(),
        }
    })
}

/// Parse the packages field from `pnpm-workspace.yaml` content.
fn parse_pnpm_workspace_packages(
    path: &Path,
    content: &str,
) -> Result<Vec<String>, RepositoryError> {
    let workspace = serde_yaml_ng::from_str::<PnpmWorkspacePackages>(content).map_err(|error| {
        RepositoryError::WorkspaceRootDiscovery {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;

    Ok(workspace.packages.unwrap_or_else(Vec::new))
}
