#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{
    Applicability, Diagnostic, DiagnosticLabel, FilePatch, Patch, PatchSet, Span,
};
use serde::{Deserialize, Serialize};

use crate::completion::CompletionContext;
use crate::refactor::ImportEditForm;
use crate::source::is_simple_identifier;
use crate::{
    ImportOrder, ModuleQueryContext, ProgramQueryContext, Range, SymbolUse,
    repository_import_relevance,
};

/// Kind of code action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeAction {
    /// The title shown in the UI.
    pub title: String,
    /// The kind of action.
    pub kind: CodeActionKind,
    /// Edits to apply.
    pub patches: PatchSet,
    /// Whether this is the preferred action for its diagnostics.
    pub is_preferred: bool,
    /// Whether this action is disabled (with reason).
    pub disabled_reason: Option<String>,
    /// The diagnostic code this action fixes (if from a diagnostic).
    pub diagnostic_code: Option<String>,
}

impl CodeAction {
    /// Create a quick fix.
    pub fn quick_fix(title: impl Into<String>, patches: PatchSet) -> Self {
        Self {
            title: title.into(),
            kind: CodeActionKind::QuickFix,
            patches,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Create a refactoring.
    pub fn refactor(title: impl Into<String>, kind: CodeActionKind, patches: PatchSet) -> Self {
        Self {
            title: title.into(),
            kind,
            patches,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Mark as preferred.
    pub fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.disabled_reason = Some(reason.into());
        self
    }

    /// Set the diagnostic code this action fixes.
    pub fn with_diagnostic_code(mut self, code: impl Into<String>) -> Self {
        self.diagnostic_code = Some(code.into());
        self
    }
}

/// Context for code action requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionContext {
    /// Requested action kinds (empty = all).
    pub only: Vec<CodeActionKind>,
    /// Whether to include disabled actions.
    pub include_disabled: bool,
}

/// Request code actions for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsRequest {
    /// The queried range.
    pub range: Range,
    /// The code action context.
    pub context: CodeActionContext,
}

/// Response payload for code actions queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

/// The stable order for one code action.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CodeActionOrder {
    /// The action-kind priority.
    kind_priority: u8,
    /// Whether this is not a preferred action.
    preferred_miss: u8,
    /// The action title.
    title: String,
    /// The diagnostic code when present.
    diagnostic_code: String,
    /// The serialized edit shape.
    patches: String,
}

/// The stable order for one text patch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PatchOrder {
    /// The source start offset.
    start: u32,
    /// The source end offset.
    end: u32,
    /// The source file id.
    file: u128,
    /// The replacement text.
    text: String,
}

/// One export candidate for an auto import action.
#[derive(Debug, Clone)]
struct ImportActionCandidate {
    /// The import relevance order.
    order: ImportOrder,
    /// The exported symbol.
    export: dir::ExportEntry,
    /// The display import path.
    display_path: String,
}

impl CodeActionOrder {
    /// Build the stable order for one action.
    fn new(action: &CodeAction) -> Self {
        Self {
            kind_priority: action.kind.priority(),
            preferred_miss: u8::from(!action.is_preferred),
            title: action.title.clone(),
            diagnostic_code: match &action.diagnostic_code {
                Some(code) => code.clone(),
                None => String::new(),
            },
            patches: Self::patch_set_text(&action.patches),
        }
    }

    /// Serialize a patch set in stable order.
    fn patch_set_text(patches: &PatchSet) -> String {
        let mut files = patches.files.clone();
        files.sort_by_key(|file_patch| file_patch.file.0);

        let mut parts = Vec::new();
        for file_patch in files {
            let file_text = Self::file_patch_text(&file_patch);
            parts.push(file_text);
        }

        parts.join("|")
    }

    /// Serialize a file patch in stable order.
    fn file_patch_text(file_patch: &FilePatch) -> String {
        let mut patches = file_patch.patches.clone();
        patches.sort_by_key(PatchOrder::new);

        let mut parts = Vec::new();
        parts.push(format!("file={}", file_patch.file.0));
        for patch in patches {
            let patch = PatchOrder::new(&patch);
            let file = patch.file;
            let start = patch.start;
            let end = patch.end;
            let text = patch.text;
            parts.push(format!("{file}:{start}-{end}=>{text}"));
        }

        parts.join(",")
    }
}

impl CodeActionKind {
    /// Return the stable action-kind priority.
    fn priority(self) -> u8 {
        match self {
            CodeActionKind::QuickFix => 0,
            CodeActionKind::Refactor => 1,
            CodeActionKind::RefactorExtract => 2,
            CodeActionKind::RefactorInline => 3,
            CodeActionKind::RefactorRewrite => 4,
            CodeActionKind::Source => 5,
            CodeActionKind::SourceFixAll => 6,
        }
    }
}

impl PatchOrder {
    /// Build the stable order for one patch.
    fn new(patch: &Patch) -> Self {
        Self {
            start: patch.span.start,
            end: patch.span.end,
            file: patch.span.file.0,
            text: patch.new_text.clone(),
        }
    }
}

impl ImportActionCandidate {
    /// Build one auto import action candidate.
    fn new(order: ImportOrder, export: dir::ExportEntry, display_path: String) -> Self {
        Self {
            order,
            export,
            display_path,
        }
    }

    /// Compare two auto import action candidates.
    fn compare(left: &Self, right: &Self) -> std::cmp::Ordering {
        (
            &left.order,
            left.display_path.as_str(),
            left.export.name.as_str(),
        )
            .cmp(&(
                &right.order,
                right.display_path.as_str(),
                right.export.name.as_str(),
            ))
    }
}

impl ModuleQueryContext<'_> {
    /// Return code actions for a range in a file.
    ///
    /// Includes quick fixes from diagnostics and available refactorings.
    pub fn code_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        diagnostics: &[Diagnostic],
        context: &CodeActionContext,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // collect quick fixes from diagnostics
        self.collect_diagnostic_fixes(diagnostics, range, &mut actions);

        // collect auto import quick fixes for unresolved symbols
        self.collect_auto_import_actions(program, range, diagnostics, &mut actions);

        // collect refactor actions
        self.collect_refactor_actions(program, range, &mut actions);

        // filter by requested kinds when specified
        if !context.only.is_empty() {
            actions.retain(|action| context.only.contains(&action.kind));
        }

        // filter out disabled actions unless requested
        if !context.include_disabled {
            actions.retain(|action| action.disabled_reason.is_none());
        }

        // sort deterministically by kind, preference, title, and edit shape
        actions.sort_by_cached_key(CodeActionOrder::new);

        // deduplicate identical actions after sorting
        actions.dedup_by(|left, right| CodeActionOrder::new(left) == CodeActionOrder::new(right));

        actions
    }

    /// Collect refactor actions for a range.
    fn collect_refactor_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        actions: &mut Vec<CodeAction>,
    ) {
        // inline at the cursor start
        if let Some(edit) = self.inline_symbol(program, range.start) {
            if !edit.is_empty() {
                actions.push(CodeAction::refactor(
                    "Inline symbol",
                    CodeActionKind::RefactorInline,
                    edit,
                ));
            }
        }

        // extract function for non empty selections
        if range.start < range.end {
            if let Some(edit) = self.extract_function(range, "extracted") {
                if !edit.is_empty() {
                    actions.push(CodeAction::refactor(
                        "Extract function",
                        CodeActionKind::RefactorExtract,
                        edit,
                    ));
                }
            }
        }

        // extract constant for non empty selections
        if range.start < range.end {
            if let Some(edit) = self.extract_variable(range, "extracted") {
                if !edit.is_empty() {
                    actions.push(CodeAction::refactor(
                        "Extract constant",
                        CodeActionKind::RefactorExtract,
                        edit,
                    ));
                }
            }
        }
    }

    /// Collect quick fixes from diagnostics that overlap with the range.
    fn collect_diagnostic_fixes(
        &self,
        diagnostics: &[Diagnostic],
        range: Span,
        actions: &mut Vec<CodeAction>,
    ) {
        let file = self.file_id();

        for diagnostic in diagnostics {
            let diagnostic_span = diagnostic.primary_label().span;

            // skip diagnostics for other files
            if diagnostic_span.file != file {
                continue;
            }

            // skip diagnostics outside the requested range
            if diagnostic_span.end < range.start || diagnostic_span.start > range.end {
                continue;
            }

            // convert automatic suggestions to quick fixes
            for suggestion in &diagnostic.suggestions {
                if suggestion.applicability != Applicability::Automatic {
                    continue;
                }

                if suggestion.patches.is_empty() {
                    continue;
                }

                let action = CodeAction::quick_fix(&suggestion.message, suggestion.patches.clone())
                    .with_diagnostic_code(&diagnostic.code)
                    .preferred();

                actions.push(action);
            }
        }
    }

    /// Collect auto import actions for unresolved symbol diagnostics.
    fn collect_auto_import_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        diagnostics: &[Diagnostic],
        actions: &mut Vec<CodeAction>,
    ) {
        let file = self.file_id();
        let exclude_module_id = Some(self.module_id());

        // scan diagnostics for unresolved symbol codes in the owning repository
        for diagnostic in diagnostics {
            let diagnostic_span = diagnostic.primary_label().span;

            // skip diagnostics outside of the requested file
            if diagnostic_span.file != file {
                continue;
            }

            // skip diagnostics outside the requested range
            if diagnostic_span.end < range.start || diagnostic_span.start > range.end {
                continue;
            }

            // only handle unresolved symbol diagnostics
            if diagnostic.code != "ER100" && diagnostic.code != "ER101" {
                continue;
            }

            // extract the missing symbol name from the diagnostic span
            let Some(symbol_name) = self.missing_symbol_name(diagnostic) else {
                continue;
            };

            let symbol_use = self.offset_auto_import_use(diagnostic_span.start);
            self.collect_symbol_auto_import_actions(
                program,
                &symbol_name,
                exclude_module_id,
                symbol_use,
                Some(&diagnostic.code),
                actions,
            );
        }

        // allow token-driven auto-imports when diagnostics are unavailable
        if let Some(symbol_name) = self.token_at_offset(range.start) {
            if is_simple_identifier(&symbol_name) {
                let symbol_use = self.offset_auto_import_use(range.start);
                self.collect_symbol_auto_import_actions(
                    program,
                    &symbol_name,
                    exclude_module_id,
                    symbol_use,
                    None,
                    actions,
                );
            }
        }
    }

    /// Collect auto import actions for a missing symbol name.
    fn collect_symbol_auto_import_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_name: &str,
        exclude_module_id: Option<destack_source::ModuleId>,
        symbol_use: SymbolUse,
        diagnostic_code: Option<&str>,
        actions: &mut Vec<CodeAction>,
    ) {
        let repository = self.repository();
        let revision = self.revision();
        let file = self.file_id();

        // search exported symbols for exact name matches
        let current_module = repository
            .module(revision, self.module_id())
            .unwrap_or_else(|error| panic!("failed to read current module: {error}"))
            .unwrap_or_else(|| panic!("missing current module {:?}", self.module_id()));
        let current_package_id = Some(current_module.package_id);
        let mut candidates = program.search_export_candidates(symbol_name, exclude_module_id);
        candidates.retain(|export| {
            let matches_name = export.name == symbol_name;
            let matches_use = symbol_use.accepts_symbol_kind(export.kind);

            matches_name && matches_use
        });

        // track seen module paths and ordered candidates
        let mut import_candidates = Vec::new();
        let mut seen_paths = HashSet::new();
        let mut action_index = 0;

        for export in candidates {
            // skip exports without a module path
            let Some(module_path) = &export.module_path else {
                continue;
            };

            // build an import path relative to the current file
            let display_path = self.import_display_path(module_path);

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
                Some(symbol_use),
                export.kind,
                export.source.module_id,
                module_path,
            ) else {
                continue;
            };

            let order = ImportOrder::new(&relevance, &display_path, &export.name);
            import_candidates.push(ImportActionCandidate::new(order, export, display_path));
        }

        // sort actions with the same import relevance as completions
        import_candidates.sort_by(ImportActionCandidate::compare);

        for candidate in import_candidates {
            let export = candidate.export;
            let display_path = candidate.display_path;
            let import_form = ImportEditForm::auto_import(Some(symbol_use), export.kind);

            // build import patches and skip already imported symbols
            let import_edits = self.build_import_edits(symbol_name, &display_path, import_form);
            if import_edits.is_empty() {
                continue;
            }

            // collect patches into a batch edit
            let mut file_edit = FilePatch::new(file);
            for edit in import_edits {
                file_edit.push(edit);
            }

            // skip empty patches
            if file_edit.is_empty() {
                continue;
            }

            let mut batch_edit = PatchSet::new();
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

    /// Resolve the auto import use filter for an offset.
    fn offset_auto_import_use(&self, offset: u32) -> SymbolUse {
        // detect the completion context at the cursor
        let cursor = self.completion_cursor(offset);

        // choose import use based on type position
        match cursor.context {
            CompletionContext::TypePosition { .. } => SymbolUse::Type,
            _ => SymbolUse::Value,
        }
    }

    /// Resolve a missing symbol name from a diagnostic.
    fn missing_symbol_name(&self, diagnostic: &Diagnostic) -> Option<String> {
        self.missing_symbol_name_from_label(diagnostic.primary_label())
    }

    /// Resolve a missing symbol name from a diagnostic label.
    fn missing_symbol_name_from_label(&self, label: &DiagnosticLabel) -> Option<String> {
        // read the source text for the span
        let span = label.span;
        let file = self.read_file(span.file);
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
