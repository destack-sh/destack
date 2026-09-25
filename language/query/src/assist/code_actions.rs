use std::cmp::Ordering;

use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_core::Blob;
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{
    Applicability, Diagnostic, DiagnosticReference, DiagnosticTarget, FilePatch, Patch, PatchSet,
    Span,
};

use crate::source::ImportBinding;
use crate::{
    DeclarationUse, ExtractVariableRequest, ImportOrder, ImportPathOrder, InlineRequest,
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryRange, QueryResult,
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
        patches.sort();

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
        patches.sort();

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

/// A code actions request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsRequest {
    /// The queried range.
    pub range: QueryRange,
    /// The code action context.
    pub context: CodeActionContext,
}

/// A code actions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

impl ModuleQueryContext<'_> {
    /// Return code actions for a range in a file.
    ///
    /// Includes quick fixes from diagnostics and available refactorings.
    pub fn code_actions(
        &self,
        request: CodeActionsRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CodeActionsResponse> {
        let range = request.range.span;
        let context = &request.context;

        // read diagnostics only for quick fixes
        let diagnostics = if context.includes(CodeActionKind::QuickFix) {
            let artifact = ArtifactKey::dir_checked(self.module_id(), self.profile_id());
            let diagnostics = self
                .repository()
                .diagnostics_for_keys(self.revision(), &[artifact])?;
            let mut diagnostics = diagnostics.group_by_file();
            let mut file_diagnostics = Vec::new();

            // retain diagnostics emitted for the requested source file
            if let Some(diagnostics) = diagnostics.remove(&range.file) {
                file_diagnostics = diagnostics;
            }

            file_diagnostics
        }
        // skip diagnostic reads for refactor-only requests
        else {
            Vec::new()
        };
        let mut actions = Vec::new();

        // collect diagnostic actions only when requested
        if context.includes(CodeActionKind::QuickFix) {
            self.collect_diagnostic_fixes(&diagnostics, range, context, &mut actions)?;
            self.collect_import_actions(program, &diagnostics, range, context, &mut actions)?;
        }

        // collect refactor actions only when requested
        self.collect_refactor_actions(program, range, context, &mut actions)?;

        // sort deterministically by kind, preference, title, and edit shape
        actions.sort_by(CodeAction::protocol_order);

        // deduplicate identical actions after sorting
        actions.dedup_by(|left, right| left.protocol_order(right).is_eq());

        Ok(CodeActionsResponse { actions })
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
        if context.includes(CodeActionKind::RefactorInline) {
            let request = InlineRequest {
                position: QueryPosition {
                    module: self.module(),
                    file_id: range.file,
                    offset: range.start,
                },
            };

            // retain one non empty inline edit
            if let Some(edit) = self.inline(request, program)?.edit
                && !edit.is_empty()
            {
                actions.push(CodeAction::refactor(
                    "Inline symbol",
                    CodeActionKind::RefactorInline,
                    edit,
                ));
            }
        }

        // extract constant for non empty selections
        if context.includes(CodeActionKind::RefactorExtract) && range.start < range.end {
            let request = ExtractVariableRequest {
                range: QueryRange {
                    module: self.module(),
                    span: range,
                },
                new_name: "extracted".to_string(),
            };

            // retain one non empty extraction edit
            if let Some(edit) = self.extract_variable(request, program)?.edit
                && !edit.is_empty()
            {
                actions.push(CodeAction::refactor(
                    "Extract constant",
                    CodeActionKind::RefactorExtract,
                    edit,
                ));
            }
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
            if !diagnostic.primary.target.matches_range(range) {
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
                || !diagnostic.primary.target.matches_range(range)
            {
                continue;
            }
            let (diagnostic_span, name, usage) = self.unresolved_reference(diagnostic)?;

            // produce one action per exact import plan
            let imports = self.import_actions(program, diagnostic_span.file, &name, usage)?;
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

    /// Return the unresolved reference selected by one diagnostic.
    fn unresolved_reference(
        &self,
        diagnostic: &Diagnostic,
    ) -> QueryResult<(Span, String, DeclarationUse)> {
        // match the diagnostic span against the retained unresolved paths
        let view = self.view()?;
        for (node, path) in self.resolutions()?.unresolved_entries() {
            let source_node_id = view.get_source_any(node.local_id);
            let Some(span) = self.source_index()?.get_main(source_node_id) else {
                continue;
            };
            if !diagnostic.primary.target.matches_range(span) {
                continue;
            }

            // select declarations that can begin the written reference
            let Some(root) = path.segments.first() else {
                continue;
            };
            let name = self.strings().get(*root).to_string();
            let usage = match node.local_id.ty {
                dir::NodeType::TypeExpression => DeclarationUse::Type,
                _ => DeclarationUse::Expression,
            };

            return Ok((span, name, usage));
        }

        Err(QueryError::missing(format!(
            "unresolved reference: {:?}",
            diagnostic.primary.target
        )))
    }

    /// Plan every exact import for one unresolved name.
    fn import_actions(
        &self,
        program: &ProgramQueryContext<'_>,
        file: tspp_source::FileId,
        name: &str,
        usage: DeclarationUse,
    ) -> QueryResult<Vec<ImportAction>> {
        let mut actions = Vec::new();
        let mut seen = FxHashSet::default();
        let candidates = program.search_export_candidates(name, Some(self.module_id()))?;
        let imports = self.import_declarations(file)?;

        // retain exact matching exports and their importable specifiers
        for candidate in candidates {
            if candidate.binding.name() != name {
                continue;
            }

            // require one eligible declaration for the unresolved reference
            let export_declarations = candidate.resolve_declarations(program)?;
            if !export_declarations
                .into_iter()
                .any(|declaration| usage.accepts_export(declaration))
            {
                continue;
            }

            // rank every specifier from the same module path
            let path = ImportPathOrder::between(
                self.repository(),
                self.revision(),
                self.module_id(),
                candidate.module,
            )?;
            let specifiers = program.import_specifiers(self.module_id(), candidate.module)?;
            for specifier in specifiers {
                let key = (
                    candidate.module,
                    candidate.binding.clone(),
                    specifier.clone(),
                );
                if !seen.insert(key) {
                    continue;
                }

                // build one action for this exact import
                let Some(patch) = imports.edit(&candidate.binding, &specifier) else {
                    continue;
                };
                let order = ImportOrder::new(path.clone(), &specifier, name);
                actions.push(ImportAction {
                    order,
                    binding: candidate.binding.clone(),
                    specifier,
                    patches: vec![patch],
                });
            }
        }
        actions.sort_by(|left, right| {
            (&left.order, &left.binding, &left.specifier).cmp(&(
                &right.order,
                &right.binding,
                &right.specifier,
            ))
        });

        Ok(actions)
    }
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

impl CodeAction {
    /// Compare code actions in stable protocol order.
    fn protocol_order(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| other.is_preferred.cmp(&self.is_preferred))
            .then_with(|| self.title.cmp(&other.title))
            .then_with(|| {
                applicability_key(self.applicability).cmp(&applicability_key(other.applicability))
            })
            .then_with(|| {
                let left = self.diagnostics.iter().map(diagnostic_key);
                let right = other.diagnostics.iter().map(diagnostic_key);

                left.cmp(right)
            })
            .then_with(|| {
                let left = self.patches.iter().map(Patch::order_key);
                let right = other.patches.iter().map(Patch::order_key);

                left.cmp(right)
            })
    }
}

/// Return the stable order for optional suggestion applicability.
fn applicability_key(applicability: Option<Applicability>) -> u8 {
    match applicability {
        None => 0,
        Some(Applicability::Automatic) => 1,
        Some(Applicability::Unsafe) => 2,
        Some(Applicability::Dangerous) => 3,
    }
}

/// Return the stable order key for one diagnostic occurrence.
fn diagnostic_key(
    diagnostic: &DiagnosticReference,
) -> (&str, Blob, bool, u64, u32, u32, Option<&str>) {
    let (is_file, file, start, end) = match diagnostic.primary.target {
        DiagnosticTarget::Span(span) => (false, span.file.0, span.start, span.end),
        DiagnosticTarget::File(file) => (true, file.0, 0, 0),
    };

    (
        &diagnostic.id,
        diagnostic.primary.blob,
        is_file,
        file,
        start,
        end,
        diagnostic.primary.message.as_deref(),
    )
}
