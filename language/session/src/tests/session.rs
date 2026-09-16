use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use destack_artifact::{
    ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactVersion, BuildId, DirImported,
    DirParsed, IndexKind,
};
use destack_repository as repository;
use destack_repository::{
    ArtifactReader, DestackLayoutOverride, Environment, Execution, Host, Repository, Revision,
    RevisionPin, Settings, Trace, TraceLevel, TraceSnapshot, TraceView,
};
use destack_source::{FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId, Uri};
use futures::executor::block_on;
use parking_lot::Mutex;

use crate::{ArtifactPriority, Executor, Session, SessionError};

const DEFAULT_ROOT: &str = "/workspace";

/// In-memory session exercise harness.
pub(crate) struct TestSession {
    /// The source root selected by session opening.
    root: PathBuf,
    /// The repository imported by the session.
    repository: Arc<Repository>,
    /// Current retained source revision.
    revision: Mutex<RevisionPin>,
    /// The live session under test.
    session: Session,
}

impl TestSession {
    /// Open fresh repository state over the same files, host, and executor.
    fn reopen(&self) -> Self {
        let (repository, revision) = Repository::open(
            self.root.clone(),
            self.repository.host().clone(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .unwrap();
        let repository = Arc::new(repository);
        let revision = Mutex::new(repository.pin(revision).unwrap());
        let session = Session::new(repository.clone(), self.session.executor()).unwrap();

        Self {
            root: self.root.clone(),
            repository,
            revision,
            session,
        }
    }

    /// Read artifacts selected by the current revision.
    fn artifacts(&self) -> ArtifactReader<'_> {
        self.repository.artifact_reader(self.revision())
    }

    /// Return the resolved import targets of one completed module.
    fn imports(&self, key: ArtifactKey) -> Vec<Option<ModuleId>> {
        let imported = self
            .artifacts()
            .read::<DirImported>((key.module_id().unwrap(), key.profile_id().unwrap()))
            .unwrap();

        imported.modules.iter().map(|edge| edge.target).collect()
    }

    /// Return the selected version of a completed artifact.
    fn version(&self, key: ArtifactKey) -> ArtifactVersion {
        self.repository
            .artifact_version(self.revision(), &key)
            .unwrap()
            .unwrap()
    }

    /// Open one test session from files below the default root.
    pub(crate) fn open(files: &[(&str, &str)]) -> Result<Self, SessionError> {
        Self::create(files, 1, Execution::Cooperative)
    }

    /// Open one test session from files below the default root with explicit worker count.
    pub(crate) fn open_with_workers(
        files: &[(&str, &str)],
        worker_count: usize,
    ) -> Result<Self, SessionError> {
        let execution = if worker_count == 1 {
            Execution::Cooperative
        } else {
            Execution::Threaded
        };

        Self::create(files, worker_count, execution)
    }

    /// Open one test session on a shared artifact executor.
    fn open_with_executor(
        files: &[(&str, &str)],
        executor: Arc<Executor>,
    ) -> Result<Self, SessionError> {
        let execution = executor.execution();

        Self::create_with_executor(files, execution, executor)
    }

    /// Create one test session with explicit executor behavior.
    fn create(
        files: &[(&str, &str)],
        worker_count: usize,
        execution: Execution,
    ) -> Result<Self, SessionError> {
        let executor = Executor::new(execution, worker_count)?;

        Self::create_with_executor(files, execution, executor)
    }

    /// Create one test session on an explicit artifact executor.
    fn create_with_executor(
        files: &[(&str, &str)],
        execution: Execution,
        executor: Arc<Executor>,
    ) -> Result<Self, SessionError> {
        let root = PathBuf::from(DEFAULT_ROOT);
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("test root directory should be created");

        for (path, content) in files {
            fs.write(&root.join(path), content.as_bytes())
                .expect("test file should write");
        }

        let host = Host::new(BuildId::test(), Environment::default(), fs.clone())
            .with_execution(execution);
        let (repository, revision) = Repository::open(
            root.clone(),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;
        let repository = Arc::new(repository);
        let root = repository.path().to_path_buf();
        let revision = repository.pin(revision).map_err(SessionError::from)?;
        let session = Session::new(repository.clone(), executor)?;

        Ok(Self {
            root,
            repository,
            revision: Mutex::new(revision),
            session,
        })
    }

    /// Replace one source file in the repository backing this session.
    pub(crate) fn edit_text(&self, path: &str, text: &str) {
        let before = self.revision();
        let blob = self
            .repository
            .retain_blob(text.as_bytes())
            .expect("test source Blob should store");
        let edit = repository::Edit::set_file(path, blob);
        let after = self
            .repository
            .edit(before, vec![edit])
            .expect("test source edit should commit")
            .after;
        let after = self
            .repository
            .pin(after)
            .expect("test source revision should remain live");
        *self.revision.lock() = after;
    }

    /// Check one module target.
    pub(crate) fn check(&self, path: &str, target: &str) -> (ArtifactVersion, TraceSnapshot) {
        let revision = self.revision();
        let module = self.module_id(path, revision);
        let profile = self.profile_id(revision, module, target);
        let key = ArtifactKey::dir_checked(module, profile);
        let trace = self.session.start_trace(TraceLevel::Timings);
        let run = self.session.provide_traced(
            revision,
            &[key],
            ArtifactPriority::Foreground,
            trace.clone(),
            None,
        );
        block_on(run.wait()).expect("test artifact should be provided");
        trace.finish();
        let version = self
            .repository
            .artifact_version(revision, &key)
            .expect("test artifact version should read")
            .expect("test artifact version should exist");
        let trace = self.trace(trace.as_ref());

        (version, trace)
    }

    /// Provide one artifact key and return its terminal outcome.
    pub(crate) fn provide(&self, key: ArtifactKey) -> ArtifactOutcome {
        let revision = self.revision();
        let run = self
            .session
            .provide(revision, &[key], ArtifactPriority::Foreground);
        let _ = block_on(run.wait());
        let version = self
            .repository
            .artifact_version(revision, &key)
            .expect("test artifact version should read")
            .expect("test artifact version should exist");

        self.repository
            .artifact_table()
            .outcome(&version)
            .expect("test artifact should have a terminal result")
    }

    /// Return the artifact key of one module target artifact.
    pub(crate) fn target_key(
        &self,
        path: &str,
        target: &str,
        key: impl Fn(ModuleId, ProfileId, TargetId) -> ArtifactKey,
    ) -> ArtifactKey {
        let revision = self.revision();
        let module = self.module_id(path, revision);
        let profile = self.profile_id(revision, module, target);
        let package = self
            .repository
            .module(revision, module)
            .expect("test module should load")
            .expect("test module should exist")
            .package_id;

        key(module, profile, TargetId::new(package, target))
    }

    /// Return the artifact key of one module profile artifact.
    pub(crate) fn profile_key(
        &self,
        path: &str,
        target: &str,
        key: impl Fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> ArtifactKey {
        let revision = self.revision();
        let module = self.module_id(path, revision);
        let profile = self.profile_id(revision, module, target);

        key(module, profile)
    }

    /// Return the current retained revision.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.lock().revision()
    }

    /// Return one module id at one revision.
    fn module_id(&self, path: &str, revision: Revision) -> ModuleId {
        self.repository
            .module_id_for_path(revision, &self.root.join(path))
            .expect("test module should resolve")
            .expect("test module should exist")
    }

    /// Return one profile id at one revision.
    fn profile_id(&self, revision: Revision, module: ModuleId, target: &str) -> ProfileId {
        let module = self
            .repository
            .module(revision, module)
            .expect("test module should load")
            .expect("test module should exist");
        let target = TargetId::new(module.package_id, target);
        let profile = self
            .repository
            .profile_for_module_target(revision, module.id, target)
            .expect("test profile should resolve");

        profile.id()
    }

    /// Return the latest detailed trace snapshot.
    fn trace(&self, trace: &Trace) -> TraceSnapshot {
        self.repository
            .snapshot_trace(self.revision(), trace, TraceView::Detailed)
            .unwrap()
    }
}

/// Preserve builtin imports when an unrelated workspace package appears.
#[test]
fn test_reuse_builtin_imports_after_adding_a_package() {
    let session = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "workspace": { "packages": ["packages/*"] }
}
"#,
        ),
        (
            "packages/app/destack.json",
            r#"{
  "name": "app"
}
"#,
        ),
        (
            "packages/app/main.ds",
            r#"export const value = 1;
"#,
        ),
    ])
    .unwrap();
    let revision = session.revision();
    let profile = session.profile_id(
        revision,
        session.module_id("packages/app/main.ds", revision),
        "js",
    );
    let module = session
        .repository
        .module_id_for_uri(
            revision,
            &Uri::from_string("destack://memory/capability.ds"),
        )
        .unwrap()
        .unwrap();
    let key = ArtifactKey::dir_imported(module, profile);
    assert_eq!(session.provide(key), ArtifactOutcome::Ok);
    let before = session.version(key);

    // add an unrelated package without changing any builtin source
    session.edit_text(
        "packages/other/destack.json",
        r#"{
  "name": "other"
}
"#,
    );
    assert_eq!(session.provide(key), ArtifactOutcome::Ok);
    assert_eq!(session.version(key), before);
}

/// Parse conditional files added to an existing module.
#[test]
fn test_parse_added_conditional_file() {
    let session = TestSession::open(&[(
        "main.ds",
        r#"export const value = 1;
"#,
    )])
    .unwrap();
    let module = session.module_id("main.ds", session.revision());
    let key = ArtifactKey::dir_parsed(module);
    assert_eq!(session.provide(key), ArtifactOutcome::Ok);
    let before = session.version(key);

    session.edit_text(
        "main.test.ds",
        r#"export const example = 2;
"#,
    );
    assert_eq!(
        session.module_id("main.test.ds", session.revision()),
        module
    );
    assert_eq!(session.provide(key), ArtifactOutcome::Ok);
    assert_ne!(session.version(key), before);
    let parsed = session.artifacts().read::<DirParsed>(module).unwrap();
    let files = parsed
        .files
        .iter()
        .map(|file| file.file_id)
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        [
            session.repository.file_id(&session.root.join("main.ds")),
            session
                .repository
                .file_id(&session.root.join("main.test.ds")),
        ]
    );
}

/// Reuse retained library imports when a fresh repository opens the same host.
#[test]
fn test_reuse_builtin_imports_across_repositories() {
    let original = TestSession::open(&[(
        "main.ds",
        r#"export const value = 1;
"#,
    )])
    .unwrap();
    let revision = original.revision();
    let module = original
        .repository
        .module_id_for_uri(
            revision,
            &Uri::from_string("destack://memory/capability.ds"),
        )
        .unwrap()
        .unwrap();
    let profile = original.profile_id(revision, original.module_id("main.ds", revision), "js");
    let key = ArtifactKey::dir_imported(module, profile);
    assert_eq!(original.provide(key), ArtifactOutcome::Ok);
    let version = original.version(key);

    // open fresh revision state while retaining the original artifacts
    let reopened = original.reopen();
    assert_eq!(reopened.provide(key), ArtifactOutcome::Ok);
    assert_eq!(reopened.version(key), version);
}

/// Parse authored library sources when they replace retained embedded modules.
#[test]
fn test_replace_embedded_module() {
    let original = TestSession::open(&[]).unwrap();
    let module = original
        .repository
        .module_id_for_uri(
            original.revision(),
            &Uri::from_string("destack://memory/capability.ds"),
        )
        .unwrap()
        .unwrap();
    let key = ArtifactKey::dir_parsed(module);
    assert_eq!(original.provide(key), ArtifactOutcome::Ok);
    let before = original.version(key);

    // reopen the same host with an authored replacement and the retained parse
    for (path, content) in [
        ("README.md", "# Destack\n"),
        (
            "destack.json",
            r#"{ "name": "destack" }
"#,
        ),
        (
            "src/memory/capability.ds",
            r#"export const replacement = 1;
"#,
        ),
    ] {
        original
            .repository
            .file_system()
            .write(&original.root.join(path), content.as_bytes())
            .unwrap();
    }
    let reopened = original.reopen();
    assert_eq!(
        reopened.module_id("src/memory/capability.ds", reopened.revision()),
        module
    );
    assert_eq!(reopened.provide(key), ArtifactOutcome::Ok);
    assert_ne!(reopened.version(key), before);
    let parsed = reopened.artifacts().read::<DirParsed>(module).unwrap();
    let files = parsed
        .files
        .iter()
        .map(|file| file.file_id)
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        [reopened
            .repository
            .file_id(&reopened.root.join("src/memory/capability.ds"))]
    );
}

/// Resolve builtin imports after missing exports and source files become available.
#[test]
fn test_resolve_added_builtin_imports() {
    let session = TestSession::open(&[
        (
            "destack.json",
            r#"{ "workspace": { "packages": ["packages/*"] } }
"#,
        ),
        (
            "packages/library/destack.json",
            r#"{ "name": "destack" }
"#,
        ),
        (
            "packages/library/src/index.ds",
            r#"import { value } from "./value";
"#,
        ),
        (
            "packages/app/destack.json",
            r#"{ "name": "app" }
"#,
        ),
        (
            "packages/app/main.ds",
            r#"import { value } from "destack:example";
"#,
        ),
    ])
    .unwrap();
    let keys = ["packages/library/src/index.ds", "packages/app/main.ds"]
        .map(|path| session.profile_key(path, "js", ArtifactKey::dir_imported));
    let before = keys.map(|key| {
        session.provide(key);
        assert_eq!(session.imports(key), [None]);

        session.version(key)
    });

    session.edit_text(
        "packages/library/src/value.ds",
        r#"export const value = 1;
"#,
    );
    let target = session.module_id("packages/library/src/value.ds", session.revision());
    for (key, expected) in keys.into_iter().zip([Some(target), None]) {
        session.provide(key);
        assert_eq!(session.imports(key), [expected]);
    }

    session.edit_text(
        "packages/library/destack.json",
        r#"{
  "name": "destack",
  "exports": { "./example": { "kind": "module", "path": "src/value.ds" } }
}
"#,
    );
    for (key, before) in keys.into_iter().zip(before) {
        assert_eq!(session.provide(key), ArtifactOutcome::Ok);
        assert_ne!(session.version(key), before);
        assert_eq!(session.imports(key), [Some(target)]);
    }
}

/// Reconsider imports when a declared workspace dependency becomes available.
#[test]
fn test_resolve_added_workspace_dependency() {
    let session = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "workspace": { "packages": ["packages/*"] }
}
"#,
        ),
        (
            "packages/app/destack.json",
            r#"{
  "name": "app",
  "dependencies": { "other": { "source": "workspace" } }
}
"#,
        ),
        (
            "packages/app/main.ds",
            r#"import { value } from "other";
"#,
        ),
    ])
    .unwrap();
    let key = session.profile_key("packages/app/main.ds", "js", ArtifactKey::dir_imported);
    session.provide(key);
    let before = session.version(key);

    session.edit_text(
        "packages/other/destack.json",
        r#"{
  "name": "other",
  "exports": { ".": { "kind": "module", "path": "index.ds" } }
}
"#,
    );
    session.edit_text(
        "packages/other/index.ds",
        r#"export const value = 1;
"#,
    );
    assert_eq!(session.provide(key), ArtifactOutcome::Ok);
    assert_ne!(session.version(key), before);
    assert_eq!(
        session.imports(key),
        [Some(session.module_id(
            "packages/other/index.ds",
            session.revision()
        ))]
    );
}

#[test]
fn test_provide_same_artifact_across_sessions() {
    let files = [
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        ("src/main.ds", "export const answer = 42;\n"),
    ];
    let executor = Executor::new(Execution::Threaded, 2).expect("shared executor should start");
    let first = TestSession::open_with_executor(&files, executor.clone())
        .expect("first session should open");
    let second =
        TestSession::open_with_executor(&files, executor).expect("second session should open");

    // provide identical task coordinates concurrently through distinct repositories
    let first = thread::spawn(move || first.check("src/main.ds", "js"));
    let second = thread::spawn(move || second.check("src/main.ds", "js"));
    let (first_version, _) = first.join().expect("first session should complete");
    let (second_version, _) = second.join().expect("second session should complete");

    assert_eq!(first_version, second_version);
}

/// Lowering stops on a module whose checking reported errors, while indexes still derive.
#[test]
fn test_poison_lowering_of_a_module_with_check_errors() {
    let files = [
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        ("src/main.ds", "export const answer: string = 42;\n"),
    ];
    let session = TestSession::open(&files).expect("test session should open");
    let checked = session.profile_key("src/main.ds", "js", ArtifactKey::dir_checked);
    let lowered = session.target_key("src/main.ds", "js", ArtifactKey::mir_lowered);
    let index = session.profile_key("src/main.ds", "js", |module, profile| {
        ArtifactKey::module_index(module, profile, IndexKind::Symbols)
    });

    // checking completes and owns the error, lowering never runs over it
    assert_eq!(session.provide(checked), ArtifactOutcome::Ok);
    assert_eq!(
        session.provide(lowered),
        ArtifactOutcome::Failed(ArtifactFailure::requirement(checked))
    );

    // query artifacts derive from the same erroneous module
    assert_eq!(session.provide(index), ArtifactOutcome::Ok);
}
