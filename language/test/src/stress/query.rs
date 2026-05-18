use std::time::Duration;

use destack_lsp_server::UriExt;
use destack_query::CompletionTrigger;
use destack_source::Span;
use {destack_lsp_types as lsp, destack_query as query};

use super::core::{StressAnchor, StressProject, StressRecipe, StressWorkspace, project_recipe_dir};
use super::ide::{StressIdeBattery, StressIdeDriver, run_ide_invariants};
use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite, discover_file_cases};
use crate::lsp::{
    NormalizedCompletionItem, NormalizedLocation, NormalizedQuickInfo, NormalizedWorkspaceSymbol,
    normalize_completion_response, normalize_definition_response, normalize_quick_info,
    normalize_references_response, normalize_workspace_symbols,
};

/// Stress test suite for direct query execution over generated projects.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryStressSuite;

/// One direct query driver over one generated stress workspace.
pub(super) struct QueryStressDriver<'a> {
    /// The generated direct query workspace.
    workspace: &'a StressWorkspace,
}

impl Suite for QueryStressSuite {
    fn name(&self) -> &'static str {
        "stress-query"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let stress_dir = project_recipe_dir();
        if !stress_dir.exists() {
            return vec![];
        }

        discover_file_cases(&stress_dir, &["toml"], "destack_test::stress::query")
            .unwrap_or_default()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_query_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

impl<'a> QueryStressDriver<'a> {
    /// Create one direct query driver.
    pub(super) fn new(workspace: &'a StressWorkspace) -> Self {
        Self { workspace }
    }

    /// Return the generated file id and offset for one anchor.
    fn file_position(
        &self,
        anchor: &StressAnchor,
    ) -> Result<(destack_source::FileId, u32), String> {
        let file_id = self.workspace.file_id(&anchor.path)?;

        Ok((file_id, anchor.offset))
    }
}

impl StressIdeDriver for QueryStressDriver<'_> {
    fn label(&self) -> &'static str {
        "query"
    }

    fn hover(&mut self, anchor: &StressAnchor) -> Result<Option<NormalizedQuickInfo>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let hover = query::hover(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
        );
        let Some(hover) = hover else {
            return Ok(None);
        };

        if let Some(range) = hover.range {
            self.workspace.validate_span(range)?;
        }

        normalize_query_hover(self.workspace, &anchor.path, &hover).map(Some)
    }

    fn goto_definition(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let definition = query::goto_definition(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
        );
        let Some(definition) = definition else {
            return Ok(Vec::new());
        };

        for span in &definition.locations {
            self.workspace.validate_span(*span)?;
        }

        normalize_query_definition_result(self.workspace, &definition.locations)
    }

    fn goto_declaration(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let declaration = query::goto_declaration(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
        );
        let Some(declaration) = declaration else {
            return Ok(Vec::new());
        };

        for span in &declaration.locations {
            self.workspace.validate_span(*span)?;
        }

        normalize_query_definition_result(self.workspace, &declaration.locations)
    }

    fn goto_type_definition(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let type_definition = query::goto_type_definition(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
        );
        let Some(type_definition) = type_definition else {
            return Ok(Vec::new());
        };

        for span in &type_definition.locations {
            self.workspace.validate_span(*span)?;
        }

        normalize_query_definition_result(self.workspace, &type_definition.locations)
    }

    fn references(&mut self, anchor: &StressAnchor) -> Result<Vec<NormalizedLocation>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let references = query::find_references(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
            true,
        );
        let Some(references) = references else {
            return Ok(Vec::new());
        };

        for span in &references.references {
            self.workspace.validate_span(*span)?;
        }

        normalize_query_references_result(self.workspace, &references.references)
    }

    fn completion(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedCompletionItem>, String> {
        let (file_id, offset) = self.file_position(anchor)?;
        let completions = query::completions(
            &self.workspace.repository,
            self.workspace.revision,
            file_id,
            offset,
            CompletionTrigger::Invoked,
        );

        for completion in &completions {
            for edit in &completion.additional_text_edits {
                self.workspace.validate_edit(edit)?;
            }
        }

        normalize_query_completions(&completions)
    }

    fn workspace_symbols(
        &mut self,
        query_text: &str,
    ) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
        let symbols = query::workspace_symbols(
            &self.workspace.repository,
            self.workspace.revision,
            query_text,
            64,
        );

        for symbol in &symbols {
            self.workspace.validate_span(symbol.range)?;
        }

        normalize_query_workspace_symbols(self.workspace, &symbols)
    }
}

/// Run one generated query stress case.
fn run_query_stress(case: &Case) -> CaseResult {
    let recipe = match StressRecipe::load(&case.path) {
        Ok(recipe) => recipe,
        Err(message) => return CaseResult::Failed { message },
    };
    let project = StressProject::from_recipe(&recipe);
    let workspace = match StressWorkspace::materialize(&case.name, project) {
        Ok(workspace) => workspace,
        Err(message) => return CaseResult::Failed { message },
    };
    let battery = StressIdeBattery::full(&workspace.project);
    let mut driver = QueryStressDriver::new(&workspace);

    if let Err(message) = run_ide_invariants(&mut driver, &battery) {
        return CaseResult::Failed { message };
    }

    eprintln!(
        "  {} files, {} anchors, {} workspace queries",
        workspace.project.files.len(),
        workspace.project.anchors.len(),
        workspace.project.workspace_queries.len(),
    );

    CaseResult::Passed
}

/// Normalize one direct query hover result through the shared quick-info path.
fn normalize_query_hover(
    workspace: &StressWorkspace,
    file_path: &str,
    hover: &query::Hover,
) -> Result<NormalizedQuickInfo, String> {
    let range = hover
        .range
        .map(|span| lsp_range_for_span(workspace, span))
        .transpose()?;
    let lsp_hover = lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: hover.to_markdown(),
        }),
        range,
    };

    normalize_quick_info(workspace.workspace_root(), file_path, &lsp_hover)
}

/// Normalize one direct query definition result through the shared location path.
fn normalize_query_definition_result(
    workspace: &StressWorkspace,
    spans: &[Span],
) -> Result<Vec<NormalizedLocation>, String> {
    let response = lsp::GotoDefinitionResponse::Array(query_locations(workspace, spans)?);

    normalize_definition_response(workspace.workspace_root(), &response)
}

/// Normalize one direct query references result through the shared location path.
fn normalize_query_references_result(
    workspace: &StressWorkspace,
    spans: &[Span],
) -> Result<Vec<NormalizedLocation>, String> {
    let locations = query_locations(workspace, spans)?;

    normalize_references_response(workspace.workspace_root(), &locations)
}

/// Normalize one direct query workspace-symbol result through the shared symbol path.
fn normalize_query_workspace_symbols(
    workspace: &StressWorkspace,
    symbols: &[query::WorkspaceSymbol],
) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
    let symbols = symbols
        .iter()
        .map(|symbol| {
            Ok(lsp::WorkspaceSymbol {
                name: symbol.name.clone(),
                kind: lsp_symbol_kind(symbol.kind),
                tags: None,
                container_name: symbol.container.clone(),
                location: lsp::OneOf::Left(query_location(workspace, symbol.range)?),
                data: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let response = lsp::OneOf::Right(symbols);

    normalize_workspace_symbols(workspace.workspace_root(), &response)
}

/// Normalize one direct query completion list through the shared completion path.
fn normalize_query_completions(
    completions: &[query::Completion],
) -> Result<Vec<NormalizedCompletionItem>, String> {
    let items = completions
        .iter()
        .map(|completion| lsp::CompletionItem {
            label: completion.label.clone(),
            kind: Some(lsp_completion_kind(completion.kind)),
            ..Default::default()
        })
        .collect();
    let response = lsp::CompletionResponse::Array(items);

    normalize_completion_response(&response)
}

/// Convert one list of direct query spans into LSP locations.
fn query_locations(
    workspace: &StressWorkspace,
    spans: &[Span],
) -> Result<Vec<lsp::Location>, String> {
    spans
        .iter()
        .map(|span| query_location(workspace, *span))
        .collect()
}

/// Convert one direct query span into one LSP location.
fn query_location(workspace: &StressWorkspace, span: Span) -> Result<lsp::Location, String> {
    let path = workspace.relative_path(span.file)?;
    let uri = lsp::Uri::from_file_path(workspace.workspace_root().join(path))
        .ok_or_else(|| format!("failed to build location uri for {path}"))?;
    let range = lsp_range_for_span(workspace, span)?;

    Ok(lsp::Location { uri, range })
}

/// Convert one direct query span into one LSP range.
fn lsp_range_for_span(workspace: &StressWorkspace, span: Span) -> Result<lsp::Range, String> {
    let file = workspace
        .repository
        .file(workspace.revision, span.file)
        .map_err(|error| format!("failed to read file snapshot: {error}"))?
        .ok_or_else(|| format!("missing file snapshot for {:?}", span.file))?;
    let (start_line, start_character) = file
        .get_position(span.start)
        .ok_or_else(|| format!("failed to resolve start position for span {span:?}"))?;
    let (end_line, end_character) = file
        .get_position(span.end)
        .ok_or_else(|| format!("failed to resolve end position for span {span:?}"))?;

    Ok(lsp::Range {
        start: lsp::Position {
            line: start_line,
            character: start_character,
        },
        end: lsp::Position {
            line: end_line,
            character: end_character,
        },
    })
}

/// Map one query symbol kind to the public LSP symbol kind.
fn lsp_symbol_kind(kind: query::SymbolKind) -> lsp::SymbolKind {
    match kind {
        query::SymbolKind::File => lsp::SymbolKind::FILE,
        query::SymbolKind::Module => lsp::SymbolKind::MODULE,
        query::SymbolKind::Namespace => lsp::SymbolKind::NAMESPACE,
        query::SymbolKind::Package => lsp::SymbolKind::PACKAGE,
        query::SymbolKind::Class => lsp::SymbolKind::CLASS,
        query::SymbolKind::Method => lsp::SymbolKind::METHOD,
        query::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
        query::SymbolKind::Field => lsp::SymbolKind::FIELD,
        query::SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        query::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        query::SymbolKind::Interface => lsp::SymbolKind::INTERFACE,
        query::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        query::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
        query::SymbolKind::Constant => lsp::SymbolKind::CONSTANT,
        query::SymbolKind::String => lsp::SymbolKind::STRING,
        query::SymbolKind::Number => lsp::SymbolKind::NUMBER,
        query::SymbolKind::Boolean => lsp::SymbolKind::BOOLEAN,
        query::SymbolKind::Array => lsp::SymbolKind::ARRAY,
        query::SymbolKind::Object => lsp::SymbolKind::OBJECT,
        query::SymbolKind::Key => lsp::SymbolKind::KEY,
        query::SymbolKind::Null => lsp::SymbolKind::NULL,
        query::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
        query::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        query::SymbolKind::Event => lsp::SymbolKind::EVENT,
        query::SymbolKind::Operator => lsp::SymbolKind::OPERATOR,
        query::SymbolKind::TypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
    }
}

/// Map one query completion kind to the public LSP completion kind.
fn lsp_completion_kind(kind: query::CompletionKind) -> lsp::CompletionItemKind {
    match kind {
        query::CompletionKind::Text => lsp::CompletionItemKind::TEXT,
        query::CompletionKind::Method => lsp::CompletionItemKind::METHOD,
        query::CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
        query::CompletionKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        query::CompletionKind::Field => lsp::CompletionItemKind::FIELD,
        query::CompletionKind::Variable => lsp::CompletionItemKind::VARIABLE,
        query::CompletionKind::Class => lsp::CompletionItemKind::CLASS,
        query::CompletionKind::Interface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionKind::Module => lsp::CompletionItemKind::MODULE,
        query::CompletionKind::Property => lsp::CompletionItemKind::PROPERTY,
        query::CompletionKind::Unit => lsp::CompletionItemKind::UNIT,
        query::CompletionKind::Value => lsp::CompletionItemKind::VALUE,
        query::CompletionKind::Enum => lsp::CompletionItemKind::ENUM,
        query::CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
        query::CompletionKind::Snippet => lsp::CompletionItemKind::SNIPPET,
        query::CompletionKind::Color => lsp::CompletionItemKind::COLOR,
        query::CompletionKind::File => lsp::CompletionItemKind::FILE,
        query::CompletionKind::Reference => lsp::CompletionItemKind::REFERENCE,
        query::CompletionKind::Folder => lsp::CompletionItemKind::FOLDER,
        query::CompletionKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        query::CompletionKind::Constant => lsp::CompletionItemKind::CONSTANT,
        query::CompletionKind::Struct => lsp::CompletionItemKind::STRUCT,
        query::CompletionKind::Event => lsp::CompletionItemKind::EVENT,
        query::CompletionKind::Operator => lsp::CompletionItemKind::OPERATOR,
        query::CompletionKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
    }
}
