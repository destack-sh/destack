use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_workspace::FileEdit;

use super::position;

/// Convert one workspace file edit into an LSP text edit.
pub(super) fn edit(edit: FileEdit) -> jsonrpc::Result<lsp::TextEdit> {
    Ok(lsp::TextEdit {
        range: position::range(&edit.file, edit.patch.span)?,
        new_text: edit.patch.new_text,
    })
}
