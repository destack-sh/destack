use std::collections::HashSet;

use serde_json::{Value, from_value};
use tspp_lsp_server::jsonrpc;
use tspp_lsp_types as lsp;
use tspp_query as query;
use tspp_source::DiagnosticReference;

use super::{Document, DocumentSet, IntoLsp};
use crate::server::{ProjectId, internal_error};

/// Code action constraints read from one client request.
pub(crate) struct CodeActionContext {
    /// The requested action kinds.
    only: Vec<query::CodeActionKind>,
    /// Exact TS++ diagnostics carried by the request.
    diagnostics: Vec<CodeActionDiagnostic>,
    /// Whether the client supplied an action kind filter.
    is_filtered: bool,
}

/// One client diagnostic and its exact TS++ identity.
struct CodeActionDiagnostic {
    /// The TS++ diagnostic identity.
    reference: DiagnosticReference,
    /// The client diagnostic.
    diagnostic: lsp::Diagnostic,
}

/// Markdown assembled for one LSP result.
#[derive(Default)]
struct Markdown {
    /// The rendered Markdown.
    text: String,
}

impl Markdown {
    /// Append one Markdown block.
    fn push(&mut self, markdown: &str) {
        if !self.text.is_empty() {
            self.text.push_str("\n\n");
        }

        self.text.push_str(markdown);
    }

    /// Append one fenced code block.
    fn push_code(&mut self, language: &str, source: &str) {
        if !self.text.is_empty() {
            self.text.push_str("\n\n");
        }

        self.text.push_str("```");
        self.text.push_str(language);
        self.text.push('\n');
        self.text.push_str(source);
        self.text.push_str("\n```");
    }

    /// Return whether no Markdown has been appended.
    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Consume the rendered Markdown.
    fn into_string(self) -> String {
        self.text
    }

    /// Consume the rendered Markdown as LSP documentation.
    fn into_documentation(self) -> lsp::Documentation {
        lsp::Documentation::MarkupContent(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: self.text,
        })
    }
}

impl From<String> for Markdown {
    /// Wrap existing Markdown.
    fn from(text: String) -> Self {
        Self { text }
    }
}

impl IntoLsp for query::CompletionItemKind {
    type Lsp = lsp::CompletionItemKind;

    /// Convert this completion item kind.
    fn into_lsp(self) -> lsp::CompletionItemKind {
        match self {
            Self::AssociatedConst | Self::Constant => lsp::CompletionItemKind::CONSTANT,
            Self::AssociatedType | Self::TypeAlias | Self::TypeParameter => {
                lsp::CompletionItemKind::TYPE_PARAMETER
            }
            Self::Method => lsp::CompletionItemKind::METHOD,
            Self::Function => lsp::CompletionItemKind::FUNCTION,
            Self::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
            Self::Field => lsp::CompletionItemKind::FIELD,
            Self::Variable | Self::ValueParameter => lsp::CompletionItemKind::VARIABLE,
            Self::Class => lsp::CompletionItemKind::CLASS,
            Self::Interface | Self::NewtypeInterface => lsp::CompletionItemKind::INTERFACE,
            Self::Newtype | Self::Struct => lsp::CompletionItemKind::STRUCT,
            Self::Extension => lsp::CompletionItemKind::CLASS,
            Self::Module => lsp::CompletionItemKind::MODULE,
            Self::Property => lsp::CompletionItemKind::PROPERTY,
            Self::Value => lsp::CompletionItemKind::VALUE,
            Self::Enum => lsp::CompletionItemKind::ENUM,
            Self::Keyword | Self::BuiltinType => lsp::CompletionItemKind::KEYWORD,
            Self::File => lsp::CompletionItemKind::FILE,
            Self::Reference | Self::Label => lsp::CompletionItemKind::REFERENCE,
            Self::Folder => lsp::CompletionItemKind::FOLDER,
            Self::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        }
    }
}

impl IntoLsp for query::SignatureHelp {
    type Lsp = lsp::SignatureHelp;

    /// Convert this signature help result.
    fn into_lsp(self) -> lsp::SignatureHelp {
        let signatures = self
            .signatures
            .into_iter()
            .map(|signature| lsp::SignatureInformation {
                label: signature.label,
                documentation: signature
                    .documentation
                    .map(|documentation| Markdown::from(documentation).into_documentation()),
                parameters: Some(
                    signature
                        .parameters
                        .into_iter()
                        .map(|parameter| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::Simple(parameter.label),
                            documentation: parameter.documentation.map(|documentation| {
                                Markdown::from(documentation).into_documentation()
                            }),
                        })
                        .collect(),
                ),
                active_parameter: None,
            })
            .collect();

        lsp::SignatureHelp {
            signatures,
            active_signature: Some(self.active_signature as u32),
            active_parameter: self.active_parameter.map(|parameter| parameter as u32),
        }
    }
}

impl IntoLsp for query::InlayHintKind {
    type Lsp = lsp::InlayHintKind;

    /// Convert this inlay hint kind.
    fn into_lsp(self) -> lsp::InlayHintKind {
        match self {
            Self::Type => lsp::InlayHintKind::TYPE,
            Self::Parameter => lsp::InlayHintKind::PARAMETER,
        }
    }
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

        // read exact TS++ diagnostics
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

    /// Return one TS++ diagnostic reference carried by the client.
    fn diagnostic_reference(
        diagnostic: &lsp::Diagnostic,
    ) -> jsonrpc::Result<Option<DiagnosticReference>> {
        if diagnostic.source.as_deref() != Some("tspp") {
            return Ok(None);
        }

        let data = diagnostic.data.clone().ok_or_else(|| {
            jsonrpc::Error::invalid_params("TS++ diagnostic is missing its source reference")
        })?;
        let reference = from_value(data).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "TS++ diagnostic has an invalid source reference: {error}"
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
    /// Build one LSP completion item from a query entry.
    pub(crate) fn completion_item(
        &self,
        index: usize,
        entry: query::CompletionEntry,
        supports_label_details: bool,
        data: Option<Value>,
    ) -> jsonrpc::Result<lsp::CompletionItem> {
        // build compact label details when the client supports them
        let label_details = if supports_label_details {
            let detail = entry.label_suffix.clone();
            let description = entry.description.clone();

            (detail.is_some() || description.is_some()).then_some(lsp::CompletionItemLabelDetails {
                detail,
                description,
            })
        } else {
            None
        };

        // configure snippet insertion
        let insert_text_format = if entry.edit.is_snippet {
            Some(lsp::InsertTextFormat::SNIPPET)
        } else {
            None
        };
        let insert_text_mode = if entry.edit.is_snippet {
            Some(lsp::InsertTextMode::ADJUST_INDENTATION)
        } else {
            None
        };
        // retain query order in clients that sort completion items
        let sort_text = Some(format!("{index:020}"));
        if entry.edit.span.file != self.id() {
            return Err(internal_error(format!(
                "completion edit targets another file: {:?}",
                entry.edit.span
            )));
        }
        let text_edit = lsp::TextEdit {
            range: self.range(entry.edit.span)?,
            new_text: entry.edit.new_text,
        };

        // map deprecation fields
        let (deprecated, tags) = if entry.is_deprecated {
            (Some(true), Some(vec![lsp::CompletionItemTag::DEPRECATED]))
        } else {
            (None, None)
        };

        // retain descriptions in the detail field for older clients
        let detail = if supports_label_details {
            None
        } else {
            entry.description.clone()
        };

        // map completion kind
        let kind = entry.kind.into_lsp();

        Ok(lsp::CompletionItem {
            label: entry.label,
            label_details,
            kind: Some(kind),
            detail,
            documentation: None,
            insert_text_format,
            insert_text_mode,
            sort_text,
            preselect: if entry.preselect { Some(true) } else { None },
            deprecated,
            tags,
            additional_text_edits: None,
            text_edit: Some(text_edit.into()),
            data,
            ..Default::default()
        })
    }

    /// Apply selected completion details to one LSP item.
    pub(crate) fn apply_completion_details(
        &self,
        item: &mut lsp::CompletionItem,
        details: query::CompletionDetailsResponse,
        supports_label_details: bool,
    ) -> jsonrpc::Result<()> {
        // render expanded declaration details
        if let Some(declaration) = details.declaration.as_deref() {
            item.detail = match (supports_label_details, item.detail.as_deref()) {
                (false, Some(description)) => Some(format!("{declaration} — {description}")),
                _ => Some(declaration.to_string()),
            };
        }

        // render declaration and authored documentation
        let mut documentation = Markdown::default();
        if let Some(declaration) = details.declaration.as_deref() {
            documentation.push_code("tspp", declaration);
        }
        if let Some(text) = details.documentation.as_deref() {
            documentation.push(text);
        }
        item.documentation =
            (!documentation.is_empty()).then(|| documentation.into_documentation());

        // map exact import edits
        let edits = details
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
            .collect::<jsonrpc::Result<Vec<_>>>()?;
        item.additional_text_edits = (!edits.is_empty()).then_some(edits);

        Ok(())
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
        let path = document.display_path(&self.root);
        let position = document.position(item.target.selection_span.start)?;
        let location = format!("{}:{}:{}", path, position.line + 1, position.character + 1);
        let mut markdown = Markdown::default();
        markdown.push(&format!("`{location}`"));
        markdown.push_code("tspp", &item.declaration);

        // add the selected type
        if let Some(selected_type) = &item.selected_type {
            markdown.push_code("tspp", selected_type);
        }

        // add documentation
        if let Some(documentation) = &item.documentation {
            markdown.push(documentation);
        }

        Ok(markdown.into_string())
    }
}

impl Document {
    /// Encode one inlay hint as an LSP inlay hint.
    pub(crate) fn inlay_hint(&self, hint: &query::InlayHint) -> jsonrpc::Result<lsp::InlayHint> {
        let position = self.position(hint.position)?;
        let kind = Some(hint.kind.into_lsp());

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
    pub(crate) fn code_lens(
        &self,
        project: ProjectId,
        lens: &query::CodeLens,
    ) -> jsonrpc::Result<lsp::CodeLens> {
        let range = self.range(lens.range)?;
        let uri = serde_json::to_value(self.uri(project)?).map_err(internal_error)?;
        let position = self.position(lens.range.start)?;
        let position = serde_json::to_value(position).map_err(internal_error)?;

        // bind the action to its declaration
        let command = match &lens.action {
            query::CodeLensAction::References { count } => {
                let suffix = if *count == 1 { "" } else { "s" };

                Some(lsp::Command {
                    title: format!("{count} reference{suffix}"),
                    command: "tspp.showReferences".to_string(),
                    arguments: Some(vec![uri, position]),
                })
            }
            query::CodeLensAction::Implementations { count } => {
                let suffix = if *count == 1 { "" } else { "s" };

                Some(lsp::Command {
                    title: format!("{count} implementation{suffix}"),
                    command: "tspp.showImplementations".to_string(),
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
