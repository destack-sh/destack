use std::collections::HashMap;
use std::sync::Arc;

use destack_linter::{Fixability, LintDiagnostic, LintLevel, LintRunner};
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, DiffOptions, File, FileId, ModuleId, print_diff,
};
use destack_workspace::{LinterOptions, Ref, Repository, Revision};

use crate::common::LineWriter;
use crate::common::format::{FormatOptions, format_diagnostics_with_writer};
use crate::console;

/// Options for fix behavior.
#[derive(Debug, Clone, Default)]
pub struct FixOptions {
    /// Apply fixes to files.
    pub apply: bool,
    /// Include unsafe fixes.
    pub include_unsafe: bool,
    /// Show diff instead of applying (dry run).
    pub diff: bool,
}

/// Result of running linting with fix collection.
#[derive(Debug)]
pub struct FixResult {
    /// Number of unfixable problems remaining.
    pub unfixable_count: usize,
}

/// Run linting and optionally apply fixes.
pub fn run_with_fixes(
    repository: Arc<Repository>,
    modules: &[ModuleId],
    diagnostic_options: &DiagnosticOptions,
    fix_options: &FixOptions,
    format_options: &FormatOptions,
    line_writer: Option<&LineWriter>,
) -> FixResult {
    let revision = current_repository_revision(repository.as_ref()).ok();

    // collect all lint diagnostics
    let mut all_diagnostics: Vec<LintDiagnostic> = Vec::new();
    for module_id in modules {
        let Some(revision) = revision else {
            break;
        };
        let Ok(module) = repository.module(revision, *module_id) else {
            continue;
        };
        let Some(module) = module else {
            continue;
        };
        let linter_options = module_linter_options(repository.as_ref(), revision, &module);
        let runner = LintRunner::from_options(&linter_options).with_fixes(true);
        let Ok(profile) = repository.module_profile(revision, *module_id) else {
            continue;
        };
        let profile = profile.as_ref().clone();
        let ast_diagnostics = runner.lint_module(
            repository.clone(),
            revision,
            module.clone(),
            profile.clone(),
            &linter_options,
            LintLevel::Ast,
        );
        let dir_diagnostics = runner.lint_module(
            repository.clone(),
            revision,
            module.clone(),
            profile,
            &linter_options,
            LintLevel::Dir,
        );
        all_diagnostics.extend(ast_diagnostics);
        all_diagnostics.extend(dir_diagnostics);
    }

    // separate fixable from unfixable
    let (fixable, unfixable): (Vec<_>, Vec<_>) = all_diagnostics
        .into_iter()
        .partition(|d| has_applicable_fix(d, fix_options.include_unsafe));

    // show diff without applying
    if fix_options.diff {
        show_diff(&repository, revision, &fixable, fix_options.include_unsafe);
    }
    // apply fixes
    else if fix_options.apply {
        let fixed_count = apply_fixes(&repository, revision, &fixable, fix_options.include_unsafe);
        if fixed_count > 0 {
            console::info(&format!("Fixed {fixed_count} problem(s)"));
        }
    }

    // report remaining unfixable issues
    if !unfixable.is_empty() {
        let collection = to_diagnostic_collection(&unfixable);
        let mapped = collection.map(diagnostic_options);
        let file_for_id = |file_id| repository_file(repository.as_ref(), revision, file_id);
        let _ = format_diagnostics_with_writer(
            &file_for_id,
            &mapped,
            format_options,
            modules.len(),
            line_writer,
        );
    }

    // report issues that should have had fixes but didn't
    let fixable_without_fix: Vec<_> = fixable
        .iter()
        .filter(|d| d.fixes.is_empty())
        .cloned()
        .collect();
    if !fixable_without_fix.is_empty() {
        let collection = to_diagnostic_collection(&fixable_without_fix);
        let mapped = collection.map(diagnostic_options);
        let file_for_id = |file_id| repository_file(repository.as_ref(), revision, file_id);
        let _ = format_diagnostics_with_writer(
            &file_for_id,
            &mapped,
            format_options,
            modules.len(),
            line_writer,
        );
    }

    let unfixable_count = unfixable.len() + fixable_without_fix.len();
    FixResult { unfixable_count }
}

/// Resolve effective linter options for one module.
fn module_linter_options(
    repository: &Repository,
    revision: Revision,
    module: &destack_workspace::Module,
) -> LinterOptions {
    if let Ok(Some(package_options)) = repository.package_options(revision, module.package_id) {
        return package_options.linter;
    }

    repository
        .workspace_options(revision)
        .ok()
        .flatten()
        .map(|options| options.package.linter)
        .unwrap_or_default()
}

/// Convert lint diagnostics to a standard diagnostic collection.
fn to_diagnostic_collection(diagnostics: &[LintDiagnostic]) -> DiagnosticCollection {
    let mut collection = DiagnosticCollection::new();
    for d in diagnostics {
        collection.insert(d.clone().into_diagnostic());
    }
    collection
}

/// Check if a diagnostic has an applicable fix.
fn has_applicable_fix(diagnostic: &LintDiagnostic, include_unsafe: bool) -> bool {
    diagnostic.fixes.iter().any(|f| {
        f.applicability == Fixability::Safe
            || (include_unsafe && f.applicability == Fixability::Unsafe)
    })
}

/// Apply fixes from diagnostics to source files.
fn apply_fixes(
    repository: &Repository,
    revision: Option<Revision>,
    diagnostics: &[LintDiagnostic],
    include_unsafe: bool,
) -> usize {
    let edits_by_file = collect_edits(diagnostics, include_unsafe);
    let mut fix_count = 0;

    for (file_id, mut edits) in edits_by_file {
        // sort by position descending to apply from end to start
        edits.sort_by(|a, b| b.0.cmp(&a.0));

        let Some(file) = repository_file(repository, revision, file_id) else {
            continue;
        };
        let mut source = file.text().to_string();

        for (start, end, new_text) in &edits {
            source.replace_range(*start..*end, new_text);
            fix_count += 1;
        }

        // write the fixed source back to disk
        if let Some(path) = file.path.as_ref()
            && let Err(e) = repository.file_system().write(path, source.as_bytes())
        {
            console::error(&format!("failed to write {}: {e}", path.display()));
        }
    }

    fix_count
}

/// Show diff of what fixes would be applied.
fn show_diff(
    repository: &Repository,
    revision: Option<Revision>,
    diagnostics: &[LintDiagnostic],
    include_unsafe: bool,
) -> usize {
    let edits_by_file = collect_edits(diagnostics, include_unsafe);
    let mut fix_count = 0;

    for (file_id, mut edits) in edits_by_file {
        edits.sort_by(|a, b| b.0.cmp(&a.0));

        let Some(file) = repository_file(repository, revision, file_id) else {
            continue;
        };
        let original = file.text().to_string();
        let mut modified = original.clone();

        for (start, end, new_text) in &edits {
            modified.replace_range(*start..*end, new_text);
            fix_count += 1;
        }

        if original != modified {
            let path = file
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| file.name.clone());
            let options = DiffOptions::new().with_path(&path);
            print_diff(&original, &modified, &options);
        }
    }

    if fix_count > 0 {
        console::info(&format!("{fix_count} fix(es) available"));
    }

    fix_count
}

/// Return the current repository revision.
fn current_repository_revision(repository: &Repository) -> Result<Revision, String> {
    let reference = Ref::for_workspace_root(repository.workspace_root());

    repository
        .current(&reference)
        .map_err(|error| format!("failed to resolve current revision: {error}"))
}

/// Load one file snapshot when both revision and file are available.
fn repository_file(
    repository: &Repository,
    revision: Option<Revision>,
    file_id: FileId,
) -> Option<Arc<File>> {
    let revision = revision?;

    repository.file(revision, file_id).ok().flatten()
}

/// Collect edits from diagnostics, grouped by file.
fn collect_edits(
    diagnostics: &[LintDiagnostic],
    include_unsafe: bool,
) -> HashMap<FileId, Vec<(usize, usize, String)>> {
    let mut edits_by_file: HashMap<FileId, Vec<(usize, usize, String)>> = HashMap::new();

    for diagnostic in diagnostics {
        for fix in &diagnostic.fixes {
            let applicable = fix.applicability == Fixability::Safe
                || (include_unsafe && fix.applicability == Fixability::Unsafe);
            if !applicable {
                continue;
            }

            for edit in &fix.edits {
                let start = edit.span.start as usize;
                let end = edit.span.end as usize;
                edits_by_file.entry(edit.span.file).or_default().push((
                    start,
                    end,
                    edit.new_text.clone(),
                ));
            }
        }
    }

    edits_by_file
}
