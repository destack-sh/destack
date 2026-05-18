use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{File, FileId, FileSystem, FileType, Uri};
use destack_workspace::{DestackFile, Environment, Ref, Repository, RepositoryError};

use super::reload::{RELOAD_EXCLUDED_DIRECTORY_NAMES, is_reload_path};
use super::{FileSystemSource, RepositorySource, RepositorySourceFilter};
use crate::SessionError;

/// Open one repository after discovering the workspace root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    fs: Arc<dyn FileSystem>,
    environment: Environment,
) -> Result<Repository, SessionError> {
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
        .with_include_path(is_reload_path);
    let change = source.poll(&repository, base_revision, RepositorySourceFilter::All)?;
    let revision = repository.commit_change(base_revision, change)?;

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
        Ok(_) => path.to_path_buf(),
        Err(error) => {
            return Err(RepositoryError::WorkspaceRootDiscovery {
                path: path.to_path_buf(),
                message: error.to_string(),
            });
        }
    };
    let mut current = input_directory.clone();

    // walk up directories looking for a workspace root
    loop {
        // destack source root
        if let Some(root) = find_destack_source_root(fs, &current)? {
            return Ok(root);
        }

        // move up to the parent
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }

    Ok(input_directory)
}

/// Find the Destack source root at one directory when present.
fn find_destack_source_root(
    fs: &dyn FileSystem,
    directory: &Path,
) -> Result<Option<PathBuf>, RepositoryError> {
    let Some(_config) = read_source_destack_config(fs, directory)? else {
        return Ok(None);
    };

    Ok(Some(directory.to_path_buf()))
}

/// Read one source root `destack.json` when present.
fn read_source_destack_config(
    fs: &dyn FileSystem,
    root: &Path,
) -> Result<Option<DestackFile>, RepositoryError> {
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

    // parse with the normal Destack config parser
    let file = File::from_text(
        FileId::from_logical_str("destack.json"),
        "destack.json".to_string(),
        Uri::from_path(&path),
        Some(path.clone()),
        FileType::Json,
        content,
    );
    let path = path.clone();
    let file = Arc::new(file);

    DestackFile::parse(&file)
        .map(Some)
        .map_err(|error| RepositoryError::WorkspaceRootDiscovery {
            path,
            message: error.to_string(),
        })
}
