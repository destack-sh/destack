use std::path::Path;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, IndexKind};
use destack_repository::{Package, Repository, RepositoryError, Revision, RevisionPin};
use destack_session::{Session, SessionError};
use destack_source::{File, FileId, ModuleId, ProfileId, TargetId};

use crate::diagnostic::Error;

use super::LocalWorkspace;

/// Pinned read state for one session at one pinned repository revision.
#[derive(Debug)]
pub(crate) struct SessionPin {
    /// The live session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
}

impl SessionPin {
    /// Create one session pin.
    pub(crate) fn new(session: Arc<Session>, revision: RevisionPin) -> Self {
        Self { session, revision }
    }

    /// Return the live session for workspace internals.
    pub(crate) fn session(&self) -> &Session {
        self.session.as_ref()
    }

    /// Return the pinned revision id.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the repository backing this revision.
    pub(crate) fn repository(&self) -> &Repository {
        self.revision.repository()
    }

    /// Return one tracked file id for a path in this revision.
    pub(crate) fn file_id(&self, path: &Path) -> Result<Option<FileId>, Error> {
        let file_id = self.session.file_id(path);
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

    /// Return checked DIR artifacts for selected modules.
    pub(crate) fn diagnostic_artifacts(
        &self,
        modules: &[ModuleId],
    ) -> Result<Vec<ArtifactKey>, Error> {
        let mut artifacts = Vec::new();

        // check selected authored modules in their package profile
        for (target_id, profile_id) in self.selected_targets()? {
            for module_id in modules.iter().copied() {
                if module_id.package_id == target_id.package_id() {
                    // TODO #Broken: should request module_linted but that is not ready yet
                    artifacts.push(ArtifactKey::dir_checked(module_id, profile_id));
                }
            }
        }

        artifacts.sort_unstable();
        artifacts.dedup();

        Ok(artifacts)
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

impl LocalWorkspace {
    /// Pin one root session at its current revision.
    pub(crate) fn pin_session(&self, root: &Path) -> Result<SessionPin, Error> {
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision = repository.pin(revision)?;

        Ok(SessionPin::new(session, revision))
    }
}
