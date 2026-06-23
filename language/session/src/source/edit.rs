use std::path::Path;

use destack_source::{
    Edit, File, FileId, FilePatch, FileSystem, FileType, Patch, Span, TextPatch, Uri,
    apply_file_patch,
};

use crate::{SessionError, SourceError};

/// Apply edits to one filesystem root.
pub(crate) fn apply_edits(
    file_system: &dyn FileSystem,
    root: &Path,
    edits: Vec<Edit>,
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
        apply_edit(file_system, root, edit)?;
    }

    Ok(())
}

/// Apply one edit to one filesystem root.
fn apply_edit(file_system: &dyn FileSystem, root: &Path, edit: Edit) -> Result<(), SessionError> {
    match edit {
        Edit::SetText { path, text } => {
            let path = root.join(path);

            write_text(file_system, &path, &text)
        }
        Edit::SetBytes { path, bytes } => {
            let path = root.join(path);

            write_bytes(file_system, &path, &bytes)
        }
        Edit::EditText { path, patches } => {
            let full_path = root.join(&path);

            patch_text(file_system, &full_path, &path, patches)
        }
        Edit::Remove { path } => {
            let path = root.join(path);

            remove_path(file_system, &path)
        }
        Edit::Move { from, to } => {
            let from = root.join(from);
            let to = root.join(to);

            move_path(file_system, &from, &to)
        }
    }
}

/// Write one text file.
fn write_text(file_system: &dyn FileSystem, path: &Path, text: &str) -> Result<(), SessionError> {
    create_parent_directory(file_system, path)?;

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
    create_parent_directory(file_system, path)?;

    file_system
        .write(path, bytes)
        .map_err(|error| SourceError::WriteFailed {
            operation: "write",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(())
}

/// Apply text patches to one text file.
fn patch_text(
    file_system: &dyn FileSystem,
    path: &Path,
    logical_path: &Path,
    patches: Vec<TextPatch>,
) -> Result<(), SessionError> {
    let text = file_system
        .read_to_string(path)
        .map_err(|error| SourceError::ReadFailed {
            operation: "read_to_string",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    let file_id = FileId::from_logical_path(logical_path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let file = File::from_text(
        file_id,
        name,
        Uri::from_file_path(path),
        Some(path.to_path_buf()),
        FileType::from_path_or_unknown(path),
        text,
    );

    // lower path level text patches into source patches
    let patches = patches
        .into_iter()
        .map(|patch| source_patch(file_id, patch))
        .collect();
    let file_patch = FilePatch::with_patches(file_id, patches);
    let text = apply_file_patch(&file, &file_patch)?;

    write_text(file_system, path, &text)
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

    write_bytes(file_system, to, &bytes)?;
    remove_path(file_system, from)
}

/// Create the parent directory for one file path.
fn create_parent_directory(file_system: &dyn FileSystem, path: &Path) -> Result<(), SessionError> {
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

/// Convert one text patch to one source patch.
fn source_patch(file_id: FileId, patch: TextPatch) -> Patch {
    let span = Span::new(file_id, patch.range.start, patch.range.end);

    Patch::replace(span, patch.text)
}
