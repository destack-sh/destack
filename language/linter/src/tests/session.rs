use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use destack_artifact::{
    ArtifactKey, BuildId, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    DiagnosticLike, DiagnosticRecord, MirLowered, ToDiagnostic,
};
use destack_mir as mir;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host, ProviderContext,
    Repository, Revision, RevisionPin, Settings,
};
use destack_session::{ArtifactPriority, Executor, Session};
use destack_source::{
    Applicability, Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticSeverity,
    DiagnosticTarget, DiffOptions, File, FileId, FilePatch, FileType, MemoryFileSystem, ModuleId,
    PackageId, PrintOptions, ProfileId, TargetId, Uri, apply_file_patch, format_diff,
    print_diagnostics,
};
use futures::executor::block_on;

use crate::{Fixability, Lint, LintCheck, LintTier, Linter, MirModule};

const SOURCE_PATH: &str = "main.ds";
const WARMUP_PATH: &str = "__warm.ds";
const TARGET_NAME: &str = "native";
const DEFAULT_DESTACK_JSON: &str = r#"{
  "name": "@test/app"
}"#;

/// One isolated lint test session.
pub(crate) struct TestSession {
    /// The lint under test.
    lint: &'static Lint,
    /// The emitted diagnostics.
    diagnostics: DiagnosticCollection,
    /// The source available to the test session.
    source: TestSource,
    /// The displayed fixture file.
    file: Arc<File>,
    /// The displayed fixture path.
    path: String,
}

/// One complete fixture for an isolated lint run.
struct LintFixture {
    /// The lint under test.
    lint: &'static Lint,
    /// The source repository.
    repository: Arc<Repository>,
    /// The revision.
    revision: RevisionPin,
    /// The artifact session.
    session: Session,
    /// The entry module.
    module: ModuleId,
    /// The active profile.
    profile: ProfileId,
    /// The selected lint artifact key.
    key: ArtifactKey,
    /// The displayed fixture file.
    file: Arc<File>,
    /// The displayed fixture path.
    path: String,
    /// Diagnostics emitted by the lint.
    diagnostics: Mutex<Vec<Diagnostic>>,
}

/// Source available to one lint test session.
enum TestSource {
    /// One repository revision.
    Revision(RevisionPin),
    /// One standalone source file.
    File(Arc<File>),
}

impl TestSource {
    /// Return one source file.
    fn file(&self, id: FileId) -> Option<Arc<File>> {
        match self {
            Self::Revision(revision) => revision
                .file(id)
                .expect("lint diagnostic source should be readable"),
            Self::File(file) => (id == file.id).then(|| file.clone()),
        }
    }
}

impl TestSession {
    /// Assert the canonical reported and accepted examples for one lint.
    #[track_caller]
    pub(crate) fn assert_example(lint: &'static Lint) {
        match lint.check.tier() {
            LintTier::Dir => Self::assert_dir_example(lint),
            LintTier::Mir => Self::assert_mir_example_sources(lint),
        }
    }

    /// Assert the reported behavior and accepted replacement of one DIR lint example.
    fn assert_dir_example(lint: &'static Lint) {
        let reported = Self::dir_path(
            lint,
            lint.example.reported.path(),
            lint.example.reported.source(),
        );
        let [diagnostic] = reported.diagnostics.diagnostics.as_slice() else {
            panic!(
                "lint '{}' example emitted {} diagnostics:\n{}",
                lint.id,
                reported.diagnostics.len(),
                reported.render_diagnostics()
            );
        };
        assert_eq!(diagnostic.id, lint.id, "lint example reported another id");

        // require one correction allowed by the declared capability
        if lint.is_fixable() {
            let [suggestion] = diagnostic.suggestions.as_slice() else {
                panic!(
                    "lint '{}' example emitted {} corrections",
                    lint.id,
                    diagnostic.suggestions.len()
                );
            };
            if lint.fixability == Fixability::Automatic {
                assert_eq!(
                    suggestion.applicability,
                    Applicability::Automatic,
                    "lint '{}' example emitted a review correction",
                    lint.id
                );
            }
            reported.assert_edits(lint.example.accepted.source(), suggestion.applicability);
        } else {
            assert!(
                diagnostic.suggestions.is_empty(),
                "non-fixable lint '{}' emitted a correction",
                lint.id
            );
        }

        // require the accepted form to remain clean
        let accepted = Self::dir_path(
            lint,
            lint.example.accepted.path(),
            lint.example.accepted.source(),
        );
        accepted.assert_no_diagnostics();
    }

    /// Assert that one MIR lint's documentation examples pass check.
    fn assert_mir_example_sources(lint: &'static Lint) {
        for example in [&lint.example.reported, &lint.example.accepted] {
            let fixture = LintFixture::new(lint, example.path(), example.source(), &[]);
            let checked = fixture.check();
            checked.assert_no_diagnostics();
        }
    }

    /// Run one isolated DIR lint fixture.
    pub(crate) fn dir(lint: &'static Lint, source: &str) -> Self {
        Self::dir_path(lint, SOURCE_PATH, source)
    }

    /// Run one isolated DIR lint fixture at one source path.
    pub(crate) fn dir_path(lint: &'static Lint, path: &str, source: &str) -> Self {
        Self::dir_files(lint, path, source, &[])
    }

    /// Run one DIR lint fixture with additional source files.
    pub(crate) fn dir_files(
        lint: &'static Lint,
        path: &str,
        source: &str,
        additional_sources: &[(&str, &str)],
    ) -> Self {
        let fixture = LintFixture::new(lint, path, source, additional_sources);

        fixture.lint()
    }

    /// Run one isolated MIR module lint fixture.
    pub(crate) fn mir(lint: &'static Lint, source: &str) -> Self {
        let file = Arc::new(
            File::from_text(
                FileId::new(0),
                "main.mir".to_string(),
                Uri::from_string("main.mir"),
                None,
                FileType::Text,
                trim_source_frame(source).to_string(),
            )
            .expect("lint MIR should load"),
        );
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("lint MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            let diagnostics = render_diagnostics_with(&parsed.diagnostics, |id| {
                (id == file.id).then(|| file.clone())
            });

            panic!("failed to parse lint MIR:\n{diagnostics}");
        }

        // build the MIR module consumed by the lint
        let (tree, target, mut layouts, dispatch, drops, effects, profile, strings, _) =
            parsed.into_parts();
        let mut builder = mir::LayoutBuilder::new(&tree, &mut layouts, target);
        builder
            .layout_reachable_types()
            .expect("lint MIR layouts should build");

        // lay out every concrete type declaration for the declaration lints
        let declared = tree
            .iter_nodes::<mir::TypeDeclaration>()
            .filter(|(_, declaration)| declaration.generics.is_empty())
            .filter_map(|(_, declaration)| declaration.definition)
            .collect::<Vec<_>>();
        for ty in declared {
            builder
                .layout_type(ty)
                .expect("lint MIR declaration layouts should build");
        }
        let lowered = MirLowered {
            tree: Arc::new(tree),
            target,
            layouts,
            dispatch,
            drops,
            witnesses: mir::WitnessTable::default(),
            effects,
            profile,
            initializer: None,
        };
        let mut module = MirModule::new(
            ModuleId::new(PackageId::new(0), 0),
            Arc::new(lowered),
            Arc::new(strings),
            None,
        );
        let LintCheck::MirModule(check) = lint.check else {
            panic!("lint '{}' is not a MIR module lint", lint.id);
        };
        let output = check(&mut module, lint).expect("MIR lint should run");

        // convert through the same diagnostic model as production lints
        let context = MirDiagnosticContext { file: file.clone() };
        let diagnostics = output
            .into_diagnostics()
            .into_iter()
            .map(|mut diagnostic| {
                diagnostic
                    .diagnostic_mut()
                    .set_severity(Some(DiagnosticSeverity::Warning));
                diagnostic
                    .to_diagnostic(&context)
                    .expect("MIR lint diagnostic should resolve")
            })
            .collect();

        Self {
            lint,
            diagnostics: DiagnosticCollection::from_diagnostics(diagnostics),
            source: TestSource::File(file.clone()),
            file,
            path: "main.mir".to_string(),
        }
    }

    /// Assert the complete rendered diagnostics.
    #[track_caller]
    pub(crate) fn assert_diagnostics(&self, expected: &str) -> &Self {
        let actual = self.render_diagnostics();

        assert_snapshot(actual, expected);

        self
    }

    /// Assert that no diagnostics were emitted.
    #[track_caller]
    pub(crate) fn assert_no_diagnostics(&self) -> &Self {
        assert!(
            self.diagnostics.is_empty(),
            "unexpected lint diagnostics:\n{}",
            self.render_diagnostics()
        );

        self
    }

    /// Assert the source produced by all automatic fixes.
    #[track_caller]
    pub(crate) fn assert_fixes(&self, expected: &str) -> &Self {
        self.assert_edits(expected, Applicability::Automatic)
    }

    /// Assert the source produced by all review suggestions.
    #[track_caller]
    pub(crate) fn assert_suggestions(&self, expected: &str) -> &Self {
        self.assert_edits(expected, Applicability::Dangerous)
    }

    /// Assert the source produced by suggestions at one applicability.
    #[track_caller]
    fn assert_edits(&self, expected: &str, applicability: Applicability) -> &Self {
        let mut file_patch = FilePatch::new(self.file.id);

        // collect matching suggestions for the source file
        for diagnostic in self.diagnostics.iter() {
            for suggestion in &diagnostic.suggestions {
                if suggestion.applicability != applicability {
                    continue;
                }
                for suggested_file in &suggestion.patches.files {
                    assert_eq!(
                        suggested_file.file, self.file.id,
                        "single-module lint test received a fix for another file"
                    );
                    for patch in &suggested_file.patches {
                        file_patch.push(patch.clone());
                    }
                }
            }
        }
        assert!(
            !file_patch.is_empty(),
            "lint test emitted no {applicability:?} edits"
        );

        // apply the exact emitted patches
        let actual =
            apply_file_patch(&self.file, &file_patch).expect("lint test fixes should apply");

        assert_snapshot(&actual, expected);

        // require the corrected source to pass the same lint again
        let corrected = match self.lint.check.tier() {
            LintTier::Dir => Self::dir_path(self.lint, &self.path, &actual),
            LintTier::Mir => Self::mir(self.lint, &actual),
        };
        corrected.assert_no_diagnostics();

        self
    }

    /// Render all diagnostics in stable source order.
    fn render_diagnostics(&self) -> String {
        render_diagnostics_with(&self.diagnostics, |id| self.source.file(id))
    }
}

impl LintFixture {
    /// Build one isolated lint fixture.
    fn new(
        lint: &'static Lint,
        path: &str,
        source: &str,
        additional_sources: &[(&str, &str)],
    ) -> Self {
        let (repository, base) = shared_repository_revision();
        let source = repository
            .retain_blob(trim_source_frame(source).as_bytes())
            .expect("lint source Blob should store");
        let mut edits = vec![Edit::set_file(path, source)];

        // add the remaining fixture files
        for (additional_path, additional_source) in additional_sources {
            let source = repository
                .retain_blob(trim_source_frame(additional_source).as_bytes())
                .expect("additional lint source Blob should store");
            edits.push(Edit::set_file(*additional_path, source));
        }

        // commit and retain the complete fixture revision
        let revision = repository
            .edit(base.revision(), edits)
            .expect("lint test revision should commit")
            .after;
        let revision = repository
            .pin(revision)
            .expect("lint test revision should remain live");
        let revision_id = revision.revision();
        let session =
            Session::new(repository.clone(), executor()).expect("lint test session should open");

        // resolve the source module and target
        let module = repository
            .module_id_for_path(revision_id, Path::new(path))
            .expect("lint test module should resolve")
            .expect("lint test module should exist");
        let module = repository
            .module(revision_id, module)
            .expect("lint test module should be readable")
            .expect("lint test module should exist");
        let target = TargetId::new(module.package_id, TARGET_NAME);
        let profile = repository
            .profile_for_module_target(revision_id, module.id, target)
            .expect("lint test profile should resolve")
            .id();
        let key = match lint.check {
            LintCheck::DirModule(_) | LintCheck::MirModule(_) => {
                ArtifactKey::module_linted(module.id, profile, target)
            }
            LintCheck::DirProgram(_) | LintCheck::MirProgram(_) => {
                ArtifactKey::program_linted(profile, target)
            }
        };
        let file = repository
            .file(revision_id, module.file_id)
            .expect("lint test source should be readable")
            .expect("lint test source should exist");

        Self {
            lint,
            repository: repository.clone(),
            revision,
            session,
            module: module.id,
            profile,
            key,
            file,
            path: path.to_string(),
            diagnostics: Mutex::new(Vec::new()),
        }
    }

    /// Check one fixture without running a lint.
    fn check(self) -> TestSession {
        let key = ArtifactKey::dir_checked(self.module, self.profile);
        self.require(&[key]);
        let diagnostics = self.read_diagnostics(&[key]);

        self.finish(diagnostics)
    }

    /// Run exactly one selected lint over this fixture.
    fn lint(self) -> TestSession {
        let linter = Linter::with_lint(self.repository.clone(), self.lint);
        let dependencies = self.require_lint_dependencies(&linter);
        let required = dependencies
            .requirements
            .iter()
            .map(|requirement| requirement.artifact_key())
            .collect::<Vec<_>>();

        // execute the selected lint after its compiler artifacts are ready
        linter
            .provide(&self)
            .unwrap_or_else(|error| panic!("lint test provider failed: {error}"));
        let mut diagnostics = self.read_diagnostics(&required);
        diagnostics.diagnostics.extend(
            self.diagnostics
                .lock()
                .expect("lint fixture diagnostics should lock")
                .iter()
                .cloned(),
        );

        self.finish(diagnostics)
    }

    /// Resolve and require the selected lint's complete dependency set.
    fn require_lint_dependencies(
        &self,
        linter: &Linter,
    ) -> destack_artifact::ArtifactDependencySet {
        loop {
            let mut dependencies = linter
                .collect(self)
                .unwrap_or_else(|error| panic!("lint test collection failed: {error}"));
            dependencies.normalize();
            let required = dependencies
                .requirements
                .iter()
                .map(|requirement| requirement.artifact_key())
                .collect::<Vec<_>>();
            self.require(&required);

            if !dependencies.is_partial {
                return dependencies;
            }
        }
    }

    /// Require fixture artifacts.
    fn require(&self, keys: &[ArtifactKey]) {
        let run =
            self.session
                .provide(self.revision.revision(), keys, ArtifactPriority::Foreground);
        if let Err(error) = block_on(run.wait()) {
            let repository = self.revision.repository();
            let diagnostics = repository
                .diagnostics(self.revision.revision(), None)
                .expect("lint test diagnostics should be readable");
            let diagnostics =
                render_diagnostics(repository, self.revision.revision(), &diagnostics);

            panic!("lint test artifact failed: {error}\n\n{diagnostics}");
        }
    }

    /// Return diagnostics produced by one fixture artifact.
    fn read_diagnostics(&self, keys: &[ArtifactKey]) -> DiagnosticCollection {
        self.revision
            .repository()
            .diagnostics_for_keys(self.revision.revision(), keys)
            .expect("lint test diagnostics should be readable")
    }

    /// Finish one lint test session from this fixture.
    fn finish(self, diagnostics: DiagnosticCollection) -> TestSession {
        TestSession {
            lint: self.lint,
            diagnostics,
            source: TestSource::Revision(self.revision),
            file: self.file,
            path: self.path,
        }
    }
}

impl DiagnosticContext for LintFixture {
    /// Resolve one lint diagnostic source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let target = match anchor {
            DiagnosticAnchor::Span(span) => DiagnosticTarget::Span(*span),
            DiagnosticAnchor::File(file) => DiagnosticTarget::File(*file),
            _ => {
                return Err(DiagnosticError::InvalidAnchor {
                    message: format!("lint test provider received {anchor:?}"),
                });
            }
        };
        let file = target.file();
        let blob = self
            .revision
            .repository()
            .file_blob(self.revision.revision(), file)
            .map_err(|error| DiagnosticError::InvalidAnchor {
                message: format!("failed to read lint diagnostic source: {error}"),
            })?
            .ok_or_else(|| DiagnosticError::InvalidAnchor {
                message: format!("lint diagnostic source {file:?} is missing"),
            })?;

        Ok(DiagnosticLabel {
            blob,
            target,
            message,
        })
    }

    /// Report unsupported diagnostic display values.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: format!("lint test provider cannot display {display:?}"),
        })
    }
}

impl ProviderContext for LintFixture {
    /// Return the revision.
    fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the selected lint artifact key.
    fn artifact_key(&self) -> ArtifactKey {
        self.key
    }

    /// Add already-resolved diagnostics.
    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>) {
        let mut output = self
            .diagnostics
            .lock()
            .expect("lint provider diagnostics should lock");
        for record in diagnostics {
            assert!(
                record.deferred_labels.is_empty(),
                "lint fixture received deferred diagnostic labels"
            );
            output.push(record.diagnostic);
        }
    }

    /// Resolve and add one lint diagnostic.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        let diagnostic = diagnostic.to_diagnostic(self)?;
        self.diagnostics
            .lock()
            .expect("lint provider diagnostics should lock")
            .push(diagnostic);

        Ok(())
    }
}

/// Diagnostic context for one raw MIR file.
struct MirDiagnosticContext {
    /// The raw MIR file.
    file: Arc<File>,
}

impl DiagnosticContext for MirDiagnosticContext {
    /// Resolve one MIR diagnostic anchor.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let DiagnosticAnchor::Span(span) = anchor else {
            return Err(DiagnosticError::InvalidAnchor {
                message: format!("raw MIR module lint emitted {anchor:?}"),
            });
        };

        Ok(DiagnosticLabel {
            blob: self.file.blob(),
            target: DiagnosticTarget::Span(*span),
            message,
        })
    }

    /// Display one diagnostic value.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: format!("raw MIR module lint cannot display {display:?}"),
        })
    }
}

/// Render diagnostics through the production source printer.
pub(super) fn render_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> String {
    let file = |file| {
        repository
            .file(revision, file)
            .expect("lint diagnostic source should be readable")
    };

    render_diagnostics_with(diagnostics, file)
}

/// Render diagnostics through one source file resolver.
fn render_diagnostics_with(
    diagnostics: &DiagnosticCollection,
    file: impl Fn(FileId) -> Option<Arc<File>>,
) -> String {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let output = lines.clone();
    let writer = Arc::new(move |line: &str| {
        output
            .lock()
            .expect("lint diagnostic output should lock")
            .push(line.to_string());
    });
    let options = PrintOptions::new()
        .with_color(false)
        .with_skip_summary(true)
        .with_line_writer(writer);
    print_diagnostics(&file, diagnostics, options).expect("lint diagnostics should render");

    lines
        .lock()
        .expect("lint diagnostic output should lock")
        .join("\n")
}

/// Return the shared linter test repository and its warmed revision.
fn shared_repository_revision() -> &'static (Arc<Repository>, RevisionPin) {
    static BASE: OnceLock<(Arc<Repository>, RevisionPin)> = OnceLock::new();

    BASE.get_or_init(|| {
        let (repository, revision) = cold_repository_revision();

        // anchor the workspace package while its builtin modules warm
        let source = repository
            .retain_blob(b"")
            .expect("warmup source Blob should store");
        let revision = repository
            .edit(revision, [Edit::set_file(WARMUP_PATH, source)])
            .expect("warmup source should commit")
            .after;

        // start one session for the complete warmup
        let session = Session::new(repository.clone(), executor())
            .expect("library warmup session should start");

        // resolve the workspace profile used by lint fixtures
        let package = repository.embedded_builtin();
        let module = repository
            .module_id_for_path(revision, WARMUP_PATH.as_ref())
            .expect("warmup module should resolve")
            .expect("warmup module should exist");
        let workspace = repository
            .module(revision, module)
            .expect("warmup module should load")
            .expect("warmup module should exist")
            .package_id;
        let workspace_target = TargetId::new(workspace, TARGET_NAME);
        let workspace_profile = repository
            .profile_for_target(revision, workspace_target)
            .expect("lint fixture profile should resolve")
            .id();

        // materialize every builtin module under the workspace profile once
        let keys = package
            .module_ids()
            .map(|module| ArtifactKey::dir_materialized(module, workspace_profile))
            .collect::<Vec<_>>();
        let run = session.provide(revision, &keys, ArtifactPriority::Foreground);
        block_on(run.wait()).expect("builtin library warmup should materialize");

        // remove the temporary module from the fixture base
        let base = repository
            .edit(revision, [Edit::remove_file(WARMUP_PATH)])
            .expect("warmup module should retire")
            .after;
        let base = repository
            .pin(base)
            .expect("shared linter test revision should remain live");

        (repository, base)
    })
}

/// Build one fresh linter test repository at its default revision.
fn cold_repository_revision() -> (Arc<Repository>, Revision) {
    // resolve the in-memory repository layout
    let root = PathBuf::new();
    let environment = Environment::default();
    let settings = Settings::default();
    let layout = DestackLayout::resolve(
        &root,
        &root,
        &environment,
        &settings,
        &DestackLayoutOverride::default(),
        None,
    );

    // create the cooperative repository
    let host = Host::new(
        BuildId::test(),
        environment,
        Arc::new(MemoryFileSystem::new()),
    )
    .with_execution(Execution::Cooperative);
    let (repository, revision) = Repository::new(root, host, settings, layout);
    let repository = Arc::new(repository);

    // commit the default package configuration
    let configuration = repository
        .retain_blob(DEFAULT_DESTACK_JSON.as_bytes())
        .expect("default lint configuration Blob should store");
    let revision = repository
        .edit(revision, [Edit::set_file("destack.json", configuration)])
        .expect("default lint configuration should commit")
        .after;

    (repository, revision)
}

/// Return the artifact executor for one linter test session.
fn executor() -> Arc<Executor> {
    // cooperative hosts execute inline, so each session schedules alone
    Executor::new(Execution::Cooperative, 1).expect("linter test artifact executor should start")
}

/// Remove one framing newline from each edge of multiline source.
fn trim_source_frame(source: &str) -> &str {
    let source = source.strip_prefix('\n').unwrap_or(source);

    source.strip_suffix('\n').unwrap_or(source)
}

/// Assert one complete lint test snapshot.
#[track_caller]
fn assert_snapshot(actual: impl AsRef<str>, expected: &str) {
    let actual = trim_source_frame(actual.as_ref());
    let expected = trim_source_frame(expected);
    if actual == expected {
        return;
    }

    let diff = format_diff(expected, actual, &DiffOptions::new());

    panic!("snapshot mismatch\n\n{diff}");
}
