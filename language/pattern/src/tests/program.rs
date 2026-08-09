use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, BuildId, DirBound, DirChecked, DirDeclared, DirExpanded, DirExported, DirParsed,
    DirResolved, MemoryBlobStore, DirElaborated,
};
use destack_core::StringPool;
use destack_repository::{
    ArtifactReader, DestackLayout, DestackLayoutOverride, Edit, Environment, Host, Ref, Repository,
    Revision, Settings,
};
use destack_session::Session;
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
        let host = Host::new(
            BuildId::test(),
            environment,
            files,
            Arc::new(MemoryBlobStore::new()),
        );
        let repository = Arc::new(Repository::new(
            root.clone(),
            host,
            Settings::default(),
            layout,
        ));
        let head = Ref::for_root(&root);
        let session = Session::new(
            root.clone(),
            root.clone(),
            repository.clone(),
            head,
            1,
            None,
        )
        .expect("create checked test session");
        session
            .reload_from_fs(session.head())
            .expect("materialize checked test workspace");

        Self { repository, root }
    }

    /// Write the fixture manifest and sources into one revision.
    fn write(&self, source: &str, dependencies: &[(String, String)]) -> Revision {
        let head = Ref::for_root(&self.root);
        let revision = self
            .repository
            .current(&head)
            .expect("read checked test revision");
        let mut edits = vec![
            Edit::set_text("destack.json", TEST_MANIFEST),
            Edit::set_text("main.ds", source),
        ];
        edits.extend(
            dependencies
                .iter()
                .map(|(path, source)| Edit::set_text(path, source)),
        );
        let revision = self
            .repository
            .fork_with_edits(revision, edits)
            .expect("write checked test program");
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
        let head = Ref::for_root(&self.root);
        let session = Session::new(
            self.root.clone(),
            self.root.clone(),
            self.repository.clone(),
            head,
            1,
            None,
        )
        .expect("create checked test session");
        block_on(session.provide(revision, &keys)).expect("provide checked test artifacts");

        session
            .revision(session.head())
            .expect("read provided checked test revision")
    }
}
