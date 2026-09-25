use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use std::{env, fs, thread};

use futures::executor::block_on;
use tspp_artifact::{
    Artifact, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactTable, ArtifactVersion,
    BuildId, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, DirView, EnvironmentBound, MirElaborated,
    MirLowered, ModuleGraph,
};
use tspp_core::BlobStore;
use tspp_dir as dir;
use tspp_mir::{FormatOptions, Formatter};
use tspp_repository::{
    Edit, Environment, Execution, Host, Repository, Revision, RevisionPin, Settings, StorageLayout,
    StorageLayoutOverride, Trace, TraceAggregate, TraceLevel, TraceReport, TraceSnapshot,
    TraceView,
};
use tspp_session::{ArtifactPriority, Executor, Session, SessionError};
use tspp_source::{MemoryFileSystem, ModuleId, PackageId, ProfileId, TargetId};

use crate::Compiler;
use crate::tests::snapshot::{
    DirRows, DirSnapshotBuilder, assert_snapshot, render_diagnostics, render_source_diagnostics,
};

use super::module::{TestModule, parse_module, parsed_dependencies};

const DEFAULT_MANIFEST: &str = r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "test"
}"#;
const WORKERS_ENV: &str = "TSPP_TEST_WORKERS";
const TRACE_ENV: &str = "TSPP_TEST_TRACE";
const TIMINGS_ENV: &str = "TSPP_TIMINGS";
const PROFILE_ENV: &str = "TSPP_PROFILE";
const TRACE_SLOW_ARTIFACTS_ENV: &str = "TSPP_TEST_TRACE_SLOW_ARTIFACTS";
const TRACE_SLOW_MS_ENV: &str = "TSPP_TEST_TRACE_SLOW_MS";
const DEFAULT_TRACE_SLOW_ARTIFACTS: usize = 8;
const SLOW_RUN_TRACE_ATTEMPTS: usize = 24;
/// Path of the module anchoring the anonymous workspace package.
const WARM_ANCHOR_PATH: &str = "__warm.tspp";

/// A test session builder.
#[derive(Debug, Default)]
pub(crate) struct TestSessionBuilder {
    /// Source files keyed by logical path.
    files: BTreeMap<String, String>,
    /// Whether to build on a fresh repository without warm bindings.
    is_cold: bool,
}

impl TestSessionBuilder {
    /// Add one source module.
    pub(crate) fn module(mut self, path: &str, source: &str) -> Self {
        self.files.insert(path.to_string(), source.to_string());

        self
    }

    /// Add one data module.
    pub(crate) fn data(mut self, path: &str, source: &str) -> Self {
        self.files.insert(path.to_string(), source.to_string());

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

/// The pipeline depth one DIR snapshot renders.
#[derive(Debug, Clone, Copy)]
enum DirStage {
    /// One early stage's own rows.
    Stage,
    /// The checked stack, for modules whose diagnostics stop materialization.
    Checked,
    /// The checked stack with the materialized tail.
    Materialized,
}

/// A compiler test session.
#[derive(Debug)]
pub(crate) struct TestSession {
    /// The repository under test.
    repository: Arc<Repository>,
    /// The immutable test revision.
    revision: RevisionPin,
    /// Production artifact session.
    session: Session,
    /// Compiler used to render checked source snapshots.
    compiler: Compiler,
    /// Whether artifact runs record detailed traces.
    is_tracing: bool,
    /// The latest detailed artifact trace.
    last_trace: Mutex<Option<Arc<Trace>>>,
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
        self.revision.revision()
    }

    /// Build a single-module test session.
    pub(crate) fn single(source: &str) -> Self {
        Self::builder().module("main.tspp", source).build()
    }

    /// Build one test session from source files.
    fn build(files: BTreeMap<String, String>, is_cold: bool) -> Self {
        let (repository, revision) = if is_cold {
            cold_repository_revision()
        } else {
            let (repository, revision) = shared_repository_revision();

            (repository.clone(), revision.revision())
        };

        // commit sealed test files
        let edits = files
            .iter()
            .map(|(path, source)| {
                let blob = repository
                    .retain_blob(source.as_bytes())
                    .expect("test source Blob should store");

                Edit::set_file(path, blob)
            })
            .collect::<Vec<_>>();
        let revision = repository
            .edit(revision, edits)
            .expect("test repository revision should commit")
            .after;
        let revision = repository
            .pin(revision)
            .expect("test repository revision should remain live");
        let revision_id = revision.revision();

        let modules_by_path = Self::build_modules(repository.as_ref(), revision_id, &files);
        let module_path_by_id = Self::module_path_by_id(repository.as_ref(), revision_id, &files);
        Self::seed_parsed_artifacts(repository.as_ref(), revision_id, &modules_by_path);
        let session = Session::new(repository.clone(), test_executor())
            .expect("compiler test session should start");
        let diagnostics = tspp_source::DiagnosticRegistry::new([]);
        let compiler = Compiler::new(repository.clone(), Arc::new(diagnostics));
        let is_tracing = env::var_os(TRACE_ENV).is_some()
            || env::var_os(TIMINGS_ENV).is_some()
            || env::var_os(TRACE_SLOW_MS_ENV).is_some()
            || env::var_os(PROFILE_ENV).is_some();

        Self {
            repository,
            revision,
            session,
            compiler,
            is_tracing,
            last_trace: Mutex::new(None),
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

    /// Return the materialized DIR key for one module.
    pub(crate) fn dir_materialized_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);

        ArtifactKey::dir_materialized(entry.module.id, entry.profile)
    }

    /// Return the lowered MIR key for one module on the native target.
    pub(crate) fn mir_lowered_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);
        let target = TargetId::new(entry.module.package_id, "native");

        ArtifactKey::mir_lowered(entry.module.id, entry.profile, target)
    }

    /// Return the elaborated MIR key for one module on the native target.
    pub(crate) fn mir_elaborated_key(&self, path: &str) -> ArtifactKey {
        let entry = self.module_entry(path);
        let target = TargetId::new(entry.module.package_id, "native");

        ArtifactKey::mir_elaborated(entry.module.id, entry.profile, target)
    }

    /// Render diagnostics produced by one artifact key.
    pub(crate) fn diagnostic_snapshot(&self, key: ArtifactKey) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision(), Some(key))
            .expect("test diagnostics should be readable");
        render_diagnostics(self.repository.as_ref(), self.revision(), &diagnostics)
    }

    /// Render diagnostics produced by a set of artifact keys.
    pub(crate) fn diagnostic_snapshot_for(&self, keys: &[ArtifactKey]) -> String {
        let diagnostics = self
            .repository
            .diagnostics_for_keys(self.revision(), keys)
            .expect("test diagnostics should be readable");
        render_diagnostics(self.repository.as_ref(), self.revision(), &diagnostics)
    }

    /// Render diagnostics with source annotations.
    pub(crate) fn render_diagnostics(&self, key: Option<ArtifactKey>) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision(), key)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(
            self.repository.as_ref(),
            self.revision(),
            &diagnostics,
            false,
        )
    }

    /// Render diagnostics with colored source annotations for a key closure.
    pub(crate) fn render_terminal_diagnostics_for(&self, keys: &[ArtifactKey]) -> String {
        let diagnostics = self
            .repository
            .diagnostics_for_keys(self.revision(), keys)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(
            self.repository.as_ref(),
            self.revision(),
            &diagnostics,
            true,
        )
    }

    /// Render diagnostics with colored source annotations.
    pub(crate) fn render_terminal_diagnostics(&self, key: Option<ArtifactKey>) -> String {
        let diagnostics = self
            .repository
            .diagnostics(self.revision(), key)
            .expect("test diagnostics should be readable");

        render_source_diagnostics(
            self.repository.as_ref(),
            self.revision(),
            &diagnostics,
            true,
        )
    }

    /// Render selected counters recorded while building one artifact.
    pub(crate) fn artifact_counters(&self, key: ArtifactKey, prefix: &str) -> String {
        self.require_all_traced([key])
            .expect("artifact should build under a timed trace");
        let trace = self
            .last_trace
            .lock()
            .expect("test trace should lock")
            .clone()
            .expect("test session should retain the timed trace");
        let attempts = trace.attempts();
        let attempt = attempts
            .iter()
            .rev()
            .find(|attempt| attempt.key == key)
            .unwrap_or_else(|| panic!("timed trace should contain {key:?}"));

        let counters = attempt
            .counters
            .iter()
            .filter(|counter| counter.name.starts_with(prefix))
            .collect::<Vec<_>>();

        counters
            .into_iter()
            .map(|counter| format!("{}={}", counter.name, counter.value))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Assert bound DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_bound(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_stage(path, rows, expected, Self::dir_bound_key);
    }

    /// Assert imported DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_imported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_stage(path, rows, expected, Self::dir_imported_key);
    }

    /// Assert imported DIR rows for multiple modules.
    #[track_caller]
    pub(crate) fn assert_dir_imported_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_stage_many(paths, rows, expected, Self::dir_imported_key);
    }

    /// Assert expanded DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_expanded(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_stage(path, rows, expected, Self::dir_expanded_key);
    }

    /// Assert exported DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_exported(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_stage(path, rows, expected, Self::dir_exported_key);
    }

    /// Assert resolved DIR rows for one module.
    #[track_caller]
    pub(crate) fn assert_dir_resolved(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_stage(path, rows, expected, Self::dir_resolved_key);
    }

    /// Assert DIR rows for one module, through the materialized artifact.
    #[track_caller]
    pub(crate) fn assert_dir(&self, path: &str, rows: DirRows, expected: &str) {
        self.assert_dir_modules(&[path], rows, expected);
    }

    /// Assert lowered MIR for one module.
    #[track_caller]
    pub(crate) fn assert_mir_lowered(&self, path: &str, expected: &str) {
        self.assert_mir(path, expected, Self::mir_lowered_key);
    }

    /// Assert the elaborated MIR snapshot for one module.
    #[track_caller]
    pub(crate) fn assert_mir_elaborated(&self, path: &str, expected: &str) {
        self.assert_mir(path, expected, Self::mir_elaborated_key);
    }

    /// Assert one lowered MIR function by name with the type declarations it mentions.
    #[track_caller]
    pub(crate) fn assert_mir_function(&self, path: &str, name: &str, expected: &str) {
        let key = self.mir_lowered_key(path);
        let mir = self.render_mir_function_snapshot(key, name);

        assert_snapshot(mir, expected);
    }

    /// Assert the elaborated MIR of one function by its formatted reference.
    #[track_caller]
    pub(crate) fn assert_mir_elaborated_function(&self, path: &str, name: &str, expected: &str) {
        let lowered = self.require_mir_lowered(self.mir_lowered_key(path));
        let elaborated: Arc<MirElaborated> = self.require_mir(self.mir_elaborated_key(path));
        let mir = self.render_function_snapshot(
            &elaborated.tree,
            lowered.target,
            &elaborated.layouts,
            self.module_entry(path).module.id,
            name,
        );

        assert_snapshot(mir, expected);
    }

    /// Assert the diagnostics of one module whose lowering fails.
    #[track_caller]
    pub(crate) fn assert_mir_diagnostics(&self, path: &str, expected: &str) {
        let key = self.mir_lowered_key(path);

        self.assert_diagnostics(key, expected);
    }

    /// Assert the diagnostics of one module's verified MIR.
    #[track_caller]
    pub(crate) fn assert_mir_verified_diagnostics(&self, path: &str, expected: &str) {
        let entry = self.module_entry(path);
        let target = TargetId::new(entry.module.package_id, "native");
        let key = ArtifactKey::mir_verified(entry.module.id, entry.profile, target);

        self.assert_diagnostics(key, expected);
    }

    /// Return one successfully lowered MIR artifact.
    pub(crate) fn mir_lowered(&self, path: &str) -> Arc<tspp_artifact::MirLowered> {
        let key = self.mir_lowered_key(path);
        let version = self
            .require_artifact_result(key)
            .unwrap_or_else(|error| panic!("test MIR artifact failed: {error}"));

        self.artifact(version)
    }

    /// Assert one rendered MIR snapshot.
    #[track_caller]
    fn assert_mir(&self, path: &str, expected: &str, artifact_key: fn(&Self, &str) -> ArtifactKey) {
        let key = artifact_key(self, path);
        let mir = self.render_mir_snapshot(key);

        assert_snapshot(mir, expected);
    }

    /// Render one function of a MIR artifact with the type declarations it mentions.
    #[track_caller]
    fn render_mir_function_snapshot(&self, key: ArtifactKey, name: &str) -> String {
        let module = key.module_id().expect("MIR artifact keys name a module");
        let lowered = self.require_mir_lowered(key);

        self.render_function_snapshot(
            &lowered.tree,
            lowered.target,
            &lowered.layouts,
            module,
            name,
        )
    }

    /// Render one function of a MIR tree by its formatted reference.
    fn render_function_snapshot(
        &self,
        tree: &tspp_mir::Tree,
        target: tspp_mir::TargetLayout,
        layouts: &tspp_mir::LayoutTable,
        module: ModuleId,
        name: &str,
    ) -> String {
        let strings = self.repository.string_pool();

        // find the function by its rendered reference, listing the module's on a miss
        let formatter = Formatter::new(tree, target, strings.as_ref(), FormatOptions::default());
        let mut names = Vec::new();
        let function = tree
            .iter_nodes::<tspp_mir::Function>()
            .find_map(|(id, _)| {
                let candidate = formatter
                    .format_function(id)
                    .expect("test MIR function reference should format");
                let found = candidate == name;
                names.push(candidate);
                found.then_some(id)
            })
            .unwrap_or_else(|| panic!("missing MIR function {name}, the module has {names:?}"));

        // format the function and the layouts of the types it mentions
        let types = tspp_mir::mentioned_types(tree, &[function]);
        let formatted = Formatter::new(tree, target, strings.as_ref(), FormatOptions::default())
            .format_functions(&[function], module)
            .expect("test MIR should format");
        let layouts = Self::render_mir_layouts(
            tree,
            target,
            layouts,
            strings.as_ref(),
            Some(function),
            |ty| {
                // an application belongs to its template's module
                let owner = match tree.get(ty) {
                    tspp_mir::Type::Application { base, .. } => *base,
                    _ => ty,
                };
                let is_imported = tree
                    .type_symbol(owner)
                    .is_some_and(|symbol| !symbol.is_defined_in(module));

                types.contains(&ty) && !is_imported
            },
        );

        Self::join_mir_rows(formatted, [&layouts])
    }

    /// Require one lowered MIR artifact, failing loudly with rendered diagnostics.
    #[track_caller]
    fn require_mir_lowered(&self, key: ArtifactKey) -> Arc<MirLowered> {
        self.require_mir(key)
    }

    /// Require one MIR artifact of any stage, panicking with its diagnostics when it fails.
    #[track_caller]
    fn require_mir<A: Artifact>(&self, key: ArtifactKey) -> Arc<A> {
        match self.require_artifact_result(key) {
            Ok(version) => self.artifact(version),
            Err(error) => {
                // follow a failed requirement chain down to the artifact that failed first
                let mut failed = key;
                let mut error = error;
                while let SessionError::ArtifactFailed { failure, .. } = &error
                    && let tspp_artifact::ArtifactFailure::Requirement { key } = failure.as_ref()
                {
                    failed = *key;
                    let Err(cause) = self.require_artifact_result(failed) else {
                        break;
                    };
                    error = cause;
                }

                panic!(
                    "test MIR artifact failed: {error}\n{}",
                    self.diagnostic_snapshot(failed)
                )
            }
        }
    }

    /// Append rendered row blocks under formatted MIR, each separated by one empty line.
    fn join_mir_rows<'a>(
        mut formatted: String,
        rows: impl IntoIterator<Item = &'a String>,
    ) -> String {
        // append each block after one empty line
        for rows in rows {
            if !rows.is_empty() {
                if !formatted.ends_with('\n') {
                    formatted.push('\n');
                }
                formatted.push('\n');
                formatted.push_str(rows);
            }
        }

        formatted
    }

    /// Render one MIR artifact as formatted MIR.
    #[track_caller]
    pub(crate) fn render_mir_snapshot(&self, key: ArtifactKey) -> String {
        let (module, profile, target) = match key {
            ArtifactKey::MirLowered {
                module,
                profile,
                target,
            }
            | ArtifactKey::MirElaborated {
                module,
                profile,
                target,
            } => (module, profile, target),
            other => panic!("test MIR snapshot over a non-MIR key {other:?}"),
        };
        let lowered = self.require_mir_lowered(ArtifactKey::mir_lowered(module, profile, target));

        // read every table from the requested stage
        match key {
            ArtifactKey::MirElaborated { .. } => {
                let elaborated = self.require_mir::<MirElaborated>(key);
                self.render_mir_tree(
                    &elaborated.tree,
                    lowered.target,
                    &elaborated.layouts,
                    &elaborated.dispatch,
                )
            }
            _ => self.render_mir_tree(
                &lowered.tree,
                lowered.target,
                &lowered.layouts,
                &lowered.dispatch,
            ),
        }
    }

    /// Render a MIR tree with its layouts and dispatch tables.
    fn render_mir_tree(
        &self,
        tree: &tspp_mir::Tree,
        target: tspp_mir::TargetLayout,
        layouts: &tspp_mir::LayoutTable,
        dispatch: &tspp_mir::DispatchTable,
    ) -> String {
        let strings = self.repository.string_pool();

        // format the MIR tree
        let formatted = Formatter::new(tree, target, strings.as_ref(), FormatOptions::default())
            .format()
            .expect("test MIR should format");

        // append the aggregate layouts under their declared names
        let layouts =
            Self::render_mir_layouts(tree, target, layouts, strings.as_ref(), None, |_| true);
        let dispatch = Self::render_mir_dispatch(dispatch, strings.as_ref());

        Self::join_mir_rows(formatted, [&layouts, &dispatch])
    }

    /// Render the dynamic dispatch rows of one MIR module.
    fn render_mir_dispatch(
        dispatch: &tspp_mir::DispatchTable,
        strings: &tspp_core::StringPool,
    ) -> String {
        let mut rows = String::new();

        // render one row per dynamic shape
        for shape in dispatch.iter_dynamic_shapes() {
            rows.push_str(&format!(
                "/// @dispatch.shape constraint={}",
                mir_type_name(shape.constraint)
            ));
            for slot in &shape.slots {
                match slot {
                    tspp_mir::DynamicSlot::Field { name, .. } => {
                        rows.push_str(&format!(" field={}", strings.get(*name)));
                    }
                    tspp_mir::DynamicSlot::Function { name, .. } => {
                        let name = name.map_or("call", |name| strings.get(name));
                        rows.push_str(&format!(" function={name}"));
                    }
                }
            }
            rows.push('\n');
        }

        // render one row per dynamic table
        for table in dispatch.iter_dynamic_tables() {
            rows.push_str(&format!(
                "/// @dispatch.table concrete={} constraint={}",
                mir_type_name(table.concrete),
                mir_type_name(table.constraint)
            ));
            for entry in &table.entries {
                match entry {
                    tspp_mir::DynamicEntry::Field { offset } => {
                        rows.push_str(&format!(" field+{offset}"));
                    }
                    tspp_mir::DynamicEntry::Function { function } => {
                        rows.push_str(&format!(" function@{}", function.id));
                    }
                    tspp_mir::DynamicEntry::Absent => {
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
        tree: &tspp_mir::Tree,
        target: tspp_mir::TargetLayout,
        layouts: &tspp_mir::LayoutTable,
        strings: &tspp_core::StringPool,
        function: Option<tspp_mir::FunctionId>,
        keeps: impl Fn(tspp_mir::TypeId) -> bool,
    ) -> String {
        let formatter = Formatter::new(tree, target, strings, FormatOptions::default());
        let format = |ty| match function {
            Some(function) => formatter.format_type_in(function, ty),
            None => formatter.format_type(ty),
        };

        // order named layouts by their declarations
        let mut named_types = BTreeSet::new();
        let mut owners = Vec::new();
        for (_, declaration) in tree.iter_nodes::<tspp_mir::TypeDeclaration>() {
            let ty = tree
                .identified_type(declaration.symbol)
                .expect("declaration has a type");
            named_types.insert(ty);
            if !keeps(ty) {
                continue;
            }
            let name = format(ty).expect("test MIR type should format");
            owners.push((ty, name));
        }

        // follow named layouts with applications and anonymous types in node order
        let mut anonymous: Vec<_> = layouts
            .types()
            .map(|(ty, _)| ty)
            .filter(|ty| !named_types.contains(ty) && keeps(*ty))
            .collect();
        anonymous.sort_by_key(|ty| ty.0);
        owners.extend(anonymous.into_iter().map(|ty| {
            let name = match tree.get(ty) {
                tspp_mir::Type::Application { .. } => {
                    format(ty).expect("test MIR type should format")
                }
                _ => format!("type@{}", ty.0),
            };

            (ty, name)
        }));

        // render one owner and its independently asserted components
        let mut rows = String::new();
        for (ty, name) in owners {
            let Some(layout) = layouts.type_layout(ty) else {
                continue;
            };
            let (size, alignment) = (layout.size, layout.alignment);

            // render an application through the definition it represents
            match (
                &layout.shape,
                tree.get(tspp_mir::Substitution::resolve(ty, tree)),
            ) {
                // render one struct row followed by its fields
                (tspp_mir::LayoutShape::Struct(shape), tspp_mir::Type::Struct { .. }) => {
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
                (tspp_mir::LayoutShape::Tuple(shape), tspp_mir::Type::Tuple { .. }) => {
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
                (tspp_mir::LayoutShape::Variant(shape), tspp_mir::Type::Variant { .. }) => {
                    rows.push_str(&format!(
                        "/// @layout.variant name={name} size={size} align={alignment}\n"
                    ));

                    // render the physical discriminant encoding
                    match &shape.encoding {
                        tspp_mir::VariantEncoding::Direct { field } => {
                            rows.push_str(&format!(
                                "/// @layout.discriminant owner={name} kind=direct offset={} byte_len={} bit_offset={} bit_len={}\n",
                                field.offset, field.byte_len, field.bit_offset, field.bit_len
                            ));
                        }
                        tspp_mir::VariantEncoding::Niche {
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
    pub(crate) fn assert_dir_and_diagnostics(
        &self,
        path: &str,
        rows: DirRows,
        expected_dir: &str,
        expected_diagnostics: &str,
    ) {
        assert!(
            !expected_diagnostics.is_empty(),
            "an empty expectation is written as the blessable placeholder r#\"\\n\"#"
        );
        let dir = self.render_dir_snapshots(&[path], rows, DirStage::Checked);
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
        assert!(
            !expected_diagnostics.is_empty(),
            "an empty expectation is written as the blessable placeholder r#\"\\n\"#"
        );
        let dir = self.render_dir_snapshots(&[path], rows, DirStage::Stage);
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
        assert!(
            !expected.is_empty(),
            "an empty expectation is written as the blessable placeholder r#\"\\n\"#"
        );
        self.print_trace_if_requested("diagnostics");

        // require the selected artifact to complete or report its own diagnostics
        match self.require_artifact_result(key) {
            Ok(_) => {}
            Err(SessionError::ArtifactFailed {
                key: failed,
                failure,
            }) if failed == key && matches!(*failure, ArtifactFailure::Diagnostics) => {}
            Err(error) => panic!("test artifact failed: {error}"),
        }

        assert_snapshot(self.diagnostic_snapshot(key), expected);
    }

    /// Assert DIR rows for multiple modules, through the materialized artifacts.
    #[track_caller]
    pub(crate) fn assert_dir_many(&self, paths: &[&str], rows: DirRows, expected: &str) {
        self.assert_dir_modules(paths, rows, expected);
    }

    /// Assert rendered DIR snapshots through the materialized artifacts.
    #[track_caller]
    fn assert_dir_modules(&self, paths: &[&str], rows: DirRows, expected: &str) {
        let dir = self.render_dir_snapshots(paths, rows, DirStage::Materialized);

        // require every stage without diagnostics
        for path in paths {
            let keys = [
                self.dir_declared_key(path),
                self.dir_checked_key(path),
                self.dir_materialized_key(path),
            ];
            assert_snapshot(self.diagnostic_snapshot_for(&keys), "");
        }
        self.print_trace_if_requested(&paths.join(","));

        assert_snapshot(dir, expected);
    }

    /// Build code module entries.
    fn build_modules(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, String>,
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
                    None,
                )
                .expect("test parsed artifact should publish");
        }
    }

    /// Build stable module paths for snapshot output.
    fn module_path_by_id(
        repository: &Repository,
        revision: Revision,
        files: &BTreeMap<String, String>,
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
    fn render_module_snapshot(&self, entry: &TestModule, selection: DirRows) -> String {
        let parsed = self.dir_parsed(entry);
        let bound = self.dir_bound(entry);
        let bindings = dir::BindingTable::from_segment(Arc::clone(&bound.bindings));
        let foreign_artifacts = self.foreign_artifacts_for(entry, selection.includes_import());
        let foreign_bindings = foreign_artifacts
            .iter()
            .map(|(bound, expanded)| {
                dir::BindingTable::from_segments(vec![
                    Arc::clone(&bound.bindings),
                    Arc::clone(&expanded.bindings),
                ])
            })
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

        builder.render()
    }

    /// Assert rendered stage DIR snapshots for multiple modules.
    #[track_caller]
    fn assert_dir_stage_many(
        &self,
        paths: &[&str],
        rows: DirRows,
        expected: &str,
        artifact_key: fn(&Self, &str) -> ArtifactKey,
    ) {
        let dir = self.render_dir_snapshots(paths, rows, DirStage::Stage);

        // require artifacts without diagnostics
        for path in paths {
            assert_snapshot(self.diagnostic_snapshot(artifact_key(self, path)), "");
        }
        self.print_trace_if_requested(&paths.join(","));

        assert_snapshot(dir, expected);
    }

    /// Assert one rendered stage DIR snapshot.
    #[track_caller]
    fn assert_dir_stage(
        &self,
        path: &str,
        rows: DirRows,
        expected: &str,
        artifact_key: fn(&Self, &str) -> ArtifactKey,
    ) {
        let dir = self.render_dir_snapshots(&[path], rows, DirStage::Stage);

        // require the artifact without diagnostics
        assert_snapshot(self.diagnostic_snapshot(artifact_key(self, path)), "");
        self.print_trace_if_requested(path);

        assert_snapshot(dir, expected);
    }

    /// Render selected DIR snapshots.
    fn render_dir_snapshots(&self, paths: &[&str], rows: DirRows, stage: DirStage) -> String {
        if paths.len() == 1 {
            let entry = self.module_entry(paths[0]);

            return self.render_dir_module_snapshot(paths[0], entry, rows, stage);
        }

        paths
            .iter()
            .map(|path| {
                let entry = self.module_entry(path);
                let body = self.render_dir_module_snapshot(path, entry, rows, stage);

                format!("=== {path} ===\n\n{body}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Render one module snapshot at the requested stage.
    fn render_dir_module_snapshot(
        &self,
        path: &str,
        entry: &TestModule,
        rows: DirRows,
        stage: DirStage,
    ) -> String {
        match stage {
            DirStage::Stage => self.render_module_snapshot(entry, rows),
            DirStage::Checked => self.render_checked_module_snapshot(path, entry, rows, false),
            DirStage::Materialized => self.render_checked_module_snapshot(path, entry, rows, true),
        }
    }

    /// Render one checked module snapshot, layering the materialized tail on request.
    fn render_checked_module_snapshot(
        &self,
        path: &str,
        entry: &TestModule,
        selection: DirRows,
        materialized: bool,
    ) -> String {
        // provide the highest requested stage before reading its component tables
        self.require_artifact(match materialized {
            true => ArtifactKey::dir_materialized(entry.module.id, entry.profile),
            false => ArtifactKey::dir_checked(entry.module.id, entry.profile),
        });
        let parsed = self.dir_parsed(entry);
        let expanded = self.dir_expanded(entry);
        let checked = self.dir_view(entry.module.id, entry.profile, false);
        let materialized =
            materialized.then(|| self.dir_view(entry.module.id, entry.profile, true));
        let bindings = materialized.as_ref().unwrap_or(&checked).bindings();
        let foreign_artifacts = self.foreign_artifacts_for(entry, true);
        let foreign_bindings = foreign_artifacts
            .iter()
            .map(|(bound, expanded)| {
                dir::BindingTable::from_segments(vec![
                    Arc::clone(&bound.bindings),
                    Arc::clone(&expanded.bindings),
                ])
            })
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
        .with_bindings(bindings)
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

        // install the final cumulative table so stage rows dedup against overrides
        if let Some(materialized) = &materialized {
            builder.set_effective_types(materialized.types().clone());
        }
        builder.add_checked(selection, &checked);

        // layer the materialized tail over the checked rows
        if let Some(materialized) = &materialized {
            builder.add_materialized(selection, materialized);
        }

        if let Some(resolved) = &resolved
            && selection.includes_import()
        {
            builder.add_resolved(selection, resolved);
        }

        // lead with the annotated render, the cleaner of the two views
        let annotated = self.annotated_snapshot(path, entry);
        let rows = builder.render();

        format!("=== annotated ===\n{annotated}\n\n=== dir ===\n{rows}")
    }

    /// Return the annotated source render for one checked module.
    fn annotated_snapshot(&self, path: &str, entry: &TestModule) -> String {
        let key = self.dir_checked_key(path);
        self.require_artifact(key);
        self.compiler
            .render_checked_source(self.revision(), key, entry.module.id, entry.profile)
            .expect("checked source should render")
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

        self.artifact(version)
    }

    /// Return bound DIR for one module entry.
    fn dir_bound(&self, entry: &TestModule) -> Arc<DirBound> {
        let key = ArtifactKey::dir_bound(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return imported DIR for one module entry.
    fn dir_imported(&self, entry: &TestModule) -> Arc<DirImported> {
        let key = ArtifactKey::dir_imported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return expanded DIR for one module entry.
    fn dir_expanded(&self, entry: &TestModule) -> Arc<DirExpanded> {
        let key = ArtifactKey::dir_expanded(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return exported DIR for one module entry.
    fn dir_exported(&self, entry: &TestModule) -> Arc<DirExported> {
        let key = ArtifactKey::dir_exported(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return resolved DIR for one module entry.
    fn dir_resolved(&self, entry: &TestModule) -> Arc<DirResolved> {
        let key = ArtifactKey::dir_resolved(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return one module's stages stacked through checked, or through materialized.
    fn dir_view(&self, module_id: ModuleId, profile: ProfileId, materialized: bool) -> DirView {
        let key = (module_id, profile);
        let reader = self.repository.artifact_reader(self.revision());
        DirView::new(
            reader.read::<DirParsed>(module_id).unwrap_or_else(read),
            reader.read::<DirBound>(key).unwrap_or_else(read),
            reader.read::<DirImported>(key).unwrap_or_else(read),
            reader.read::<DirExpanded>(key).unwrap_or_else(read),
            Some(reader.read::<DirResolved>(key).unwrap_or_else(read)),
            Some(reader.read::<DirDeclared>(key).unwrap_or_else(read)),
            Some(reader.read::<DirElaborated>(key).unwrap_or_else(read)),
            Some(reader.read::<DirChecked>(key).unwrap_or_else(read)),
            match materialized {
                true => Some(reader.read::<DirMaterialized>(key).unwrap_or_else(read)),
                false => None,
            },
            None,
        )
    }

    /// Return checked DIR for one module id.
    pub(crate) fn dir_checked_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Arc<DirChecked> {
        let key = ArtifactKey::dir_checked(module_id, profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return elaborated DIR for one module entry.
    fn dir_elaborated(&self, entry: &TestModule) -> Arc<DirElaborated> {
        let key = ArtifactKey::dir_elaborated(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return materialized DIR for one module entry.
    fn dir_materialized(&self, entry: &TestModule) -> Arc<DirMaterialized> {
        let key = ArtifactKey::dir_materialized(entry.module.id, entry.profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Return declared DIR for one module id.
    fn dir_declared_module(&self, module_id: ModuleId, profile: ProfileId) -> Arc<DirDeclared> {
        let key = ArtifactKey::dir_declared(module_id, profile);
        let version = self.require_artifact(key);

        self.artifact(version)
    }

    /// Require one artifact through the production session.
    fn require_artifact(&self, key: ArtifactKey) -> ArtifactVersion {
        self.require_artifact_result(key).unwrap_or_else(|error| {
            match self.repository.diagnostics(self.revision(), Some(key)) {
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
            self.provide(&[key])?;

            return self.artifact_version(key);
        };
        let started = Instant::now();
        let result = self
            .provide(&[key])
            .and_then(|_| self.artifact_version(key));

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

        self.provide(&keys)
    }

    /// Provide artifact roots through one explicitly traced run.
    fn provide(&self, keys: &[ArtifactKey]) -> Result<(), SessionError> {
        let trace_level = if self.is_tracing {
            TraceLevel::Timings
        } else {
            TraceLevel::Disabled
        };
        let trace = self.session.start_trace(trace_level);
        let run = self.session.provide_traced(
            self.revision(),
            keys,
            ArtifactPriority::Foreground,
            trace.clone(),
            None,
        );
        let result = block_on(run.wait());
        trace.finish();

        // retain traces only when one diagnostic mode requested them
        if self.is_tracing {
            self.merge_profile(&trace);
            *self.last_trace.lock().expect("test trace should lock") = Some(trace);
        }

        result
    }

    /// Return one ready artifact version from the test revision.
    fn artifact_version(&self, key: ArtifactKey) -> Result<ArtifactVersion, SessionError> {
        self.repository
            .artifact_version(self.revision(), &key)?
            .ok_or_else(|| SessionError::Internal {
                detail: format!("test artifact has no version after provide: {key:?}"),
            })
    }

    /// Require all artifacts while recording one detailed trace.
    pub(crate) fn require_all_traced(
        &self,
        keys: impl IntoIterator<Item = ArtifactKey>,
    ) -> Result<(), SessionError> {
        // record one detailed trace around the whole run
        let keys = keys.into_iter().collect::<Vec<_>>();
        let trace = self.session.start_trace(TraceLevel::Timings);
        let run = self.session.provide_traced(
            self.revision(),
            &keys,
            ArtifactPriority::Foreground,
            trace.clone(),
            None,
        );
        let result = block_on(run.wait());
        trace.finish();

        self.merge_profile(&trace);
        *self.last_trace.lock().expect("test trace should lock") = Some(trace);

        result
    }

    /// Return the detailed artifact trace for this test session.
    pub(crate) fn trace(&self) -> TraceSnapshot {
        let trace = self
            .last_trace
            .lock()
            .expect("test trace should lock")
            .clone()
            .expect("test session should have one traced artifact run");
        let label = |key: &ArtifactKey| {
            let module_display = |module| {
                self.repository
                    .module_display(self.revision(), module)
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

    /// Merge one run trace into the shared profile aggregate.
    fn merge_profile(&self, trace: &Arc<Trace>) {
        let Some(path) = env::var_os(PROFILE_ENV) else {
            return;
        };

        static AGGREGATE: OnceLock<Mutex<(TraceAggregate, Option<Arc<Trace>>)>> = OnceLock::new();
        let aggregate = AGGREGATE.get_or_init(Default::default);
        let mut aggregate = aggregate.lock().expect("profile aggregate should lock");
        let (aggregate, merged) = &mut *aggregate;

        // merge each run trace once, then flush the running table
        if merged.as_ref().is_some_and(|last| Arc::ptr_eq(last, trace)) {
            return;
        }
        aggregate.merge(trace);
        *merged = Some(trace.clone());
        if aggregate.runs() % 64 == 0 {
            fs::write(&path, aggregate.render()).expect("profile table should write");
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

    /// Return the repository artifact table.
    pub(crate) fn artifacts(&self) -> Arc<ArtifactTable> {
        self.repository.artifact_table().clone()
    }

    /// Return one exact typed test artifact.
    fn artifact<A: Artifact>(&self, version: ArtifactVersion) -> Arc<A> {
        self.artifacts()
            .artifact::<A>(&version)
            .expect("decode test artifact")
            .expect("test artifact should exist")
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
        let reader = self.repository.artifact_reader(self.revision());
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
        // provide foreign modules together before reading their tables
        let externals = self.entry_external_modules(entry);
        self.require_all(
            externals
                .iter()
                .map(|module| ArtifactKey::dir_checked(*module, entry.profile)),
        )
        .expect("foreign checked artifacts should provide");

        externals
            .into_iter()
            .map(|module_id| {
                let checked = self.dir_view(module_id, entry.profile, false);

                (
                    Some(checked.generics().clone()),
                    checked.definitions().clone(),
                    checked.types().clone(),
                    checked.statics().clone(),
                )
            })
            .collect()
    }

    /// Return the external modules the entry's declare and check passes read.
    fn entry_external_modules(&self, entry: &TestModule) -> Vec<ModuleId> {
        let keys = [
            ArtifactKey::dir_declared(entry.module.id, entry.profile),
            ArtifactKey::dir_checked(entry.module.id, entry.profile),
        ];
        let mut modules = tspp_core::FxIndexSet::default();
        for key in keys {
            let dependencies = self
                .repository
                .artifact_dependency_keys(self.revision(), &key)
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
        let reader = self.repository.artifact_reader(self.revision());
        if let Ok(environment) = reader.read::<EnvironmentBound>(entry.profile) {
            let language = environment.language.items_by_symbol.keys().copied();
            for symbol in language {
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

    /// Read the module graph of one package under one profile.
    pub(crate) fn module_graph(&self, package: PackageId, profile: ProfileId) -> Arc<ModuleGraph> {
        let version = self.require_artifact(ArtifactKey::module_graph(package, profile));
        self.artifact(version)
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
        let graph = self.module_graph(self.module_entry(first).module.id.package_id, profile);

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
fn shared_repository_revision() -> &'static (Arc<Repository>, RevisionPin) {
    static BASE: OnceLock<(Arc<Repository>, RevisionPin)> = OnceLock::new();

    BASE.get_or_init(|| {
        let (repository, revision) = cold_repository_revision();

        // store the empty warmup anchor
        let blob = repository
            .retain_blob(b"")
            .expect("warm anchor Blob should store");

        // anchor the anonymous workspace package so its profile exists
        //  while the warmup checks under it
        let revision = repository
            .edit(revision, [Edit::set_file(WARM_ANCHOR_PATH, blob)])
            .expect("warmup anchor should commit")
            .after;

        // start one warmup session on the shared base
        let session = Session::new(repository.clone(), test_executor())
            .expect("library warmup session should start");

        // resolve the workspace profile the fixture modules check under
        let package = repository.embedded_builtin();
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

        // materialize every builtin module under the workspace profile the test forks build in
        let keys = package
            .module_ids()
            .map(|module| ArtifactKey::dir_materialized(module, workspace_profile))
            .collect::<Vec<_>>();
        let run = session.provide(revision, &keys, ArtifactPriority::Foreground);
        block_on(run.wait()).expect("library warmup should check");

        // drop the anchor for the shared base: the removal invalidates
        //  nothing the warm artifacts depend on, so tests inherit them
        //  by ancestry without the anchor in their file tree
        let base = repository
            .edit(
                revision,
                [Edit::RemoveFile {
                    logical_path: WARM_ANCHOR_PATH.to_string(),
                }],
            )
            .expect("warmup anchor should retire")
            .after;
        let base = repository
            .pin(base)
            .expect("shared compiler test revision should remain live");

        (repository, base)
    })
}

/// Build one fresh compiler-test repository at the default-config revision.
fn cold_repository_revision() -> (Arc<Repository>, Revision) {
    let root = PathBuf::new();
    let environment = Environment::default();
    let layout = StorageLayout::resolve(
        &root,
        &root,
        &environment,
        &Settings::default(),
        &StorageLayoutOverride::default(),
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
    )
    .with_blob_store(shared_blob_store())
    .with_execution(execution);
    let (repository, revision) = Repository::new(root, host, Settings::default(), layout);
    let repository = Arc::new(repository);

    // store the default compiler test configuration
    let blob = repository
        .retain_blob(DEFAULT_MANIFEST.as_bytes())
        .expect("test configuration Blob should store");

    // commit the default compiler test configuration
    let revision = repository
        .edit(revision, [Edit::add_file("package.json", blob)])
        .expect("test repository default config should commit")
        .after;
    (repository, revision)
}

/// Return the configured compiler-test session worker count.
fn test_worker_count() -> usize {
    env::var(WORKERS_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
}

/// Return the artifact executor for one compiler test session.
fn test_executor() -> Arc<Executor> {
    let worker_count = test_worker_count();

    // cooperative hosts execute inline, so each session schedules alone
    if worker_count == 1 {
        return Executor::new(Execution::Cooperative, 1)
            .expect("compiler test artifact executor should start");
    }

    static EXECUTOR: OnceLock<Arc<Executor>> = OnceLock::new();

    EXECUTOR
        .get_or_init(|| {
            Executor::new(Execution::Threaded, worker_count)
                .expect("compiler test artifact executor should start")
        })
        .clone()
}

/// Return one rendered MIR type reference.
fn mir_type_name(ty: tspp_mir::TypeId) -> String {
    format!("type@{}", ty.0)
}

/// Return the blob store shared by every test session in this process.
fn shared_blob_store() -> Arc<BlobStore> {
    static STORE: OnceLock<Arc<BlobStore>> = OnceLock::new();

    STORE.get_or_init(|| Arc::new(BlobStore::new())).clone()
}

/// Fail one test DIR read.
fn read<T>(error: tspp_repository::ProviderError) -> T {
    panic!("read test DIR: {error}")
}
