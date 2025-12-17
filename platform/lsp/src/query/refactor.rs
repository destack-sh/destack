use std::collections::HashMap;

use destack_source::BatchEdit;
use destack_workspace::Session;
use tower_lsp_server::lsp_types as lsp;

use super::common::byte_span_to_range;

/// Convert a batch edit to an LSP workspace edit.
#[allow(clippy::mutable_key_type)]
pub fn batch_edit_to_workspace_edit(session: &Session, batch: &BatchEdit) -> lsp::WorkspaceEdit {
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
    for file_edit in batch.files.iter() {
        let file = session.files.get(file_edit.file);
        let Some(uri) = file.uri.as_ref().parse::<lsp::Uri>().ok() else {
            continue;
        };

        let text_edits: Vec<lsp::TextEdit> = file_edit
            .edits
            .iter()
            .map(|edit| lsp::TextEdit {
                range: byte_span_to_range(&file, edit.span),
                new_text: edit.new_text.clone(),
            })
            .collect();

        changes.insert(uri, text_edits);
    }

    lsp::WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    }
}
