use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use tspp_artifact::{ArtifactKey, IndexKind};
use tspp_query::Module;
use tspp_repository::{Package, Repository, RepositoryError, Revision, RevisionPin};
use tspp_session::{Session, SessionError};
use tspp_source::{File, FileId, ModuleId, ProfileId, TargetId};

use crate::Error;

use super::{State, Workspace};

/// Pinned workspace state at one immutable repository revision.
#[derive(Debug)]
pub(crate) struct WorkspacePin {
    /// The canonical workspace root.
    root: PathBuf,
    /// The shared computation session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
    /// Mutable workspace state selecting the physical revision.
    state: Weak<parking_lot::Mutex<State>>,
}

impl WorkspacePin {
    /// Create one workspace pin.
    pub(crate) fn new(
        root: PathBuf,
        session: Arc<Session>,
        revision: RevisionPin,
        state: Weak<parking_lot::Mutex<State>>,
    ) -> Self {
        Self {
            root,
            session,
            revision,
            state,
        }
    }

    /// Return the live session for workspace internals.
    pub(crate) fn session(&self) -> &Session {
        self.session.as_ref()
    }

    /// Return the pinned revision id.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the retained repository revision.
    pub(crate) fn into_revision(self) -> RevisionPin {
        self.revision
    }

    /// Persist this revision when physical workspace state still selects it.
    pub(crate) fn persist_artifacts(&self) -> bool {
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        let state = state.lock();
        if state.physical.revision() != self.revision.revision() {
            return false;
        }

        self.revision.persist_artifacts()
    }

    /// Return the repository backing this revision.
    pub(crate) fn repository(&self) -> &Repository {
        self.revision.repository()
    }

    /// Return one tracked file id for a path in this revision.
    pub(crate) fn file_id(&self, path: &Path) -> Result<Option<FileId>, Error> {
        let path = if let Ok(path) = self.repository().file_system().canonicalize(path) {
            path
        } else {
            path.to_path_buf()
        };
        let logical_path = path
            .strip_prefix(&self.root)
            .map_err(|_| Error::PathNotInRoot { path: path.clone() })?;
        let logical_path = logical_path.to_string_lossy().replace('\\', "/");
        let file_id = FileId::from_logical_str(&logical_path);
        let file = self.repository().file(self.revision(), file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id from this revision.
    pub(crate) fn file(&self, file_id: FileId) -> Result<Arc<File>, Error> {
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }

    /// Return the target and profile selected for one package.
    pub(super) fn selected_target(
        &self,
        package: &Package,
    ) -> Result<Option<(TargetId, ProfileId)>, Error> {
        let selected = self
            .repository()
            .package_default_target(self.revision(), package.id)?;
        let Some((target_id, _)) = selected else {
            if package.targets.is_empty() {
                return Ok(None);
            }

            return Err(Error::TargetNotSelected {
                package_id: package.id,
            });
        };

        let profile = self
            .repository()
            .profile_for_target(self.revision(), target_id)?;

        Ok(Some((target_id, profile.id())))
    }

    /// Return the targets and profiles selected for configured packages.
    pub(super) fn selected_targets(&self) -> Result<Vec<(TargetId, ProfileId)>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let mut selected = Vec::new();

        // select one semantic target from every authored package
        for package_id in repository.package_ids(revision)? {
            let package = repository.package(revision, package_id)?.ok_or(
                RepositoryError::MissingPackage {
                    package: package_id,
                },
            )?;
            if !package.kind.is_authored() {
                continue;
            }

            if let Some(target) = self.selected_target(&package)? {
                selected.push(target);
            }
        }

        Ok(selected)
    }

    /// Return the distinct profiles selected for configured packages.
    pub(super) fn selected_profile_ids(&self) -> Result<Vec<ProfileId>, Error> {
        let mut profile_ids = self
            .selected_targets()?
            .into_iter()
            .map(|(_, profile_id)| profile_id)
            .collect::<Vec<_>>();
        profile_ids.sort_unstable();
        profile_ids.dedup();

        Ok(profile_ids)
    }

    /// Resolve one module in its package's selected program.
    pub(crate) fn module(&self, module_id: ModuleId) -> Result<Module, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let module = repository
            .module(revision, module_id)?
            .ok_or(RepositoryError::MissingModule { module: module_id })?;
        let package = repository.package(revision, module.package_id)?.ok_or(
            RepositoryError::MissingPackage {
                package: module.package_id,
            },
        )?;
        let Some((_, profile_id)) = self.selected_target(&package)? else {
            return Err(Error::TargetNotSelected {
                package_id: package.id,
            });
        };

        Ok(Module {
            module_id,
            profile_id,
        })
    }

    /// Select requested modules belonging to active programs.
    pub(crate) fn selected_modules(&self, module_ids: &[ModuleId]) -> Result<Vec<Module>, Error> {
        let mut modules = Vec::new();

        // pair modules with the selected program owned by their package
        for (target_id, profile_id) in self.selected_targets()? {
            for module_id in module_ids.iter().copied() {
                if module_id.package_id == target_id.package_id() {
                    modules.push(Module {
                        module_id,
                        profile_id,
                    });
                }
            }
        }
        modules.sort_unstable_by_key(|module| (module.profile_id, module.module_id));
        modules.dedup();

        Ok(modules)
    }

    /// Return DIR diagnostic artifacts for exact program modules.
    pub(crate) fn diagnostic_artifacts(&self, modules: &[Module]) -> Vec<ArtifactKey> {
        let mut artifacts = Vec::new();

        // request each DIR phase through check for every exact module
        for module in modules {
            artifacts.extend([
                ArtifactKey::dir_parsed(module.module_id),
                ArtifactKey::dir_bound(module.module_id, module.profile_id),
                ArtifactKey::dir_imported(module.module_id, module.profile_id),
                ArtifactKey::dir_expanded(module.module_id, module.profile_id),
                ArtifactKey::dir_exported(module.module_id, module.profile_id),
                ArtifactKey::dir_resolved(module.module_id, module.profile_id),
                ArtifactKey::dir_declared(module.module_id, module.profile_id),
                ArtifactKey::dir_checked(module.module_id, module.profile_id),
            ]);
        }

        artifacts.sort_unstable();
        artifacts.dedup();

        artifacts
    }

    /// Return every program index root for the selected profiles.
    pub(crate) fn program_indexes(&self) -> Result<Vec<ArtifactKey>, Error> {
        let mut artifacts = Vec::new();

        // index every family for each selected semantic program
        for profile_id in self.selected_profile_ids()? {
            artifacts
                .extend(IndexKind::ALL.map(|kind| ArtifactKey::program_index(profile_id, kind)));
        }
        artifacts.sort_unstable();
        artifacts.dedup();

        Ok(artifacts)
    }
}

impl Workspace {
    /// Pin current physical workspace state.
    pub(crate) fn pin_physical(&self) -> Result<WorkspacePin, Error> {
        let state = self.lock()?;
        let revision = state.physical.clone();

        Ok(WorkspacePin::new(
            self.root.clone(),
            self.session(),
            revision,
            Arc::downgrade(&self.state),
        ))
    }

    /// Pin this workspace at one exact revision.
    pub(crate) fn pin(&self, revision: Revision) -> Result<WorkspacePin, Error> {
        let revision = self.repository.pin(revision)?;

        Ok(WorkspacePin::new(
            self.root.clone(),
            self.session(),
            revision,
            Arc::downgrade(&self.state),
        ))
    }
}
