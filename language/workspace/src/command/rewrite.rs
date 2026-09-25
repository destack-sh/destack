use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_dir as dir;
use tspp_pattern::{Rewrite, Rewriter};
use tspp_repository::{Commit, TraceView};
use tspp_serde::Reflect;
use tspp_source::{
    DiagnosticCollection, DiffOptions, Edit, File, FilePatch, Uri, apply_file_patch, format_diff,
};

use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::selection::PatternSelection;
use super::{CommandContext, CommandError, CommandOutcome, CommandResult};

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
    /// Whether package.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional manifest path override.
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
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
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
        let pattern = context.add_memory_file("rewrite/pattern.tspp-pattern", &self.pattern)?;
        let replacement =
            context.add_memory_file("rewrite/replacement.tspp-pattern", &self.replacement)?;
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
            let path = format!("rewrite/predicate-{}.tspp-pattern", index + 1);
            let file = context.add_memory_file(&path, predicate)?;
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
    pub(crate) async fn run_rewrite_command(
        &mut self,
        input: &RewriteInput,
    ) -> CommandResult<CommandOutcome<RewritePayload>> {
        let trace = self.trace();
        let inputs = trace.span("command.resolve inputs", || self.resolve_command_inputs())?;
        let modules = trace.span("command.resolve modules", || self.resolve_modules(&inputs))?;
        let rewrite = match trace.span("command.compile rewrite", || input.compile(self))? {
            Ok(rewrite) => rewrite,
            Err(diagnostics) => {
                let exit_code = diagnostics.get_status_code();
                let outcome = CommandOutcome::new(diagnostics, exit_code, modules.len(), 0, 0)
                    .with_data(RewritePayload::default());

                return Ok(outcome);
            }
        };

        // provide parsed DIR before structural selection
        let revision = self.revision();
        let parsed_keys = modules
            .iter()
            .map(|module| ArtifactKey::dir_parsed(*module))
            .collect::<Vec<_>>();
        trace
            .span_async(
                "command.parse sources",
                self.provide(revision, &parsed_keys),
            )
            .await?;
        let diagnostics = self.command_diagnostics(revision, &parsed_keys)?;
        let exit_code = diagnostics.get_status_code();
        if exit_code != 0 {
            let outcome = CommandOutcome::new(diagnostics, exit_code, modules.len(), 0, 0)
                .with_data(RewritePayload::default());

            return Ok(outcome);
        }

        // retain structural matches before requesting checked DIR
        let mut selection = trace.span("command.select matches", || {
            PatternSelection::find(rewrite.pattern(), &modules, revision, self)
        })?;
        let profiles = if rewrite.pattern().predicates().is_empty() {
            Default::default()
        } else {
            selection.group_by_profile(revision, self)?
        };
        let profile_count = profiles.len();
        let mut programs = Vec::with_capacity(profile_count);
        let mut checked_keys = Vec::new();
        trace
            .span_async("command.check matches", async {
                for (profile, roots) in profiles {
                    checked_keys.extend(
                        roots
                            .iter()
                            .map(|module| ArtifactKey::dir_checked(*module, profile)),
                    );
                    let program = self
                        .provide_program_context(profile, &roots, revision)
                        .await?;
                    programs.push((roots, program));
                }

                Ok::<(), CommandError>(())
            })
            .await?;

        // stop before predicate evaluation when checked roots are invalid
        let diagnostics = if checked_keys.is_empty() {
            diagnostics
        } else {
            self.command_diagnostics(revision, &checked_keys)?
        };
        let exit_code = diagnostics.get_status_code();
        if exit_code != 0 {
            let outcome =
                CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                    .with_data(RewritePayload::default());

            return Ok(outcome);
        }

        // evaluate predicates against each exact checked program
        trace.span("command.evaluate predicates", || -> CommandResult<()> {
            for (roots, program) in &programs {
                selection.retain(rewrite.pattern(), roots, program)?;
            }

            Ok(())
        })?;

        // render retained matches into non-overlapping file patches
        let (mut changes, rewrite_diagnostics) = trace.span(
            "command.render replacements",
            || -> CommandResult<(Vec<PendingRewrite>, DiagnosticCollection)> {
                let mut changes = Vec::new();
                let mut diagnostics = DiagnosticCollection::new();
                for module in selection.modules {
                    let view = dir::View::new(&module.parsed.tree);
                    for file in module.files {
                        let source = file.file;
                        let rewriter = Rewriter::new(&rewrite, view, source.as_ref());
                        let patch = match rewriter.rewrite(file.matches) {
                            Ok(patch) => patch,
                            Err(file_diagnostics) => {
                                diagnostics.merge_from(&file_diagnostics);

                                continue;
                            }
                        };
                        if patch.is_empty() {
                            continue;
                        }

                        let rewritten = apply_file_patch(source.as_ref(), &patch)
                            .map_err(|error| error.to_string())?;
                        changes.push(PendingRewrite {
                            file: source,
                            patch,
                            rewritten,
                        });
                    }
                }

                Ok((changes, diagnostics))
            },
        )?;

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
            trace.span("command.apply replacements", || {
                self.apply_rewrite_changes(&changes, mode)
            })?
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
            .edit(self.base, edits)
            .map_err(|error| CommandError::source(error.to_string()))?;

        Ok(Some(commit))
    }
}
