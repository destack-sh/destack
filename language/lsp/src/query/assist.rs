use std::collections::HashSet;

use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::DiagnosticReference;
use serde_json::{Value, from_value};

use super::{Document, DocumentSet};
use crate::server::internal_error;

/// Code action constraints read from one client request.
pub(crate) struct CodeActionContext {
    /// The requested action kinds.
    only: Vec<query::CodeActionKind>,
    /// Exact Destack diagnostics carried by the request.
    diagnostics: Vec<CodeActionDiagnostic>,
    /// Whether the client supplied an action kind filter.
    is_filtered: bool,
}

/// One client diagnostic and its exact Destack identity.
struct CodeActionDiagnostic {
    /// The Destack diagnostic identity.
    reference: DiagnosticReference,
    /// The client diagnostic.
    diagnostic: lsp::Diagnostic,
}

impl CodeActionContext {
    /// Return whether the client filter excludes every supported action kind.
    pub(crate) fn excludes_all(&self) -> bool {
        self.is_filtered && self.only.is_empty()
    }

    /// Build the query code action context.
    pub(crate) fn query_context(&self) -> query::CodeActionContext {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.reference.clone())
            .collect();

        query::CodeActionContext {
            only: self.only.clone(),
            diagnostics: Some(diagnostics),
        }
    }

    /// Return client diagnostics addressed by one query action.
    fn diagnostics(
        &self,
        action: &query::CodeAction,
    ) -> jsonrpc::Result<Option<Vec<lsp::Diagnostic>>> {
        let mut diagnostics = Vec::with_capacity(action.diagnostics.len());

        // match every exact query diagnostic to its client counterpart
        for reference in &action.diagnostics {
            let Some(diagnostic) = self
                .diagnostics
                .iter()
                .find(|diagnostic| &diagnostic.reference == reference)
            else {
                return Err(internal_error(format!(
                    "code action diagnostic is absent from the request: {reference:?}"
                )));
            };
            diagnostics.push(diagnostic.diagnostic.clone());
        }

        Ok((!diagnostics.is_empty()).then_some(diagnostics))
    }
}

impl TryFrom<&lsp::CodeActionContext> for CodeActionContext {
    type Error = jsonrpc::Error;

    /// Decode one LSP code action context.
    fn try_from(context: &lsp::CodeActionContext) -> Result<Self, Self::Error> {
        let mut only = Vec::new();
        let mut seen = HashSet::new();

        // read requested action kinds
        if let Some(kinds) = context.only.as_ref() {
            for kind in kinds {
                for query_kind in Self::action_kinds(kind) {
                    if seen.insert(query_kind) {
                        only.push(query_kind);
                    }
                }
            }
        }

        // read exact Destack diagnostics
        let mut diagnostics = Vec::new();
        for diagnostic in &context.diagnostics {
            let Some(reference) = Self::diagnostic_reference(diagnostic)? else {
                continue;
            };
            diagnostics.push(CodeActionDiagnostic {
                reference,
                diagnostic: diagnostic.clone(),
            });
        }

        Ok(Self {
            only,
            diagnostics,
            is_filtered: context.only.as_ref().is_some_and(|kinds| !kinds.is_empty()),
        })
    }
}

impl CodeActionContext {
    /// Return query action kinds selected by one LSP action kind.
    fn action_kinds(kind: &lsp::CodeActionKind) -> Vec<query::CodeActionKind> {
        let kind = kind.as_str();

        // quick fixes and sub kinds
        if kind == lsp::CodeActionKind::QUICKFIX.as_str() || kind.starts_with("quickfix.") {
            vec![query::CodeActionKind::QuickFix]
        }
        // extract refactors and sub kinds
        else if kind == lsp::CodeActionKind::REFACTOR_EXTRACT.as_str()
            || kind.starts_with("refactor.extract.")
        {
            vec![query::CodeActionKind::RefactorExtract]
        }
        // inline refactors and sub kinds
        else if kind == lsp::CodeActionKind::REFACTOR_INLINE.as_str()
            || kind.starts_with("refactor.inline.")
        {
            vec![query::CodeActionKind::RefactorInline]
        }
        // all refactors
        else if kind == lsp::CodeActionKind::REFACTOR.as_str() {
            vec![
                query::CodeActionKind::RefactorExtract,
                query::CodeActionKind::RefactorInline,
            ]
        } else {
            Vec::new()
        }
    }

    /// Return one Destack diagnostic reference carried by the client.
    fn diagnostic_reference(
        diagnostic: &lsp::Diagnostic,
    ) -> jsonrpc::Result<Option<DiagnosticReference>> {
        if diagnostic.source.as_deref() != Some("destack") {
            return Ok(None);
        }

        let data = diagnostic.data.clone().ok_or_else(|| {
            jsonrpc::Error::invalid_params("Destack diagnostic is missing its source reference")
        })?;
        let reference = from_value(data).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "Destack diagnostic has an invalid source reference: {error}"
            ))
        })?;

        Ok(Some(reference))
    }

    /// Build one LSP code action from a query result.
    pub(crate) fn code_action(
        &self,
        action: &query::CodeAction,
        edit: Option<lsp::WorkspaceEdit>,
        data: Option<Value>,
    ) -> jsonrpc::Result<lsp::CodeActionOrCommand> {
        let diagnostics = self.diagnostics(action)?;

        // map action kind
        let kind = match action.kind {
            query::CodeActionKind::QuickFix => lsp::CodeActionKind::QUICKFIX,
            query::CodeActionKind::RefactorExtract => lsp::CodeActionKind::REFACTOR_EXTRACT,
            query::CodeActionKind::RefactorInline => lsp::CodeActionKind::REFACTOR_INLINE,
        };
        let code_action = lsp::CodeAction {
            title: action.title.clone(),
            kind: Some(kind),
            diagnostics,
            edit,
            command: None,
            is_preferred: Some(action.is_preferred),
            disabled: None,
            data,
        };

        Ok(lsp::CodeActionOrCommand::CodeAction(code_action))
    }
}

impl Document {
    /// Build one LSP completion item from a query result.
    pub(crate) fn completion_item(
        &self,
        index: usize,
        item: query::CompletionItem,
    ) -> jsonrpc::Result<lsp::CompletionItem> {
        let insert_text_format = if item.edit.is_snippet {
            Some(lsp::InsertTextFormat::SNIPPET)
        } else {
            None
        };
        let insert_text_mode = if item.edit.is_snippet {
            Some(lsp::InsertTextMode::ADJUST_INDENTATION)
        } else {
            None
        };
        // build additional text edits
        let additional_text_edits = if item.additional_edits.is_empty() {
            None
        } else {
            let edits = item
                .additional_edits
                .iter()
                .map(|edit| {
                    if edit.span.file != self.id() {
                        return Err(internal_error(format!(
                            "completion edit targets another file: {:?}",
                            edit.span
                        )));
                    }

                    Ok(lsp::TextEdit {
                        range: self.range(edit.span)?,
                        new_text: edit.new_text.clone(),
                    })
                })
                .collect::<jsonrpc::Result<_>>()?;

            Some(edits)
        };

        // retain query order in clients that sort completion items
        let sort_text = Some(format!("{index:020}"));
        if item.edit.span.file != self.id() {
            return Err(internal_error(format!(
                "completion edit targets another file: {:?}",
                item.edit.span
            )));
        }
        let text_edit = lsp::TextEdit {
            range: self.range(item.edit.span)?,
            new_text: item.edit.new_text,
        };

        // build presentation fields
        let documentation = item.documentation.map(|documentation| {
            lsp::Documentation::MarkupContent(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: documentation,
            })
        });
        let (deprecated, tags) = if item.is_deprecated {
            (Some(true), Some(vec![lsp::CompletionItemTag::DEPRECATED]))
        } else {
            (None, None)
        };

        // map completion kind
        let kind = match item.kind {
            query::CompletionItemKind::AssociatedConst => lsp::CompletionItemKind::CONSTANT,
            query::CompletionItemKind::AssociatedType => lsp::CompletionItemKind::TYPE_PARAMETER,
            query::CompletionItemKind::Method => lsp::CompletionItemKind::METHOD,
            query::CompletionItemKind::Function => lsp::CompletionItemKind::FUNCTION,
            query::CompletionItemKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
            query::CompletionItemKind::Field => lsp::CompletionItemKind::FIELD,
            query::CompletionItemKind::Variable => lsp::CompletionItemKind::VARIABLE,
            query::CompletionItemKind::Class => lsp::CompletionItemKind::CLASS,
            query::CompletionItemKind::Interface => lsp::CompletionItemKind::INTERFACE,
            query::CompletionItemKind::NewtypeInterface => lsp::CompletionItemKind::INTERFACE,
            query::CompletionItemKind::Newtype => lsp::CompletionItemKind::STRUCT,
            query::CompletionItemKind::TypeAlias => lsp::CompletionItemKind::TYPE_PARAMETER,
            query::CompletionItemKind::Extension => lsp::CompletionItemKind::CLASS,
            query::CompletionItemKind::Module => lsp::CompletionItemKind::MODULE,
            query::CompletionItemKind::Property => lsp::CompletionItemKind::PROPERTY,
            query::CompletionItemKind::Value => lsp::CompletionItemKind::VALUE,
            query::CompletionItemKind::Enum => lsp::CompletionItemKind::ENUM,
            query::CompletionItemKind::Keyword => lsp::CompletionItemKind::KEYWORD,
            query::CompletionItemKind::File => lsp::CompletionItemKind::FILE,
            query::CompletionItemKind::Reference => lsp::CompletionItemKind::REFERENCE,
            query::CompletionItemKind::Label => lsp::CompletionItemKind::REFERENCE,
            query::CompletionItemKind::Folder => lsp::CompletionItemKind::FOLDER,
            query::CompletionItemKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
            query::CompletionItemKind::Constant => lsp::CompletionItemKind::CONSTANT,
            query::CompletionItemKind::Struct => lsp::CompletionItemKind::STRUCT,
            query::CompletionItemKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
            query::CompletionItemKind::ValueParameter => lsp::CompletionItemKind::VARIABLE,
            query::CompletionItemKind::BuiltinType => lsp::CompletionItemKind::KEYWORD,
        };

        Ok(lsp::CompletionItem {
            label: item.label,
            kind: Some(kind),
            detail: item.detail,
            documentation,
            insert_text_format,
            insert_text_mode,
            sort_text,
            preselect: if item.preselect { Some(true) } else { None },
            deprecated,
            tags,
            additional_text_edits,
            text_edit: Some(text_edit.into()),
            ..Default::default()
        })
    }
}

impl DocumentSet {
    /// Render hover content as LSP Markdown.
    pub(crate) fn render_hover(&self, hover: &query::Hover) -> jsonrpc::Result<String> {
        let mut markdown =
            Vec::with_capacity(hover.items.len() + usize::from(hover.documentation.is_some()));

        // render documentation attached directly to the authored node
        if let Some(documentation) = &hover.documentation {
            markdown.push(documentation.clone());
        }

        // format declaration targets
        for item in &hover.items {
            markdown.push(self.render_hover_item(item)?);
        }

        Ok(markdown.join("\n\n---\n\n"))
    }

    /// Render one hover item as Markdown.
    fn render_hover_item(&self, item: &query::HoverItem) -> jsonrpc::Result<String> {
        let document = self.document(item.target.span.file)?;
        let uri = document.uri()?;
        let position = document.position(item.target.selection_span.start)?;
        let location = format!(
            "{}:{}:{}",
            uri.as_str(),
            position.line + 1,
            position.character + 1
        );
        let mut markdown = String::new();
        markdown.push_str("**Signature**\n\n");
        markdown.push_str("```ds\n");
        markdown.push_str(&item.signature);
        markdown.push_str("\n```");

        // add the selected type
        if let Some(type_text) = &item.type_text {
            markdown.push_str("\n\n**Type**\n\n");
            markdown.push_str("```ds\n");
            markdown.push_str(type_text);
            markdown.push_str("\n```");
        }

        // add documentation
        if let Some(documentation) = &item.documentation {
            markdown.push_str("\n\n**Documentation**\n\n");
            markdown.push_str(documentation);
        }

        // add the declaration location
        markdown.push_str("\n\n**Location**\n\n`");
        markdown.push_str(&location);
        markdown.push('`');

        Ok(markdown)
    }
}

impl Document {
    /// Build LSP signature help from one query result.
    pub(crate) fn signature_help(&self, help: query::SignatureHelp) -> lsp::SignatureHelp {
        let signatures = help
            .signatures
            .into_iter()
            .map(|signature| lsp::SignatureInformation {
                label: signature.label,
                documentation: signature.documentation.map(Self::documentation),
                parameters: Some(
                    signature
                        .parameters
                        .into_iter()
                        .map(|parameter| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::Simple(parameter.label),
                            documentation: parameter.documentation.map(Self::documentation),
                        })
                        .collect(),
                ),
                active_parameter: None,
            })
            .collect();

        lsp::SignatureHelp {
            signatures,
            active_signature: Some(help.active_signature as u32),
            active_parameter: help.active_parameter.map(|parameter| parameter as u32),
        }
    }

    /// Build LSP Markdown documentation.
    fn documentation(documentation: String) -> lsp::Documentation {
        lsp::Documentation::MarkupContent(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: documentation,
        })
    }
}

impl Document {
    /// Encode one inlay hint as an LSP inlay hint.
    pub(crate) fn inlay_hint(&self, hint: &query::InlayHint) -> jsonrpc::Result<lsp::InlayHint> {
        let position = self.position(hint.position)?;
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
}

impl Document {
    /// Encode one code lens as an LSP code lens.
    pub(crate) fn code_lens(&self, lens: &query::CodeLens) -> jsonrpc::Result<lsp::CodeLens> {
        let range = self.range(lens.range)?;
        let uri = serde_json::to_value(self.uri()?).map_err(internal_error)?;
        let position = self.position(lens.range.start)?;
        let position = serde_json::to_value(position).map_err(internal_error)?;

        // bind the action to its declaration
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
}
