use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactStore, ArtifactVersion, DirBound, DirExported,
    DirImported, DirParsed, MemoryCacheStore,
};
use destack_source::{DiagnosticCollection, FileContent, MemoryFileSystem, ModuleId};
use destack_workspace::{Edit, HostEnvironment, ProviderError, Ref, Repository, Revision};

use crate::tests::snapshot::{DirSnapshotBuilder, DirSnapshotSet};

use super::module::{TestModule, parse_module, parsed_dependency};
use super::provider::TestProvider;

/// A compiler test builder.
#[derive(Debug, Default)]
pub(crate) struct TestCompilerBuilder {
    /// Source files keyed by logical path.
    files: BTreeMap<String, FileContent>,
    /// Global module logical paths.
    globals: Vec<String>,
}

impl TestCompilerBuilder {
    /// Add one source module.
    pub(crate) fn module(mut self, path: &str, source: &str) -> Self {
        self.files.insert(
            path.to_string(),
            FileContent::Text {
                content: source.to_string(),
            },
        );

        self
    }

    /// Add one data module.
    pub(crate) fn data(mut self, path: &str, source: &str) -> Self {
        self.files.insert(
            path.to_string(),
            FileContent::Text {
                content: source.to_string(),
            },
        );

        self
    }

    /// Add one global source module.
    pub(crate) fn global(mut self, path: &str, source: &str) -> Self {
        self.globals.push(path.to_string());
        self.files.insert(
            path.to_string(),
            FileContent::Text {
                content: source.to_string(),
            },
        );

        self
    }

    /// Build the test compiler.
    pub(crate) fn build(self) -> TestCompiler {
        TestCompiler::build(self.files, self.globals)
    }
}

/// A compiler test harness.
#[derive(Debug)]
pub(crate) struct TestCompiler {
    /// The repository under test.
    repository: Arc<Repository>,
    /// The immutable test revision.
    revision: Revision,
    /// Synchronous compiler artifact provider.
    provider: TestProvider,
    /// Modules keyed by logical path.
    modules_by_path: BTreeMap<String, TestModule>,
    /// Module paths keyed by module id.
    module_path_by_id: BTreeMap<ModuleId, String>,
    /// Global module logical paths.
    globals: Vec<String>,
}

impl TestCompiler {
    /// Create a new test compiler builder.
    pub(crate) fn new() -> TestCompilerBuilder {
        TestCompilerBuilder::default()
    }

    /// Build one test compiler from source files.
    fn build(files: BTreeMap<String, FileContent>, globals: Vec<String>) -> Self {
        let repository = Arc::new(Repository::new(
            PathBuf::new(),
            Arc::new(MemoryCacheStore::new()),
            Arc::new(MemoryFileSystem::new()),
            HostEnvironment::default(),
        ));
        let reference = Ref::for_workspace_root(repository.workspace_root());
        let revision = repository
            .current(&reference)
            .expect("test repository root ref should exist");

        // publish sealed test files
        let edits = files
            .iter()
            .map(|(path, content)| Edit::AddFile {
                logical_path: path.clone(),
                content: content.clone(),
            })
            .collect::<Vec<_>>();
        let revision = repository
            .fork_with_edits(revision, edits)
            .expect("test repository revision should publish");

        let modules_by_path = Self::build_modules(repository.as_ref(), revision, &files);
        let module_path_by_id = Self::module_path_by_id(repository.as_ref(), revision, &files);
        Self::seed_parsed_artifacts(repository.as_ref(), revision, &modules_by_path);
        let provider = TestProvider::new(repository.clone(), revision);

        Self {
            repository,
            revision,
            provider,
            modules_by_path,
            module_path_by_id,
            globals,
        }
    }

    /// Provide imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        path: &str,
    ) -> Result<ArtifactVersion, ProviderError> {
        self.require_artifact_result(self.dir_imported_key(path))
    }

    /// Provide exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        path: &str,
    ) -> Result<ArtifactVersion, ProviderError> {
        self.require_artifact_result(self.dir_exported_key(path))
    }

    /// Return the imported DIR key for one module.
    pub(crate) fn dir_imported_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_imported(entry.module.id, entry.profile)
    }

    /// Return the exported DIR key for one module.
    pub(crate) fn dir_exported_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_exported(entry.module.id, entry.profile)
    }

    /// Assert diagnostic codes produced by one module artifact.
    pub(crate) fn assert_diagnostic_codes(&self, key: ArtifactKey, expected: &[&str]) {
        let diagnostics = self
            .repository
            .diagnostics(self.revision, Some(key))
            .expect("test diagnostics should be readable");
        let codes = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert_eq!(codes, expected);
    }

    /// Render one module DIR snapshot.
    pub(crate) fn dir_snapshot(&self, path: &str, selection: DirSnapshotSet) -> String {
        let entry = self.module_entry(path);

        self.render_module_snapshot(entry, selection)
    }

    /// Render selected module DIR snapshots.
    pub(crate) fn dir_snapshots(&self, paths: &[&str], selection: DirSnapshotSet) -> String {
        paths
            .iter()
            .map(|path| {
                let entry = self.module_entry(path);
                let body = self.render_module_snapshot(entry, selection);

                format!("=== {path} ===\n{body}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Render global environment rows.
    pub(crate) fn global_environment_snapshot(&self) -> String {
        let mut lines = Vec::new();
        for path in &self.globals {
            lines.push(format!("/// @global.module path={path}"));
        }
        lines.push(format!(
            "/// @global.summary modules={}",
            self.globals.len()
        ));

        lines.join("\n")
    }

    /// Build code module entries.
    fn build_modules(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, FileContent>,
    ) -> BTreeMap<String, TestModule> {
        let mut entries = BTreeMap::new();

        // collect code modules
        for path in files.keys() {
            if let Some(module) = Self::build_module(repository, revision, path) {
                entries.insert(path.clone(), module);
            }
        }

        entries
    }

    /// Build one code module entry.
    fn build_module(repository: &Repository, revision: Revision, path: &str) -> Option<TestModule> {
        // resolve repository module
        let module_id = repository
            .module_id_for_path(revision, path.as_ref())
            .expect("test module lookup should work")?;
        let module = repository
            .module(revision, module_id)
            .expect("test module lookup should work")?;
        if !module.is_code() {
            return None;
        }

        // parse module source
        let source = repository
            .file(revision, module.file_id)
            .expect("test file lookup should work")
            .expect("test file should exist")
            .text()
            .to_string();
        let dir_parsed = parse_module(module.as_ref(), &source, repository);

        // resolve effective profile
        let profile = repository
            .module_profile(revision, module.id)
            .expect("test module profile should resolve")
            .id();

        Some(TestModule {
            source,
            module,
            profile,
            dir_parsed,
        })
    }

    /// Seed parsed DIR artifacts from the test source text.
    fn seed_parsed_artifacts(
        repository: &Repository,
        revision: Revision,
        entries: &BTreeMap<String, TestModule>,
    ) {
        for entry in entries.values() {
            let dependency = parsed_dependency(repository, revision, entry.module.as_ref());
            let key = ArtifactKey::dir_parsed(entry.module.id);
            let version = ArtifactVersion::new(key, [dependency.clone()]);

            repository
                .complete_artifact(
                    revision,
                    version,
                    ArtifactPayload::DirParsed(entry.dir_parsed.clone()),
                    vec![dependency],
                    DiagnosticCollection::new(),
                )
                .expect("test parsed artifact should publish");
        }
    }

    /// Build stable module paths for snapshot output.
    fn module_path_by_id(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, FileContent>,
    ) -> BTreeMap<ModuleId, String> {
        files
            .iter()
            .filter_map(|(path, _)| {
                let module_id = repository
                    .module_id_for_path(revision, path.as_ref())
                    .expect("test module lookup should work")?;

                Some((module_id, path.clone()))
            })
            .collect()
    }

    /// Render one module snapshot.
    fn render_module_snapshot(&self, entry: &TestModule, selection: DirSnapshotSet) -> String {
        let parsed = self.dir_parsed(entry);
        let bound = self.dir_bound(entry);
        let bindings = bound.binding_table();
        let mut builder = DirSnapshotBuilder::new(
            &entry.source,
            &parsed.tree,
            self.repository.string_pool().as_ref(),
        )
        .with_bindings(&bindings)
        .with_module_paths(&self.module_path_by_id);

        builder.add_bound(selection, &bound);
        if selection.includes_dependency() || selection.includes_export() {
            let imported = self.dir_imported(entry);
            builder.add_imported(selection, &imported);
        }

        if selection.includes_export() {
            let exported = self.dir_exported(entry);
            builder.add_exported(selection, &exported);
        }

        builder.render()
    }

    /// Return parsed DIR for one module entry.
    fn dir_parsed(&self, entry: &TestModule) -> Arc<DirParsed> {
        let key = ArtifactKey::dir_parsed(entry.module.id);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_parsed(&version)
            .expect("test parsed artifact should exist")
    }

    /// Return bound DIR for one module entry.
    fn dir_bound(&self, entry: &TestModule) -> Arc<DirBound> {
        let key = ArtifactKey::dir_bound(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_bound(&version)
            .expect("test bound artifact should exist")
    }

    /// Return imported DIR for one module entry.
    fn dir_imported(&self, entry: &TestModule) -> Arc<DirImported> {
        let key = ArtifactKey::dir_imported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_imported(&version)
            .expect("test imported artifact should exist")
    }

    /// Return exported DIR for one module entry.
    fn dir_exported(&self, entry: &TestModule) -> Arc<DirExported> {
        let key = ArtifactKey::dir_exported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_exported(&version)
            .expect("test exported artifact should exist")
    }

    /// Require one artifact through the test provider.
    fn require_artifact(&self, key: ArtifactKey) -> ArtifactVersion {
        self.require_artifact_result(key).unwrap_or_else(|error| {
            let diagnostics = self
                .repository
                .diagnostics(self.revision, Some(key))
                .unwrap_or_else(|_| DiagnosticCollection::new());

            panic!("test artifact should be ready: {error}, diagnostics={diagnostics:?}")
        })
    }

    /// Require one artifact through the test provider.
    fn require_artifact_result(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        self.provider.require(key)
    }

    /// Return the repository artifact store.
    fn artifacts(&self) -> Arc<ArtifactStore> {
        self.repository.artifact_store().clone()
    }

    /// Return one module entry by path.
    fn module_entry(&self, path: &str) -> &TestModule {
        self.modules_by_path
            .get(path)
            .unwrap_or_else(|| panic!("missing test module path '{path}'"))
    }
}
