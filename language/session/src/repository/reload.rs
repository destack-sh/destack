use std::io;
use std::path::Path;

use destack_source::FileType;
use destack_workspace::Ref;

use crate::{
    FileChange, FileSystemSource, FileUpdate, FileUpdateKind, RepositoryChange, Session,
    SessionError,
};

/// Directory names excluded by filesystem reload scans.
pub(crate) const RELOAD_EXCLUDED_DIRECTORY_NAMES: &[&str] = &[".git", "node_modules", "target"];

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
        let _mutation_guard = self.enter_mutation();
        let repository = self.repository();
        let before = self.revision(reference)?;

        // collect filesystem source truth
        let mut source = FileSystemSource::new(repository.as_ref(), self.root())
            .with_excluded_directory_names(RELOAD_EXCLUDED_DIRECTORY_NAMES)
            .with_tracked_path(is_reload_path);

        // apply repository source changes
        let change = RepositoryChange::from_source(repository.as_ref(), before, &mut source)?;
        let file_ids = change.file_ids();
        let revision = change.apply(repository.as_ref(), before)?;

        // project repository changes for callers
        let files = self.project_file_updates(before, revision, file_ids)?;

        self.set_ref(reference, revision)?;

        Ok(files)
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
