use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use futures::executor::block_on;
use tspp_artifact::{
    ArtifactKey, BuildId, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded,
    DirExported, DirImported, DirParsed, DirResolved, DirView,
};
use tspp_core::StringPool;
use tspp_repository::{
    ArtifactReader, Edit, Environment, Execution, Host, Repository, Revision, RevisionPin,
    Settings, StorageLayout, StorageLayoutOverride,
};
use tspp_session::{ArtifactPriority, Executor, Session};
use tspp_source::{File, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};

use crate::{ModuleContext, ProgramContext};

/// The minimal package manifest used by checked pattern fixtures.
const TEST_MANIFEST: &str = r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "test",
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// One checked in-memory program used by semantic pattern tests.
pub(crate) struct TestProgram {
    /// The in-memory artifact repository.
    repository: Arc<Repository>,
    /// The empty repository revision.
    revision: RevisionPin,
    /// The workspace root.
    root: PathBuf,
}

impl TestProgram {
    /// Compile one program through checked DIR.
    pub(crate) fn compile(
        source: &str,
        dependencies: &[(String, String)],
    ) -> (Arc<File>, ModuleId, ProgramContext, Arc<StringPool>) {
        let program = Self::new();
        let revision = program.write(source, dependencies);
        let retained_revision = program
            .repository
            .pin(revision)
            .expect("checked test revision should remain live");
        let revision = retained_revision.revision();
        let source_path = program.root.join("main.tspp");
        let paths = std::iter::once(source_path.clone())
            .chain(dependencies.iter().map(|(path, _)| program.root.join(path)))
            .collect::<Vec<_>>();
        let modules = paths
            .iter()
            .map(|path| {
                program
                    .repository
                    .module_id_for_path(revision, path)
                    .expect("resolve checked test module")
                    .expect("checked test module exists")
            })
            .collect::<Vec<_>>();
        let modules = modules
            .iter()
            .map(|module| (*module, program.profile(revision, *module)))
            .collect::<Vec<_>>();
        let revision = program.provide(revision, &modules);
        let artifacts = ArtifactReader::new(&program.repository, revision);
        let strings = program.repository.string_pool().clone();
        let contexts = modules
            .iter()
            .map(|(module, profile)| {
                let key = (*module, *profile);
                let view = DirView::checked(
                    artifacts.read::<DirParsed>(*module).unwrap_or_else(read),
                    artifacts.read::<DirBound>(key).unwrap_or_else(read),
                    artifacts.read::<DirImported>(key).unwrap_or_else(read),
                    artifacts.read::<DirExpanded>(key).unwrap_or_else(read),
                    artifacts.read::<DirResolved>(key).unwrap_or_else(read),
                    artifacts.read::<DirDeclared>(key).unwrap_or_else(read),
                    artifacts.read::<DirElaborated>(key).unwrap_or_else(read),
                    artifacts.read::<DirChecked>(key).unwrap_or_else(read),
                );
                let exported = artifacts.read::<DirExported>(key).unwrap_or_else(read);

                ModuleContext::new(view, exported).expect("build checked test module")
            })
            .collect::<Vec<_>>();
        let context = ProgramContext::new(contexts).expect("build checked test program");
        let module = modules[0].0;
        let file_id = program.repository.file_id(&source_path);
        let file = program
            .repository
            .file(revision, file_id)
            .expect("read checked test source")
            .expect("checked test source exists");

        (file, module, context, strings)
    }

    /// Create one empty in-memory workspace repository.
    fn new() -> Self {
        let root = PathBuf::from("/pattern-test");
        let files = Arc::new(MemoryFileSystem::new());
        files
            .create_dir_all(&root)
            .expect("create checked test workspace");
        let environment = Environment::capture_process();
        let layout = StorageLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &StorageLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, files);
        let (repository, revision) =
            Repository::new(root.clone(), host, Settings::default(), layout);
        let repository = Arc::new(repository);
        let revision = repository
            .pin(revision)
            .expect("empty pattern test revision should remain live");

        Self {
            repository,
            revision,
            root,
        }
    }

    /// Write the fixture manifest and sources into one revision.
    fn write(&self, source: &str, dependencies: &[(String, String)]) -> Revision {
        let revision = self.revision.revision();
        let manifest = self
            .repository
            .retain_blob(TEST_MANIFEST.as_bytes())
            .expect("test manifest Blob should store");
        let source = self
            .repository
            .retain_blob(source.as_bytes())
            .expect("test source Blob should store");
        let mut edits = vec![
            Edit::set_file("package.json", manifest),
            Edit::set_file("main.tspp", source),
        ];
        for (path, source) in dependencies {
            let blob = self
                .repository
                .retain_blob(source.as_bytes())
                .expect("test dependency Blob should store");
            edits.push(Edit::set_file(path, blob));
        }
        self.repository
            .edit(revision, edits)
            .expect("write checked test program")
            .after
    }

    /// Return the built-in default target profile.
    fn profile(&self, revision: Revision, module: ModuleId) -> ProfileId {
        let module = self
            .repository
            .module(revision, module)
            .expect("read checked test module")
            .expect("checked test module exists");
        let target = TargetId::new(module.package_id, "default");
        let profile = self
            .repository
            .profile_for_module_target(revision, module.id, target)
            .expect("resolve checked test profile");

        profile.id()
    }

    /// Provide every artifact consumed by a module context.
    fn provide(&self, revision: Revision, modules: &[(ModuleId, ProfileId)]) -> Revision {
        let keys = modules
            .iter()
            .flat_map(|(module, profile)| {
                [
                    ArtifactKey::dir_parsed(*module),
                    ArtifactKey::dir_bound(*module, *profile),
                    ArtifactKey::dir_imported(*module, *profile),
                    ArtifactKey::dir_expanded(*module, *profile),
                    ArtifactKey::dir_exported(*module, *profile),
                    ArtifactKey::dir_resolved(*module, *profile),
                    ArtifactKey::dir_checked(*module, *profile),
                ]
            })
            .collect::<Vec<_>>();
        let session =
            Session::new(self.repository.clone(), executor()).expect("create checked test session");
        let run = session.provide(revision, &keys, ArtifactPriority::Foreground);
        block_on(run.wait()).expect("provide checked test artifacts");

        revision
    }
}

/// Return the shared session executor for checked pattern tests.
fn executor() -> Arc<Executor> {
    static EXECUTOR: OnceLock<Arc<Executor>> = OnceLock::new();

    EXECUTOR
        .get_or_init(|| {
            Executor::new(Execution::Threaded, Executor::default_worker_count())
                .expect("create checked test artifact executor")
        })
        .clone()
}

/// Fail one test DIR read.
fn read<T>(error: tspp_repository::ProviderError) -> T {
    panic!("read test DIR: {error}")
}
