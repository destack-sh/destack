use std::collections::HashMap;
use std::sync::Arc;

use destack_lsp_types as lsp;
use destack_repository::{Repository, Revision};
use destack_source::{BatchEdit, File, FileId};

use super::common::byte_span_to_range;
use crate::uri::lsp_uri_for_file;

/// File content view for workspace edits.
enum FileForEdit {
    /// A borrowed file reference.
    Borrowed(Arc<File>),
    /// An owned file reference.
    Owned(Box<File>),
}

impl FileForEdit {
    /// Return a file reference for edits.
    fn file(&self) -> &File {
        match self {
            Self::Borrowed(file) => file,
            Self::Owned(file) => file.as_ref(),
        }
    }
}

/// Resolve file content for workspace edit calculations.
fn file_for_workspace_edit(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Option<FileForEdit> {
    let file = repository.file(revision, file_id).ok().flatten()?;
    if file.has_line_index() {
        return Some(FileForEdit::Borrowed(file));
    }

    // fall back to reading from the file system
    let path = file.path.as_ref()?;
    let content = repository.file_system().read_to_string(path).ok()?;
    let loaded = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        content,
    );

    Some(FileForEdit::Owned(Box::new(loaded)))
}

/// Convert a batch edit to an LSP workspace edit.
#[allow(clippy::mutable_key_type)]
pub fn batch_edit_to_workspace_edit(
    repository: &Repository,
    revision: Revision,
    batch: &BatchEdit,
) -> lsp::WorkspaceEdit {
    // collect edits grouped by uri
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();

    // build edits per file
    for file_edit in batch.files.iter() {
        let Some(file_view) = file_for_workspace_edit(repository, revision, file_edit.file) else {
            continue;
        };
        let file = file_view.file();
        let Some(uri) = lsp_uri_for_file(file) else {
            continue;
        };

        let text_edits: Vec<lsp::TextEdit> = file_edit
            .edits
            .iter()
            .map(|edit| lsp::TextEdit {
                range: byte_span_to_range(file, edit.span),
                new_text: edit.new_text.clone(),
            })
            .collect();

        changes.insert(uri, text_edits);
    }

    // return the assembled workspace edit
    lsp::WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    }
}
