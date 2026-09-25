use std::cmp::Reverse;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_dir as dir;
use tspp_pattern::{Binding, Pattern, PatternMatch};
use tspp_repository::TraceView;
use tspp_serde::Reflect;
use tspp_source::{DiagnosticCollection, File, Span, Uri};

use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::selection::PatternSelection;
use super::{CommandContext, CommandError, CommandOutcome, CommandResult};

/// Request to query source with one structural pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryInput {
    /// Revision selected for this query.
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
    /// The authored structural pattern.
    pub pattern: String,
    /// The contextual node type, or expression matching by default.
    pub kind: Option<dir::NodeType>,
    /// Semantic predicate expressions applied in authored order.
    pub predicates: Vec<String>,
    /// Whether the response should retain matched source files.
    pub include_sources: bool,
}

impl_command_input_options!(QueryInput {
    pattern: String::new(),
    kind: None,
    predicates: Vec::new(),
    include_sources: false,
});

/// One captured source value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct QueryCaptureValue {
    /// The captured source span.
    pub span: Span,
    /// The exact captured source text.
    pub text: String,
}

/// One named pattern capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct QueryCapture {
    /// The metavariable name without leading dollar signs.
    pub name: String,
    /// The captured values, empty for a repeated empty match.
    pub values: Vec<QueryCaptureValue>,
}

/// One source node selected by a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct QueryMatch {
    /// The source file URI.
    pub uri: Uri,
    /// The physical source path when one exists.
    pub path: Option<PathBuf>,
    /// The complete selected source span.
    pub span: Span,
    /// The one-based source line.
    pub line: u32,
    /// The one-based byte column.
    pub column: u32,
    /// The exact selected source text.
    pub text: String,
    /// Named captures in pattern declaration order.
    pub captures: Vec<QueryCapture>,
}

impl QueryMatch {
    /// Build one source match from structural bindings and source spans.
    fn new(
        pattern: &Pattern,
        view: dir::View<'_>,
        file: &File,
        pattern_match: PatternMatch,
    ) -> CommandResult<Self> {
        let span = view
            .get_decorated_span(pattern_match.root)
            .ok_or_else(|| CommandError::internal("matched node has no source span"))?;
        let text = file
            .get_span_str(span)
            .ok_or_else(|| CommandError::internal("matched node span belongs to another file"))?
            .to_string();
        let (line, column) = file
            .get_position(span.start)
            .ok_or_else(|| CommandError::internal("matched node has no source position"))?;
        let mut captures = Vec::with_capacity(pattern.metavariables().len());

        // project every named binding into exact source text
        for (variable, declaration) in pattern.metavariables().iter() {
            let name = pattern.strings().get(declaration.name()).to_string();
            let binding = pattern_match.bindings.get(variable).ok_or_else(|| {
                CommandError::internal(format!("match is missing binding ${name}"))
            })?;
            let spans = match binding {
                Binding::Node(node) => {
                    vec![view.get_decorated_span(*node).ok_or_else(|| {
                        CommandError::internal("captured node has no source span")
                    })?]
                }
                Binding::Nodes(nodes) => nodes
                    .iter()
                    .map(|node| {
                        view.get_decorated_span(*node).ok_or_else(|| {
                            CommandError::internal("captured node has no source span")
                        })
                    })
                    .collect::<CommandResult<Vec<_>>>()?,
                Binding::Name { span, .. } => vec![*span],
            };
            let values = spans
                .into_iter()
                .map(|span| {
                    let text = file.get_span_str(span).ok_or_else(|| {
                        CommandError::internal("captured span belongs to another file")
                    })?;

                    Ok(QueryCaptureValue {
                        span,
                        text: text.to_string(),
                    })
                })
                .collect::<CommandResult<Vec<_>>>()?;
            captures.push(QueryCapture { name, values });
        }

        Ok(Self {
            uri: file.uri.clone(),
            path: file.path.clone(),
            span,
            line: line + 1,
            column: column + 1,
            text,
            captures,
        })
    }
}

/// Payload for query command output.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct QueryPayload {
    /// Named captures declared by the compiled pattern.
    pub captures: Vec<String>,
    /// Matches in module, file, and source order.
    pub matches: Vec<QueryMatch>,
}

impl QueryPayload {
    /// Build query output from one compiled pattern and its selected matches.
    fn new(pattern: &Pattern, matches: Vec<QueryMatch>) -> Self {
        let captures = pattern
            .metavariables()
            .iter()
            .map(|(_, declaration)| pattern.strings().get(declaration.name()).to_string())
            .collect();

        Self { captures, matches }
    }
}

impl QueryInput {
    /// Compile the structural pattern and semantic predicates.
    fn compile(
        &self,
        context: &mut CommandContext<'_>,
    ) -> CommandResult<Result<Pattern, DiagnosticCollection>> {
        let file = context.add_memory_file("query/pattern.tspp-pattern", &self.pattern)?;
        let strings = context.repository.string_pool().clone();
        let pattern = match self.kind {
            Some(kind) => Pattern::parse_context(file, kind, strings),
            None => Pattern::parse(file, strings),
        };
        let mut pattern = match pattern {
            Ok(pattern) => pattern,
            Err(diagnostics) => return Ok(Err(diagnostics)),
        };
        let mut diagnostics = DiagnosticCollection::new();

        // compile every predicate against the structural metavariable table
        for (index, predicate) in self.predicates.iter().enumerate() {
            let path = format!("query/predicate-{}.tspp-pattern", index + 1);
            let file = context.add_memory_file(&path, predicate)?;
            if let Err(predicate_diagnostics) = pattern.add_predicate(file) {
                diagnostics.merge_from(&predicate_diagnostics);
            }
        }

        if diagnostics.is_empty() {
            Ok(Ok(pattern))
        } else {
            Ok(Err(diagnostics))
        }
    }
}

impl CommandContext<'_> {
    /// Execute a structural query command.
    pub(crate) async fn run_query_command(
        &mut self,
        input: &QueryInput,
    ) -> CommandResult<CommandOutcome<QueryPayload>> {
        let trace = self.trace();
        let inputs = trace.span("command.resolve inputs", || self.resolve_command_inputs())?;
        let modules = trace.span("command.resolve modules", || self.resolve_modules(&inputs))?;
        let pattern = match trace.span("command.compile pattern", || input.compile(self))? {
            Ok(pattern) => pattern,
            Err(diagnostics) => {
                let exit_code = diagnostics.get_status_code();
                let outcome = CommandOutcome::new(diagnostics, exit_code, modules.len(), 0, 0)
                    .with_data(QueryPayload::default());

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
                .with_data(QueryPayload::default());

            return Ok(outcome);
        }

        // retain structural matches before requesting checked DIR
        let mut selection = trace.span("command.select matches", || {
            PatternSelection::find(&pattern, &modules, revision, self)
        })?;
        let profiles = if pattern.predicates().is_empty() {
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
                    .with_data(QueryPayload::default());

            return Ok(outcome);
        }

        // evaluate predicates against each exact checked program
        trace.span("command.evaluate predicates", || -> CommandResult<()> {
            for (roots, program) in &programs {
                selection.retain(&pattern, roots, program)?;
            }

            Ok(())
        })?;

        // project retained structural bindings into source results
        let (matches, files) = trace.span(
            "command.project matches",
            || -> CommandResult<(Vec<QueryMatch>, Vec<Arc<File>>)> {
                let mut matches = Vec::new();
                let mut files = Vec::new();
                for module in selection.modules {
                    let view = dir::View::new(&module.parsed.tree);
                    for file in module.files {
                        let source = file.file;
                        if input.include_sources {
                            files.push(source.clone());
                        }
                        let mut selected = file
                            .matches
                            .into_iter()
                            .map(|pattern_match| {
                                QueryMatch::new(&pattern, view, source.as_ref(), pattern_match)
                            })
                            .collect::<CommandResult<Vec<_>>>()?;
                        selected.sort_unstable_by_key(|pattern_match| {
                            (pattern_match.span.start, Reverse(pattern_match.span.end))
                        });
                        matches.extend(selected);
                    }
                }

                Ok((matches, files))
            },
        )?;

        let payload = QueryPayload::new(&pattern, matches);

        Ok(
            CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                .with_files(files)
                .with_data(payload),
        )
    }
}
