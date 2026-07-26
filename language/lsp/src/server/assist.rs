use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::File;

use super::error::{internal_error, workspace_error};
use super::position;
use super::source::SourceFiles;
use crate::uri;

/// Format hover items as LSP Markdown.
pub(super) fn hover(items: &[query::HoverItem], files: &SourceFiles) -> jsonrpc::Result<String> {
    let mut markdown = Vec::with_capacity(items.len());

    // format every exact declaration target
    for item in items {
        // load the source that owns both target ranges
        let file = files.file(item.target.span.file).map_err(workspace_error)?;
        let path = file
            .path
            .as_ref()
            .ok_or_else(|| internal_error("hover target file has no source path"))?;
        let position = position::position(&file, item.target.selection_span.start)?;
        let location = format!(
            "{}:{}:{}",
            path.to_string_lossy(),
            position.line + 1,
            position.character + 1
        );

        markdown.push(hover_item(item, &location));
    }

    Ok(markdown.join("\n\n---\n\n"))
}

/// Format one hover item as Markdown.
fn hover_item(item: &query::HoverItem, location: &str) -> String {
    let mut markdown = String::new();
    markdown.push_str("**Signature**\n\n");
    markdown.push_str("```ds\n");
    markdown.push_str(&item.signature);
    markdown.push_str("\n```");

    // add a distinct selected type when available
    if let Some(type_text) = &item.type_text {
        markdown.push_str("\n\n**Type**\n\n");
        markdown.push_str("```ds\n");
        markdown.push_str(type_text);
        markdown.push_str("\n```");
    }

    // add documentation when available
    if let Some(documentation) = &item.documentation {
        markdown.push_str("\n\n**Documentation**\n\n");
        markdown.push_str(documentation);
    }

    // add the exact declaration location
    markdown.push_str("\n\n**Location**\n\n`");
    markdown.push_str(location);
    markdown.push('`');

    markdown
}

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
