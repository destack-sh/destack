use destack_dir as dir;
use std::collections::HashSet;

use destack_ast as ast;
use destack_source::{Applicability, BatchEdit, Diagnostic, Edit, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use super::{extract_function, extract_variable, inline_symbol};
use crate::Session;
use crate::query::assist::{CompletionContext, detect_completion_context};
use crate::query::common::{
    ImportEditMode, build_import_display_path, build_import_edits_with_mode, extract_identifier,
    get_module_by_file_id, is_simple_identifier, matches_symbol_space_filter, program_for_file,
    search_importable_symbols_for_program, token_at_offset,
};
use destack_dir::SymbolSpace;

/// Kind of code action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic.
    QuickFix,
    /// Refactoring action.
    Refactor,
    /// Extract refactoring (extract function, variable, etc.).
    RefactorExtract,
    /// Inline refactoring.
    RefactorInline,
    /// Rewrite refactoring.
    RefactorRewrite,
    /// Source organization (imports, etc.).
    Source,
    /// Organize imports.
    SourceOrganizeImports,
    /// Fix all issues of a type.
    SourceFixAll,
}

/// A code action (quick fix or refactoring).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeAction {
    /// The title shown in the UI.
    pub title: String,
    /// The kind of action.
    pub kind: CodeActionKind,
    /// Edits to apply.
    pub edits: BatchEdit,
    /// Whether this is the preferred action for its diagnostics.
    pub is_preferred: bool,
    /// Whether this action is disabled (with reason).
    pub disabled_reason: Option<String>,
    /// The diagnostic code this action fixes (if from a diagnostic).
    pub diagnostic_code: Option<String>,
}

impl CodeAction {
    /// Create a quick fix.
    pub fn quick_fix(title: impl Into<String>, edits: BatchEdit) -> Self {
        // build a quick fix action
        Self {
            title: title.into(),
            kind: CodeActionKind::QuickFix,
            edits,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Create a refactoring.
    pub fn refactor(title: impl Into<String>, kind: CodeActionKind, edits: BatchEdit) -> Self {
        // build a refactor action
        Self {
            title: title.into(),
            kind,
            edits,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Mark as preferred.
    pub fn preferred(mut self) -> Self {
        // mark the action as preferred
        self.is_preferred = true;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        // mark the action as disabled with a reason
        self.disabled_reason = Some(reason.into());
        self
    }

    /// Set the diagnostic code this action fixes.
    pub fn with_diagnostic_code(mut self, code: impl Into<String>) -> Self {
        // record the diagnostic code
        self.diagnostic_code = Some(code.into());
        self
    }
}

/// Context for code action requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodeActionContext {
    /// Requested action kinds (empty = all).
    pub only: Vec<CodeActionKind>,
    /// Whether to include disabled actions.
    pub include_disabled: bool,
}

/// Request code actions for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeActionsRequest {
    /// The document URI.
    pub uri: Uri,
    /// The start byte offset in the document.
    pub start: u32,
    /// The end byte offset in the document.
    pub end: u32,
    /// The code action context.
    pub context: CodeActionContext,
}

/// Response payload for code actions queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

/// Get code actions for a range in a file.
///
/// Includes quick fixes from diagnostics and available refactorings.
pub fn code_actions(
    session: &Session,
    file: FileId,
    range: Span,
    context: &CodeActionContext,
) -> Vec<CodeAction> {
    // start with an empty action list
    let mut actions = Vec::new();

    // collect quick fixes from diagnostics
    collect_diagnostic_fixes(session, file, range, &mut actions);

    // collect auto import quick fixes for unresolved symbols
    collect_auto_import_actions(session, file, range, &mut actions);

    // collect organize imports action
    collect_organize_imports_action(session, file, &mut actions);

    // collect refactor actions
    collect_refactor_actions(session, file, range, &mut actions);

    // filter by requested kinds when specified
    if !context.only.is_empty() {
        actions.retain(|a| context.only.contains(&a.kind));
    }

    // filter out disabled actions unless requested
    if !context.include_disabled {
        actions.retain(|a| a.disabled_reason.is_none());
    }

    // sort deterministically by kind, preference, title, and edit shape
    actions.sort_by_cached_key(code_action_key);

    // deduplicate identical actions after sorting
    actions.dedup_by(|left, right| code_action_key(left) == code_action_key(right));

    actions
}

/// Collect refactor actions for a range.
fn collect_refactor_actions(
    session: &Session,
    file: FileId,
    range: Span,
    actions: &mut Vec<CodeAction>,
) {
    // inline at the cursor start
    if let Some(result) = inline_symbol(session, file, range.start)
        && !result.is_empty()
    {
        actions.push(CodeAction::refactor(
            "Inline symbol",
            CodeActionKind::RefactorInline,
            result.edits,
        ));
    }

    // extract function for non empty selections
    if range.start < range.end
        && let Some(result) = extract_function(session, file, range, "extracted")
        && !result.is_empty()
    {
        actions.push(CodeAction::refactor(
            "Extract function",
            CodeActionKind::RefactorExtract,
            result.edits,
        ));
    }

    // extract constant for non empty selections
    if range.start < range.end
        && let Some(result) = extract_variable(session, file, range, "extracted")
        && !result.is_empty()
    {
        actions.push(CodeAction::refactor(
            "Extract constant",
            CodeActionKind::RefactorExtract,
            result.edits,
        ));
    }
}

/// Collect organize imports actions for a file.
fn collect_organize_imports_action(session: &Session, file: FileId, actions: &mut Vec<CodeAction>) {
    // resolve the module and query context
    let module = get_module_by_file_id(session, file);
    let Some(module) = module else {
        return;
    };
    let module = module.read();
    let ctx = session.query_context(&module);
    let Some(ctx) = ctx else {
        return;
    };

    // resolve the file text for slicing
    let source_file = session.files.get(file);
    let source = source_file.text();

    // collect top level import expressions in order
    let mut imports: Vec<(Span, String, String)> = Vec::new();

    for expr_id in &ctx.ast.roots {
        // stop once we hit the first non import expression after imports
        let expr = ctx.ast.tree.get(*expr_id);
        let target = match expr {
            ast::Expression::Import { target, .. } => Some(*target),
            ast::Expression::Statement(inner_id) => {
                let inner = ctx.ast.tree.get(*inner_id);
                if let ast::Expression::Import { target, .. } = inner {
                    Some(*target)
                } else {
                    None
                }
            }
            _ => None,
        };

        let Some(target) = target else {
            if !imports.is_empty() {
                break;
            }
            continue;
        };

        // resolve the import span and raw text
        let span = ctx.ast.tree.source_map.get(expr_id.id);
        let text = source
            .get(span.start as usize..span.end as usize)
            .unwrap_or("")
            .trim_end()
            .to_string();

        // resolve the import target for sorting
        let target_text = ctx.ast.strings.get(target).to_string();

        // store the import entry for sorting
        imports.push((span, target_text, text));
    }

    // skip when there is nothing to organize
    if imports.len() < 2 {
        return;
    }

    // sort imports by module path then text for stability
    let mut sorted = imports.clone();
    sorted.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(&right.2)));

    // skip when already organized
    let current_text: Vec<&str> = imports.iter().map(|entry| entry.2.as_str()).collect();
    let sorted_text: Vec<&str> = sorted.iter().map(|entry| entry.2.as_str()).collect();
    if current_text == sorted_text {
        return;
    }

    // build the replacement text for the import block
    let replacement = sorted
        .iter()
        .map(|entry| entry.2.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // compute the replacement span bounds for the import block
    let block_start = imports.first().map(|entry| entry.0.start).unwrap_or(0);
    let block_end = imports
        .last()
        .map(|entry| entry.0.end)
        .unwrap_or(block_start);

    // build the replacement span
    let block_span = Span::new(file, block_start, block_end);

    // build the edit batch for the code action
    let mut file_edit = FileEdit::new(file);
    file_edit.push(Edit::replace(block_span, replacement));

    let mut batch_edit = BatchEdit::new();
    batch_edit.files.push(file_edit);

    // register the organize imports action
    let action = CodeAction::refactor(
        "Organize Imports",
        CodeActionKind::SourceOrganizeImports,
        batch_edit,
    );
    actions.push(action);
}

/// Collect auto import actions for unresolved symbol diagnostics.
fn collect_auto_import_actions(
    session: &Session,
    file: FileId,
    range: Span,
    actions: &mut Vec<CodeAction>,
) {
    // track whether we added any actions
    let mut added_any = false;

    // resolve the current module for import exclusions
    let exclude_module_id = get_module_by_file_id(session, file).map(|module| module.read().id);

    // scan diagnostics for unresolved symbol codes
    for program in session.programs.iter() {
        // load diagnostics for the file
        let program = program.value();
        let diagnostics = program.diagnostic_store.diagnostics_for_file(file);
        for diagnostic in diagnostics {
            // skip diagnostics outside of the requested file
            if diagnostic.file_id != file {
                continue;
            }

            let diag_span = &diagnostic.primary_span.span;
            if diag_span.end < range.start || diag_span.start > range.end {
                continue;
            }

            // only handle unresolved symbol diagnostics
            if diagnostic.code != "ER100" && diagnostic.code != "ER101" {
                continue;
            }

            // extract the missing symbol name from the diagnostic span
            let Some(symbol_name) = missing_symbol_name(session, &diagnostic) else {
                continue;
            };

            let before_len = actions.len();
            let (import_mode, space_filter) =
                auto_import_mode_for_offset(session, file, diag_span.start);
            collect_auto_import_actions_for_symbol(
                session,
                file,
                &symbol_name,
                exclude_module_id,
                import_mode,
                space_filter,
                Some(&diagnostic.code),
                actions,
            );
            added_any |= actions.len() > before_len;
        }
    }

    // fall back to the token under the cursor when no diagnostics produced actions
    if !added_any
        && let Some(symbol_name) = token_at_offset(session, file, range.start)
        && is_simple_identifier(&symbol_name)
    {
        let (import_mode, space_filter) = auto_import_mode_for_offset(session, file, range.start);
        collect_auto_import_actions_for_symbol(
            session,
            file,
            &symbol_name,
            exclude_module_id,
            import_mode,
            space_filter,
            None,
            actions,
        );
    }
}

/// Collect auto import actions for a missing symbol name.
#[allow(clippy::too_many_arguments)]
fn collect_auto_import_actions_for_symbol(
    session: &Session,
    file: FileId,
    symbol_name: &str,
    exclude_module_id: Option<destack_source::ModuleId>,
    import_mode: ImportEditMode,
    space_filter: Option<SymbolSpace>,
    diagnostic_code: Option<&str>,
    actions: &mut Vec<CodeAction>,
) {
    // search exported symbols for exact name matches
    let program = program_for_file(session, file);
    let mut candidates =
        search_importable_symbols_for_program(session, &program, symbol_name, exclude_module_id);
    candidates.retain(|export| {
        export.name == symbol_name
            && matches_symbol_space_filter(export.kind, export.space, space_filter)
    });
    candidates.sort_by(|left, right| {
        let left_key = auto_import_space_rank(left.space);
        let right_key = auto_import_space_rank(right.space);
        left.module_path
            .cmp(&right.module_path)
            .then_with(|| left_key.cmp(&right_key))
    });

    // track seen module paths and preferred action index
    let mut seen_paths = HashSet::new();
    let mut action_index = 0;

    for export in candidates {
        // skip exports without a module path
        let Some(module_path) = &export.module_path else {
            continue;
        };

        // build an import path relative to the current file
        let display_path = build_import_display_path(session, file, module_path);

        // skip duplicate module path entries
        if !seen_paths.insert(display_path.clone()) {
            continue;
        }

        // build import edits and skip already imported symbols
        let import_edits =
            build_import_edits_with_mode(session, file, symbol_name, &display_path, import_mode);
        if import_edits.is_empty() {
            continue;
        }

        // collect edits into a batch edit
        let mut file_edit = FileEdit::new(file);
        for edit in import_edits {
            file_edit.push(edit);
        }

        // skip empty edits
        if file_edit.is_empty() {
            continue;
        }

        let mut batch_edit = BatchEdit::new();
        batch_edit.files.push(file_edit);

        // build the code action entry
        let title = format!("Import {symbol_name} from \"{display_path}\"");
        let mut action = CodeAction::quick_fix(title, batch_edit);
        if let Some(code) = diagnostic_code {
            action = action.with_diagnostic_code(code);
        }

        // prefer the first action for the symbol
        if action_index == 0 {
            action = action.preferred();
        }

        actions.push(action);
        action_index += 1;
    }
}

/// Rank export spaces for auto import actions.
fn auto_import_space_rank(space: dir::SymbolSpace) -> u8 {
    // return the rank for the space ordering
    match space {
        dir::SymbolSpace::Value => 0,
        dir::SymbolSpace::TypeValue => 1,
        dir::SymbolSpace::Type => 2,
        dir::SymbolSpace::Label => 3,
    }
}

/// Resolve the import mode and space filter for an offset.
fn auto_import_mode_for_offset(
    session: &Session,
    file: FileId,
    offset: u32,
) -> (ImportEditMode, Option<SymbolSpace>) {
    // detect the completion context at the cursor
    let context = detect_completion_context(session, file, offset);

    // choose import mode based on type position
    match context.context {
        CompletionContext::TypePosition { .. } => (ImportEditMode::Type, Some(SymbolSpace::Type)),
        _ => (ImportEditMode::Value, None),
    }
}

/// Resolve a missing symbol name from a diagnostic.
fn missing_symbol_name(session: &Session, diagnostic: &Diagnostic) -> Option<String> {
    // prefer extracting the symbol from the diagnostic span
    if let Some(name) = missing_symbol_name_from_span(session, diagnostic.primary_span.span) {
        return Some(name);
    }

    // fall back to extracting the symbol from the diagnostic message
    missing_symbol_name_from_message(&diagnostic.message)
}

/// Resolve a missing symbol name from a diagnostic span.
fn missing_symbol_name_from_span(session: &Session, span: Span) -> Option<String> {
    // read the source text for the span
    let file = session.files.get(span.file);
    let source = file.get_span_str(span)?;
    let name = source.trim();

    // ensure the span is a simple identifier
    if !is_simple_identifier(name) {
        return None;
    }

    // return the cleaned identifier
    Some(name.to_string())
}

/// Resolve a missing symbol name from a diagnostic message.
fn missing_symbol_name_from_message(message: &str) -> Option<String> {
    // extract the name from the error message prefix
    let name = message.strip_prefix("missing symbol ")?;
    let name = name.trim().trim_matches(&['\'', '"', '`'][..]);

    // extract the first identifier from the message suffix
    extract_identifier(name)
}

/// Collect quick fixes from diagnostics that overlap with the range.
fn collect_diagnostic_fixes(
    session: &Session,
    file: FileId,
    range: Span,
    actions: &mut Vec<CodeAction>,
) {
    // iterate through all programs to find diagnostics for this file
    for program in session.programs.iter() {
        // load diagnostics for the file
        let program = program.value();
        let diagnostics = program.diagnostic_store.diagnostics_for_file(file);
        for diagnostic in diagnostics {
            // skip diagnostics for other files
            if diagnostic.file_id != file {
                continue;
            }

            // check if diagnostic overlaps with the requested range
            let diag_span = &diagnostic.primary_span.span;
            if diag_span.end < range.start || diag_span.start > range.end {
                continue;
            }

            // convert suggestions to code actions
            if let Some(suggestions) = &diagnostic.suggestions {
                for suggestion in suggestions {
                    // skip non automatic suggestions
                    if suggestion.applicability != Applicability::Automatic {
                        continue;
                    }

                    // create edit from suggestion
                    let Some(replacement) = &suggestion.replacement else {
                        continue;
                    };

                    // build the batch edit from suggestion spans
                    let mut file_edit = FileEdit::new(file);
                    for labeled_span in &suggestion.spans {
                        file_edit.push(Edit::replace(labeled_span.span, replacement.clone()));
                    }

                    // skip empty edits
                    if file_edit.is_empty() {
                        continue;
                    }

                    let mut batch_edit = BatchEdit::new();
                    batch_edit.files.push(file_edit);

                    // build a preferred quick fix
                    let action = CodeAction::quick_fix(&suggestion.message, batch_edit)
                        .with_diagnostic_code(&diagnostic.code)
                        .preferred();

                    actions.push(action);
                }
            }
        }
    }
}

/// Build a stable ordering key for a code action.
fn code_action_key(action: &CodeAction) -> (u8, u8, String, String, String) {
    // rank kinds so quick fixes come before refactors and source actions
    let kind_rank = code_action_kind_rank(action.kind);

    // prefer preferred actions within the same kind
    let preferred_rank = if action.is_preferred { 0 } else { 1 };

    // include the diagnostic code when present to keep related fixes grouped
    let diagnostic_code = action.diagnostic_code.clone().unwrap_or_default();

    // fold the edit shape into the key to make ordering and deduplication stable
    let edit_key = batch_edit_key(&action.edits);

    (
        kind_rank,
        preferred_rank,
        action.title.clone(),
        diagnostic_code,
        edit_key,
    )
}

/// Rank code action kinds for stable ordering.
fn code_action_kind_rank(kind: CodeActionKind) -> u8 {
    // return the sort rank for this kind
    match kind {
        CodeActionKind::QuickFix => 0,
        CodeActionKind::Refactor => 1,
        CodeActionKind::RefactorExtract => 2,
        CodeActionKind::RefactorInline => 3,
        CodeActionKind::RefactorRewrite => 4,
        CodeActionKind::Source => 5,
        CodeActionKind::SourceOrganizeImports => 6,
        CodeActionKind::SourceFixAll => 7,
    }
}

/// Build a stable key for a batch edit.
fn batch_edit_key(edit: &BatchEdit) -> String {
    // clone and sort file edits by file id
    let mut files = edit.files.clone();
    files.sort_by_key(|file_edit| file_edit.file.0);

    // serialize edits in a deterministic order
    let mut parts = Vec::new();
    for file_edit in files {
        let file_key = file_edit_key(&file_edit);
        parts.push(file_key);
    }

    parts.join("|")
}

/// Build a stable key for a file edit.
fn file_edit_key(file_edit: &FileEdit) -> String {
    // clone and sort edits by span and text
    let mut edits = file_edit.edits.clone();
    edits.sort_by_key(edit_key);

    // serialize all edits for this file
    let mut parts = Vec::new();
    parts.push(format!("file={}", file_edit.file.0));
    for edit in edits {
        let (start, end, span_file, text) = edit_key(&edit);
        parts.push(format!("{span_file}:{start}-{end}=>{text}"));
    }

    parts.join(",")
}

/// Build a stable key for a single edit.
fn edit_key(edit: &Edit) -> (u32, u32, u32, String) {
    // extract span coordinates and replacement text
    let span_file = edit.span.file.0;
    let start = edit.span.start;
    let end = edit.span.end;
    let text = edit.new_text.clone();

    (start, end, span_file, text)
}
