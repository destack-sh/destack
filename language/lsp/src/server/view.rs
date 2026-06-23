use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_source::FileId;
use destack_workspace::{
    Error, FileImage, FileImagesRequest, FileSnapshot, FileSnapshotRequest, RootSnapshot,
    ViewRequest, ViewResult, Workspace,
};

/// Return one root snapshot for the path root.
pub(super) fn root_snapshot(
    workspace: &dyn Workspace,
    path: &Path,
    target: Option<String>,
) -> Result<RootSnapshot, Error> {
    let root = workspace.root(path)?;
    let view = workspace.view(&root, ViewRequest::Root { target })?;
    let ViewResult::Root(snapshot) = view else {
        return Err(Error::Internal {
            detail: "workspace returned a non-root view".to_string(),
        });
    };

    Ok(snapshot)
}

/// Return one file snapshot for a path.
pub(super) fn file_snapshot(
    workspace: &dyn Workspace,
    path: PathBuf,
    target: Option<String>,
) -> Result<Option<FileSnapshot>, Error> {
    let root = workspace.root(&path)?;
    let request = FileSnapshotRequest { path, target };
    let view = workspace.view(&root, ViewRequest::File(request))?;
    let ViewResult::File(snapshot) = view else {
        return Err(Error::Internal {
            detail: "workspace returned a non-file view".to_string(),
        });
    };

    Ok(snapshot)
}

/// Return source file images for one revision.
pub(super) fn file_images(
    workspace: &dyn Workspace,
    path: &Path,
    revision: Revision,
    file_ids: Vec<FileId>,
) -> Result<Vec<FileImage>, Error> {
    let root = workspace.root(path)?;
    let request = FileImagesRequest { revision, file_ids };
    let view = workspace.view(&root, ViewRequest::FileImages(request))?;
    let ViewResult::FileImages(images) = view else {
        return Err(Error::Internal {
            detail: "workspace returned a non-file-images view".to_string(),
        });
    };

    Ok(images)
}
