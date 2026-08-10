use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use destack_artifact::{
    ArtifactKey, BuildId, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded,
    DirExported, DirParsed, DirResolved,
};
use destack_core::StringPool;
use destack_repository::{
    ArtifactReader, DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host,
    MemoryBlobStore, Ref, Repository, Revision, Settings,
};
use destack_session::{ArtifactPriority, Executor, Session};
use destack_source::{File, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};
use futures::executor::block_on;

use crate::{ModuleContext, ProgramContext};

/// The minimal package manifest used by checked pattern fixtures.
const TEST_MANIFEST: &str = r#"{
  "name": "test",
  "targets": {
    "default": {
      "include": ["**/*.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// One checked in-memory program used by semantic pattern tests.
pub(crate) struct TestProgram {
    /// The in-memory artifact repository.
    repository: Arc<Repository>,
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
        let source_path = program.root.join("main.ds");
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
                let parsed = artifacts
                    .read::<DirParsed>(*module)
                    .expect("read parsed test DIR");
                let bound = artifacts
                    .read::<DirBound>((*module, *profile))
                    .expect("read bound test DIR");
                let expanded = artifacts
                    .read::<DirExpanded>((*module, *profile))
                    .expect("read expanded test DIR");
                let exported = artifacts
                    .read::<DirExported>((*module, *profile))
                    .expect("read exported test DIR");
                let resolved = artifacts
                    .read::<DirResolved>((*module, *profile))
                    .expect("read resolved test DIR");
                let declared = artifacts
                    .read::<DirDeclared>((*module, *profile))
                    .expect("read declared test DIR");
                let elaborated = artifacts
                    .read::<DirElaborated>((*module, *profile))
                    .expect("read elaborated test DIR");
                let checked = artifacts
                    .read::<DirChecked>((*module, *profile))
                    .expect("read checked test DIR");

                ModuleContext::new(
                    parsed, bound, expanded, exported, resolved, declared, elaborated, checked,
                )
                .expect("build checked test module")
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
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, files)
            .with_blob_store(Arc::new(MemoryBlobStore::new()));
        let repository = Arc::new(Repository::new(
            root.clone(),
            host,
            Settings::default(),
            layout,
        ));
        Self { repository, root }
    }

    /// Write the fixture manifest and sources into one revision.
    fn write(&self, source: &str, dependencies: &[(String, String)]) -> Revision {
        let head = Ref::for_root(&self.root);
        let revision = self
            .repository
            .current(&head)
            .expect("read checked test revision");
        let manifest = self
            .repository
            .put_blob(TEST_MANIFEST.as_bytes())
            .expect("test manifest Blob should store");
        let source = self
            .repository
            .put_blob(source.as_bytes())
            .expect("test source Blob should store");
        let mut edits = vec![
            Edit::set_file("destack.json", manifest),
            Edit::set_file("main.ds", source),
        ];
        for (path, source) in dependencies {
            let blob = self
                .repository
                .put_blob(source.as_bytes())
                .expect("test dependency Blob should store");
            edits.push(Edit::set_file(path, blob));
        }
        let revision = self
            .repository
            .edit(revision, edits)
            .expect("write checked test program")
            .after;
        self.repository
            .set_ref(&head, revision)
            .expect("publish checked test program");

        revision
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
