use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_dir as dir;
use destack_pattern::{Rewrite, Rewriter};
use destack_repository::ArtifactReader;
use destack_serde::Reflect;
use destack_source::{
    DiagnosticCollection, DiffOptions, Edit, File, FilePatch, Uri, apply_file_patch, format_diff,
};
use serde::{Deserialize, Serialize};

use crate::Commit;

use super::super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::super::{CommandContext, CommandError, CommandOutcome, CommandResult};
use super::program::CheckedPrograms;

/// Execution mode for a structural rewrite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RewriteMode {
    /// Print unified diffs without mutating source files.
    #[default]
    Diff,
    /// Report whether rewrites are required without printing or writing them.
    Check,
    /// Write rewritten source files.
    Write,
}

/// Request to rewrite source with one structural pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RewriteInput {
    /// Revision selected for this rewrite.
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
    /// The authored structural search pattern.
    pub pattern: String,
    /// The authored structural replacement.
    pub replacement: String,
    /// The contextual node type, or expression rewriting by default.
    pub kind: Option<dir::NodeType>,
    /// Semantic predicate expressions applied in authored order.
    pub predicates: Vec<String>,
    /// Source mutation behavior.
    pub mode: RewriteMode,
}

impl_command_input_options!(RewriteInput {
    pattern: String::new(),
    replacement: String::new(),
    kind: None,
    predicates: Vec::new(),
    mode: RewriteMode::Diff,
});

/// One source file changed by a rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RewriteChange {
    /// The source file URI.
    pub uri: Uri,
    /// The physical source path when one exists.
    pub path: Option<PathBuf>,
    /// The exact replacements applied to this source revision.
    pub patch: FilePatch,
}

/// Payload for rewrite command output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RewritePayload {
    /// Changed files in module and physical file order.
    pub changes: Vec<RewriteChange>,
    /// The total number of replacements.
    pub replacements: usize,
    /// The source commit produced by write mode.
    pub commit: Option<Commit>,
}

/// One rendered file change awaiting output or application.
struct PendingRewrite {
    /// The exact source file interpreted by the patch.
    file: Arc<File>,
    /// The exact replacements selected for the file.
    patch: FilePatch,
    /// The source text after applying the patch.
    rewritten: String,
}

impl RewriteInput {
    /// Compile the structural rewrite and semantic predicates.
    fn compile(
        &self,
        context: &mut CommandContext<'_>,
    ) -> CommandResult<Result<Rewrite, DiagnosticCollection>> {
        let pattern = context.add_file(
            PathBuf::from(".destack/command/rewrite/pattern.ds-pattern"),
            &self.pattern,
        )?;
        let replacement = context.add_file(
            PathBuf::from(".destack/command/rewrite/replacement.ds-pattern"),
            &self.replacement,
        )?;
        let strings = context.repository.string_pool().clone();
        let rewrite = match self.kind {
            Some(kind) => Rewrite::parse_context(pattern, replacement, kind, strings),
            None => Rewrite::parse(pattern, replacement, strings),
        };
        let mut rewrite = match rewrite {
            Ok(rewrite) => rewrite,
            Err(diagnostics) => return Ok(Err(diagnostics)),
        };
        let mut diagnostics = DiagnosticCollection::new();

        // compile every predicate against the search metavariable table
        for (index, predicate) in self.predicates.iter().enumerate() {
            let path = format!(
                ".destack/command/rewrite/predicate-{}.ds-pattern",
                index + 1
            );
            let file = context.add_file(PathBuf::from(path), predicate)?;
            if let Err(predicate_diagnostics) = rewrite.add_predicate(file) {
                diagnostics.merge_from(&predicate_diagnostics);
            }
        }

        if diagnostics.is_empty() {
            Ok(Ok(rewrite))
        } else {
            Ok(Err(diagnostics))
        }
    }
}

impl CommandContext<'_> {
    /// Execute a structural rewrite command.
    pub(crate) fn run_rewrite_command(
        &mut self,
        input: &RewriteInput,
    ) -> CommandResult<CommandOutcome<RewritePayload>> {
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let rewrite = match input.compile(self)? {
            Ok(rewrite) => rewrite,
            Err(diagnostics) => {
                let exit_code = diagnostics.get_status_code();
                let outcome = CommandOutcome::new(diagnostics, exit_code, modules.len(), 0, 0)
                    .with_data(RewritePayload::default());

                return Ok(outcome);
            }
        };

        // request only parsed DIR for structural rewrites
        let checked = if input.predicates.is_empty() {
            let revision = self.revision()?;
            let keys = modules
                .iter()
                .map(|module| ArtifactKey::dir_parsed(*module))
                .collect::<Vec<_>>();
            self.session
                .provide(revision, &keys)
                .map_err(|error| error.to_string())?;

            None
        }
        // request the complete loaded checked closure for semantic predicates
        else {
            Some(CheckedPrograms::new(self, &modules)?)
        };

        // stop before matching when the requested artifact closure is invalid
        let artifact_keys = match checked.as_ref() {
            Some(checked) => checked.artifact_keys().to_vec(),
            None => modules
                .iter()
                .map(|module| ArtifactKey::dir_parsed(*module))
                .collect(),
        };
        let revision = self.revision()?;
        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = checked.as_ref().map_or(0, CheckedPrograms::profile_count);
        if exit_code != 0 {
            let outcome =
                CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                    .with_data(RewritePayload::default());

            return Ok(outcome);
        }

        let artifacts = ArtifactReader::new(self.repository.as_ref(), revision);
        let mut changes = Vec::new();
        let mut rewrite_diagnostics = DiagnosticCollection::new();
        for module in modules.iter().copied() {
            let parsed = artifacts
                .dir_parsed(module)
                .map_err(|error| error.to_string())?;
            let view = dir::View::new(&parsed.tree);

            // rewrite each physical source file independently
            for parsed_file in &parsed.files {
                let file = self
                    .repository
                    .file(revision, parsed_file.file_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| {
                        CommandError::internal(format!(
                            "missing rewrite source file {:?}",
                            parsed_file.file_id
                        ))
                    })?;
                let rewriter = match checked.as_ref() {
                    Some(checked) => {
                        let (module, program) = checked.module(module)?;

                        Rewriter::with_module(&rewrite, view, module, program, file.as_ref())
                    }
                    None => Rewriter::new(&rewrite, view, file.as_ref()),
                };
                let candidates = view.iter_node_ids_in_file(file.id);
                let patch = match rewriter.rewrite(candidates) {
                    Ok(patch) => patch,
                    Err(diagnostics) => {
                        rewrite_diagnostics.merge_from(&diagnostics);

                        continue;
                    }
                };
                if patch.is_empty() {
                    continue;
                }

                let rewritten =
                    apply_file_patch(file.as_ref(), &patch).map_err(|error| error.to_string())?;
                changes.push(PendingRewrite {
                    file,
                    patch,
                    rewritten,
                });
            }
        }

        let mut diagnostics = diagnostics;
        diagnostics.merge_from(&rewrite_diagnostics);
        let exit_code = diagnostics.get_status_code();
        let mode = if input.dry_run && input.mode == RewriteMode::Write {
            RewriteMode::Diff
        } else {
            input.mode
        };

        // require an authoritative artifact closure and unambiguous edit set
        let commit = if exit_code != 0 {
            changes.clear();
            None
        } else {
            self.apply_rewrite_changes(&changes, mode)?
        };
        let replacements = changes.iter().map(|change| change.patch.len()).sum();
        let exit_code = if exit_code == 0 && mode == RewriteMode::Check && replacements > 0 {
            1
        } else {
            exit_code
        };
        let changes = changes
            .into_iter()
            .map(|change| RewriteChange {
                uri: change.file.uri.clone(),
                path: change.file.path.clone(),
                patch: change.patch,
            })
            .collect();
        let payload = RewritePayload {
            changes,
            replacements,
            commit,
        };

        Ok(
            CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                .with_data(payload),
        )
    }

    /// Execute one complete rewrite change set in the selected mode.
    fn apply_rewrite_changes(
        &mut self,
        changes: &[PendingRewrite],
        mode: RewriteMode,
    ) -> CommandResult<Option<Commit>> {
        match mode {
            RewriteMode::Diff => {
                for change in changes {
                    let path = change
                        .file
                        .path
                        .as_ref()
                        .map(|path| self.repository.logical_path(path))
                        .unwrap_or_else(|| change.file.uri.to_string());
                    let options = DiffOptions::new().with_path(path).with_color(false);
                    let diff = format_diff(change.file.text(), &change.rewritten, &options);
                    self.output.push_stdout(diff.into_bytes());
                }

                Ok(None)
            }
            RewriteMode::Check => Ok(None),
            RewriteMode::Write => self.write_rewrite_changes(changes),
        }
    }

    /// Write a complete rewrite change set against its source revision.
    fn write_rewrite_changes(
        &mut self,
        changes: &[PendingRewrite],
    ) -> CommandResult<Option<Commit>> {
        if changes.is_empty() {
            return Ok(None);
        }

        // require a physical path for every changed source before writing
        let edits = changes
            .iter()
            .map(|change| {
                let path = change.file.path.clone().ok_or_else(|| {
                    CommandError::internal(format!(
                        "cannot write non-filesystem source {}",
                        change.file.uri
                    ))
                })?;

                Ok(Edit::SetText {
                    path,
                    text: change.rewritten.clone(),
                })
            })
            .collect::<CommandResult<Vec<_>>>()?;
        let commit = self
            .workspace
            .write_source_edits_if_current(&self.root, self.base, edits)
            .map_err(|error| CommandError::source(error.to_string()))?;

        Ok(Some(commit))
    }
}
