use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use destack_artifact::{
    ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactVersion, BuildId, IndexKind,
};
use destack_repository as repository;
use destack_repository::{
    DestackLayoutOverride, Environment, Execution, Host, Repository, Revision, RevisionPin,
    Settings, Trace, TraceLevel, TraceSnapshot, TraceView,
};
use destack_source::{FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};
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
        trace
            .snapshot(
                TraceView::Detailed,
                |_| Ok::<_, ()>(None),
                |_| Ok::<_, ()>(None),
            )
            .unwrap()
    }
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
