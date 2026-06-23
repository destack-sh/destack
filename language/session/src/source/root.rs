use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::MemoryBlobStore;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Host, Ref, Repository, RepositoryError,
    Settings, default_blob_store,
};
use destack_source::{Edit, FileSystem, MemoryFileSystem};

use super::edit::apply_edits;
use super::fs::FileSystemSource;
use crate::SessionError;

/// Open one repository after discovering the source root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    fs: Arc<dyn FileSystem>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, SessionError> {
    let host = Host::new(environment, fs, default_blob_store());

    open_repository(path, host, settings, layout_override)
}

/// Open one repository from one in-memory source.
pub fn open_repository_from_memory(
    root: PathBuf,
    edits: Vec<Edit>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, SessionError> {
    let file_system = Arc::new(MemoryFileSystem::new());
    apply_edits(file_system.as_ref(), &root, edits)?;

    // keep memory sessions fully in memory
    let host = Host::new(environment, file_system, Arc::new(MemoryBlobStore::new()));

    open_repository(root, host, settings, layout_override)
}

/// Open one repository from explicit host capabilities.
pub fn open_repository(
    path: PathBuf,
    host: Host,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, SessionError> {
    let root = find_source_root(host.files().as_ref(), &path)?;
    let environment = host.environment();
    let cwd = environment.cwd.as_deref().unwrap_or(&path);
    let layout = DestackLayout::resolve(&root, cwd, environment, &settings, &layout_override, None);

    // create repository at the selected source root
    let repository = Repository::new(root.clone(), host, settings, layout);
    let root_ref = Ref::for_root(&root);
    let base_revision = repository.current(&root_ref)?;

    // read the complete source tree
    let source = FileSystemSource::new(&repository, &root, base_revision);
    let edits = source.edits()?;
    let revision = repository.commit_edits(base_revision, edits)?;

    repository.set_ref(&root_ref, revision)?;

    Ok(repository)
}

/// Find the source root for one filesystem input path.
fn find_source_root(file_system: &dyn FileSystem, path: &Path) -> Result<PathBuf, RepositoryError> {
    let metadata =
        file_system
            .metadata(path)
            .map_err(|error| RepositoryError::WorkspaceRootDiscovery {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

    // normalize file inputs to their containing directory
    let input_directory = if metadata.is_file {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    }
    // use directory inputs directly
    else {
        path.to_path_buf()
    };
    let mut current = input_directory.clone();

    // walk up directories looking for a source root
    loop {
        if FileSystemSource::read_destack_config(file_system, &current)?.is_some() {
            return Ok(current);
        }

        // move up to the parent
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }

    Ok(input_directory)
}
