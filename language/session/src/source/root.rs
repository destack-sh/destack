use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Ref, Repository, RepositoryError, Settings,
};
use destack_source::{FileSystem, MemoryFileSystem};

use super::Edit;
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
    let root = find_source_root(fs.as_ref(), &path)?;
    let cwd = environment.cwd.as_deref().unwrap_or(&path);
    let layout =
        DestackLayout::resolve(&root, cwd, &environment, &settings, &layout_override, None);

    // create repository at the selected source root
    let repository = Repository::new(
        root.clone(),
        Arc::new(DiskCacheStore::new()),
        fs,
        environment,
        settings,
        layout,
    );
    let root_ref = Ref::for_root(&root);
    let base_revision = repository.current(&root_ref)?;

    // read the complete source tree
    let source = FileSystemSource::new(&repository, &root, base_revision);
    let edits = source.edits()?;
    let revision = repository.commit_edits(base_revision, edits)?;

    repository.set_ref(&root_ref, revision)?;

    Ok(repository)
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
    Edit::apply_all(file_system.as_ref(), &root, edits)?;

    open_repository_from_fs(root, file_system, environment, settings, layout_override)
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
