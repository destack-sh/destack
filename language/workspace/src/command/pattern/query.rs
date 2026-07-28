use std::cmp::Reverse;
use std::path::PathBuf;

use destack_artifact::ArtifactKey;
use destack_dir as dir;
use destack_pattern::{Binding, Matcher, Pattern, PatternMatch};
use destack_repository::ArtifactReader;
use destack_serde::Reflect;
use destack_source::{DiagnosticCollection, File, Span, Uri};
use serde::{Deserialize, Serialize};

use super::super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::super::{CommandContext, CommandError, CommandOutcome, CommandResult};
use super::program::CheckedPrograms;

/// Request to query source with one structural pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryInput {
    /// Revision selected for this query.
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
        let file = context.add_file(
            PathBuf::from(".destack/command/query/pattern.ds-pattern"),
            &self.pattern,
        )?;
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
            let path = format!(".destack/command/query/predicate-{}.ds-pattern", index + 1);
            let file = context.add_file(PathBuf::from(path), predicate)?;
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
    pub(crate) fn run_query_command(
        &mut self,
        input: &QueryInput,
    ) -> CommandResult<CommandOutcome<QueryPayload>> {
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let pattern = match input.compile(self)? {
            Ok(pattern) => pattern,
            Err(diagnostics) => {
                let exit_code = diagnostics.get_status_code();
                let outcome = CommandOutcome::new(diagnostics, exit_code, modules.len(), 0, 0)
                    .with_data(QueryPayload::default());

                return Ok(outcome);
            }
        };

        // request only parsed DIR for structural queries
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
                    .with_data(QueryPayload::default());

            return Ok(outcome);
        }

        let artifacts = ArtifactReader::new(self.repository.as_ref(), revision);
        let mut matches = Vec::new();
        let mut files = Vec::new();
        for module in modules.iter().copied() {
            let parsed = artifacts
                .dir_parsed(module)
                .map_err(|error| error.to_string())?;
            let view = dir::View::new(&parsed.tree);
            let matcher = match checked.as_ref() {
                Some(checked) => {
                    let (module, program) = checked.module(module)?;

                    Matcher::with_module(&pattern, view, module, program)
                }
                None => Matcher::new(&pattern, view),
            };

            // query each physical source file independently
            for parsed_file in &parsed.files {
                let file = self
                    .repository
                    .file(revision, parsed_file.file_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| {
                        CommandError::internal(format!(
                            "missing query source file {:?}",
                            parsed_file.file_id
                        ))
                    })?;
                let candidates = view.iter_node_ids_in_file(file.id);
                let selected = matcher
                    .find(candidates)
                    .map_err(|error| CommandError::internal(error.to_string()))?;
                if input.include_sources && !selected.is_empty() {
                    files.push(file.clone());
                }
                let mut selected = selected
                    .into_iter()
                    .map(|pattern_match| {
                        QueryMatch::new(&pattern, view, file.as_ref(), pattern_match)
                    })
                    .collect::<CommandResult<Vec<_>>>()?;
                selected.sort_unstable_by_key(|pattern_match| {
                    (pattern_match.span.start, Reverse(pattern_match.span.end))
                });
                matches.extend(selected);
            }
        }

        let payload = QueryPayload::new(&pattern, matches);

        Ok(
            CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                .with_files(files)
                .with_data(payload),
        )
    }
}
