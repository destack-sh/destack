use std::cmp::Ordering;

use destack_serde::Reflect;
use destack_source::{
    Applicability, Diagnostic, DiagnosticReference, DiagnosticTarget, FilePatch, Patch, PatchSet,
    Span,
};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::source::{ImportBinding, is_simple_identifier};
use crate::{
    ImportCandidate, ImportOrder, ModuleQueryContext, ProgramQueryContext, QueryError, QueryRange,
    QueryResult, SymbolUse,
};

/// Kind of code action.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic.
    QuickFix,
    /// Extraction refactoring.
    RefactorExtract,
    /// Inline refactoring.
    RefactorInline,
}

/// One source edit offered by the editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeAction {
    /// The title shown in the UI.
    pub title: String,
    /// The kind of action.
    pub kind: CodeActionKind,
    /// Edits to apply.
    pub patches: PatchSet,
    /// Applicability of a diagnostic suggestion.
    pub applicability: Option<Applicability>,
    /// Whether this is the preferred action for its diagnostics.
    pub is_preferred: bool,
    /// The exact diagnostics addressed by this action.
    pub diagnostics: Vec<DiagnosticReference>,
}

impl CodeAction {
    /// Create a quick fix.
    fn quick_fix(
        title: impl Into<String>,
        mut patches: PatchSet,
        applicability: Applicability,
    ) -> Self {
        sort_action_patches(&mut patches);

        Self {
            title: title.into(),
            kind: CodeActionKind::QuickFix,
            patches,
            applicability: Some(applicability),
            is_preferred: false,
            diagnostics: Vec::new(),
        }
    }

    /// Create a refactoring.
    fn refactor(title: impl Into<String>, kind: CodeActionKind, mut patches: PatchSet) -> Self {
        sort_action_patches(&mut patches);

        Self {
            title: title.into(),
            kind,
            patches,
            applicability: None,
            is_preferred: false,
            diagnostics: Vec::new(),
        }
    }

    /// Mark as preferred.
    fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    /// Add one exact diagnostic addressed by this action.
    fn with_diagnostic(mut self, diagnostic: &Diagnostic) -> Self {
        self.diagnostics.push(diagnostic.into());

        self
    }
}

/// Context for code action requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionContext {
    /// Requested action kinds, or every kind when empty.
    pub only: Vec<CodeActionKind>,
    /// Exact requested diagnostics, or every matching diagnostic when absent.
    pub diagnostics: Option<Vec<DiagnosticReference>>,
}

impl CodeActionContext {
    /// Return whether one action kind was requested.
    pub fn includes(&self, kind: CodeActionKind) -> bool {
        self.only.is_empty() || self.only.contains(&kind)
    }

    /// Return whether one exact diagnostic was requested.
    fn includes_diagnostic(&self, diagnostic: &Diagnostic) -> bool {
        self.diagnostics.as_ref().is_none_or(|diagnostics| {
            diagnostics.iter().any(|requested| {
                requested.id == diagnostic.id && requested.primary == diagnostic.primary
            })
        })
    }
}

/// Request code actions for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsRequest {
    /// The queried range.
    pub range: QueryRange,
    /// The code action context.
    pub context: CodeActionContext,
}

/// Response payload for code actions queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

/// One exact import action before response construction.
struct ImportAction {
    /// The shared import candidate order.
    order: ImportOrder,
    /// The binding introduced by the import.
    binding: ImportBinding,
    /// The import specifier emitted in source.
    specifier: String,
    /// The source patches that add the import.
    patches: Vec<Patch>,
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
    ) -> QueryResult<Vec<CodeAction>> {
        let mut actions = Vec::new();

        // collect diagnostic actions only when requested
        if context.includes(CodeActionKind::QuickFix) {
            self.collect_diagnostic_fixes(diagnostics, range, context, &mut actions)?;
            self.collect_import_actions(program, diagnostics, range, context, &mut actions)?;
        }

        // collect refactor actions only when requested
        self.collect_refactor_actions(program, range, context, &mut actions)?;

        // sort deterministically by kind, preference, title, and edit shape
        actions.sort_by(compare_code_actions);

        // deduplicate identical actions after sorting
        actions.dedup_by(|left, right| compare_code_actions(left, right).is_eq());

        Ok(actions)
    }

    /// Collect refactor actions for a range.
    fn collect_refactor_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        context: &CodeActionContext,
        actions: &mut Vec<CodeAction>,
    ) -> QueryResult<()> {
        // inline at the cursor start
        if context.includes(CodeActionKind::RefactorInline)
            && let Some(edit) = self.inline(program, range.file, range.start)?
            && !edit.is_empty()
        {
            actions.push(CodeAction::refactor(
                "Inline symbol",
                CodeActionKind::RefactorInline,
                edit,
            ));
        }

        // extract constant for non empty selections
        if context.includes(CodeActionKind::RefactorExtract)
            && range.start < range.end
            && let Some(edit) = self.extract_variable(program, range, "extracted")?
            && !edit.is_empty()
        {
            actions.push(CodeAction::refactor(
                "Extract constant",
                CodeActionKind::RefactorExtract,
                edit,
            ));
        }

        Ok(())
    }

    /// Collect quick fixes from diagnostics that overlap with the range.
    fn collect_diagnostic_fixes(
        &self,
        diagnostics: &[Diagnostic],
        range: Span,
        context: &CodeActionContext,
        actions: &mut Vec<CodeAction>,
    ) -> QueryResult<()> {
        for diagnostic in diagnostics {
            if !context.includes_diagnostic(diagnostic) {
                continue;
            }

            // retain only diagnostics selected by the request
            if !diagnostic_target_matches_range(diagnostic.primary.target, range) {
                continue;
            }

            // convert exact diagnostic suggestions to quick fixes
            for suggestion in &diagnostic.suggestions {
                if suggestion.patches.is_empty() {
                    return Err(QueryError::missing(format!(
                        "code action suggestion: {:?}",
                        diagnostic.id.clone()
                    )));
                }

                let mut action = CodeAction::quick_fix(
                    &suggestion.message,
                    suggestion.patches.clone(),
                    suggestion.applicability,
                )
                .with_diagnostic(diagnostic);
                if suggestion.applicability == Applicability::Automatic {
                    action = action.preferred();
                }

                actions.push(action);
            }
        }

        Ok(())
    }

    /// Collect import actions for exact unresolved-reference diagnostics.
    fn collect_import_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        diagnostics: &[Diagnostic],
        range: Span,
        context: &CodeActionContext,
        actions: &mut Vec<CodeAction>,
    ) -> QueryResult<()> {
        for diagnostic in diagnostics {
            if diagnostic.id != "unresolved-reference"
                || !context.includes_diagnostic(diagnostic)
                || !diagnostic_target_matches_range(diagnostic.primary.target, range)
            {
                continue;
            }
            let diagnostic_span = diagnostic
                .primary
                .target
                .span()
                .ok_or_else(|| QueryError::invalid("unresolved-reference diagnostic target"))?;

            // read the exact unresolved authored name
            let source = self
                .repository()
                .file(self.revision(), diagnostic_span.file)?
                .ok_or(QueryError::missing(format!(
                    "source file: {:?}",
                    diagnostic_span.file
                )))?;
            if source.content_id() != diagnostic.primary_label().content {
                return Err(QueryError::stale(format!(
                    "code action diagnostic: {:?}",
                    diagnostic.id.clone()
                )));
            }
            let name = source
                .get_span_str(diagnostic_span)
                .ok_or(QueryError::invalid(format!(
                    "source span: {diagnostic_span:?}"
                )))?;
            if !is_simple_identifier(name) {
                return Err(QueryError::invalid(format!(
                    "unresolved-reference diagnostic span: {diagnostic_span:?}"
                )));
            }
            let symbol_use = if self.is_type_position(diagnostic_span.file, diagnostic_span.start) {
                SymbolUse::Type
            } else {
                SymbolUse::Value
            };

            // produce one action per exact import plan
            let imports = self.import_actions(program, diagnostic_span.file, name, symbol_use)?;
            for (index, import) in imports.into_iter().enumerate() {
                let title = format!("Import {name} from \"{}\"", import.specifier);
                let mut file_edit = FilePatch::with_patches(diagnostic_span.file, import.patches);
                file_edit.sort();
                let mut patches = PatchSet::new();
                patches.push(file_edit);
                let mut action = CodeAction::quick_fix(title, patches, Applicability::Automatic)
                    .with_diagnostic(diagnostic);
                if index == 0 {
                    action = action.preferred();
                }
                actions.push(action);
            }
        }

        Ok(())
    }

    /// Plan every exact import for one unresolved name.
    fn import_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        file: destack_source::FileId,
        name: &str,
        symbol_use: SymbolUse,
    ) -> QueryResult<Vec<ImportAction>> {
        let mut imports = Vec::new();
        let mut seen = FxHashSet::default();
        let candidates = program.search_export_candidates(name, Some(self.module_id()))?;

        // retain exact matching exports and their importable specifiers
        for candidate in candidates {
            if candidate.binding.name() != name
                || !symbol_use.accepts_export(candidate.declaration)
                || !seen.insert((candidate.module, candidate.binding.clone()))
            {
                continue;
            }
            let specifiers = program.import_specifiers(self.module_id(), candidate.module)?;
            for specifier in specifiers {
                let patches = self.build_import_edits(file, &candidate.binding, &specifier)?;
                if patches.is_empty() {
                    continue;
                }
                let order = ImportCandidate {
                    repository: self.repository(),
                    revision: self.revision(),
                    current_module_id: self.module_id(),
                    export_name: name,
                    expected_use: Some(symbol_use),
                    declaration: candidate.declaration,
                    module_id: candidate.module,
                }
                .order(&specifier)?;
                imports.push(ImportAction {
                    order,
                    binding: candidate.binding.clone(),
                    specifier,
                    patches,
                });
            }
        }
        imports.sort_by(|left, right| {
            (&left.order, &left.binding, &left.specifier).cmp(&(
                &right.order,
                &right.binding,
                &right.specifier,
            ))
        });

        Ok(imports)
    }
}

/// Return whether one diagnostic target is selected by a request.
fn diagnostic_target_matches_range(diagnostic: DiagnosticTarget, range: Span) -> bool {
    match diagnostic {
        DiagnosticTarget::Span(diagnostic) if range.is_empty() => {
            diagnostic.owns_cursor(range.start)
        }
        DiagnosticTarget::Span(diagnostic) => diagnostic.intersects(range),
        DiagnosticTarget::File(file) => file == range.file,
    }
}

/// Sort and remove empty file patches once before returning an action.
fn sort_action_patches(patches: &mut PatchSet) {
    patches.files.retain(|file| !file.patches.is_empty());
    for file in &mut patches.files {
        file.patches
            .sort_by(|left, right| patch_order(left).cmp(&patch_order(right)));
    }
    patches.files.sort_by_key(|file| file.file);
}

/// Compare code actions in stable protocol order.
fn compare_code_actions(left: &CodeAction, right: &CodeAction) -> Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| right.is_preferred.cmp(&left.is_preferred))
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| {
            applicability_order(left.applicability).cmp(&applicability_order(right.applicability))
        })
        .then_with(|| compare_diagnostics(&left.diagnostics, &right.diagnostics))
        .then_with(|| compare_patches(&left.patches, &right.patches))
}

/// Return the stable order for optional suggestion applicability.
fn applicability_order(applicability: Option<Applicability>) -> u8 {
    match applicability {
        None => 0,
        Some(Applicability::Automatic) => 1,
        Some(Applicability::Unsafe) => 2,
        Some(Applicability::Dangerous) => 3,
    }
}

/// Compare exact diagnostic occurrences without allocating order keys.
fn compare_diagnostics(left: &[DiagnosticReference], right: &[DiagnosticReference]) -> Ordering {
    left.iter()
        .map(diagnostic_order)
        .cmp(right.iter().map(diagnostic_order))
}

/// Return the stable order key for one diagnostic occurrence.
fn diagnostic_order(
    diagnostic: &DiagnosticReference,
) -> (&str, u128, bool, u64, u32, u32, Option<&str>) {
    let (is_file, file, start, end) = match diagnostic.primary.target {
        DiagnosticTarget::Span(span) => (false, span.file.0, span.start, span.end),
        DiagnosticTarget::File(file) => (true, file.0, 0, 0),
    };

    (
        &diagnostic.id,
        diagnostic.primary.content.0,
        is_file,
        file,
        start,
        end,
        diagnostic.primary.message.as_deref(),
    )
}

/// Compare canonical patch sets without allocating order keys.
fn compare_patches(left: &PatchSet, right: &PatchSet) -> Ordering {
    left.iter()
        .map(patch_order)
        .cmp(right.iter().map(patch_order))
}

/// Return the stable order key for one source patch.
fn patch_order(patch: &Patch) -> (u64, u32, u32, &str) {
    (
        patch.span.file.0,
        patch.span.start,
        patch.span.end,
        &patch.new_text,
    )
}
