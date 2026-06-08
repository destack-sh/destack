use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::Revision;
use destack_source::{
    Applicability, BatchEdit, DiagnosticCollection, DiffOptions, File, FileId, ModuleId,
    apply_batch_edit, format_diff,
};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Lint/fix options for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandLintOptions {
    /// Apply fixes.
    pub fix: bool,
    /// Include unsafe fixes.
    pub unsafe_fixes: bool,
    /// Show diff instead of applying fixes.
    pub diff: bool,
}

/// Options for the check command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCheckOptions {
    /// Whether linting should run when supported.
    pub lint: bool,
    /// Lint/fix options for the check command.
    pub lint_options: CommandLintOptions,
}

impl CommandContext<'_> {
    /// Execute a check command.
    pub(super) fn run_check_command(
        &mut self,
        options: &CommandCheckOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // collect the requested roots
        let lint_enabled = options.lint || options.lint_options.fix || options.lint_options.diff;
        let mut artifact_keys = Vec::new();
        for module_id in &modules {
            artifact_keys.push(self.check_root_for_module(revision, *module_id, lint_enabled)?);
        }

        // provide the requested roots
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
        self.apply_diagnostic_suggestions(revision, &diagnostics, &options.lint_options)?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.selected_profile_count(revision, &modules)?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
        ))
    }

    /// Execute a lint command.
    pub(super) fn run_lint_command(
        &mut self,
        options: &CommandLintOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // collect the requested roots
        let mut artifact_keys = Vec::new();
        for module_id in &modules {
            artifact_keys.push(self.check_root_for_module(revision, *module_id, true)?);
        }

        // provide the requested roots
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
        self.apply_diagnostic_suggestions(revision, &diagnostics, options)?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.selected_profile_count(revision, &modules)?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
        ))
    }

    /// Build the requested check root for one module.
    fn check_root_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        lint_enabled: bool,
    ) -> CommandResult<ArtifactKey> {
        let profile = self.selected_profile_id(revision, module_id)?;

        if lint_enabled {
            return Ok(ArtifactKey::module_linted(module_id, profile));
        }

        Ok(ArtifactKey::dir_checked(module_id, profile))
    }

    /// Apply or print diagnostic suggestions requested by the command.
    fn apply_diagnostic_suggestions(
        &mut self,
        revision: Revision,
        diagnostics: &DiagnosticCollection,
        options: &CommandLintOptions,
    ) -> CommandResult<usize> {
        if !options.fix && !options.diff {
            return Ok(0);
        }

        // collect applicable edits from all diagnostics
        let edits = self.collect_suggestion_edits(diagnostics, options.unsafe_fixes);
        if edits.is_empty() {
            return Ok(0);
        }

        // apply edits against the exact checked file revision
        let files = self.files_for_edits(revision, &edits)?;
        let updates = apply_batch_edit(&edits, |file_id| files.get(&file_id).map(Arc::as_ref))
            .map_err(|error| error.to_string())?;

        // print or persist the edited text
        if options.diff {
            self.print_fix_diff(&files, &updates)?;
        } else {
            self.write_fixed_files(&files, &updates)?;
            self.output
                .push_stderr(format!("Fixed {} problem(s)\n", edits.total_edits()).into_bytes());
        }

        Ok(edits.total_edits())
    }

    /// Return source files required by one edit batch.
    fn files_for_edits(
        &self,
        revision: Revision,
        edits: &BatchEdit,
    ) -> CommandResult<HashMap<FileId, Arc<File>>> {
        let mut files = HashMap::new();

        for file_edit in &edits.files {
            // edits must reference files in the checked revision
            let file = self
                .repository
                .file(revision, file_edit.file)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    format!(
                        "missing file for diagnostic suggestion: {:?}",
                        file_edit.file
                    )
                })?;

            files.insert(file_edit.file, file);
        }

        Ok(files)
    }

    /// Emit diffs for fixed files.
    fn print_fix_diff(
        &mut self,
        files: &HashMap<FileId, Arc<File>>,
        updates: &HashMap<FileId, String>,
    ) -> CommandResult<()> {
        for (file_id, fixed) in updates {
            // diff output can use display names for non filesystem files
            let file = files
                .get(file_id)
                .ok_or_else(|| format!("missing file for diagnostic suggestion: {file_id:?}"))?;

            let path = file
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| file.name.clone());
            let original = file.text();
            if original == fixed {
                continue;
            }

            // emit one unified diff per touched file
            let diff_options = DiffOptions::new().with_path(path);
            let diff = format_diff(original, fixed, &diff_options);
            self.output.push_stdout(diff.into_bytes());
        }

        Ok(())
    }

    /// Write fixed source text back through the repository file system.
    fn write_fixed_files(
        &self,
        files: &HashMap<FileId, Arc<File>>,
        updates: &HashMap<FileId, String>,
    ) -> CommandResult<()> {
        for (file_id, fixed) in updates {
            // write mode requires an actual filesystem path
            let file = files
                .get(file_id)
                .ok_or_else(|| format!("missing file for diagnostic suggestion: {file_id:?}"))?;

            let path = file
                .path
                .as_ref()
                .ok_or_else(|| format!("cannot apply fix for non-filesystem file {}", file.uri))?;

            self.repository
                .file_system()
                .write(path, fixed.as_bytes())
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }

        Ok(())
    }

    /// Collect machine applicable suggestions from diagnostics.
    fn collect_suggestion_edits(
        &self,
        diagnostics: &DiagnosticCollection,
        include_unsafe: bool,
    ) -> BatchEdit {
        let mut batch = BatchEdit::new();

        for diagnostic in diagnostics.iter() {
            for suggestion in &diagnostic.suggestions {
                // keep safe fixes by default, unsafe fixes only by request
                let applicable = suggestion.applicability == Applicability::Automatic
                    || (include_unsafe && suggestion.applicability == Applicability::Unsafe);
                if !applicable {
                    continue;
                }

                // merge all edits into one batch
                for edit in suggestion.edits.iter().cloned() {
                    batch.add(edit);
                }
            }
        }

        batch
    }
}
