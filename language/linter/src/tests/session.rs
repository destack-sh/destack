use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use destack_artifact::{
    ArtifactKey, BuildId, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    MemoryBlobStore, MirLowered, NullArtifactStore, ToDiagnostic,
};
use destack_mir as mir;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host, Ref, Repository,
    Revision, Settings,
};
use destack_session::Session;
use destack_source::{
    Applicability, Content, DiagnosticCollection, DiagnosticLabel, DiagnosticSeverity,
    DiagnosticTarget, DiffOptions, File, FileId, FilePatch, FileType, MemoryFileSystem, ModuleId,
    PackageId, PrintOptions, ProfileId, TargetId, Uri, apply_file_patch, format_diff,
    print_diagnostics,
};
use serde_json::{Map, Value, json};

use crate::{Fixability, LINTS, Lint, LintCheck, LintScope, LintTier, MirModule};

const SOURCE_PATH: &str = "main.ds";
const TARGET_NAME: &str = "native";

/// One isolated lint test session.
pub(crate) struct TestSession {
    /// The lint under test.
    lint: &'static Lint,
    /// The emitted diagnostics.
    diagnostics: DiagnosticCollection,
    /// The displayed fixture file.
    file: Arc<File>,
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
        let reported = Self::dir(lint, lint.example.reported());
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
            reported.assert_edits(lint.example.accepted(), suggestion.applicability);
        } else {
            assert!(
                diagnostic.suggestions.is_empty(),
                "non-fixable lint '{}' emitted a correction",
                lint.id
            );
        }

        // require the accepted form to remain clean
        let accepted = Self::dir(lint, lint.example.accepted());
        accepted.assert_no_diagnostics();
    }

    /// Assert that one MIR lint's documentation examples pass check.
    fn assert_mir_example_sources(lint: &'static Lint) {
        for source in [lint.example.reported(), lint.example.accepted()] {
            let checked = Self::source(lint, source, |module, profile, _| {
                ArtifactKey::dir_checked(module, profile)
            });
            checked.assert_no_diagnostics();
        }
    }

    /// Run one isolated checked DIR lint fixture.
    pub(crate) fn dir(lint: &'static Lint, source: &str) -> Self {
        Self::source(lint, source, |module, profile, target| {
            match lint.check.scope() {
                LintScope::Module => ArtifactKey::module_linted(module, profile, target),
                LintScope::Program => ArtifactKey::program_linted(profile, target),
            }
        })
    }

    /// Run one source fixture through the selected artifact.
    fn source(
        lint: &'static Lint,
        source: &str,
        artifact: impl FnOnce(ModuleId, ProfileId, TargetId) -> ArtifactKey,
    ) -> Self {
        let (repository, base) = shared_repository();
        let configuration = lint_configuration(lint);
        let edits = [
            Edit::AddFile {
                logical_path: "destack.json".to_string(),
                content: Content::Text {
                    content: configuration,
                },
            },
            Edit::AddFile {
                logical_path: SOURCE_PATH.to_string(),
                content: Content::Text {
                    content: trim_source_frame(source).to_string(),
                },
            },
        ];
        let revision = repository
            .fork_with_edits(*base, edits)
            .expect("lint test revision should publish");

        // resolve the source module and target
        let module = repository
            .module_id_for_path(revision, Path::new(SOURCE_PATH))
            .expect("lint test module should resolve")
            .expect("lint test module should exist");
        let module = repository
            .module(revision, module)
            .expect("lint test module should be readable")
            .expect("lint test module should exist");
        let target = TargetId::new(module.package_id, TARGET_NAME);
        let profile = repository
            .profile_for_module_target(revision, module.id, target)
            .expect("lint test profile should resolve")
            .id();

        // provide the lint artifact through one private session
        let reference = next_reference();
        let session = Session::fork(
            PathBuf::new(),
            PathBuf::new(),
            repository.clone(),
            reference,
            revision,
            1,
            None,
        )
        .expect("lint test session should open");
        let key = artifact(module.id, profile, target);
        if let Err(error) = session.require(revision, key) {
            let diagnostics = repository
                .diagnostics(revision, None)
                .expect("lint test diagnostics should be readable");
            let diagnostics = render_diagnostics(repository, revision, &diagnostics);

            panic!("lint test artifact failed: {error}\n\n{diagnostics}");
        }
        let diagnostics = repository
            .diagnostics_for_keys(revision, &[key])
            .expect("lint test diagnostics should be readable");
        let file = repository
            .file(revision, module.file_id)
            .expect("lint test source should be readable")
            .expect("lint test source should exist");

        Self {
            lint,
            diagnostics,
            file,
        }
    }

    /// Run one isolated MIR module lint fixture.
    pub(crate) fn mir(lint: &'static Lint, source: &str) -> Self {
        let file = Arc::new(File::from_text(
            FileId::new(0),
            "main.mir".to_string(),
            Uri::from_string("main.mir"),
            None,
            FileType::Text,
            trim_source_frame(source).to_string(),
        ));
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
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings, _) =
            parsed.into_parts();
        let lowered = MirLowered {
            tree,
            target,
            types,
            layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
            initializer: None,
        };
        let mut module = MirModule::new(
            ModuleId::new(PackageId::new(0), 0),
            Arc::new(lowered),
            Arc::new(strings),
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
            file,
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

        // require the corrected source to pass the same lint after checking again
        let corrected = Self::dir(self.lint, &actual);
        corrected.assert_no_diagnostics();

        self
    }

    /// Render all diagnostics in stable source order.
    fn render_diagnostics(&self) -> String {
        let file = self.file.clone();

        render_diagnostics_with(&self.diagnostics, move |id| {
            (id == file.id).then(|| file.clone())
        })
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
            content: self.file.content_id(),
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

/// Build a configuration that enables only the selected lint.
fn lint_configuration(selected: &Lint) -> String {
    let rules = LINTS
        .iter()
        .map(|lint| {
            let level = if lint.id == selected.id {
                "warning"
            } else {
                "off"
            };

            (lint.id.to_string(), Value::String(level.to_string()))
        })
        .collect::<Map<_, _>>();
    let configuration = json!({
        "name": "@test/app",
        "linter": {
            "rules": rules,
        },
    });

    serde_json::to_string_pretty(&configuration).expect("lint test configuration should serialize")
}

/// Return the shared linter test repository and its empty revision.
pub(super) fn shared_repository() -> &'static (Arc<Repository>, Revision) {
    static REPOSITORY: OnceLock<(Arc<Repository>, Revision)> = OnceLock::new();

    REPOSITORY.get_or_init(|| {
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
        let host = Host::new(
            BuildId::test(),
            environment,
            Arc::new(MemoryFileSystem::new()),
            Arc::new(MemoryBlobStore::new()),
        )
        .with_execution(Execution::Inline);
        let repository = Repository::new(root, host, settings, layout)
            .with_artifact_store(Arc::new(NullArtifactStore::new()));
        let repository = Arc::new(repository);
        let reference = Ref::for_root(repository.path());
        let revision = repository
            .current(&reference)
            .expect("lint test repository ref should exist");

        (repository, revision)
    })
}

/// Allocate one private lint test ref.
fn next_reference() -> Ref {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    Ref::new(format!("linter-test:{id}"))
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
