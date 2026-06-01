use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactStore, ArtifactVersion, DirBound, DirCheckedModule,
    DirExpanded, DirExported, DirImported, DirParsed, DirResolved, MemoryCacheStore,
};
use destack_source::{
    DiagnosticCollection, DiffOptions, FileContent, MemoryFileSystem, ModuleId, TargetId,
    format_diff,
};
use destack_workspace::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, ProviderError, Ref, Repository,
    Revision, Settings,
};

use crate::tests::snapshot::{DirRows, DirSnapshotBuilder, render_diagnostics};

use super::module::{TestModule, parse_module, parsed_dependencies};
use super::provider::TestProvider;

/// A test session builder.
#[derive(Debug, Default)]
pub(crate) struct TestSessionBuilder {
    /// Source files keyed by logical path.
    files: BTreeMap<String, FileContent>,
}

impl TestSessionBuilder {
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

    /// Build the test session.
    pub(crate) fn build(self) -> TestSession {
        TestSession::build(self.files)
    }
}

/// A compiler test session.
#[derive(Debug)]
pub(crate) struct TestSession {
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
}

#[allow(dead_code)]
impl TestSession {
    /// Create a new test session builder.
    pub(crate) fn new() -> TestSessionBuilder {
        TestSessionBuilder::default()
    }

    /// Build a single-module test session.
    pub(crate) fn single(source: &str) -> Self {
        Self::new().module("main.ds", source).build()
    }

    /// Build one test session from source files.
    fn build(files: BTreeMap<String, FileContent>) -> Self {
        let root = PathBuf::new();
        let environment = Environment::default();
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let repository = Arc::new(Repository::new(
            root,
            Arc::new(MemoryCacheStore::new()),
            Arc::new(MemoryFileSystem::new()),
            environment,
            Settings::default(),
            layout,
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

    /// Provide resolved DIR for one module.
    pub(crate) fn provide_dir_resolved(
        &self,
        path: &str,
    ) -> Result<ArtifactVersion, ProviderError> {
        self.require_artifact_result(self.dir_resolved_key(path))
    }

    /// Return the bound DIR key for one module.
    pub(crate) fn dir_bound_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_bound(entry.module.id, entry.profile)
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

    /// Return the resolved DIR key for one module.
    pub(crate) fn dir_resolved_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_resolved(entry.module.id, entry.profile)
    }

    /// Return the expanded DIR key for one module.
    pub(crate) fn dir_expanded_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_expanded(entry.module.id, entry.profile)
    }

    /// Return the checked DIR key for one module.
    pub(crate) fn dir_checked_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_checked(entry.module.id, entry.profile)
    }

    /// Return the checked component key for one module.
    pub(crate) fn dir_checked_component_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);
        let version = self.require_artifact(self.dir_checked_key(path));
        let checked = self
            .artifacts()
            .dir_checked(&version)
            .expect("test checked facade should exist");

        ArtifactKey::dir_checked_component(checked.entry, checked.component, entry.profile)
    }

    /// Render diagnostics produced by one artifact key.
    pub(crate) fn diagnostic_snapshot(&self, key: ArtifactKey) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision, Some(key))
            .expect("test diagnostics should be readable");

        render_diagnostics(self.repository.as_ref(), self.revision, &diagnostics)
    }

    /// Return one text artifact sidecar.
    pub(crate) fn artifact_text_sidecar(
        &self,
        key: ArtifactKey,
        name: &str,
        labels: &BTreeMap<String, String>,
    ) -> String {
        self.require_artifact(key);

        let sidecar = self
            .repository
            .artifact_sidecar(self.revision, key, name, labels)
            .expect("test artifact sidecar should be readable")
            .unwrap_or_else(|| panic!("test artifact sidecar `{name}` should exist"));

        match sidecar.content {
            FileContent::Text { content } => content,
            FileContent::Binary { .. } => {
                panic!("test artifact sidecar `{name}` should be text")
            }
        }
    }

    /// Assert bound DIR rows for one module.
    pub(crate) fn assert_dir_bound(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_bound_key, false);
    }

    /// Assert imported DIR rows for one module.
    pub(crate) fn assert_dir_imported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_imported_key, false);
    }

    /// Assert imported DIR rows for multiple modules.
    pub(crate) fn assert_dir_imported_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_many(paths, rows, expected, Self::dir_imported_key, false);
    }

    /// Assert expanded DIR rows for one module.
    pub(crate) fn assert_dir_expanded(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_expanded_key, false);
    }

    /// Assert exported DIR rows for one module.
    pub(crate) fn assert_dir_exported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_exported_key, false);
    }

    /// Assert resolved DIR rows for one module.
    pub(crate) fn assert_dir_resolved(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_resolved_key, false);
    }

    /// Assert checked DIR rows for one module.
    pub(crate) fn assert_dir_checked(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_checked_key, true);
    }

    /// Assert checked DIR rows and diagnostics for one module.
    pub(crate) fn assert_dir_checked_and_diagnostics(
        &self,
        path: &str,
        rows: DirRows,
        expected_dir: &str,
        expected_diagnostics: &str,
    ) {
        let dir = self.render_dir_snapshots(&[path], rows, true);
        let diagnostics = self.diagnostic_snapshot(self.dir_checked_component_key(path));

        assert_equal(dir, expected_dir);
        assert_equal(diagnostics, expected_diagnostics);
    }

    /// Assert imported DIR diagnostics for one module.
    pub(crate) fn assert_dir_imported_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_imported(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_imported_key(path), expected);
    }

    /// Assert exported DIR diagnostics for one module.
    pub(crate) fn assert_dir_exported_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_exported(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_exported_key(path), expected);
    }

    /// Assert resolved DIR diagnostics for one module.
    pub(crate) fn assert_dir_resolved_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_resolved(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_resolved_key(path), expected);
    }

    /// Assert diagnostics for one artifact key.
    pub(crate) fn assert_diagnostics(&self, key: ArtifactKey, expected: &str) {
        assert_equal(self.diagnostic_snapshot(key), expected);
    }

    /// Assert checked DIR rows for multiple modules.
    pub(crate) fn assert_dir_checked_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_many(paths, rows, expected, Self::dir_checked_key, true);
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
        let file_id = repository.file_id(path.as_ref());
        if file_id != module.file_id {
            return None;
        }

        // parse module source
        let source = repository
            .file(revision, module.file_id)
            .expect("test file lookup should work")
            .expect("test file should exist")
            .text()
            .to_string();
        let dir_parsed = parse_module(module.as_ref(), repository, revision);

        // resolve explicit test profile
        let target_id = TargetId::new(module.package_id, "default");
        let profile = repository
            .target_profile(revision, target_id)
            .expect("test module profile should resolve")
            .expect("test module profile should exist")
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
            let dependencies = parsed_dependencies(repository, revision, entry.module.as_ref());
            let key = ArtifactKey::dir_parsed(entry.module.id);
            let version = ArtifactVersion::new(key, dependencies.clone());

            repository
                .complete_artifact(
                    revision,
                    version,
                    ArtifactPayload::DirParsed(entry.dir_parsed.clone()),
                    dependencies,
                    DiagnosticCollection::new(),
                    Vec::new(),
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
        let mut paths = files
            .iter()
            .filter_map(|(path, _)| {
                let module_id = repository
                    .module_id_for_path(revision, path.as_ref())
                    .expect("test module lookup should work")?;
                let module = repository
                    .module(revision, module_id)
                    .expect("test module lookup should work")?;
                if repository.file_id(path.as_ref()) != module.file_id {
                    return None;
                }

                Some((module_id, path.clone()))
            })
            .collect::<BTreeMap<_, _>>();

        // include builtin modules referenced by language item types
        let package = repository.builtin_package().package_id();
        for builtin in repository.builtin_package().files() {
            let module_id = builtin.module_id(package);

            paths.insert(module_id, builtin.uri.to_string());
        }

        paths
    }

    /// Render one module snapshot.
    fn render_module_snapshot(&self, path: &str, entry: &TestModule, selection: DirRows) -> String {
        let parsed = self.dir_parsed(entry);
        let bound = self.dir_bound(entry);
        let bindings = bound.binding_table();
        let foreign_artifacts = self.foreign_artifacts_for(entry, selection.includes_import());
        let foreign_bindings = foreign_artifacts
            .iter()
            .map(|(bound, expanded)| expanded.binding_table(bound))
            .collect::<Vec<_>>();
        let mut builder = DirSnapshotBuilder::new(
            &entry.source,
            &parsed.tree,
            self.repository.string_pool().as_ref(),
        )
        .with_bindings(&bindings)
        .with_module_paths(&self.module_path_by_id)
        .with_foreign_bindings(foreign_bindings);

        builder.add_bound(selection, &bound);
        if selection.includes_dependency() || selection.includes_export() {
            let imported = self.dir_imported(entry);
            builder.add_imported(selection, &imported);
        }

        if selection.includes_expanded() {
            let expanded = self.dir_expanded(entry);
            builder.add_expanded(selection, &expanded);
        }

        if selection.includes_export() {
            let exported = self.dir_exported(entry);
            builder.add_exported(selection, &exported);
        }

        if selection.includes_import() {
            let resolved = self.dir_resolved(entry);
            builder.add_resolved(selection, &resolved);
        }

        if selection.includes_metadata() {
            let metadata_rows = selection.metadata_rows();

            for phase in metadata_phases(metadata_rows) {
                let key = self.metadata_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let metadata = self.artifact_text_sidecar(key, "metadata", &labels);
                let rows = metadata_rows_for_phase(metadata_rows, phase);

                builder.add_metadata(&rows, &metadata);
            }
        }

        builder.render()
    }

    /// Assert one rendered DIR snapshot.
    fn assert_dir(
        &self,
        path: &str,
        rows: DirRows,
        expected: &str,
        artifact_key: fn(&Self, &str) -> ArtifactKey,
        is_checked: bool,
    ) {
        self.assert_dir_many(&[path], rows, expected, artifact_key, is_checked);
    }

    /// Assert rendered DIR snapshots.
    fn assert_dir_many(
        &self,
        paths: &[&str],
        rows: DirRows,
        expected: &str,
        artifact_key: fn(&Self, &str) -> ArtifactKey,
        is_checked: bool,
    ) {
        let dir = self.render_dir_snapshots(paths, rows, is_checked);

        // require artifacts without diagnostics by default
        for path in paths {
            let key = if is_checked {
                self.dir_checked_component_key(path)
            } else {
                artifact_key(self, path)
            };
            assert_equal(self.diagnostic_snapshot(key), "");
        }

        assert_equal(dir, expected);
    }

    /// Render selected DIR snapshots.
    fn render_dir_snapshots(&self, paths: &[&str], rows: DirRows, is_checked: bool) -> String {
        if paths.len() == 1 {
            let entry = self.module_entry(paths[0]);
            if is_checked {
                return self.render_checked_module_snapshot(paths[0], entry, rows);
            }

            return self.render_module_snapshot(paths[0], entry, rows);
        }

        paths
            .iter()
            .map(|path| {
                let entry = self.module_entry(path);
                let body = if is_checked {
                    self.render_checked_module_snapshot(path, entry, rows)
                } else {
                    self.render_module_snapshot(path, entry, rows)
                };

                format!("=== {path} ===\n{body}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Render one checked module snapshot.
    fn render_checked_module_snapshot(
        &self,
        path: &str,
        entry: &TestModule,
        selection: DirRows,
    ) -> String {
        let parsed = self.dir_parsed(entry);
        let bound = self.dir_bound(entry);
        let expanded = self.dir_expanded(entry);
        let checked = self.dir_checked(entry);
        let bindings = expanded.binding_table(&bound);
        let foreign_artifacts = self.foreign_artifacts_for(entry, true);
        let foreign_bindings = foreign_artifacts
            .iter()
            .map(|(bound, expanded)| expanded.binding_table(bound))
            .collect::<Vec<_>>();
        let mut builder = DirSnapshotBuilder::new(
            &entry.source,
            &parsed.tree,
            self.repository.string_pool().as_ref(),
        )
        .with_bindings(&bindings)
        .with_module_paths(&self.module_path_by_id)
        .with_foreign_bindings(foreign_bindings);

        if selection.includes_expanded() {
            builder.add_expanded(selection, &expanded);
        }

        // load resolved imports when semantic labels need language items
        let resolved = if selection.uses_type_labels() || selection.includes_import() {
            Some(self.dir_resolved(entry))
        } else {
            None
        };

        if let Some(resolved) = &resolved {
            builder.add_language_items(&resolved.imports);
        }

        builder.add_checked(selection, &bound, &expanded, &checked);

        if selection.includes_metadata() {
            let metadata_rows = selection.metadata_rows();

            for phase in metadata_phases(metadata_rows) {
                let key = self.metadata_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let metadata = self.artifact_text_sidecar(key, "metadata", &labels);
                let rows = metadata_rows_for_phase(metadata_rows, phase);

                builder.add_metadata(&rows, &metadata);
            }
        }

        if let Some(resolved) = &resolved {
            if selection.includes_import() {
                builder.add_resolved(selection, resolved);
            }
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

    /// Return expanded DIR for one module entry.
    fn dir_expanded(&self, entry: &TestModule) -> Arc<DirExpanded> {
        let key = ArtifactKey::dir_expanded(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_expanded(&version)
            .expect("test expanded artifact should exist")
    }

    /// Return exported DIR for one module entry.
    fn dir_exported(&self, entry: &TestModule) -> Arc<DirExported> {
        let key = ArtifactKey::dir_exported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_exported(&version)
            .expect("test exported artifact should exist")
    }

    /// Return resolved DIR for one module entry.
    fn dir_resolved(&self, entry: &TestModule) -> Arc<DirResolved> {
        let key = ArtifactKey::dir_resolved(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .dir_resolved(&version)
            .expect("test resolved artifact should exist")
    }

    /// Return checked DIR for one module entry.
    fn dir_checked(&self, entry: &TestModule) -> Arc<DirCheckedModule> {
        let key = ArtifactKey::dir_checked(entry.module.id, entry.profile);
        let version = self.require_artifact(key);
        let checked = self
            .artifacts()
            .dir_checked(&version)
            .expect("test checked facade should exist");
        let component_key =
            ArtifactKey::dir_checked_component(checked.entry, checked.component, entry.profile);
        let component_version = self.require_artifact(component_key);
        let component = self
            .artifacts()
            .dir_checked_component(&component_version)
            .expect("test checked component should exist");
        let entry = component
            .module(entry.module.id)
            .expect("test checked component should contain module");

        Arc::new(entry.checked.clone())
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

    /// Return the artifact key that owns one phase metadata sidecar.
    fn metadata_artifact_key(&self, path: &str, phase: &str) -> ArtifactKey {
        match phase {
            "bind" => self.dir_bound_key(path),
            "import" => self.dir_imported_key(path),
            "export" => self.dir_exported_key(path),
            "resolve" => self.dir_resolved_key(path),
            "check" => self.dir_checked_component_key(path),
            _ => panic!("unsupported metadata phase `{phase}`"),
        }
    }

    /// Return the repository artifact store.
    fn artifacts(&self) -> Arc<ArtifactStore> {
        self.repository.artifact_store().clone()
    }

    /// Return foreign bound and expanded artifacts needed for labels.
    fn foreign_artifacts_for(
        &self,
        entry: &TestModule,
        include_foreign: bool,
    ) -> Vec<(Arc<DirBound>, Arc<DirExpanded>)> {
        if !include_foreign {
            return Vec::new();
        }

        let mut artifacts = self
            .modules_by_path
            .values()
            .filter(|foreign| foreign.module.id != entry.module.id)
            .map(|foreign| (self.dir_bound(foreign), self.dir_expanded(foreign)))
            .collect::<Vec<_>>();

        // include builtin labels for language item references
        for module_id in self.repository.builtin_package().module_ids() {
            if module_id == entry.module.id {
                continue;
            }

            let key = ArtifactKey::dir_bound(module_id, entry.profile);
            let bound_version = self.require_artifact(key);
            let bound = self
                .artifacts()
                .dir_bound(&bound_version)
                .expect("test builtin bound artifact should exist");
            let key = ArtifactKey::dir_expanded(module_id, entry.profile);
            let expanded_version = self.require_artifact(key);
            let expanded = self
                .artifacts()
                .dir_expanded(&expanded_version)
                .expect("test builtin expanded artifact should exist");

            artifacts.push((bound, expanded));
        }

        artifacts
    }

    /// Return one module entry by path.
    fn module_entry(&self, path: &str) -> &TestModule {
        self.modules_by_path
            .get(path)
            .unwrap_or_else(|| panic!("missing test module path '{path}'"))
    }
}

/// Return selected metadata phases in stable order.
fn metadata_phases(rows: &[&'static str]) -> Vec<&'static str> {
    rows.iter()
        .map(|row| metadata_phase(row))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return selected metadata rows for one phase.
fn metadata_rows_for_phase(rows: &[&'static str], phase: &'static str) -> Vec<&'static str> {
    rows.iter()
        .copied()
        .filter(|row| metadata_phase(row) == phase)
        .collect()
}

/// Return the phase prefix for one metadata row.
fn metadata_phase(row: &'static str) -> &'static str {
    row.split_once('.')
        .map(|(phase, _)| phase)
        .unwrap_or_else(|| panic!("metadata row `{row}` must include a phase prefix"))
}

/// Assert exact multiline text equality.
fn assert_equal(actual: impl AsRef<str>, expected: &str) {
    let actual = actual.as_ref();
    let expected = expected.trim_matches('\n');
    if actual == expected {
        return;
    }
    let diff = format_diff(expected, actual, &DiffOptions::new());

    panic!("snapshot mismatch\n\n{diff}");
}
