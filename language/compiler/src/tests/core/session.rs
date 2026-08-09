use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::{env, thread};

use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactTable, ArtifactVersion, BuildId, DirBound, DirChecked,
    DirDeclared, DirElaborated, DirExpanded, DirExported, DirImported, DirParsed, DirResolved,
    EnvironmentBound, MemoryBlobStore, MirLowered, ModuleGraph, NullArtifactStore,
};
use destack_dir as dir;
use destack_mir::{FormatOptions, Formatter};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host, Ref, Repository,
    Revision, Settings, Trace, TraceAggregate, TraceReport, TraceSnapshot, TraceView,
};
use destack_session::{Session, SessionError};
use destack_source::{Content, MemoryFileSystem, ModuleId, ProfileId, TargetId};
use futures::executor::block_on;

use crate::tests::snapshot::{
    DirRows, DirSnapshotBuilder, assert_snapshot, render_diagnostics, render_source_diagnostics,
};

use super::module::{TestModule, parse_module, parsed_dependencies};

const DEFAULT_DESTACK_JSON: &str = r#"{
  "name": "test",
  "compiler": {
    "emitStats": true,
    "emitEvents": true,
    "emitCheckedTypes": true
  }
}"#;
const WORKERS_ENV: &str = "DESTACK_TEST_WORKERS";
const TRACE_ENV: &str = "DESTACK_TEST_TRACE";
const TIMINGS_ENV: &str = "DESTACK_TIMINGS";
const PROFILE_ENV: &str = "DESTACK_PROFILE";
const TRACE_SLOW_ARTIFACTS_ENV: &str = "DESTACK_TEST_TRACE_SLOW_ARTIFACTS";
const TRACE_SLOW_MS_ENV: &str = "DESTACK_TEST_TRACE_SLOW_MS";
const DEFAULT_TRACE_SLOW_ARTIFACTS: usize = 8;
const SLOW_RUN_TRACE_ATTEMPTS: usize = 24;
/// Path of the module anchoring the anonymous workspace package.
const WARM_ANCHOR_PATH: &str = "__warm.ds";

/// A test session builder.
#[derive(Debug, Default)]
pub(crate) struct TestSessionBuilder {
    /// Source files keyed by logical path.
    files: BTreeMap<String, Content>,
    /// Whether to build on a fresh repository without warm bindings.
    is_cold: bool,
}

impl TestSessionBuilder {
    /// Add one source module.
    pub(crate) fn module(mut self, path: &str, source: &str) -> Self {
        self.files.insert(
            path.to_string(),
            Content::Text {
                content: source.to_string(),
            },
        );

        self
    }

    /// Add one data module.
    pub(crate) fn data(mut self, path: &str, source: &str) -> Self {
        self.files.insert(
            path.to_string(),
            Content::Text {
                content: source.to_string(),
            },
        );

        self
    }

    /// Build on a fresh repository so every artifact really executes.
    pub(crate) fn cold(mut self) -> Self {
        self.is_cold = true;

        self
    }

    /// Build the test session.
    pub(crate) fn build(self) -> TestSession {
        TestSession::build(self.files, self.is_cold)
    }
}

/// A compiler test session.
#[derive(Debug)]
pub(crate) struct TestSession {
    /// The repository under test.
    repository: Arc<Repository>,
    /// The immutable test revision.
    revision: Revision,
    /// Production artifact session.
    session: Session,
    /// Modules keyed by logical path.
    modules_by_path: BTreeMap<String, TestModule>,
    /// Module paths keyed by module id.
    module_path_by_id: BTreeMap<ModuleId, String>,
}

#[allow(dead_code)]
impl TestSession {
    /// Create a new test session builder.
    pub(crate) fn builder() -> TestSessionBuilder {
        TestSessionBuilder::default()
    }

    /// Return the repository under test.
    pub(crate) fn repository(&self) -> &Repository {
        &self.repository
    }

    /// Return the immutable test revision.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Build a single-module test session.
    pub(crate) fn single(source: &str) -> Self {
        Self::builder().module("main.ds", source).build()
    }

    /// Build one test session from source files.
    fn build(files: BTreeMap<String, Content>, is_cold: bool) -> Self {
        let (repository, revision) = if is_cold {
            cold_repository_revision()
        } else {
            let (repository, revision) = shared_repository_revision();

            (repository.clone(), *revision)
        };

        // publish sealed test files
        let edits = files
            .iter()
            .map(|(path, content)| Edit::SetFile {
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
        let workers = test_worker_count();
        let root = repository.path().to_path_buf();
        let session = Session::fork(
            root.clone(),
            root,
            repository.clone(),
            next_reference(),
            revision,
            workers,
            None,
        )
        .expect("compiler test session should start");
        let is_tracing = env::var_os(TRACE_ENV).is_some()
            || env::var_os(TIMINGS_ENV).is_some()
            || env::var_os(TRACE_SLOW_MS_ENV).is_some()
            || env::var_os(PROFILE_ENV).is_some();
        session.set_tracing(is_tracing);

        Self {
            repository,
            revision,
            session,
            modules_by_path,
            module_path_by_id,
        }
    }

    /// Provide imported DIR for one module.
    pub(crate) fn provide_dir_imported(&self, path: &str) -> Result<ArtifactVersion, SessionError> {
        self.require_artifact_result(self.dir_imported_key(path))
    }

    /// Provide exported DIR for one module.
    pub(crate) fn provide_dir_exported(&self, path: &str) -> Result<ArtifactVersion, SessionError> {
        self.require_artifact_result(self.dir_exported_key(path))
    }

    /// Provide resolved DIR for one module.
    pub(crate) fn provide_dir_resolved(&self, path: &str) -> Result<ArtifactVersion, SessionError> {
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

    /// Return the declared DIR key for one module.
    pub(crate) fn dir_declared_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_declared(entry.module.id, entry.profile)
    }

    /// Return the checked DIR key for one module.
    pub(crate) fn dir_checked_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_checked(entry.module.id, entry.profile)
    }

    /// Return the lowered MIR key for one module on the native target.
    pub(crate) fn mir_lowered_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);
        let target = TargetId::new(entry.module.package_id, "native");

        ArtifactKey::mir_lowered(entry.module.id, entry.profile, target)
    }

    /// Render diagnostics produced by one artifact key.
    pub(crate) fn diagnostic_snapshot(&self, key: ArtifactKey) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision, Some(key))
            .expect("test diagnostics should be readable");
        render_diagnostics(self.repository.as_ref(), self.revision, &diagnostics)
    }

    /// Render diagnostics produced by a set of artifact keys.
    pub(crate) fn diagnostic_snapshot_for(&self, keys: &[ArtifactKey]) -> String {
        let diagnostics = self
            .repository
            .diagnostics_for_keys(self.revision, keys)
            .expect("test diagnostics should be readable");
        render_diagnostics(self.repository.as_ref(), self.revision, &diagnostics)
    }

    /// Render diagnostics with source annotations.
    pub(crate) fn render_diagnostics(&self, key: Option<ArtifactKey>) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision, key)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(self.repository.as_ref(), self.revision, &diagnostics, false)
    }

    /// Render diagnostics with colored source annotations for a key closure.
    pub(crate) fn render_terminal_diagnostics_for(&self, keys: &[ArtifactKey]) -> String {
        let diagnostics = self
            .repository
            .diagnostics_for_keys(self.revision, keys)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(self.repository.as_ref(), self.revision, &diagnostics, true)
    }

    /// Render diagnostics with colored source annotations.
    pub(crate) fn render_terminal_diagnostics(&self, key: Option<ArtifactKey>) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision, key)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(self.repository.as_ref(), self.revision, &diagnostics, true)
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
            Content::Text { content } => content,
            Content::Binary { .. } => {
                panic!("test artifact sidecar `{name}` should be text")
            }
        }
    }

    /// Assert bound DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_bound(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_bound_key, false);
    }

    /// Assert imported DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_imported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_imported_key, false);
    }

    /// Assert imported DIR rows for multiple modules.
    #[track_caller]
    pub(crate) fn assert_dir_imported_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_many(paths, rows, expected, Self::dir_imported_key, false);
    }

    /// Assert expanded DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_expanded(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_expanded_key, false);
    }

    /// Assert exported DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_exported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_exported_key, false);
    }

    /// Assert resolved DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_resolved(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_resolved_key, false);
    }

    /// Assert checked DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_checked(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir(path, rows, expected, Self::dir_checked_key, true);
    }

    /// Assert lowered MIR for one module.
    #[track_caller]
    pub(crate) fn assert_mir_lowered(&self, path: &str, expected: &str) {
        self.assert_mir(path, expected, Self::mir_lowered_key);
    }

    /// Assert the diagnostics of one module whose lowering fails.
    #[track_caller]
    pub(crate) fn assert_mir_diagnostics(&self, path: &str, expected: &str) {
        let key = self.mir_lowered_key(path);
        let _ = self.require_artifact_result(key);

        assert_snapshot(self.diagnostic_snapshot(key), expected);
    }

    /// Assert the diagnostics of one module's verified MIR.
    #[track_caller]
    pub(crate) fn assert_mir_verified_diagnostics(&self, path: &str, expected: &str) {
        let entry = self.module_entry(path);
        let target = TargetId::new(entry.module.package_id, "native");
        let key = ArtifactKey::mir_verified(entry.module.id, entry.profile, target);
        let _ = self.require_artifact_result(key);

        assert_snapshot(self.diagnostic_snapshot(key), expected);
    }

    /// Return one successfully lowered MIR artifact.
    pub(crate) fn mir_lowered(&self, path: &str) -> Arc<destack_artifact::MirLowered> {
        let key = self.mir_lowered_key(path);
        let version = self
            .require_artifact_result(key)
            .unwrap_or_else(|error| panic!("test MIR artifact failed: {error}"));

        self.artifacts()
            .artifact::<MirLowered>(&version)
            .unwrap_or_else(|| panic!("test MIR artifact should exist"))
    }

    /// Assert one rendered MIR snapshot.
    #[track_caller]
    fn assert_mir(&self, path: &str, expected: &str, artifact_key: fn(&Self, &str) -> ArtifactKey) {
        let key = artifact_key(self, path);
        let mir = self.render_mir_snapshot(key);

        assert_snapshot(mir, expected);
    }

    /// Render one MIR artifact as formatted MIR.
    #[track_caller]
    fn render_mir_snapshot(&self, key: ArtifactKey) -> String {
        // build through the provider, failing loudly with rendered diagnostics
        let version = match self.require_artifact_result(key) {
            Ok(version) => version,
            Err(error) => panic!(
                "test MIR artifact failed: {error}\n{}",
                self.diagnostic_snapshot(key)
            ),
        };
        let Some(lowered) = self.artifacts().artifact::<MirLowered>(&version) else {
            panic!(
                "test MIR artifact should exist\n{}",
                self.diagnostic_snapshot(key)
            )
        };

        // format the MIR tree against the repository names
        let strings = self.repository.string_pool();

        let formatted = Formatter::new(
            &lowered.tree,
            lowered.target,
            strings.as_ref(),
            FormatOptions::default(),
        )
        .format()
        .expect("test MIR should format");

        // append the aggregate layouts under their declared names
        let layouts = Self::render_mir_layouts(
            &lowered.tree,
            lowered.target,
            &lowered.layouts,
            strings.as_ref(),
        );
        let dispatch = Self::render_mir_dispatch(&lowered.dispatch, strings.as_ref());

        let mut formatted = formatted;
        for rows in [&layouts, &dispatch] {
            if !rows.is_empty() {
                formatted.push('\n');
                formatted.push_str(rows);
            }
        }

        formatted
    }

    /// Render the dynamic dispatch rows of one MIR module.
    fn render_mir_dispatch(
        dispatch: &destack_mir::DispatchTable,
        strings: &destack_core::StringPool,
    ) -> String {
        let mut rows = String::new();

        // render one row per dynamic shape
        for shape in &dispatch.dynamic_shapes {
            rows.push_str(&format!(
                "/// @dispatch.shape constraint={}",
                mir_type_name(shape.constraint)
            ));
            for slot in &shape.slots {
                match slot {
                    destack_mir::DynamicSlot::Field { name, .. } => {
                        rows.push_str(&format!(" field={}", strings.get(*name)));
                    }
                    destack_mir::DynamicSlot::Function { name, .. } => {
                        let name = name.map_or("call", |name| strings.get(name));
                        rows.push_str(&format!(" function={name}"));
                    }
                }
            }
            rows.push('\n');
        }

        // render one row per dynamic table
        for table in &dispatch.dynamic_tables {
            rows.push_str(&format!(
                "/// @dispatch.table concrete={} constraint={}",
                mir_type_name(table.concrete),
                mir_type_name(table.constraint)
            ));
            for entry in &table.entries {
                match entry {
                    destack_mir::DynamicEntry::Field { offset } => {
                        rows.push_str(&format!(" field+{offset}"));
                    }
                    destack_mir::DynamicEntry::Function { function } => {
                        rows.push_str(&format!(" function@{}", function.id));
                    }
                    destack_mir::DynamicEntry::Absent => {
                        rows.push_str(" absent");
                    }
                }
            }
            rows.push('\n');
        }

        rows
    }

    /// Render the aggregate layout rows of one MIR module.
    fn render_mir_layouts(
        tree: &destack_mir::Tree,
        target: destack_mir::TargetLayout,
        layouts: &destack_mir::LayoutTable,
        strings: &destack_core::StringPool,
    ) -> String {
        let formatter = Formatter::new(tree, target, strings, FormatOptions::default());

        // order named layouts by their declarations
        let mut named_types = BTreeSet::new();
        let mut owners = Vec::new();
        for (_, declaration) in tree.iter_nodes::<destack_mir::TypeDeclaration>() {
            named_types.insert(declaration.ty);
            let name = formatter
                .format_type(declaration.ty)
                .expect("test MIR type should format");
            owners.push((declaration.ty, name));
        }

        // follow named layouts with anonymous types in node order
        let mut anonymous: Vec<_> = layouts
            .types
            .keys()
            .filter(|ty| !named_types.contains(ty))
            .copied()
            .collect();
        anonymous.sort_by_key(|ty| ty.id);
        owners.extend(
            anonymous
                .into_iter()
                .map(|ty| (ty, format!("type@{}", ty.id))),
        );

        // render one owner and its independently asserted components
        let mut rows = String::new();
        for (ty, name) in owners {
            let Some(id) = layouts.types.get(&ty) else {
                continue;
            };
            let layout = &layouts.entries[id.index()];
            let (size, alignment) = (layout.size, layout.alignment);

            match (&layout.shape, tree.get(ty)) {
                // render one struct row followed by its fields
                (destack_mir::LayoutShape::Struct(shape), destack_mir::Type::Struct { .. }) => {
                    rows.push_str(&format!(
                        "/// @layout.struct name={name} size={size} align={alignment}\n"
                    ));

                    // render fields in declaration order
                    for (index, field) in shape.fields.iter().enumerate() {
                        rows.push_str(&format!("/// @layout.field owner={name} index={index}"));
                        if let Some(field_name) = field.name {
                            let field_name = strings.get(field_name);
                            rows.push_str(&format!(" name={field_name}"));
                        }

                        // append the physical placement
                        rows.push_str(&format!(
                            " offset={} size={} align={}\n",
                            field.offset, field.size, field.alignment
                        ));
                    }
                }
                // render one tuple row followed by its elements
                (destack_mir::LayoutShape::Tuple(shape), destack_mir::Type::Tuple { .. }) => {
                    rows.push_str(&format!(
                        "/// @layout.tuple name={name} size={size} align={alignment}\n"
                    ));

                    // render elements in logical order
                    for (index, element) in shape.elements.iter().enumerate() {
                        rows.push_str(&format!(
                            "/// @layout.element owner={name} index={index} offset={} size={} align={}\n",
                            element.offset, element.size, element.alignment
                        ));
                    }
                }
                // render one variant row followed by its encoding and cases
                (destack_mir::LayoutShape::Variant(shape), destack_mir::Type::Variant { .. }) => {
                    rows.push_str(&format!(
                        "/// @layout.variant name={name} size={size} align={alignment}\n"
                    ));

                    // render the physical discriminant encoding
                    match &shape.encoding {
                        destack_mir::VariantEncoding::Direct { field } => {
                            rows.push_str(&format!(
                                "/// @layout.discriminant owner={name} kind=direct offset={} byte_len={} bit_offset={} bit_len={}\n",
                                field.offset, field.byte_len, field.bit_offset, field.bit_len
                            ));
                        }
                        destack_mir::VariantEncoding::Niche {
                            field,
                            untagged_case,
                            niche_start,
                        } => {
                            rows.push_str(&format!(
                                "/// @layout.discriminant owner={name} kind=niche offset={} byte_len={} bit_offset={} bit_len={} untagged={} niche_start={}\n",
                                field.offset,
                                field.byte_len,
                                field.bit_offset,
                                field.bit_len,
                                untagged_case,
                                niche_start.bits()
                            ));
                        }
                    }

                    // render cases in logical order
                    for (index, case) in shape.cases.iter().enumerate() {
                        rows.push_str(&format!(
                            "/// @layout.case owner={name} index={index} discriminant={} payload_offset={}\n",
                            case.discriminant.bits(),
                            case.payload_offset
                        ));
                    }
                }
                _ => continue,
            }
        }

        rows
    }

    /// Assert checked DIR rows and diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_checked_and_diagnostics(
        &self,
        path: &str,
        rows: DirRows,
        expected_dir: &str,
        expected_diagnostics: &str,
    ) {
        let dir = self.render_dir_snapshots(&[path], rows, true);
        let keys = [self.dir_declared_key(path), self.dir_checked_key(path)];
        let diagnostics = self.diagnostic_snapshot_for(&keys);
        self.print_trace_if_requested(path);

        assert_snapshot(dir, expected_dir);
        assert_snapshot(diagnostics, expected_diagnostics);
    }

    /// Assert resolved DIR rows and diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_resolved_and_diagnostics(
        &self,
        path: &str,
        rows: DirRows,
        expected_dir: &str,
        expected_diagnostics: &str,
    ) {
        let dir = self.render_dir_snapshots(&[path], rows, false);
        let diagnostics = self.diagnostic_snapshot(self.dir_resolved_key(path));
        self.print_trace_if_requested(path);

        assert_snapshot(dir, expected_dir);
        assert_snapshot(diagnostics, expected_diagnostics);
    }

    /// Assert imported DIR diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_imported_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_imported(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_imported_key(path), expected);
    }

    /// Assert exported DIR diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_exported_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_exported(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_exported_key(path), expected);
    }

    /// Assert checked DIR diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_checked_diagnostics(&self, path: &str, expected: &str) {
        self.print_trace_if_requested("diagnostics");

        // stack checked diagnostics over the declared stage's own
        self.require_artifact(self.dir_checked_key(path));
        let keys = [self.dir_declared_key(path), self.dir_checked_key(path)];

        assert_snapshot(self.diagnostic_snapshot_for(&keys), expected);
    }

    /// Assert resolved DIR diagnostics for one module.
    #[track_caller]
    pub(crate) fn assert_dir_resolved_diagnostics(&self, path: &str, expected: &str) {
        self.provide_dir_resolved(path)
            .expect("artifact should be provided with diagnostics");
        self.assert_diagnostics(self.dir_resolved_key(path), expected);
    }

    /// Assert diagnostics for one artifact key.
    #[track_caller]
    pub(crate) fn assert_diagnostics(&self, key: ArtifactKey, expected: &str) {
        self.print_trace_if_requested("diagnostics");

        // diagnostics read stored collections, so the artifact builds first
        self.require_artifact(key);

        assert_snapshot(self.diagnostic_snapshot(key), expected);
    }

    /// Assert checked DIR rows for multiple modules.
    #[track_caller]
    pub(crate) fn assert_dir_checked_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_many(paths, rows, expected, Self::dir_checked_key, true);
    }

    /// Build code module entries.
    fn build_modules(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, Content>,
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
        let resolved = repository
            .profile_for_target(revision, target_id)
            .expect("test module profile should resolve");
        let profile = resolved.id();

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

            repository
                .complete_artifact(
                    revision,
                    key,
                    ArtifactPayload::DirParsed(Arc::new(entry.dir_parsed.clone())),
                    dependencies,
                    Vec::new(),
                    Vec::new(),
                    None,
                )
                .expect("test parsed artifact should publish");
        }
    }

    /// Build stable module paths for snapshot output.
    fn module_path_by_id(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, Content>,
    ) -> BTreeMap<ModuleId, String> {
        let mut paths = files
            .keys()
            .filter_map(|path| {
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
        for module_id in repository
            .builtin_module_ids(revision)
            .expect("builtin test modules should resolve")
        {
            let module = repository
                .module(revision, module_id)
                .expect("builtin test module lookup should work")
                .expect("builtin test module should exist");

            paths.insert(module_id, module.uri.to_string());
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

            for phase in sidecar_phases(metadata_rows) {
                let key = self.phase_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let metadata = self.artifact_text_sidecar(key, "metadata", &labels);
                let rows = sidecar_rows_for_phase(metadata_rows, phase);

                builder.add_metadata(&rows, &metadata);
            }
        }

        if selection.includes_events() {
            let event_rows = selection.event_rows();

            for phase in sidecar_phases(event_rows) {
                let key = self.phase_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let events = self.artifact_text_sidecar(key, "events", &labels);
                let rows = sidecar_rows_for_phase(event_rows, phase);

                builder.add_events(&rows, &events);
            }
        }

        builder.render()
    }

    /// Assert one rendered DIR snapshot.
    #[track_caller]
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
    #[track_caller]
    fn assert_dir_many(
        &self,
        paths: &[&str],
        rows: DirRows,
        expected: &str,
        artifact_key: fn(&Self, &str) -> ArtifactKey,
        is_checked: bool,
    ) {
        let rows = rows.with_environment();
        let dir = self.render_dir_snapshots(paths, rows, is_checked);

        // require artifacts without diagnostics by default
        for path in paths {
            let key = if is_checked {
                self.dir_checked_key(path)
            } else {
                artifact_key(self, path)
            };
            assert_snapshot(self.diagnostic_snapshot(key), "");
        }
        self.print_trace_if_requested(&paths.join(","));

        assert_snapshot(dir, expected);
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

                format!("=== {path} ===\n\n{body}")
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
        let declared = self.dir_declared_module(entry.module.id, entry.profile);
        let elaborated_version =
            self.require_artifact(ArtifactKey::dir_elaborated(entry.module.id, entry.profile));
        let elaborated = self
            .artifacts()
            .artifact::<DirElaborated>(&elaborated_version)
            .expect("test elaborated artifact should exist");
        let checked = self.dir_checked(entry);
        let bindings = checked.binding_table(&bound, &expanded, &declared, &elaborated);
        let foreign_artifacts = self.foreign_artifacts_for(entry, true);
        let foreign_bindings = foreign_artifacts
            .iter()
            .map(|(bound, expanded)| expanded.binding_table(bound))
            .collect::<Vec<_>>();
        let foreign_tables = if selection.uses_type_labels() {
            self.foreign_checked_tables_for(entry)
        } else {
            Vec::new()
        };
        let mut builder = DirSnapshotBuilder::new(
            &entry.source,
            &parsed.tree,
            self.repository.string_pool().as_ref(),
        )
        .with_bindings(&bindings)
        .with_module_paths(&self.module_path_by_id)
        .with_foreign_bindings(foreign_bindings)
        .with_foreign_tables(foreign_tables);

        if selection.includes_expanded() {
            builder.add_expanded(selection, &expanded);
        }

        // load resolved imports when semantic labels need import names
        let resolved = if selection.uses_type_labels() || selection.includes_import() {
            Some(self.dir_resolved(entry))
        } else {
            None
        };

        if let Some(resolved) = &resolved {
            builder.add_global_names(&resolved.imports);
            builder.add_language_items(&resolved.imports);
        }

        let elaborated_version =
            self.require_artifact(ArtifactKey::dir_elaborated(entry.module.id, entry.profile));
        let elaborated = self
            .artifacts()
            .artifact::<DirElaborated>(&elaborated_version)
            .expect("test elaborated artifact should exist");
        builder.add_checked(
            selection,
            &bound,
            &expanded,
            &declared,
            &elaborated,
            &checked,
        );

        if selection.includes_metadata() {
            let metadata_rows = selection.metadata_rows();

            for phase in sidecar_phases(metadata_rows) {
                let key = self.phase_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let metadata = self.artifact_text_sidecar(key, "metadata", &labels);
                let rows = sidecar_rows_for_phase(metadata_rows, phase);

                builder.add_metadata(&rows, &metadata);
            }
        }

        if selection.includes_events() {
            let event_rows = selection.event_rows();

            for phase in sidecar_phases(event_rows) {
                let key = self.phase_artifact_key(path, phase);
                let labels = BTreeMap::from([("phase".to_string(), phase.to_string())]);
                let events = self.artifact_text_sidecar(key, "events", &labels);
                let rows = sidecar_rows_for_phase(event_rows, phase);

                builder.add_events(&rows, &events);
            }
        }

        if let Some(resolved) = &resolved
            && selection.includes_import()
        {
            builder.add_resolved(selection, resolved);
        }

        // lead with the annotated render, the cleaner of the two views
        let annotated = self.annotated_snapshot(path, entry);
        let rows = builder.render();

        format!("=== annotated ===\n{annotated}\n\n=== checked ===\n{rows}")
    }

    /// Return the annotated source render for one checked module.
    fn annotated_snapshot(&self, path: &str, entry: &TestModule) -> String {
        let key = self.dir_checked_key(path);
        let labels = BTreeMap::from([
            ("phase".to_string(), "check".to_string()),
            ("module".to_string(), entry.module.uri.to_string()),
        ]);

        self.artifact_text_sidecar(key, "annotated", &labels)
            .trim_matches('\n')
            .to_string()
    }

    /// Return parsed DIR for one module entry.
    fn dir_parsed(&self, entry: &TestModule) -> Arc<DirParsed> {
        self.dir_parsed_module(entry.module.id)
    }

    /// Return parsed DIR for one module id.
    pub(crate) fn dir_parsed_module(&self, module_id: ModuleId) -> Arc<DirParsed> {
        let key = ArtifactKey::dir_parsed(module_id);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirParsed>(&version)
            .expect("test parsed artifact should exist")
    }

    /// Return bound DIR for one module entry.
    fn dir_bound(&self, entry: &TestModule) -> Arc<DirBound> {
        let key = ArtifactKey::dir_bound(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirBound>(&version)
            .expect("test bound artifact should exist")
    }

    /// Return imported DIR for one module entry.
    fn dir_imported(&self, entry: &TestModule) -> Arc<DirImported> {
        let key = ArtifactKey::dir_imported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirImported>(&version)
            .expect("test imported artifact should exist")
    }

    /// Return expanded DIR for one module entry.
    fn dir_expanded(&self, entry: &TestModule) -> Arc<DirExpanded> {
        let key = ArtifactKey::dir_expanded(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirExpanded>(&version)
            .expect("test expanded artifact should exist")
    }

    /// Return exported DIR for one module entry.
    fn dir_exported(&self, entry: &TestModule) -> Arc<DirExported> {
        let key = ArtifactKey::dir_exported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirExported>(&version)
            .expect("test exported artifact should exist")
    }

    /// Return resolved DIR for one module entry.
    fn dir_resolved(&self, entry: &TestModule) -> Arc<DirResolved> {
        let key = ArtifactKey::dir_resolved(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirResolved>(&version)
            .expect("test resolved artifact should exist")
    }

    /// Return checked DIR for one module entry.
    fn dir_checked(&self, entry: &TestModule) -> Arc<DirChecked> {
        self.dir_checked_module(entry.module.id, entry.profile)
    }

    /// Return checked DIR for one module id.
    pub(crate) fn dir_checked_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Arc<DirChecked> {
        let key = ArtifactKey::dir_checked(module_id, profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirChecked>(&version)
            .expect("test checked module should exist")
    }

    /// Return declared DIR for one module id.
    fn dir_declared_module(&self, module_id: ModuleId, profile: ProfileId) -> Arc<DirDeclared> {
        let key = ArtifactKey::dir_declared(module_id, profile);
        let version = self.require_artifact(key);

        self.artifacts()
            .artifact::<DirDeclared>(&version)
            .expect("test declared module should exist")
    }

    /// Require one artifact through the production session.
    fn require_artifact(&self, key: ArtifactKey) -> ArtifactVersion {
        self.require_artifact_result(key).unwrap_or_else(|error| {
            match self.repository.diagnostics(self.revision, Some(key)) {
                Ok(diagnostics) => panic!(
                    "test artifact {key:?} should be ready: {error}, diagnostics={diagnostics:?}"
                ),
                Err(diagnostic_error) => panic!(
                    "test artifact {key:?} should be ready: {error}; reading diagnostics failed: {diagnostic_error}"
                ),
            }
        })
    }

    /// Require one artifact through the production session.
    pub(crate) fn require_artifact_result(
        &self,
        key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        let Some(threshold) = env::var(TRACE_SLOW_MS_ENV)
            .ok()
            .and_then(|value| value.parse::<u128>().ok())
        else {
            let result = block_on(self.session.require(self.revision, key));
            self.merge_profile();

            return result;
        };
        let started = std::time::Instant::now();
        let result = block_on(self.session.require(self.revision, key));
        self.merge_profile();

        // print the run trace when it exceeds the requested threshold
        if started.elapsed().as_millis() > threshold {
            self.print_trace("slow-run", SLOW_RUN_TRACE_ATTEMPTS);
        }

        result
    }

    /// Require all artifacts through the production session.
    pub(crate) fn require_all(
        &self,
        keys: impl IntoIterator<Item = ArtifactKey>,
    ) -> Result<(), SessionError> {
        let keys = keys.into_iter().collect::<Vec<_>>();

        block_on(self.session.provide(self.revision, &keys))
    }

    /// Require all artifacts while recording one detailed trace.
    pub(crate) fn require_all_traced(
        &self,
        keys: impl IntoIterator<Item = ArtifactKey>,
    ) -> Result<(), SessionError> {
        // record one detailed trace around the whole run
        let keys = keys.into_iter().collect::<Vec<_>>();
        self.session.set_tracing(true);
        let trace = self.session.start_trace();
        let result = block_on(self.session.provide_traced(
            self.revision,
            &keys,
            Arc::clone(&trace),
        ));
        self.session.finish_trace(trace);

        result
    }

    /// Return the detailed artifact trace for this test session.
    pub(crate) fn trace(&self) -> TraceSnapshot {
        let trace = self
            .session
            .last_trace()
            .expect("test session should have one artifact run");
        let label = |key: &ArtifactKey| {
            let module_display = |module| {
                self.repository
                    .module_display(self.revision, module)
                    .ok()
                    .flatten()
            };

            key.module_id().and_then(module_display)
        };

        trace
            .snapshot(
                TraceView::Detailed,
                |key| Ok::<_, ()>(label(key)),
                |_| Ok::<_, ()>(None),
            )
            .unwrap()
    }

    /// Print the detailed artifact trace for this test session.
    pub(crate) fn print_trace(&self, name: &str, slow_attempts: usize) {
        let trace = self.trace();

        TraceReport::new()
            .row(name, trace)
            .color()
            .timelines()
            .span_totals()
            .slow_attempts(slow_attempts)
            .print();
    }

    /// Merge the last run's trace into the shared profile aggregate.
    fn merge_profile(&self) {
        let Some(path) = env::var_os(PROFILE_ENV) else {
            return;
        };
        let Some(trace) = self.session.last_trace() else {
            return;
        };

        static AGGREGATE: OnceLock<std::sync::Mutex<(TraceAggregate, Option<Arc<Trace>>)>> =
            OnceLock::new();
        let aggregate = AGGREGATE.get_or_init(Default::default);
        let mut aggregate = aggregate.lock().expect("profile aggregate should lock");
        let (aggregate, merged) = &mut *aggregate;

        // merge each run trace once, then flush the running table
        if merged
            .as_ref()
            .is_some_and(|last| Arc::ptr_eq(last, &trace))
        {
            return;
        }
        aggregate.merge(&trace);
        *merged = Some(trace);
        if aggregate.runs() % 64 == 0 {
            std::fs::write(&path, aggregate.render()).expect("profile table should write");
        }
    }

    /// Print the artifact trace when the test trace filter matches.
    fn print_trace_if_requested(&self, label: &str) {
        let Ok(filter) = env::var(TRACE_ENV) else {
            return;
        };
        if !trace_filter_matches(&filter, label) {
            return;
        }

        let slow_attempts = env::var(TRACE_SLOW_ARTIFACTS_ENV)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_TRACE_SLOW_ARTIFACTS);
        let name = trace_name(label);

        self.print_trace(&name, slow_attempts);
    }

    /// Return the artifact key that owns one phase sidecar.
    fn phase_artifact_key(&self, path: &str, phase: &str) -> ArtifactKey {
        match phase {
            "bind" => self.dir_bound_key(path),
            "import" => self.dir_imported_key(path),
            "export" => self.dir_exported_key(path),
            "resolve" => self.dir_resolved_key(path),
            "check" => self.dir_checked_key(path),
            _ => panic!("unsupported sidecar phase `{phase}`"),
        }
    }

    /// Return the repository artifact table.
    pub(crate) fn artifacts(&self) -> Arc<ArtifactTable> {
        self.repository.artifact_table().clone()
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

        // include labels for the external modules the entry's passes read
        let builtins = self
            .entry_external_modules(entry)
            .into_iter()
            .filter(|module_id| *module_id != entry.module.id)
            .collect::<Vec<_>>();

        // provide every referenced label artifact in one run, then read directly
        let keys = builtins
            .iter()
            .flat_map(|module_id| {
                [
                    ArtifactKey::dir_bound(*module_id, entry.profile),
                    ArtifactKey::dir_expanded(*module_id, entry.profile),
                ]
            })
            .collect::<Vec<_>>();
        self.require_all(keys)
            .expect("test referenced label artifacts should be ready");
        let reader = self.repository.artifact_reader(self.revision);
        for module_id in builtins {
            let bound = reader
                .read::<DirBound>((module_id, entry.profile))
                .expect("test builtin bound artifact should exist");
            let expanded = reader
                .read::<DirExpanded>((module_id, entry.profile))
                .expect("test builtin expanded artifact should exist");

            artifacts.push((bound, expanded));
        }

        artifacts
    }

    /// Return foreign checked tables needed for semantic labels.
    fn foreign_checked_tables_for(
        &self,
        entry: &TestModule,
    ) -> Vec<(
        Option<dir::GenericTable<'static>>,
        dir::DefinitionTable<'static>,
        dir::TypeTable<'static>,
        dir::StaticTable<'static>,
    )> {
        // stack each foreign module's declared and checked tables
        let externals = self.entry_external_modules(entry);

        externals
            .into_iter()
            .map(|module_id| {
                let bound_version =
                    self.require_artifact(ArtifactKey::dir_bound(module_id, entry.profile));
                let bound = self
                    .artifacts()
                    .artifact::<DirBound>(&bound_version)
                    .expect("test external bound artifact should exist");
                let expanded_version =
                    self.require_artifact(ArtifactKey::dir_expanded(module_id, entry.profile));
                let expanded = self
                    .artifacts()
                    .artifact::<DirExpanded>(&expanded_version)
                    .expect("test external expanded artifact should exist");
                let declared = self.dir_declared_module(module_id, entry.profile);
                let elaborated_version =
                    self.require_artifact(ArtifactKey::dir_elaborated(module_id, entry.profile));
                let elaborated = self
                    .artifacts()
                    .artifact::<DirElaborated>(&elaborated_version)
                    .expect("test external elaborated artifact should exist");
                let checked = self.dir_checked_module(module_id, entry.profile);
                let generics = checked.generic_table(&declared, &elaborated);
                let definitions = checked.definition_table(&elaborated);
                let types = checked.type_table(&bound, &expanded, &declared, &elaborated);
                let statics = checked.static_table(&bound, &expanded, &declared, &elaborated);

                (Some(generics), definitions, types, statics)
            })
            .collect()
    }

    /// Return the external modules the entry's declare and check passes read.
    fn entry_external_modules(&self, entry: &TestModule) -> Vec<ModuleId> {
        let keys = [
            ArtifactKey::dir_declared(entry.module.id, entry.profile),
            ArtifactKey::dir_checked(entry.module.id, entry.profile),
        ];
        let mut modules = destack_core::FxIndexSet::default();
        for key in keys {
            let dependencies = self
                .repository
                .artifact_dependency_keys(self.revision, &key)
                .expect("test artifact dependencies should read");
            for dependency in dependencies {
                if let Some(module) = dependency.module_id()
                    && module != entry.module.id
                {
                    modules.insert(module);
                }
            }
        }

        // include modules named by the bound environment
        let reader = self.repository.artifact_reader(self.revision);
        if let Ok(environment) = reader.read_content::<EnvironmentBound>(entry.profile) {
            let language = environment.language.items_by_symbol.keys().copied();
            let builtins = environment.language.symbols.values().copied();
            for symbol in language.chain(builtins) {
                if symbol.module_id != entry.module.id {
                    modules.insert(symbol.module_id);
                }
            }
            for resolutions in environment.global_resolutions_by_key.values() {
                for target in resolutions
                    .iter()
                    .flat_map(|resolution| resolution.target.iter())
                {
                    let module = match target {
                        dir::ReferenceTarget::Symbol(symbol) => symbol.module_id,
                        dir::ReferenceTarget::Namespace(module) => module,
                    };
                    if module != entry.module.id {
                        modules.insert(module);
                    }
                }
            }
        }

        // include the modules of resolved import and language item symbols
        let resolved = self.dir_resolved(entry);
        for (_, target) in resolved.imports.symbol_targets() {
            if target.module_id != entry.module.id {
                modules.insert(target.module_id);
            }
        }
        for symbol in resolved.imports.language_symbols() {
            if symbol.module_id != entry.module.id {
                modules.insert(symbol.module_id);
            }
        }
        for module in resolved.references.target_modules() {
            if module != entry.module.id {
                modules.insert(module);
            }
        }

        modules.into_iter().collect()
    }

    /// Read the module graph for one profile.
    pub(crate) fn module_graph(&self, profile: ProfileId) -> Arc<ModuleGraph> {
        let version = self.require_artifact(ArtifactKey::module_graph(profile));
        self.artifacts()
            .artifact::<ModuleGraph>(&version)
            .expect("test module graph should exist")
    }

    /// Assert the component graph induced by selected source modules.
    #[track_caller]
    pub(crate) fn assert_module_graph(&self, paths: &[&str], expected: &str) {
        let Some(first) = paths.first() else {
            panic!("component graph snapshot needs at least one module");
        };
        let profile = self.module_entry(first).profile;
        let modules = paths
            .iter()
            .map(|path| {
                let entry = self.module_entry(path);
                assert_eq!(
                    entry.profile, profile,
                    "component graph snapshot modules must share one profile"
                );

                (*path, entry.module.id)
            })
            .collect::<BTreeMap<_, _>>();
        let paths_by_module = modules
            .iter()
            .map(|(path, module)| (*module, *path))
            .collect::<BTreeMap<_, _>>();
        let graph = self.module_graph(profile);

        // render exact selected edge rows
        let mut snapshot = String::new();
        for (path, module) in &modules {
            let edges = graph
                .edges(*module)
                .unwrap_or_else(|| panic!("module graph misses edges for {module:?}"));
            let targets = selected_paths(edges, &paths_by_module);
            snapshot.push_str(&format!("module {path} -> [{}]\n", targets.join(", ")));
        }

        assert_snapshot(snapshot, expected);
    }

    /// Return one module entry by path.
    fn module_entry(&self, path: &str) -> &TestModule {
        self.modules_by_path
            .get(path)
            .unwrap_or_else(|| panic!("missing test module path '{path}'"))
    }
}

/// Return selected paths targeted by one module edge row.
fn selected_paths<'a>(
    modules: Arc<[ModuleId]>,
    paths_by_module: &'a BTreeMap<ModuleId, &'a str>,
) -> Vec<&'a str> {
    let mut paths = modules
        .iter()
        .filter_map(|module| paths_by_module.get(module).copied())
        .collect::<Vec<_>>();
    paths.sort_unstable();

    paths
}

impl Drop for TestSession {
    fn drop(&mut self) {
        // print the artifact timings per test when requested
        let Ok(filter) = env::var(TIMINGS_ENV) else {
            return;
        };
        if !trace_filter_matches(&filter, "") {
            return;
        }

        let slow_attempts = env::var(TRACE_SLOW_ARTIFACTS_ENV)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_TRACE_SLOW_ARTIFACTS);
        let name = current_test_name().unwrap_or_else(|| "session".to_string());

        self.print_trace(&name, slow_attempts);
    }
}

/// Return whether one trace filter selects the current test session.
fn trace_filter_matches(filter: &str, label: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return false;
    }

    if matches!(filter, "1" | "all" | "*") {
        return true;
    }

    let test = current_test_name();

    label.contains(filter) || test.as_deref().is_some_and(|test| test.contains(filter))
}

/// Return the best display name for one traced test session.
fn trace_name(label: &str) -> String {
    if let Some(test) = current_test_name() {
        format!("{test} {label}")
    } else {
        label.to_string()
    }
}

/// Return the current Rust test name when the harness names this thread.
fn current_test_name() -> Option<String> {
    thread::current().name().map(ToOwned::to_owned)
}

/// Return the shared compiler-test repository and default-config revision.
fn shared_repository_revision() -> &'static (Arc<Repository>, Revision) {
    static BASE: OnceLock<(Arc<Repository>, Revision)> = OnceLock::new();

    BASE.get_or_init(|| {
        let (repository, revision) = cold_repository_revision();

        // anchor the anonymous workspace package so its profile exists
        //  while the warmup checks under it
        let revision = repository
            .fork_with_edits(
                revision,
                [Edit::SetFile {
                    logical_path: WARM_ANCHOR_PATH.to_string(),
                    content: Content::Text {
                        content: String::new(),
                    },
                }],
            )
            .expect("warmup anchor should publish");

        // start one warmup session on the shared base
        let root = repository.path().to_path_buf();
        let session = Session::fork(
            root.clone(),
            root,
            repository.clone(),
            next_reference(),
            revision,
            1,
            None,
        )
        .expect("library warmup session should start");

        // resolve the builtin library's own profile and the workspace
        //  profile fixture modules check under
        let package = repository.embedded_builtin();
        let target = TargetId::new(package.package_id(), "default");
        let library_profile = repository
            .profile_for_target(revision, target)
            .expect("builtin library target profile should resolve")
            .id();
        let anchor = repository
            .module_id_for_path(revision, WARM_ANCHOR_PATH.as_ref())
            .expect("warmup anchor module should resolve")
            .expect("warmup anchor module should exist");
        let workspace = repository
            .module(revision, anchor)
            .expect("warmup anchor module should load")
            .expect("warmup anchor module should be tracked")
            .package_id;
        let workspace_profile = repository
            .profile_for_target(revision, TargetId::new(workspace, "default"))
            .expect("workspace target profile should resolve")
            .id();

        // check every builtin module under both profiles once so test
        //  forks inherit warm bindings for whichever world they build
        let keys = package
            .module_ids()
            .flat_map(|module| {
                [
                    ArtifactKey::dir_checked(module, library_profile),
                    ArtifactKey::dir_checked(module, workspace_profile),
                ]
            })
            .collect::<Vec<_>>();
        block_on(session.provide(revision, &keys)).expect("library warmup should check");

        // drop the anchor for the shared base: the removal invalidates
        //  nothing the warm artifacts depend on, so tests inherit them
        //  by ancestry without the anchor in their file tree
        let base = repository
            .fork_with_edits(
                revision,
                [Edit::RemoveFile {
                    logical_path: WARM_ANCHOR_PATH.to_string(),
                }],
            )
            .expect("warmup anchor should retire");

        (repository, base)
    })
}

/// Build one fresh compiler-test repository at the default-config revision.
fn cold_repository_revision() -> (Arc<Repository>, Revision) {
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
    // run providers inline when the test uses one worker
    let execution = match test_worker_count() {
        1 => Execution::Cooperative,
        _ => Execution::Threaded,
    };
    let host = Host::new(
        BuildId::test(),
        environment,
        Arc::new(MemoryFileSystem::new()),
        shared_blob_store(),
    )
    .with_execution(execution);
    let repository = Arc::new(
        Repository::new(root, host, Settings::default(), layout)
            .with_artifact_store(Arc::new(NullArtifactStore::new())),
    );
    let reference = Ref::for_root(repository.path());
    let revision = repository
        .current(&reference)
        .expect("test repository root ref should exist");

    // publish the default compiler-test configuration
    let revision = repository
        .fork_with_edits(
            revision,
            [Edit::AddFile {
                logical_path: "destack.json".to_string(),
                content: Content::Text {
                    content: DEFAULT_DESTACK_JSON.to_string(),
                },
            }],
        )
        .expect("test repository default config should publish");

    (repository, revision)
}

/// Return the configured compiler-test session worker count.
fn test_worker_count() -> usize {
    env::var(WORKERS_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
}

/// Allocate one private compiler test ref.
fn next_reference() -> Ref {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    Ref::new(format!("compiler-test:{id}"))
}

/// Return selected sidecar phases in stable order.
fn sidecar_phases(rows: &[&'static str]) -> Vec<&'static str> {
    rows.iter()
        .map(|row| sidecar_phase(row))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return selected sidecar rows for one phase.
fn sidecar_rows_for_phase(rows: &[&'static str], phase: &'static str) -> Vec<&'static str> {
    rows.iter()
        .copied()
        .filter(|row| sidecar_phase(row) == phase)
        .collect()
}

/// Return the phase prefix for one sidecar row.
fn sidecar_phase(row: &'static str) -> &'static str {
    row.split_once('.')
        .map(|(phase, _)| phase)
        .unwrap_or_else(|| panic!("sidecar row `{row}` must include a phase prefix"))
}

/// Return one rendered MIR type reference.
fn mir_type_name(ty: destack_mir::LocalNodeId<destack_mir::Type>) -> String {
    format!("type@{}", ty.id)
}

/// Return the blob store shared by every test session in this process.
fn shared_blob_store() -> Arc<MemoryBlobStore> {
    static STORE: OnceLock<Arc<MemoryBlobStore>> = OnceLock::new();

    Arc::clone(STORE.get_or_init(|| Arc::new(MemoryBlobStore::new())))
}
