use destack_source::File;
use {destack_lsp_types as lsp, destack_query as query};

use super::common::{byte_span_to_range, byte_to_utf16_position};

/// Convert an inlay hint to an LSP inlay hint.
pub fn inlay_hint_to_lsp(file: &File, hint: &query::InlayHint) -> Option<lsp::InlayHint> {
    let (line, character) = byte_to_utf16_position(file, hint.position)?;
    let position = lsp::Position { line, character };
    let kind = match hint.kind {
        query::InlayHintKind::Type => Some(lsp::InlayHintKind::TYPE),
        query::InlayHintKind::Parameter => Some(lsp::InlayHintKind::PARAMETER),
    };
    Some(lsp::InlayHint {
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
pub fn code_lens_to_lsp(file: &File, lens: &query::CodeLens) -> lsp::CodeLens {
    let range = byte_span_to_range(file, lens.range);
    let suffix = |count: usize| if count == 1 { "" } else { "s" };
    let command = match &lens.action {
        query::CodeLensAction::References { count } => Some(lsp::Command {
            title: format!("{count} reference{}", suffix(*count)),
            command: "destack.showReferences".to_string(),
            arguments: None,
        }),
        query::CodeLensAction::Implementations { count } => Some(lsp::Command {
            title: format!("{count} implementation{}", suffix(*count)),
            command: "destack.showImplementations".to_string(),
            arguments: None,
        }),
        query::CodeLensAction::RunTest { test_name } => Some(lsp::Command {
            title: format!("▶ Run {test_name}"),
            command: "destack.runTest".to_string(),
            arguments: Some(vec![serde_json::Value::String(test_name.clone())]),
        }),
        query::CodeLensAction::DebugTest { test_name } => Some(lsp::Command {
            title: format!("🐛 Debug {test_name}"),
            command: "destack.debugTest".to_string(),
            arguments: Some(vec![serde_json::Value::String(test_name.clone())]),
        }),
        query::CodeLensAction::Custom {
            title,
            command,
            arguments,
        } => Some(lsp::Command {
            title: title.clone(),
            command: command.clone(),
            arguments: if arguments.is_empty() {
                None
            } else {
                Some(
                    arguments
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                )
            },
        }),
    };
    lsp::CodeLens {
        range,
        command,
        data: None,
    }
}
