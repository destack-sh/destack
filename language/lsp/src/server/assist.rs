use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::File;

use super::error::internal_error;
use super::position;
use crate::uri;

/// Convert an inlay hint to an LSP inlay hint.
pub(super) fn hint(file: &File, hint: &query::InlayHint) -> jsonrpc::Result<lsp::InlayHint> {
    let position = position::position(file, hint.position)?;
    let kind = match hint.kind {
        query::InlayHintKind::Type => Some(lsp::InlayHintKind::TYPE),
        query::InlayHintKind::Parameter => Some(lsp::InlayHintKind::PARAMETER),
    };

    Ok(lsp::InlayHint {
        position,
        label: lsp::InlayHintLabel::String(hint.label.clone()),
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: Some(hint.padding_left),
        padding_right: Some(hint.padding_right),
        data: None,
    })
}

/// Convert a code lens to an LSP code lens.
pub(super) fn lens(file: &File, lens: &query::CodeLens) -> jsonrpc::Result<lsp::CodeLens> {
    let range = position::range(file, lens.range)?;
    let uri = serde_json::to_value(uri::file(file)?).map_err(internal_error)?;
    let position = position::position(file, lens.range.start)?;
    let position = serde_json::to_value(position).map_err(internal_error)?;

    // bind every action to its exact declaration source
    let command = match &lens.action {
        query::CodeLensAction::References { count } => {
            let suffix = if *count == 1 { "" } else { "s" };

            Some(lsp::Command {
                title: format!("{count} reference{suffix}"),
                command: "destack.showReferences".to_string(),
                arguments: Some(vec![uri, position]),
            })
        }
        query::CodeLensAction::Implementations { count } => {
            let suffix = if *count == 1 { "" } else { "s" };

            Some(lsp::Command {
                title: format!("{count} implementation{suffix}"),
                command: "destack.showImplementations".to_string(),
                arguments: Some(vec![uri, position]),
            })
        }
    };
    Ok(lsp::CodeLens {
        range,
        command,
        data: None,
    })
}
