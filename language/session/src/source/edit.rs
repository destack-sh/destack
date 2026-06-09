use std::path::Path;

use destack_source as source;
use destack_source::{FileSystem, Uri};

use crate::{Edit, SessionError, SourceError, TextEdit};

impl Edit {
    /// Apply edits to one filesystem root.
    pub(crate) fn apply_all(
        file_system: &dyn FileSystem,
        root: &Path,
        edits: Vec<Self>,
    ) -> Result<(), SessionError> {
        file_system
            .create_dir_all(root)
            .map_err(|error| SourceError::WriteFailed {
                operation: "create_dir_all",
                path: root.to_path_buf(),
                message: error.to_string(),
            })?;

        // apply edits in input order
        for edit in edits {
            edit.apply(file_system, root)?;
        }

        Ok(())
    }

    /// Apply this edit to one filesystem root.
    fn apply(self, file_system: &dyn FileSystem, root: &Path) -> Result<(), SessionError> {
        match self {
            Self::SetText { path, text } => {
                let path = root.join(path);

                Self::write_text(file_system, &path, &text)
            }
            Self::SetBytes { path, bytes } => {
                let path = root.join(path);

                Self::write_bytes(file_system, &path, &bytes)
            }
            Self::EditText { path, edits } => {
                let path = root.join(path);

                Self::edit_text(file_system, &path, edits)
            }
            Self::Remove { path } => {
                let path = root.join(path);

                Self::remove_path(file_system, &path)
            }
            Self::Move { from, to } => {
                let from = root.join(from);
                let to = root.join(to);

                Self::move_path(file_system, &from, &to)
            }
        }
    }

    /// Write one text file.
    fn write_text(
        file_system: &dyn FileSystem,
        path: &Path,
        text: &str,
    ) -> Result<(), SessionError> {
        Self::create_parent_directory(file_system, path)?;

        file_system
            .write_string(path, text)
            .map_err(|error| SourceError::WriteFailed {
                operation: "write_string",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Write one binary file.
    fn write_bytes(
        file_system: &dyn FileSystem,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), SessionError> {
        Self::create_parent_directory(file_system, path)?;

        file_system
            .write(path, bytes)
            .map_err(|error| SourceError::WriteFailed {
                operation: "write",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Apply text edits to one text file.
    fn edit_text(
        file_system: &dyn FileSystem,
        path: &Path,
        edits: Vec<TextEdit>,
    ) -> Result<(), SessionError> {
        let text = file_system
            .read_to_string(path)
            .map_err(|error| SourceError::ReadFailed {
                operation: "read_to_string",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let file_id = source::FileId::from_logical_path(path);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let file = source::File::from_text(
            file_id,
            name,
            Uri::from_file_path(path),
            Some(path.to_path_buf()),
            source::FileType::from_path_or_unknown(path),
            text,
        );

        // apply source edits using source byte ranges
        let edits = edits
            .into_iter()
            .map(|edit| edit.source_edit(file_id))
            .collect();
        let file_edit = source::FileEdit::with_edits(file_id, edits);
        let text = source::apply_file_edit(&file, &file_edit)?;

        Self::write_text(file_system, path, &text)
    }

    /// Remove one path.
    fn remove_path(file_system: &dyn FileSystem, path: &Path) -> Result<(), SessionError> {
        file_system
            .remove_path(path)
            .map_err(|error| SourceError::WriteFailed {
                operation: "remove_path",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Move one file path.
    fn move_path(file_system: &dyn FileSystem, from: &Path, to: &Path) -> Result<(), SessionError> {
        let bytes = file_system
            .read(from)
            .map_err(|error| SourceError::ReadFailed {
                operation: "read",
                path: from.to_path_buf(),
                message: error.to_string(),
            })?;

        Self::write_bytes(file_system, to, &bytes)?;
        Self::remove_path(file_system, from)
    }

    /// Create the parent directory for one file path.
    fn create_parent_directory(
        file_system: &dyn FileSystem,
        path: &Path,
    ) -> Result<(), SessionError> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        file_system
            .create_dir_all(parent)
            .map_err(|error| SourceError::WriteFailed {
                operation: "create_dir_all",
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;

        Ok(())
    }
}

impl TextEdit {
    /// Convert this text edit to one source edit.
    fn source_edit(self, file_id: source::FileId) -> source::Edit {
        let span = source::Span::new(file_id, self.range.start, self.range.end);

        source::Edit::replace(span, self.text)
    }
}
