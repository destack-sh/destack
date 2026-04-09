use std::path::Path;
use std::sync::Arc;

use destack_compiler::ImportError;
use destack_source::ModuleId;
use destack_workspace::{Change, Module, Repository, Revision};

use crate::{Session, SessionError};

impl Session {
    /// Return the current module id for a path in one revision when already tracked.
    pub(crate) fn existing_module_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<ModuleId>, SessionError> {
        let repository = self.repository();

        repository
            .module_id_for_path(revision, path)
            .map_err(SessionError::from)
    }

    /// Admit one module path into a revision when it is not tracked yet.
    pub(crate) fn admit_module_for_revision(
        &self,
        mut revision: Revision,
        path: &Path,
    ) -> Result<(Revision, ModuleId), SessionError> {
        let repository = self.repository();
        let path = path.to_path_buf();

        loop {
            // reuse the existing module identity when it is already tracked
            if let Some(module_id) = self.existing_module_id_for_path(revision, &path)? {
                return Ok((revision, module_id));
            }

            match self.compiler().resolve_path_to_module(revision, &path) {
                Ok(module_id) => return Ok((revision, module_id)),
                Err(ImportError::Yield { requirement }) => {
                    if !requirement.has_file_requirements() {
                        return Err(SessionError::ResolvePathFailed {
                            path: path.clone(),
                            detail: "module resolution yielded non-file requirements".to_string(),
                        });
                    }

                    // yielded source changes
                    let mut change = Change::empty();
                    requirement.for_each_file(|requirement| {
                        change.extend_from(&requirement.change);
                    });

                    revision = self.apply_source_change(repository.as_ref(), revision, change)?;
                }
                Err(error) => {
                    return Err(SessionError::ResolvePathFailed {
                        path: path.clone(),
                        detail: error.to_string(),
                    });
                }
            }
        }
    }

    /// Return one tracked module for a revision.
    pub(super) fn module_for_revision(
        &self,
        repository: &Repository,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Arc<Module>, SessionError> {
        repository
            .module(revision, module_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::ModuleIdNotTracked { module_id })
    }
}
