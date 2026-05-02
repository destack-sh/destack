use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{File, FileId, FileSystem, FileType, Uri};
use destack_workspace::{
    DestackDeclaration, HostEnvironment, Ref, Repository, RepositoryError, WorkspacesField,
    parse_json_file,
};
use serde_json::Value;

use super::reload::{RELOAD_EXCLUDED_DIRECTORY_NAMES, is_reload_path};
use super::{FileSystemSource, RepositoryChange};

/// Open one repository after discovering the workspace root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    fs: Arc<dyn FileSystem>,
    environment: HostEnvironment,
) -> Result<Repository, RepositoryError> {
    let root = find_source_root_from_fs(fs.as_ref(), &path)?;

    // create repository at the selected source root
    let repository = Repository::new(
        root.clone(),
        Arc::new(DiskCacheStore::new()),
        fs,
        environment,
    );
    let workspace_ref = Ref::for_workspace_root(&root);
    let base_revision = repository.current(&workspace_ref)?;

    // import the initial filesystem truth
    let mut source = FileSystemSource::new(&repository, &root)
        .with_excluded_directory_names(RELOAD_EXCLUDED_DIRECTORY_NAMES)
        .with_tracked_path(is_reload_path);
    let change = RepositoryChange::from_source(&repository, base_revision, &mut source)?;
    let revision = change.apply(&repository, base_revision)?;

    repository.set_ref(&workspace_ref, revision)?;

    Ok(repository)
}

/// Find the source root for one filesystem input path.
fn find_source_root_from_fs(fs: &dyn FileSystem, path: &Path) -> Result<PathBuf, RepositoryError> {
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
        if let Some(root) = find_destack_workspace_root(fs, &current)? {
            return Ok(root);
        }

        // npm or yarn workspace root
        if let Some(root) = find_npm_workspace_root(fs, &current)? {
            return Ok(root);
        }

        // pnpm workspace root
        if let Some(root) = find_pnpm_workspace_root(fs, &current)? {
            return Ok(root);
        }

        // plain package boundary
        if is_package_root(fs, &current) {
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

/// Find the Destack workspace root at one directory when present.
fn find_destack_workspace_root(
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

/// Find the npm or yarn workspace root at one directory when present.
fn find_npm_workspace_root(
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let package_json_path = directory.join("package.json");

    // read package manifest when present
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

    // parse package manifest as workspace config
    let package = parse_package_json(&package_json_path, bytes)?;
    let Some(workspaces) = package.get("workspaces").cloned() else {
        return Ok(None);
    };
    let workspaces: WorkspacesField = serde_json::from_value(workspaces).map_err(|error| {
        RepositoryError::WorkspaceRootDiscovery {
            path: package_json_path.clone(),
            message: error.to_string(),
        }
    })?;

    if workspaces.patterns().is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Parse one `package.json` file from bytes.
fn parse_package_json(path: &Path, bytes: Vec<u8>) -> Result<Value, RepositoryError> {
    let (name, uri) = Uri::from_path_with_name(path);
    let file_id = FileId::from_logical_path(path);

    // package manifests are always text
    let content =
        String::from_utf8(bytes).map_err(|error| RepositoryError::WorkspaceRootDiscovery {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    // parse through the source file JSON path
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        FileType::Json,
        content,
    );

    parse_json_file(&file).map_err(|error| RepositoryError::WorkspaceRootDiscovery {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Find the pnpm workspace root at one directory when present.
fn find_pnpm_workspace_root(
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let workspace_path = directory.join("pnpm-workspace.yaml");

    // read pnpm workspace manifest when present
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

    // non-empty package globs identify a workspace root
    let patterns = parse_pnpm_workspace_packages(&workspace_path, &content)?;
    if patterns.is_empty() {
        return Ok(None);
    }

    Ok(Some(directory.to_path_buf()))
}

/// Return true when one directory is a plain package root.
fn is_package_root(fs: &dyn FileSystem, directory: &Path) -> bool {
    let package_json_path = directory.join("package.json");

    fs.metadata(&package_json_path)
        .is_ok_and(|metadata| metadata.is_file)
}

/// Read one workspace root `destack.json` when present.
fn read_workspace_destack_config(
    fs: &dyn FileSystem,
    root: &Path,
) -> Result<Option<DestackDeclaration>, RepositoryError> {
    let path = root.join("destack.json");

    // read Destack config when present
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

    // parse with the normal Destack declaration parser
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
    let workspace = serde_yaml_ng::from_str::<Value>(content).map_err(|error| {
        RepositoryError::WorkspaceRootDiscovery {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;

    let Some(packages) = workspace.get("packages").cloned() else {
        return Ok(Vec::new());
    };

    serde_json::from_value(packages).map_err(|error| RepositoryError::WorkspaceRootDiscovery {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}
