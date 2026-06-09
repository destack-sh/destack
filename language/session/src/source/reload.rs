use std::collections::HashSet;
use std::io;
use std::path::Path;

use destack_repository::Ref;

use crate::{Change, Edit, FileSystemSource, Session, SessionError};

impl Session {
    /// Reload filesystem source files into one ref.
    pub fn reload_from_fs(&self, reference: &Ref) -> Result<Vec<Change>, SessionError> {
        let repository = self.repository();
        let before = self.revision(reference)?;

        // collect filesystem source truth
        let source = FileSystemSource::new(repository.as_ref(), self.root(), before);
        let edits = source.edits()?;
        let mut file_ids = Vec::new();
        let mut seen_file_ids = HashSet::new();

        // collect changed file ids in edit order
        for edit in &edits {
            for file_id in edit.affected_file_ids() {
                if seen_file_ids.insert(file_id) {
                    file_ids.push(file_id);
                }
            }
        }

        let before_pin = repository.pin(before)?;
        let revision = repository.commit_edits(before, edits)?;
        let revision_pin = repository.pin(revision)?;

        // publish when the ref still points at the scanned base
        self.publish_revision(reference, before, revision)?;

        let updates =
            self.project_file_changes(before_pin.revision(), revision_pin.revision(), file_ids)?;

        Ok(updates)
    }

    /// Read one filesystem path as a session edit.
    pub fn read_filesystem_edit(&self, path: &Path) -> io::Result<Edit> {
        let repository = self.repository();

        FileSystemSource::read_edit(repository.as_ref(), path)
    }

    /// Return whether filesystem source imports one path.
    pub fn imports_filesystem_path(&self, path: &Path) -> bool {
        FileSystemSource::tracks_path(path)
    }
}
