use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use destack_artifact::{ArtifactKey, MemoryBlobStore, NullArtifactStore};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host, Ref, Repository,
    Revision, Settings,
};
use destack_session::Session;
use destack_source::{
    Applicability, Content, DiagnosticCollection, DiffOptions, File, FileId, FilePatch,
    MemoryFileSystem, PrintOptions, TargetId, apply_file_patch, format_diff, print_diagnostics,
};
use serde_json::{Map, Value, json};

use crate::{LINTS, Lint};

const SOURCE_PATH: &str = "main.ds";
const TARGET_NAME: &str = "native";

/// One isolated lint test session.
pub(crate) struct TestSession {
    /// The shared repository.
    repository: Arc<Repository>,
    /// The immutable test revision.
    revision: Revision,
    /// The emitted diagnostics.
    diagnostics: DiagnosticCollection,
    /// The source file id.
    file: FileId,
}

impl TestSession {
    /// Run one isolated lint test.
    pub(crate) fn new(lint: &Lint, source: &str) -> Self {
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
        let key = ArtifactKey::module_linted(module.id, profile, target);
        session
            .require(revision, key)
            .expect("lint test artifact should be provided");
        let diagnostics = repository
            .diagnostics(revision, Some(key))
            .expect("lint test diagnostics should be readable");

        Self {
            repository: repository.clone(),
            revision,
            diagnostics,
            file: module.file_id,
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
        let mut file_patch = FilePatch::new(self.file);

        // collect matching suggestions for the source file
        for diagnostic in self.diagnostics.iter() {
            for suggestion in &diagnostic.suggestions {
                if suggestion.applicability != applicability {
                    continue;
                }
                for suggested_file in &suggestion.patches.files {
                    assert_eq!(
                        suggested_file.file, self.file,
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
        let file = self.file(self.file);
        let actual = apply_file_patch(&file, &file_patch).expect("lint test fixes should apply");

        assert_snapshot(actual, expected);

        self
    }

    /// Render all diagnostics in stable source order.
    fn render_diagnostics(&self) -> String {
        render_diagnostics(&self.repository, self.revision, &self.diagnostics)
    }

    /// Return one source file in the test revision.
    fn file(&self, file: FileId) -> Arc<File> {
        self.repository
            .file(self.revision, file)
            .expect("lint test source should be readable")
            .expect("lint test source should exist")
    }
}

/// Render diagnostics through the production source printer.
pub(super) fn render_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
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
    let file = |file| {
        repository
            .file(revision, file)
            .expect("lint diagnostic source should be readable")
    };

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
