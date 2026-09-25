use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_core::FxIndexSet;
use tspp_repository::{Revision, TraceView};
use tspp_serde::Reflect;
use tspp_source::{
    Applicability, DiagnosticCollection, DiffOptions, File, FileId, PatchSet, apply_patch_set,
    format_diff,
};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;

/// Diagnostic fix behavior for one check.
#[derive(Debug, Clone, Copy)]
struct FixOptions {
    /// Apply fixes.
    fix: bool,
    /// Include unsafe fixes.
    unsafe_fixes: bool,
    /// Show diff instead of applying fixes.
    diff: bool,
}

/// Request to check source state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CheckInput {
    /// Revision selected for this check.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Whether linting should run when supported.
    pub lint: bool,
    /// Apply lint fixes.
    pub fix: bool,
    /// Include unsafe lint fixes.
    pub unsafe_fixes: bool,
    /// Show lint diff instead of applying fixes.
    pub diff: bool,
    /// Trace detail returned for this command.
    #[serde(default)]
    pub trace: Option<TraceView>,
}

impl CheckInput {
    /// Return diagnostic fix behavior selected for this check.
    fn fix_options(&self) -> FixOptions {
        FixOptions {
            fix: self.fix,
            unsafe_fixes: self.unsafe_fixes,
            diff: self.diff,
        }
    }
}

impl_command_input_options!(CheckInput {
    lint: true,
    fix: false,
    unsafe_fixes: false,
    diff: false,
});
impl CommandContext<'_> {
    /// Execute a check command.
    pub(crate) async fn run_check_command(
        &mut self,
        input: &CheckInput,
    ) -> CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision();

        // collect the requested roots
        let fix_options = input.fix_options();
        let mut artifact_keys = FxIndexSet::default();
        for module_id in modules.iter().copied() {
            // request lint completion at both supported scopes
            if input.lint {
                let target = self.resolve_target_for_module(revision, module_id, None)?;
                let profile = self.target_profile_id(revision, module_id, target.id)?;

                artifact_keys.insert(ArtifactKey::module_linted(module_id, profile, target.id));
                artifact_keys.insert(ArtifactKey::program_linted(profile, target.id));
            }
            // otherwise stop after type checking
            else {
                let profile = self.selected_profile_id(revision, module_id)?;

                artifact_keys.insert(ArtifactKey::dir_checked(module_id, profile));
            }
        }
        let artifact_keys = artifact_keys.into_iter().collect::<Vec<_>>();

        // complete the requested diagnostic roots
        self.complete(revision, &artifact_keys).await?;

        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
        self.apply_diagnostic_suggestions(revision, &diagnostics, fix_options)?;
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

    /// Apply or print diagnostic suggestions requested by the command.
    fn apply_diagnostic_suggestions(
        &mut self,
        revision: Revision,
        diagnostics: &DiagnosticCollection,
        options: FixOptions,
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
        let updates = apply_patch_set(&edits, |file_id| files.get(&file_id).map(Arc::as_ref))
            .map_err(|error| error.to_string())?;

        // print or persist the edited text
        if options.diff {
            self.print_fix_diff(&files, &updates)?;
        } else {
            self.write_fixed_files(&files, &updates)?;
            self.output
                .push_stderr(format!("Fixed {} problem(s)\n", edits.total_patches()).into_bytes());
        }

        Ok(edits.total_patches())
    }

    /// Return source files required by one edit batch.
    fn files_for_edits(
        &self,
        revision: Revision,
        edits: &PatchSet,
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
    ) -> PatchSet {
        let mut batch = PatchSet::new();

        for diagnostic in diagnostics.iter() {
            for suggestion in &diagnostic.suggestions {
                // keep safe fixes by default, unsafe fixes only by request
                let applicable = suggestion.applicability == Applicability::Automatic
                    || (include_unsafe && suggestion.applicability == Applicability::Unsafe);
                if !applicable {
                    continue;
                }

                // merge all edits into one batch
                for edit in suggestion.patches.iter().cloned() {
                    batch.add(edit);
                }
            }
        }

        batch
    }
}
