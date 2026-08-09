use std::path::Path;

use dashmap::mapref::entry::Entry;
use destack_source::{Content, FileType};

use crate::repository::{Ref, Repository, RepositoryError, Revision};

impl Repository {
    /// Create one new ref pointing at another ref's current revision.
    pub fn fork_ref(&self, from: &Ref, to: Ref) -> Result<Revision, RepositoryError> {
        let revision = self.current(from)?;
        self.refs.insert(to, revision);

        Ok(revision)
    }

    /// Set one ref to one existing revision identity.
    pub fn set_ref(
        &self,
        reference: &Ref,
        revision: Revision,
    ) -> Result<Revision, RepositoryError> {
        let _revision = self.revision(revision)?;
        self.refs.insert(reference.clone(), revision);

        Ok(revision)
    }

    /// Advance one ref when it still points at the expected revision.
    pub fn advance_ref(
        &self,
        reference: &Ref,
        from: Revision,
        to: Revision,
    ) -> Result<bool, RepositoryError> {
        let did_advance = match self.refs.entry(reference.clone()) {
            // reject refs that moved since the caller read them
            Entry::Occupied(entry) if *entry.get() != from => false,

            // publish the requested revision
            Entry::Occupied(mut entry) => {
                let _revision = self.revision(to)?;
                entry.insert(to);
                true
            }

            // missing refs are repository state errors
            Entry::Vacant(_entry) => {
                return Err(RepositoryError::MissingRef {
                    reference: reference.clone(),
                });
            }
        };

        Ok(did_advance)
    }

    /// Load one workspace file payload from the attached file system.
    pub fn load_workspace_file_content(&self, path: &Path) -> Result<Content, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);
        let file_system = self.file_system();

        // read binary payload
        if file_type.is_binary() {
            let content = file_system
                .read(path)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

            Ok(Content::Binary { content })
        }
        // read text payload
        else {
            let content =
                file_system
                    .read_to_string(path)
                    .map_err(|error| RepositoryError::FileSystem {
                        operation: "read_to_string",
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?;

            Ok(Content::Text { content })
        }
    }
}
