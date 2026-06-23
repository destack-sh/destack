use std::collections::HashMap;
use std::sync::Arc;

use destack_lsp_types as lsp;
use destack_source::{File, FileId, PatchSet};

use super::position::byte_span_to_range;
use crate::uri::lsp_uri_for_file;

/// Convert a batch edit to an LSP workspace edit.
#[allow(clippy::mutable_key_type)]
pub(super) fn patch_set_to_workspace_edit(
    batch: &PatchSet,
    file_for_id: &mut impl FnMut(FileId) -> Option<Arc<File>>,
) -> lsp::WorkspaceEdit {
    // collect edits grouped by uri
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();

    // build edits per file
    for file_edit in batch.files.iter() {
        let Some(file) = file_for_id(file_edit.file) else {
            continue;
        };
        let Some(uri) = lsp_uri_for_file(&file) else {
            continue;
        };

        let text_edits: Vec<lsp::TextEdit> = file_edit
            .patches
            .iter()
            .map(|edit| lsp::TextEdit {
                range: byte_span_to_range(&file, edit.span),
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
