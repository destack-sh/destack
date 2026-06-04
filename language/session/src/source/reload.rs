use std::io;
use std::path::Path;

use destack_source::FileType;
use destack_workspace::Ref;

use crate::{
    FileChange, FileSystemSource, FileUpdate, FileUpdateKind, Session, SessionError, Source,
};

/// Return true when filesystem reload should track one path.
pub(crate) fn is_reload_path(path: &Path) -> bool {
    // known source file types
    let Some(file_type) = FileType::from_path(path) else {
        return false;
    };

    file_type.is_code()
        || file_type.is_data()
        || file_type.is_text()
        || file_type.is_binary()
        || FileUpdateKind::for_path(path).is_config_change()
}

impl Session {
    /// Reload filesystem source files into one ref.
    pub fn reload_from_fs(&self, reference: &Ref) -> Result<Vec<FileUpdate>, SessionError> {
        let repository = self.repository();
        let before = self.revision(reference)?;

        // collect filesystem source truth
        let mut source = FileSystemSource::new(repository.as_ref(), self.root());
        let source_import = source.import()?;
        let change = source_import.change(repository.as_ref(), before)?;
        let file_ids = change.file_ids().to_vec();
        let before_pin = repository.pin(before)?;
        let revision = repository.commit_change(before, change)?;
        let revision_pin = repository.pin(revision)?;

        // publish when the ref still points at the scanned base
        let was_published = repository.advance_ref(reference, before, revision)?;
        if !was_published {
            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: before,
                current: self.revision(reference)?,
            });
        }

        let updates =
            self.project_file_updates(before_pin.revision(), revision_pin.revision(), file_ids)?;

        Ok(updates)
    }

    /// Read one filesystem path as a file update.
    pub fn read_file_from_fs(&self, path: &Path) -> io::Result<FileChange> {
        let repository = self.repository();

        // preserve bytes for binary formats
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = repository.file_system().read(path)?;

            return Ok(FileChange::Bytes { content });
        }

        // use text for source readable files
        let content = repository.file_system().read_to_string(path)?;

        Ok(FileChange::Text { content })
    }

    /// Return whether filesystem reload should track one path.
    pub fn is_reload_path(&self, path: &Path) -> bool {
        is_reload_path(path)
    }
}
