use std::collections::HashMap;

use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_source::PatchSet;

use super::error::{internal_error, workspace_error};
use super::position;
use super::source::SourceFiles;
use crate::uri;

/// Convert a batch edit to an LSP workspace edit.
#[allow(clippy::mutable_key_type)]
pub(super) fn workspace(
    batch: &PatchSet,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::WorkspaceEdit> {
    // collect edits grouped by uri
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();

    // build edits per file
    for file_edit in batch.files.iter() {
        let file = files.file(file_edit.file).map_err(workspace_error)?;
        let uri = uri::file(&file)?;

        let mut patches = file_edit.patches.iter().collect::<Vec<_>>();
        patches.sort_by_key(|edit| (edit.span.start, edit.span.end));

        // reject overlapping edits before sending an invalid workspace edit
        let mut previous_end = 0;
        for edit in &patches {
            if edit.span.start < previous_end {
                return Err(internal_error(format!(
                    "source patches overlap in file {:?}",
                    file.id
                )));
            }
            previous_end = edit.span.end;
        }

        let text_edits = patches
            .into_iter()
            .map(|edit| {
                Ok(lsp::TextEdit {
                    range: position::range(&file, edit.span)?,
                    new_text: edit.new_text.clone(),
                })
            })
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        if changes.insert(uri, text_edits).is_some() {
            return Err(internal_error(format!(
                "patch set contains duplicate file {:?}",
                file.id
            )));
        }
    }

    // return the assembled workspace edit
    Ok(lsp::WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}
