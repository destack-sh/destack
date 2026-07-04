use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query::Module;
use destack_repository::{FormatterOptions, Repository, Revision, RevisionPin};
use destack_serde::Reflect;
use destack_session::{Session, SessionError};
use destack_source::{File, FileId, PackageId, ProfileId, TargetId};
use serde::{Deserialize, Serialize};

use crate::FileImage;
use crate::diagnostic::Error;
use crate::protocol::{FileImagesRequest, FileSnapshot, FileSnapshotRequest, RootSnapshot};

use super::LocalWorkspace;

/// Pinned read state for one session at one pinned repository revision.
#[derive(Debug)]
pub struct Snapshot {
    /// The live session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
}

impl Snapshot {
    /// Create one snapshot.
    pub(super) fn new(session: Arc<Session>, revision: RevisionPin) -> Self {
        Self { session, revision }
    }

    /// Return the live session for workspace internals.
    pub(super) fn session(&self) -> &Session {
        self.session.as_ref()
    }

    /// Return the pinned revision id.
    pub fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the repository backing this revision.
    pub fn repository(&self) -> &Repository {
        self.revision.repository()
    }

    /// Return one tracked file id for a path in this revision.
    pub fn file_id(&self, path: &Path) -> Result<Option<FileId>, Error> {
        let file_id = self.session.file_id(path);
        let file = self.repository().file(self.revision(), file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id from this revision.
    pub fn file(&self, file_id: FileId) -> Result<Arc<File>, Error> {
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }

    /// Return one file snapshot for a path in this revision.
    pub fn file_view(self, path: &Path) -> Result<FileView, Error> {
        let file_id = self.file_id(path)?.ok_or_else(|| Error::FileMissing {
            path: path.to_path_buf(),
        })?;
        let file = self.file(file_id)?;

        Ok(FileView {
            session: self,
            file_id,
            file,
        })
    }
}

/// Pinned read state for one file inside a pinned session revision.
#[derive(Debug)]
pub struct FileView {
    /// The pinned session revision.
    pub session: Snapshot,
    /// The source file id.
    pub file_id: FileId,
    /// The source file image.
    pub file: Arc<File>,
}

/// Request to read one workspace snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ViewRequest {
    /// Return root query context.
    Root {
        /// Target name used to select profiles.
        target: Option<String>,
    },
    /// Return one file snapshot.
    File(FileSnapshotRequest),
    /// Return source file images.
    FileImages(FileImagesRequest),
}

/// Result of reading one workspace snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ViewResult {
    /// Root query context.
    Root(RootSnapshot),
    /// Source file snapshot.
    File(Option<FileSnapshot>),
    /// Source file images.
    FileImages(Vec<FileImage>),
}

/// Request to read diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticsRequest {
    /// Return diagnostics for every open root.
    All,
    /// Return diagnostics for one root.
    Root(PathBuf),
    /// Return diagnostics for one file.
    File(PathBuf),
}

impl FileView {
    /// Return the pinned revision id.
    pub fn revision(&self) -> Revision {
        self.session.revision()
    }

    /// Return the repository backing this file snapshot.
    pub fn repository(&self) -> &Repository {
        self.session.repository()
    }
}

impl LocalWorkspace {
    /// Return a pinned snapshot over the current revision for one root session.
    pub fn snapshot(&self, root: &Path) -> Result<Snapshot, Error> {
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision = repository.pin(revision)?;

        Ok(Snapshot::new(session, revision))
    }

    /// Return a pinned snapshot over one file at the current revision.
    pub fn file_view(&self, path: &Path) -> Result<FileView, Error> {
        let root = self.root_at(path)?;
        let session = self.snapshot(&root)?;

        session.file_view(path)
    }

    /// Return query context for one root.
    pub fn root_snapshot(&self, root: &Path, target: Option<&str>) -> Result<RootSnapshot, Error> {
        let revision = self.revision(root)?;
        let package_id = self
            .repository
            .nearest_package(revision, root)?
            .map(|package| package.id);
        let profile_ids = if let Some(package_id) = package_id {
            self.package_profiles(revision, package_id, target)?
        } else {
            Vec::new()
        };

        Ok(RootSnapshot {
            revision,
            profile_ids,
        })
    }

    /// Return one source file snapshot.
    pub fn file_snapshot(
        &self,
        root: &Path,
        request: FileSnapshotRequest,
    ) -> Result<Option<FileSnapshot>, Error> {
        // require the requested path to belong to the requested root
        let owning_root = self.root_at(&request.path)?;
        if owning_root != root {
            return Err(Error::PathNotInRoot {
                path: request.path.clone(),
            });
        }

        // load the file snapshot from the current root revision
        let view = match self.file_view(&request.path) {
            Ok(view) => view,
            Err(Error::FileMissing { .. }) => return Ok(None),
            Err(error) => return Err(error),
        };
        let repository = view.repository();
        let revision = view.revision();
        let file_id = view.file_id;
        let module_id = repository.module_id_for_file(revision, file_id)?;

        // attach module identity when the file is a module
        let module = if let Some(module_id) = module_id {
            let module =
                repository
                    .module(revision, module_id)?
                    .ok_or_else(|| Error::Internal {
                        detail: format!("module {module_id:?} is missing from revision {revision}"),
                    })?;
            let profile_ids =
                self.package_profiles(revision, module.package_id, request.target.as_deref())?;
            profile_ids.first().copied().map(|profile_id| Module {
                module_id,
                profile_id,
            })
        } else {
            None
        };
        let file = FileImage::from(view.file.as_ref());
        let formatter = self.formatter_options(revision, &request.path)?;

        Ok(Some(FileSnapshot {
            revision,
            file_id,
            module,
            formatter,
            file,
        }))
    }

    /// Return source file images for one revision.
    pub fn file_images(
        &self,
        root: &Path,
        request: FileImagesRequest,
    ) -> Result<Vec<FileImage>, Error> {
        self.session(root)?;
        let mut images = Vec::new();
        for file_id in request.file_ids {
            let Some(file) = self.repository.file(request.revision, file_id)? else {
                continue;
            };
            images.push(FileImage::from(file.as_ref()));
        }

        Ok(images)
    }

    /// Return selected profile ids for one package.
    fn package_profiles(
        &self,
        revision: Revision,
        package_id: PackageId,
        target: Option<&str>,
    ) -> Result<Vec<ProfileId>, Error> {
        // use the explicit target when supplied by the client
        if let Some(target) = target {
            let target_id = TargetId::new(package_id, target);
            let profile = self.repository.profile_for_target(revision, target_id)?;

            return Ok(vec![profile.id()]);
        }

        // otherwise use the package default target when one is unambiguous
        let default_target = self
            .repository
            .package_default_target(revision, package_id)?;
        let Some((target_id, _)) = default_target else {
            return Ok(Vec::new());
        };
        let profile = self.repository.profile_for_target(revision, target_id)?;

        Ok(vec![profile.id()])
    }

    /// Return selected formatter options for one path.
    fn formatter_options(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<FormatterOptions, Error> {
        // prefer package-local formatter options
        let package = self.repository.nearest_package(revision, path)?;
        if let Some(package) = package
            && let Some(config) = self
                .repository
                .destack_for_package_id(revision, package.id)?
        {
            return Ok(config.formatter);
        }

        // fall back to workspace formatter options
        let config = self.repository.destack_for_workspace(revision)?;
        let formatter = config
            .map(|config| config.formatter)
            .unwrap_or_else(FormatterOptions::default);

        Ok(formatter)
    }
}
