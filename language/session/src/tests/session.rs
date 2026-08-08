use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactVersion, BuildId, MemoryBlobStore};
use destack_repository as repository;
use destack_repository::{
    DestackLayoutOverride, Environment, Execution, Host, Ref, Repository, Revision, Settings,
    Trace, TraceSnapshot, TraceView, open_repository,
};
use destack_source::{FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};
use futures::executor::block_on;

use crate::{ArtifactPriority, Session, SessionError};

const DEFAULT_ROOT: &str = "/workspace";

/// In-memory session exercise harness.
pub(crate) struct TestSession {
    /// The source root selected by session opening.
    root: PathBuf,
    /// The repository imported by the session.
    repository: Arc<Repository>,
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

    /// Create one test session with explicit executor behavior.
    fn create(
        files: &[(&str, &str)],
        worker_count: usize,
        execution: Execution,
    ) -> Result<Self, SessionError> {
        let root = PathBuf::from(DEFAULT_ROOT);
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("test root directory should be created");

        for (path, content) in files {
            fs.write_string(&root.join(path), content)
                .expect("test file should write");
        }

        let host = Host::new(
            BuildId::test(),
            Environment::default(),
            fs.clone(),
            Arc::new(MemoryBlobStore::new()),
        )
        .with_execution(execution);
        let repository = open_repository(
            root.clone(),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;
        let repository = Arc::new(repository);
        let root = repository.path().to_path_buf();
        let session = Session::new(repository.clone(), worker_count)?;

        Ok(Self {
            root,
            repository,
            session,
        })
    }

    /// Replace one source file in the repository backing this session.
    pub(crate) fn edit_text(&self, path: &str, text: &str) {
        let before = self.revision();
        let edit = repository::Edit::set_text(path, text);
        let after = self
            .repository
            .commit_edits(before, vec![edit])
            .expect("test source edit should commit");
        let after_pin = self
            .repository
            .pin(after)
            .expect("test source revision should remain live");
        let was_published = self
            .repository
            .advance_ref(&self.head(), before, after_pin.revision())
            .expect("test source edit should publish");

        assert!(was_published, "test repository head should remain current");
    }

    /// Check one module target.
    pub(crate) fn check(&self, path: &str, target: &str) -> (ArtifactVersion, TraceSnapshot) {
        let revision = self.revision();
        let module = self.module_id(path, revision);
        let profile = self.profile_id(revision, module, target);
        let key = ArtifactKey::dir_checked(module, profile);
        let trace = self.session.start_trace(true);
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

    /// Return the current head revision.
    pub(crate) fn revision(&self) -> Revision {
        self.repository
            .current(&self.head())
            .expect("test revision should exist")
    }

    /// Return the default session ref.
    fn head(&self) -> Ref {
        Ref::for_root(&self.root)
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
