#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use destack_source::{
    Applicability, BatchEdit, Diagnostic, DiagnosticLabel, Edit, FileEdit, FileId, Span,
};
use serde::{Deserialize, Serialize};

use crate::assist::CompletionContext;
use crate::core::{
    ModuleQueryContext, QueryRange, WorkspaceQueryContext, import_sort_key,
    repository_import_relevance,
};
use crate::dir::{ImportEditSpace, matches_export_space_filter};
use crate::source::is_simple_identifier;
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
    /// The queried range.
    pub range: QueryRange,
    /// The code action context.
    pub context: CodeActionContext,
}

/// Response payload for code actions queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

/// Collect quick fixes from diagnostics that overlap with the range.
fn collect_diagnostic_fixes(
    diagnostics: &[Diagnostic],
    file: FileId,
    range: Span,
    actions: &mut Vec<CodeAction>,
) {
    for diagnostic in diagnostics {
        let diagnostic_span = diagnostic.primary_label().span;

        // skip diagnostics for other files
        if diagnostic_span.file != file {
            continue;
        }

        // check if diagnostic overlaps with the requested range
        if diagnostic_span.end < range.start || diagnostic_span.start > range.end {
            continue;
        }

        // convert suggestions to code actions
        for suggestion in &diagnostic.suggestions {
            // skip non automatic suggestions
            if suggestion.applicability != Applicability::Automatic {
                continue;
            }

            // skip empty edits
            if suggestion.edits.is_empty() {
                continue;
            }

            // build a preferred quick fix
            let action = CodeAction::quick_fix(&suggestion.message, suggestion.edits.clone())
                .with_diagnostic_code(&diagnostic.code)
                .preferred();

            actions.push(action);
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
        CodeActionKind::SourceFixAll => 6,
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
fn edit_key(edit: &Edit) -> (u32, u32, u128, String) {
    // extract span coordinates and replacement text
    let span_file = edit.span.file.0;
    let start = edit.span.start;
    let end = edit.span.end;
    let text = edit.new_text.clone();

    (start, end, span_file, text)
}

impl ModuleQueryContext<'_> {
    /// Get code actions for a range in a file.
    ///
    /// Includes quick fixes from diagnostics and available refactorings.
    pub fn code_actions(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        range: Span,
        diagnostics: &[Diagnostic],
        context: &CodeActionContext,
    ) -> Vec<CodeAction> {
        let ctx = self;
        let file = ctx.file_id();

        // start with an empty action list
        let mut actions = Vec::new();

        // collect quick fixes from diagnostics
        collect_diagnostic_fixes(diagnostics, file, range, &mut actions);

        // collect auto import quick fixes for unresolved symbols
        ctx.collect_auto_import_actions(workspace, range, diagnostics, &mut actions);

        // collect refactor actions
        ctx.collect_refactor_actions(workspace, range, &mut actions);

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
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        range: Span,
        actions: &mut Vec<CodeAction>,
    ) {
        let ctx = self;
        // inline at the cursor start
        if let Some(edit) = ctx.inline_symbol(workspace, range.start)
            && !edit.is_empty()
        {
            actions.push(CodeAction::refactor(
                "Inline symbol",
                CodeActionKind::RefactorInline,
                edit,
            ));
        }

        // extract function for non empty selections
        if range.start < range.end
            && let Some(edit) = ctx.extract_function(range, "extracted")
            && !edit.is_empty()
        {
            actions.push(CodeAction::refactor(
                "Extract function",
                CodeActionKind::RefactorExtract,
                edit,
            ));
        }

        // extract constant for non empty selections
        if range.start < range.end
            && let Some(edit) = ctx.extract_variable(range, "extracted")
            && !edit.is_empty()
        {
            actions.push(CodeAction::refactor(
                "Extract constant",
                CodeActionKind::RefactorExtract,
                edit,
            ));
        }
    }

    /// Collect auto import actions for unresolved symbol diagnostics.
    fn collect_auto_import_actions(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        range: Span,
        diagnostics: &[Diagnostic],
        actions: &mut Vec<CodeAction>,
    ) {
        let ctx = self;
        let file = ctx.file_id();
        let exclude_module_id = Some(ctx.module_id());

        // scan diagnostics for unresolved symbol codes in the owning repository
        for diagnostic in diagnostics {
            let diagnostic_span = diagnostic.primary_label().span;

            // skip diagnostics outside of the requested file
            if diagnostic_span.file != file {
                continue;
            }

            if diagnostic_span.end < range.start || diagnostic_span.start > range.end {
                continue;
            }

            // only handle unresolved symbol diagnostics
            if diagnostic.code != "ER100" && diagnostic.code != "ER101" {
                continue;
            }

            // extract the missing symbol name from the diagnostic span
            let Some(symbol_name) = ctx.missing_symbol_name(diagnostic) else {
                continue;
            };

            let space_filter = ctx.auto_import_form_filter_for_offset(diagnostic_span.start);
            ctx.collect_auto_import_actions_for_symbol(
                workspace,
                &symbol_name,
                exclude_module_id,
                space_filter,
                Some(&diagnostic.code),
                actions,
            );
        }

        // allow token-driven auto-imports when diagnostics are unavailable
        if let Some(symbol_name) = ctx.token_at_offset(range.start)
            && is_simple_identifier(&symbol_name)
        {
            let space_filter = ctx.auto_import_form_filter_for_offset(range.start);
            ctx.collect_auto_import_actions_for_symbol(
                workspace,
                &symbol_name,
                exclude_module_id,
                space_filter,
                None,
                actions,
            );
        }
    }

    /// Collect auto import actions for a missing symbol name.
    fn collect_auto_import_actions_for_symbol(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        symbol_name: &str,
        exclude_module_id: Option<destack_source::ModuleId>,
        space_filter: Option<SymbolSpace>,
        diagnostic_code: Option<&str>,
        actions: &mut Vec<CodeAction>,
    ) {
        let ctx = self;
        let repository = ctx.repository();
        let revision = ctx.revision();
        let file = ctx.file_id();

        // search exported symbols for exact name matches
        let Some(current_module) = repository.module(revision, ctx.module_id()).ok().flatten()
        else {
            return;
        };
        let current_package_id = Some(current_module.package_id);
        let mut candidates = workspace.search_importable_symbols(symbol_name, exclude_module_id);
        candidates.retain(|export| {
            export.name == symbol_name && matches_export_space_filter(export.space, space_filter)
        });

        // track seen module paths and preferred action index
        let mut ranked_candidates = Vec::new();
        let mut seen_paths = HashSet::new();
        let mut action_index = 0;

        for export in candidates {
            // skip exports without a module path
            let Some(module_path) = &export.module_path else {
                continue;
            };

            // build an import path relative to the current file
            let display_path = ctx.import_display_path(module_path);

            // skip duplicate module path entries
            if !seen_paths.insert(display_path.clone()) {
                continue;
            }

            // compute shared import relevance
            let Some(relevance) = repository_import_relevance(
                repository,
                revision,
                file,
                current_package_id,
                symbol_name,
                &export.name,
                space_filter,
                export.space,
                export.module_id,
                module_path,
            ) else {
                continue;
            };

            let sort_key = import_sort_key(&relevance, &display_path, &export.name);
            ranked_candidates.push((sort_key, export, display_path));
        }

        // sort actions with the same import relevance as completions
        ranked_candidates.sort_by(|left, right| {
            let left_key = (&left.0, &left.2, &left.1.name);
            let right_key = (&right.0, &right.2, &right.1.name);
            left_key.cmp(&right_key)
        });

        for (_, export, display_path) in ranked_candidates {
            let import_form = ImportEditSpace::for_auto_import(space_filter, export.space);

            // build import edits and skip already imported symbols
            let import_edits = ctx.build_import_edits(symbol_name, &display_path, import_form);
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

    /// Resolve the auto import space filter for an offset.
    fn auto_import_form_filter_for_offset(&self, offset: u32) -> Option<SymbolSpace> {
        let ctx = self;
        // detect the completion context at the cursor
        let context = ctx.completion_input_at_offset(offset);

        // choose import visibility based on type position
        match context.context {
            CompletionContext::TypePosition { .. } => Some(SymbolSpace::Type),
            _ => Some(SymbolSpace::Value),
        }
    }

    /// Resolve a missing symbol name from a diagnostic.
    fn missing_symbol_name(&self, diagnostic: &Diagnostic) -> Option<String> {
        let ctx = self;
        ctx.missing_symbol_name_from_label(diagnostic.primary_label())
    }

    /// Resolve a missing symbol name from a diagnostic label.
    fn missing_symbol_name_from_label(&self, label: &DiagnosticLabel) -> Option<String> {
        let ctx = self;
        // read the source text for the span
        let span = label.span;
        let file = ctx
            .repository()
            .file(ctx.revision(), span.file)
            .ok()
            .flatten()?;
        if file.content_id() != label.content {
            return None;
        }
        let source = file.get_span_str(span)?;
        let name = source.trim();

        // ensure the span is a simple identifier
        if !is_simple_identifier(name) {
            return None;
        }

        // return the cleaned identifier
        Some(name.to_string())
    }
}
