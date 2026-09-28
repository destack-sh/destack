use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use tspp_artifact::ArtifactKey;
use tspp_doc::{Example, ExampleFile, Expectation, Probe};
use tspp_query::{HoverRequest, Module, QueryPosition, QueryRequest, QueryResponse};
use tspp_repository::{Edit, Repository, Revision, TraceLevel};
use tspp_session::{ArtifactPriority, Executor, Session};
use tspp_source::{
    Diagnostic, DiagnosticCollection, DiagnosticSeverity, DiagnosticTarget, FileId, Span, TargetId,
};
use tspp_workspace::{RevisionPolicy, RunQueryInput, Workspace};

/// The root of the in-memory example repository.
const EXAMPLE_ROOT: &str = "/examples";
/// The manifest every example package declares.
const EXAMPLE_MANIFEST: &str = concat!(
    r#"{
  "packageManager": "tspp@"#,
    env!("CARGO_PKG_VERSION"),
    r#"",
  "name": "example",
  "targets": { "default": { "output": "program", "include": ["**/*.tspp"] } },
  "defaultTarget": "default"
}
"#
);
/// The target every example package checks.
const EXAMPLE_TARGET: &str = "default";

/// Checks documentation examples against the toolchain in one in-memory repository.
#[derive(Debug)]
pub struct ExampleRunner {
    /// The in-memory example workspace.
    workspace: Workspace,
    /// The repository behind the workspace.
    repository: Arc<Repository>,
    /// The session providing example artifacts.
    session: Arc<Session>,
    /// The base revision holding only the example manifest.
    base: Revision,
}

/// One example file bound to its module in a checked revision.
struct ExampleModule<'a> {
    /// The example file.
    file: &'a ExampleFile,
    /// The file id in the checked revision.
    file_id: FileId,
    /// The module and profile the file checks as.
    module: Module,
}

impl ExampleRunner {
    /// Open the in-memory example workspace with the shared example package.
    pub fn new(executor: Arc<Executor>) -> Result<Self, String> {
        let workspace = Workspace::memory(EXAMPLE_ROOT, Vec::new(), executor)
            .map_err(|error| format!("failed to open the example workspace: {error}"))?;
        let session = workspace.session();
        let repository = session.repository();
        let empty = workspace
            .revision()
            .map_err(|error| format!("failed to read the example revision: {error}"))?;

        // declare the shared example package once
        let manifest = repository
            .retain_blob(EXAMPLE_MANIFEST.as_bytes())
            .map_err(|error| format!("failed to store the example manifest: {error}"))?;
        let base = repository
            .edit(empty, [Edit::set_file("package.json", manifest)])
            .map_err(|error| format!("failed to declare the example package: {error}"))?
            .after;

        Ok(Self {
            workspace,
            repository,
            session,
            base,
        })
    }

    /// Check one example, returning every failed expectation.
    pub async fn check(&self, example: &Example) -> Result<Vec<String>, String> {
        let revision = self.fork(example)?;
        let (modules, target) = self.bind(example, revision)?;

        // compare diagnostics, then query types and read values
        let diagnostics = self.lint(revision, &modules, target).await?;
        let mut failures = compare_diagnostics(&modules, &diagnostics);
        for module in &modules {
            for probe in &module.file.probes {
                match &probe.expectation {
                    Expectation::Type(expected) => {
                        let actual = self.hover_type(revision, module, probe.range.start).await?;
                        if actual.as_deref() != Some(expected.as_str()) {
                            let actual = actual
                                .map_or("no type".to_string(), |actual| format!("`{actual}`"));
                            failures.push(format!(
                                "{}: expected type `{expected}`, found {actual}",
                                location(module.file, probe.range.clone())
                            ));
                        }
                    }
                    Expectation::Value(_) => failures.push(format!(
                        "{}: value probes are not implemented",
                        location(module.file, probe.range.clone())
                    )),
                    Expectation::Diagnostic { .. } => {}
                }
            }
        }

        Ok(failures)
    }

    /// Fork one revision holding the example's files on the base manifest.
    fn fork(&self, example: &Example) -> Result<Revision, String> {
        let edits = example
            .files
            .iter()
            .map(|file| {
                let blob = self
                    .repository
                    .retain_blob(file.source.as_bytes())
                    .map_err(|error| format!("failed to store {}: {error}", file.path))?;

                Ok(Edit::set_file(&file.path, blob))
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.repository
            .edit(self.base, edits)
            .map(|commit| commit.after)
            .map_err(|error| format!("failed to place example files: {error}"))
    }

    /// Bind every example file to its module, all in the one example package.
    fn bind<'a>(
        &self,
        example: &'a Example,
        revision: Revision,
    ) -> Result<(Vec<ExampleModule<'a>>, TargetId), String> {
        let mut target = None;
        let mut modules = Vec::with_capacity(example.files.len());

        // resolve each file's module and the package they share
        for file in &example.files {
            let path = Path::new(&file.path);
            let module_id = self
                .repository
                .module_id_for_path(revision, path)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("{} is not a source module", file.path))?;
            let package_id = self
                .repository
                .module(revision, module_id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("{} has no module", file.path))?
                .package_id;
            let module_target = TargetId::new(package_id, EXAMPLE_TARGET);
            if *target.get_or_insert(module_target) != module_target {
                return Err(format!("{} belongs to another package", file.path));
            }

            modules.push((file, module_id, self.repository.file_id(path)));
        }
        let Some(target) = target else {
            return Err("example has no files".to_string());
        };
        let profile_id = self
            .repository
            .profile_for_target(revision, target)
            .map_err(|error| error.to_string())?
            .id();

        // pair every module with the one example profile
        let modules = modules
            .into_iter()
            .map(|(file, module_id, file_id)| ExampleModule {
                file,
                file_id,
                module: Module {
                    module_id,
                    profile_id,
                },
            })
            .collect();

        Ok((modules, target))
    }

    /// Lint every example module and the example program, returning their diagnostics.
    async fn lint(
        &self,
        revision: Revision,
        modules: &[ExampleModule<'_>],
        target: TargetId,
    ) -> Result<DiagnosticCollection, String> {
        let mut keys = modules
            .iter()
            .map(|module| {
                ArtifactKey::module_linted(
                    module.module.module_id,
                    module.module.profile_id,
                    target,
                )
            })
            .collect::<Vec<_>>();
        if let Some(module) = modules.first() {
            keys.push(ArtifactKey::program_linted(
                module.module.profile_id,
                target,
            ));
        }

        // complete the lint artifacts, then read their diagnostics
        self.session
            .provide(revision, &keys, ArtifactPriority::Foreground)
            .complete()
            .await
            .map_err(|error| format!("failed to check the example: {error}"))?;

        self.repository
            .diagnostics_for_keys(revision, &keys)
            .map_err(|error| format!("failed to read example diagnostics: {error}"))
    }

    /// Return the hover type at one offset: the selected type, else the declaration.
    async fn hover_type(
        &self,
        revision: Revision,
        module: &ExampleModule<'_>,
        offset: u32,
    ) -> Result<Option<String>, String> {
        let request = QueryRequest::Hover(HoverRequest {
            position: QueryPosition {
                module: module.module,
                file_id: module.file_id,
                offset,
            },
        });
        let input = RunQueryInput {
            revision: RevisionPolicy::Exact(revision),
            request,
        };

        // run the hover query on the example revision
        let run = self
            .workspace
            .start_query(input, TraceLevel::Disabled)
            .map_err(|error| format!("failed to query a type: {error}"))?;
        let response = run
            .wait()
            .await
            .map_err(|error| format!("failed to query a type: {error}"))?
            .response;
        let QueryResponse::Hover(response) = response else {
            return Err("hover returned another response".to_string());
        };

        Ok(response.hover.and_then(|hover| {
            hover
                .items
                .into_iter()
                .next()
                .map(|item| item.selected_type.unwrap_or(item.declaration))
        }))
    }
}

/// Compare the diagnostics of one example against its diagnostic probes.
fn compare_diagnostics(
    modules: &[ExampleModule<'_>],
    diagnostics: &DiagnosticCollection,
) -> Vec<String> {
    let mut expected = modules
        .iter()
        .flat_map(|module| module.file.probes.iter().map(move |probe| (module, probe)))
        .filter(|(_, probe)| matches!(probe.expectation, Expectation::Diagnostic { .. }))
        .collect::<Vec<_>>();
    let mut failures = Vec::new();

    // match each reported diagnostic against one expectation
    for diagnostic in diagnostics.iter() {
        let (file_id, span) = match diagnostic.primary.target {
            DiagnosticTarget::Span(span) => (span.file, Some(span)),
            DiagnosticTarget::File(file_id) => (file_id, None),
        };
        let position = expected.iter().position(|(module, probe)| {
            span.is_some_and(|span| expects(module, probe, diagnostic, span))
        });

        // consume a matching expectation
        if let Some(position) = position {
            expected.swap_remove(position);
        }
        // report an unexpected error inside or outside the example
        else if diagnostic.severity == DiagnosticSeverity::Error {
            let module = modules.iter().find(|module| module.file_id == file_id);
            let place = match (module, span) {
                (Some(module), Some(span)) => location(module.file, span.start..span.end),
                (Some(module), None) => module.file.path.clone(),
                (None, _) => "outside the example".to_string(),
            };
            failures.push(format!(
                "{place}: unexpected error[{}]: {}",
                diagnostic.id, diagnostic.message
            ));
        }
    }

    // report expectations nothing matched
    for (module, probe) in expected {
        if let Expectation::Diagnostic {
            severity,
            code,
            message,
        } = &probe.expectation
        {
            failures.push(format!(
                "{}: expected {}[{code}]: {message}",
                location(module.file, probe.range.clone()),
                severity.family_name()
            ));
        }
    }

    failures
}

/// Return whether one diagnostic probe expects one reported diagnostic at its span.
fn expects(module: &ExampleModule<'_>, probe: &Probe, diagnostic: &Diagnostic, span: Span) -> bool {
    let Expectation::Diagnostic {
        severity,
        code,
        message,
    } = &probe.expectation
    else {
        return false;
    };

    module.file_id == span.file
        && probe.range == (span.start..span.end)
        && *severity == diagnostic.severity
        && *code == diagnostic.id
        && *message == diagnostic.message
}

/// Return `path:line:column` for one range in an example file, with its end when it spans text.
fn location(file: &ExampleFile, range: Range<u32>) -> String {
    let (line, column) = position(file, range.start);
    let (end_line, end_column) = position(file, range.end);

    // name an empty range by its start
    if range.is_empty() {
        format!("{}:{line}:{column}", file.path)
    }
    // name the end column of a range on one line
    else if end_line == line {
        format!("{}:{line}:{column}-{end_column}", file.path)
    }
    // name the end line and column of a range across lines
    else {
        format!("{}:{line}:{column}-{end_line}:{end_column}", file.path)
    }
}

/// Return the one-based line and column of one offset in an example file.
fn position(file: &ExampleFile, offset: u32) -> (usize, usize) {
    let before = &file.source[..offset as usize];
    let line = before.matches('\n').count() + 1;
    let column = before.len() - before.rfind('\n').map_or(0, |index| index + 1) + 1;

    (line, column)
}
