#![allow(dead_code, unreachable_pub)]

use std::fmt::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use destack_artifact::{
    ArtifactKey, ArtifactStamp, CacheStore, Data, DirAnalyzed, DirBase, DirDeclared, DirElaborated,
    DirInterface, DirPatched, DirPrepared, DirResolved, DiskCacheStore, EmitFormat,
    ExportedSymbolTable, MemoryCacheStore, ModuleGraph, ModuleOutput, PackageOutput,
};
use destack_ast::NodeParentIndex;
use destack_core::ImmutableStringPool;
use destack_dir::{
    Argument, CaptureKind, CaptureSet, CaptureTable, Declaration, Declarator, Expression,
    FunctionKind, GlobalSymbolId, Key, LocalNodeId, LocalNodeIdAny, LocalScopeId, Pattern,
    ScalarLiteral, StringId, Symbol, SymbolTable, Tree, TypeTable,
};
use destack_engine::Value;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_linter::Linter;
use destack_mir as mir;
use destack_mir::{MirFormatOptions, format_mir};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileContent, FileId, FileSystem,
    FileType, MemoryFileSystem, ModuleId, ModuleVersion, MultiSpan, PackageId, PhysicalFileSystem,
    TargetId, Uri, print_diff,
};
use destack_vm::{
    Allocator, Heap, HeapLimits, HeapOptions, Isolate, IsolateId, IsolateOptions, SharedHeap,
    SharedHeapLimits, StaticSpace, Word,
};
use destack_workspace::{
    AmbientSnapshot, BoundsCheckPolicy, BundleFormat, BundleMode, CacheMode, Change,
    CheckFailurePolicy, DivisionCheckPolicy, Edit, EsTarget, Module, Package, Profile, ProfileId,
    Ref, Repository, Revision, ShiftCheckPolicy, SourceMapMode, Target, TargetDiscovery,
    TargetGeneratedCodeOptions, TargetGeneratedCodePreset,
};
use serde_json::{Value as JsonValue, json};

use crate::{CompilePhase, Compiler, CompilerContext, CompilerOptions, default_workers};

use super::provide_artifacts_to_completion;
use super::tracing::init_tracing;

const DEFAULT_TEST_TIMEOUT_SECONDS: u64 = 10;
static TEST_COMPILE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Get the test timeout from environment variable or use default.
fn test_timeout_seconds() -> u64 {
    let base_timeout = std::env::var("TEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TEST_TIMEOUT_SECONDS);

    // libtest does not reliably export its thread count, so fall back to host parallelism
    let test_threads = configured_test_threads() as u64;

    // scale timeout under heavily parallel cargo test runs
    let timeout_scale = (((test_threads - 1) / 2) + 1).min(6);

    base_timeout * timeout_scale
}

/// Get the effective libtest thread count or a reasonable host fallback.
fn configured_test_threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|parallelism| parallelism.get())
                .unwrap_or(1)
        })
        .max(1)
}

/// Choose a worker count for parallel tests without oversubscribing the host.
fn test_parallel_workers() -> u16 {
    let default_workers = default_workers();
    if default_workers <= 1 {
        return default_workers;
    }

    let test_threads = configured_test_threads()
        .min(u16::MAX as usize)
        .try_into()
        .unwrap_or(u16::MAX)
        .max(1);
    let base = (default_workers / test_threads).max(1);
    let min_workers = 2;
    base.max(min_workers).min(default_workers)
}

/// A test file system.
#[derive(Debug, Clone)]
pub enum TestFileSystem {
    /// An in-memory file system.
    Memory { fs: Arc<MemoryFileSystem> },
    /// A physical file system.
    Physical {
        root_directory: PathBuf,
        fs: Arc<PhysicalFileSystem>,
    },
}

impl TestFileSystem {
    /// Create a new in-memory file system.
    pub fn fs(&self) -> Arc<dyn FileSystem> {
        match self {
            Self::Memory { fs } => fs.clone(),
            Self::Physical { fs, .. } => fs.clone(),
        }
    }

    /// Return a cache backend suited to this file system.
    pub fn cache(&self) -> Arc<dyn CacheStore> {
        match self {
            Self::Memory { .. } => Arc::new(MemoryCacheStore::new()),
            Self::Physical { .. } => Arc::new(DiskCacheStore::new()),
        }
    }
}

/// Resolved decorator info for a function symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoratorInfo {
    /// The resolved decorator target symbol.
    pub target_symbol: GlobalSymbolId,
    /// Whether the first decorator argument is a string literal.
    pub first_argument_is_string_literal: bool,
}

/// One cloned DIR snapshot for test helpers.
#[derive(Debug, Clone)]
pub(crate) struct TestDir {
    /// The cloned tree.
    pub(crate) tree: Tree,
    /// The cloned symbol table.
    pub(crate) symbols: SymbolTable,
    /// The cloned type table.
    pub(crate) types: TypeTable,
    /// The cloned capture table.
    pub(crate) captures: CaptureTable,
    /// The cloned root expressions.
    pub(crate) roots: Vec<LocalNodeId<Expression>>,
    /// The stable fallback node.
    pub(crate) anchor_node: LocalNodeIdAny,
    /// The namespace scope.
    pub(crate) namespace_scope: LocalScopeId,
    /// The cloned exported-symbol table.
    pub(crate) exported_symbols: ExportedSymbolTable,
}

impl TestDir {
    /// Clone one test snapshot from one patched DIR.
    fn from_patched(dir: &DirPatched, exported_symbols: ExportedSymbolTable) -> Self {
        Self {
            tree: dir.tree.as_ref().clone(),
            symbols: dir.symbols.as_ref().clone(),
            types: dir.types.as_ref().clone(),
            captures: dir.captures.as_ref().clone(),
            roots: dir.roots.as_ref().clone(),
            anchor_node: dir.anchor_node,
            namespace_scope: dir.namespace_scope,
            exported_symbols,
        }
    }
}

/// A test wrapper for a repository workspace view.
#[derive(Debug)]
pub struct TestWorkspaceView {
    /// The wrapped repository.
    repository: Arc<Repository>,
    /// The workspace root used for revision reads.
    root_directory: PathBuf,
}

impl TestWorkspaceView {
    /// Create a test repository view over one workspace root.
    pub(crate) fn new(repository: Arc<Repository>, root_directory: PathBuf) -> Self {
        Self {
            repository,
            root_directory,
        }
    }

    /// Return the current workspace revision.
    pub fn current_revision(&self) -> destack_workspace::Revision {
        let reference = Ref::for_workspace_root(&self.root_directory);
        self.repository
            .current(&reference)
            .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"))
    }

    /// Return the underlying repository.
    pub fn repository(&self) -> &Repository {
        self.repository.as_ref()
    }

    /// Return one revision-scoped module snapshot.
    pub fn module_descriptor(&self, module_id: ModuleId) -> Arc<Module> {
        self.repository
            .module(self.current_revision(), module_id)
            .unwrap_or_else(|error| panic!("failed to read module {module_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing module {module_id:?}"))
    }

    /// Return one revision-scoped package snapshot.
    pub fn package_descriptor(&self, package_id: PackageId) -> Arc<Package> {
        self.repository
            .package(self.current_revision(), package_id)
            .unwrap_or_else(|error| panic!("failed to read package {package_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing package {package_id:?}"))
    }

    /// Return one revision-scoped file snapshot.
    pub fn source_file(&self, file_id: FileId) -> Arc<File> {
        self.repository
            .file(self.current_revision(), file_id)
            .unwrap_or_else(|error| panic!("failed to read file {file_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing file {file_id:?}"))
    }

    /// Return every visible module snapshot in the current revision.
    pub fn visible_modules(&self) -> Vec<Arc<Module>> {
        self.repository
            .workspace_module_ids(self.current_revision())
            .unwrap_or_else(|error| panic!("failed to collect workspace modules: {error}"))
            .into_iter()
            .map(|module_id| self.module_descriptor(module_id))
            .collect()
    }

    /// Return the current workspace module count.
    pub fn tracked_module_count(&self) -> usize {
        self.repository
            .workspace_module_ids(self.current_revision())
            .unwrap_or_else(|error| panic!("failed to collect workspace modules: {error}"))
            .len()
    }

    /// Return the workspace root path for this test view.
    pub fn root_directory(&self) -> &PathBuf {
        &self.root_directory
    }

    /// Replace one tracked source file snapshot.
    pub fn replace_source_file(&self, file: File) {
        let reference = Ref::for_workspace_root(&self.root_directory);
        let path = file
            .path
            .as_ref()
            .unwrap_or_else(|| panic!("cannot replace source file without workspace path"));
        let logical_path = self.repository.normalize_workspace_path(path);

        // publish the revision change before updating the indexed file entry
        let change = Edit::SetFile {
            logical_path,
            content: file.content.payload().clone(),
        };
        self.repository
            .apply(&reference, Change::from(change))
            .unwrap_or_else(|error| panic!("failed to publish source file replacement: {error}"));
    }

    /// Refresh one tracked source file from the backing file system.
    pub fn refresh_source_file_from_file_system(&self, file_id: FileId) -> Arc<File> {
        let file = self.source_file(file_id);
        let path = file
            .path
            .clone()
            .unwrap_or_else(|| panic!("cannot refresh source file without workspace path"));

        // prefer text loads and fall back to raw bytes for binary files
        let loaded_file = match self.repository.file_system().read_to_string(&path) {
            Ok(content) => File::from_text(
                file.id,
                file.name.clone(),
                file.uri.clone(),
                Some(path.clone()),
                file.ty,
                content,
            ),
            Err(text_error) => match self.repository.file_system().read(&path) {
                Ok(content) => File::from_binary(
                    file.id,
                    file.name.clone(),
                    file.uri.clone(),
                    Some(path.clone()),
                    file.ty,
                    content,
                ),
                Err(binary_error) => {
                    panic!(
                        "failed to refresh source file from fs at {}: text error: {text_error}; binary error: {binary_error}",
                        path.display()
                    )
                }
            },
        };

        self.replace_source_file(loaded_file);

        self.source_file(file_id)
    }

    /// Return the default profile id for one module in the current revision.
    pub fn default_profile_id_for_module(&self, module_id: ModuleId) -> ProfileId {
        self.repository
            .default_profile_id_for_module(self.current_revision(), module_id)
            .unwrap_or_else(|error| {
                panic!("failed to compute default profile for module {module_id:?}: {error}")
            })
    }

    /// Return the target profile id for one module in the current revision.
    pub fn profile_id_for_target(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<ProfileId> {
        self.repository
            .profile_id_for_target(self.current_revision(), module_id, target_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to compute target profile for module {module_id:?} and target {target_id:?}: {error}"
                )
            })
    }

    /// Return the target profile id or the default profile in the current revision.
    pub fn profile_id_for_target_or_default(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> ProfileId {
        self.repository
            .profile_id_for_target_or_default(
                self.current_revision(),
                module_id,
                target_id,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "failed to compute target-or-default profile for module {module_id:?} and target {target_id:?}: {error}"
                )
            })
    }

    /// Return the module id for one uri in the current revision.
    pub fn module_id_for_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.repository
            .module_id_for_uri(self.current_revision(), uri)
            .unwrap_or_else(|error| panic!("failed to resolve module uri {uri:?}: {error}"))
    }

    /// Return the source file id for one workspace path in the current revision.
    pub fn source_file_id_for_path(&self, path: &Path) -> Option<FileId> {
        let file_id = self.repository.file_id_for_workspace_path(path);
        self.repository
            .file(self.current_revision(), file_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to resolve source file '{}': {error}",
                    path.display()
                )
            })
            .map(|_| file_id)
    }

    /// Return the module id for one workspace path in the current revision.
    pub fn module_id_for_path(&self, path: &Path) -> Option<ModuleId> {
        self.repository
            .module_id_for_path(self.current_revision(), path)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to resolve module path '{}': {error}",
                    path.display()
                )
            })
    }
}

impl Deref for TestWorkspaceView {
    type Target = Repository;

    fn deref(&self) -> &Self::Target {
        self.repository.as_ref()
    }
}

/// A repository-backed compiler test harness.
#[derive(Debug)]
pub struct TestProgram {
    /// The file system.
    pub fs: TestFileSystem,
    /// The repository.
    pub repository: Arc<Repository>,
    /// The repository workspace view.
    pub program: Arc<TestWorkspaceView>,
    /// The compiler.
    pub compiler: Arc<Compiler>,
    /// The linter.
    pub linter: Arc<Linter>,
    /// The latest diagnostics from one test compiler operation.
    latest_diagnostics: Mutex<DiagnosticCollection>,
    /// Pending artifact roots for the next test compile.
    pending_artifact_keys: Mutex<Vec<ArtifactKey>>,
    /// Optional override for the default profile in tests.
    pub default_profile_override: Option<ProfileId>,
}

/// One exact diagnostic expectation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDiagnostic {
    /// The stable diagnostic code.
    pub code: String,
    /// The diagnostic message.
    pub message: String,
}

/// Resolve a root expression id.
pub fn root_expression_id(
    roots: &[LocalNodeId<Expression>],
    _tree: &Tree,
    index: usize,
) -> LocalNodeId<Expression> {
    // select the requested root
    roots
        .get(index)
        .copied()
        .unwrap_or_else(|| panic!("missing root at index {index}"))
}

/// Find a let declarator by binding name.
pub fn expect_let_declarator_by_name(
    roots: &[LocalNodeId<Expression>],
    tree: &Tree,
    name: StringId,
) -> LocalNodeId<Declarator> {
    // scan for a matching let declarator
    for root_id in roots {
        let let_expression_id = match tree.get(*root_id) {
            Expression::Let { .. } => Some(*root_id),
            _ => None,
        };

        // skip non let roots
        let Some(let_expression_id) = let_expression_id else {
            continue;
        };
        let Expression::Let { declarators, .. } = tree.get(let_expression_id) else {
            continue;
        };

        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            let pattern = tree.get(declarator.pattern);
            let Pattern::Binding {
                name: binding_name, ..
            } = pattern
            else {
                continue;
            };

            if *binding_name == name {
                return *declarator_id;
            }
        }
    }

    panic!("expected let declarator");
}

/// A reusable MIR interpreter for repeated calls in tests.
#[derive(Debug)]
pub struct TestIsolate {
    /// The underlying MIR interpreter.
    isolate: Isolate,
    /// The worker static space used by the interpreter.
    statics: StaticSpace,
    /// The authoritative heap for the isolate.
    heap: Heap,
    /// The world-shared heap for the isolate.
    shared: SharedHeap,
    /// The shared collector worker used by the isolate.
    shared_gc: destack_vm::SharedGcWorker,
}

impl TestIsolate {
    /// Run a MIR function by name and return its output value.
    pub fn run_function_by_name(&mut self, function: &str, arguments: &[Value]) -> Value {
        let value = self
            .isolate
            .run_function_by_name(
                &mut self.statics,
                &mut self.heap,
                &self.shared,
                &self.shared_gc,
                function,
                arguments,
            )
            .expect("execution failed");

        materialized_plain_value(&value)
    }
}

/// Return one plain VM value.
pub(crate) fn materialized_plain_value(value: &Value) -> Value {
    value.clone()
}

impl TestProgram {
    /// Return the declared DIR data for one module and profile.
    pub(crate) fn artifact_dir_declared_data(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> DirDeclared {
        self.repository
            .dir_declared(self.artifact_revision(), module_id, profile)
            .map(|dir| dir.as_ref().clone())
            .unwrap_or_else(|| panic!("missing declared dir for module {module_id:?}"))
    }

    /// Clone the latest published DIR forward into one patched artifact.
    fn artifact_dir_exact_maybe(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Option<DirPatched> {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return Some(dir.as_ref().clone());
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return Some(DirPatched::from_elaborated_with(
                dir.as_ref(),
                dir.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            let elaborated = DirElaborated::from_analyzed_with(
                dir.as_ref(),
                dir.tree.as_ref().clone(),
                dir.symbols.as_ref().clone(),
                dir.types.as_ref().clone(),
            );
            return Some(DirPatched::from_elaborated_with(
                &elaborated,
                elaborated.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            let declared = self
                .repository
                .dir_declared(self.artifact_revision(), module_id, profile)
                .unwrap_or_else(|| panic!("missing declared dir for module {module_id:?}"));
            let analyzed = DirAnalyzed::from_interface_and_declared_with(
                dir.as_ref(),
                declared.as_ref(),
                dir.types.as_ref().clone(),
                declared.captures.as_ref().clone(),
            );
            let elaborated = DirElaborated::from_analyzed_with(
                &analyzed,
                analyzed.tree.as_ref().clone(),
                analyzed.symbols.as_ref().clone(),
                analyzed.types.as_ref().clone(),
            );
            return Some(DirPatched::from_elaborated_with(
                &elaborated,
                elaborated.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            let resolved = self
                .repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
                .unwrap_or_else(|| panic!("missing resolved dir for module {module_id:?}"));
            let interface =
                DirInterface::from_resolved_and_declared(resolved.as_ref(), dir.as_ref());
            let analyzed = DirAnalyzed::from_interface_and_declared_with(
                &interface,
                dir.as_ref(),
                interface.types.as_ref().clone(),
                dir.captures.as_ref().clone(),
            );
            let elaborated = DirElaborated::from_analyzed_with(
                &analyzed,
                analyzed.tree.as_ref().clone(),
                analyzed.symbols.as_ref().clone(),
                analyzed.types.as_ref().clone(),
            );
            return Some(DirPatched::from_elaborated_with(
                &elaborated,
                elaborated.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            let declared = DirDeclared::from_resolved_with(
                dir.as_ref(),
                dir.symbols.as_ref().clone(),
                dir.types.as_ref().clone(),
                CaptureTable::new(),
            );
            let interface = DirInterface::from_resolved_and_declared(dir.as_ref(), &declared);
            let analyzed = DirAnalyzed::from_interface_and_declared_with(
                &interface,
                &declared,
                interface.types.as_ref().clone(),
                declared.captures.as_ref().clone(),
            );
            let elaborated = DirElaborated::from_analyzed_with(
                &analyzed,
                analyzed.tree.as_ref().clone(),
                analyzed.symbols.as_ref().clone(),
                analyzed.types.as_ref().clone(),
            );
            return Some(DirPatched::from_elaborated_with(
                &elaborated,
                elaborated.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            let resolved = DirResolved::from_prepared_with(
                dir.as_ref(),
                dir.tree.as_ref().clone(),
                dir.symbols.as_ref().clone(),
                dir.types.as_ref().clone(),
                dir.export_assignment,
                Vec::new(),
                dir.module_binding_exports.as_ref().clone(),
                dir.imported_modules.as_ref().clone(),
                dir.exported_symbols.as_ref().clone(),
            );
            let declared = DirDeclared::from_resolved_with(
                &resolved,
                resolved.symbols.as_ref().clone(),
                resolved.types.as_ref().clone(),
                CaptureTable::new(),
            );
            let interface = DirInterface::from_resolved_and_declared(&resolved, &declared);
            let analyzed = DirAnalyzed::from_interface_and_declared_with(
                &interface,
                &declared,
                interface.types.as_ref().clone(),
                declared.captures.as_ref().clone(),
            );
            let elaborated = DirElaborated::from_analyzed_with(
                &analyzed,
                analyzed.tree.as_ref().clone(),
                analyzed.symbols.as_ref().clone(),
                analyzed.types.as_ref().clone(),
            );
            return Some(DirPatched::from_elaborated_with(
                &elaborated,
                elaborated.tree.as_ref().clone(),
            ));
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| {
                let prepared = DirPrepared::from_base_with(
                    dir.as_ref(),
                    profile,
                    dir.tree.as_ref().clone(),
                    dir.symbols.as_ref().clone(),
                    dir.roots.as_ref().clone(),
                    None,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                );
                let resolved = DirResolved::from_prepared_with(
                    &prepared,
                    prepared.tree.as_ref().clone(),
                    prepared.symbols.as_ref().clone(),
                    prepared.types.as_ref().clone(),
                    prepared.export_assignment,
                    Vec::new(),
                    prepared.module_binding_exports.as_ref().clone(),
                    prepared.imported_modules.as_ref().clone(),
                    prepared.exported_symbols.as_ref().clone(),
                );
                let declared = DirDeclared::from_resolved_with(
                    &resolved,
                    resolved.symbols.as_ref().clone(),
                    resolved.types.as_ref().clone(),
                    CaptureTable::new(),
                );
                let interface = DirInterface::from_resolved_and_declared(&resolved, &declared);
                let analyzed = DirAnalyzed::from_interface_and_declared_with(
                    &interface,
                    &declared,
                    interface.types.as_ref().clone(),
                    declared.captures.as_ref().clone(),
                );
                let elaborated = DirElaborated::from_analyzed_with(
                    &analyzed,
                    analyzed.tree.as_ref().clone(),
                    analyzed.symbols.as_ref().clone(),
                    analyzed.types.as_ref().clone(),
                );
                DirPatched::from_elaborated_with(&elaborated, elaborated.tree.as_ref().clone())
            })
    }

    /// Clone the latest published DIR forward into one patched artifact.
    pub(crate) fn artifact_dir_data_maybe(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Option<TestDir> {
        self.artifact_dir_exact_maybe(module_id, profile)
            .map(|dir| {
                TestDir::from_patched(&dir, self.artifact_exported_symbols(module_id, profile))
            })
    }

    /// Clone the latest published DIR forward into one patched artifact.
    pub(crate) fn artifact_dir(&self, module_id: ModuleId, profile: ProfileId) -> TestDir {
        self.artifact_dir_data_maybe(module_id, profile)
            .unwrap_or_else(|| panic!("missing artifact dir for module {module_id:?}"))
    }

    /// Clone the latest published DIR forward into one patched artifact.
    pub(crate) fn artifact_dir_data(&self, module_id: ModuleId, profile: ProfileId) -> TestDir {
        self.artifact_dir(module_id, profile)
    }

    /// Return one cloned test DIR with its stable source id.
    pub(crate) fn artifact_dir_context(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> (Arc<Module>, TestDir, LocalNodeIdAny) {
        let module = self.program.module_descriptor(module_id);
        let dir = self.artifact_dir(module_id, profile);
        let source_id = dir.roots[0].into_any();

        (module, dir, source_id)
    }

    /// Clone the latest published DIR tree for one module and profile.
    pub(crate) fn artifact_tree(&self, module_id: ModuleId, profile: ProfileId) -> Tree {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.tree.as_ref().clone();
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.tree.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact tree for module {module_id:?}"))
    }

    /// Clone the latest published DIR symbols for one module and profile.
    pub(crate) fn artifact_symbols(&self, module_id: ModuleId, profile: ProfileId) -> SymbolTable {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.symbols.as_ref().clone();
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.symbols.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact symbols for module {module_id:?}"))
    }

    /// Clone the latest published DIR types for one module and profile.
    pub(crate) fn artifact_types(&self, module_id: ModuleId, profile: ProfileId) -> TypeTable {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.types.as_ref().clone();
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.types.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact types for module {module_id:?}"))
    }

    /// Clone the latest published DIR roots for one module and profile.
    pub(crate) fn artifact_roots(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Vec<LocalNodeId<Expression>> {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.roots.as_ref().clone();
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.roots.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact roots for module {module_id:?}"))
    }

    /// Clone the latest published DIR captures for one module and profile.
    pub(crate) fn artifact_captures(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> CaptureTable {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        if self
            .repository
            .dir_interface(self.artifact_revision(), module_id, profile)
            .is_some()
            && let Some(dir) =
                self.repository
                    .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        CaptureTable::new()
    }

    /// Return the latest published namespace scope for one module and profile.
    pub(crate) fn artifact_namespace_scope(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> LocalScopeId {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.namespace_scope;
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.namespace_scope)
            .unwrap_or_else(|| panic!("missing artifact namespace scope for module {module_id:?}"))
    }

    /// Return the latest published anchor node for one module and profile.
    pub(crate) fn artifact_anchor_node(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> LocalNodeIdAny {
        if let Some(dir) = self
            .repository
            .dir_patched(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_elaborated(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_analyzed(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_declared(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.anchor_node;
        }

        self.repository
            .dir_base(self.artifact_revision(), module_id)
            .map(|dir| dir.anchor_node)
            .unwrap_or_else(|| panic!("missing artifact anchor node for module {module_id:?}"))
    }

    /// Clone the latest exported-symbol table for one module and profile.
    pub(crate) fn artifact_exported_symbols(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ExportedSymbolTable {
        if let Some(dir) =
            self.repository
                .dir_interface(self.artifact_revision(), module_id, profile)
        {
            return dir.exported_symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_resolved(self.artifact_revision(), module_id, profile)
        {
            return dir.exported_symbols.as_ref().clone();
        }

        if let Some(dir) =
            self.repository
                .dir_prepared(self.artifact_revision(), module_id, profile)
        {
            return dir.exported_symbols.as_ref().clone();
        }

        ExportedSymbolTable::new()
    }

    /// Return the resolved DIR artifact for one module's default profile.
    pub(crate) fn dir_resolved(&self, module_id: ModuleId) -> DirResolved {
        let profile = self.default_profile_id(module_id);
        let dir = self
            .repository
            .dir_resolved(self.artifact_revision(), module_id, profile)
            .unwrap_or_else(|| panic!("missing resolved dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Return the base DIR artifact for one module.
    pub(crate) fn dir_base(&self, module_id: ModuleId) -> DirBase {
        let dir = self
            .repository
            .dir_base(self.artifact_revision(), module_id)
            .unwrap_or_else(|| panic!("missing base dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Return the declared DIR artifact for one module's default profile.
    pub(crate) fn dir_declared(&self, module_id: ModuleId) -> DirDeclared {
        let profile = self.default_profile_id(module_id);
        let dir = self
            .repository
            .dir_declared(self.artifact_revision(), module_id, profile)
            .unwrap_or_else(|| panic!("missing declared dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Clone the published MIR tree and strings for one module, profile, and target.
    pub(crate) fn artifact_mir_parts(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
    ) -> (mir::Tree, ImmutableStringPool) {
        if let Some(mir) =
            self.repository
                .mir_optimized(self.artifact_revision(), module_id, profile, *target_id)
        {
            return (mir.tree.clone(), mir.strings.clone().into_immutable());
        }

        let mir = self
            .repository
            .mir_base(self.artifact_revision(), module_id, profile, *target_id)
            .unwrap_or_else(|| panic!("missing artifact mir for module {module_id:?}"));

        (mir.tree.clone(), mir.strings.clone().into_immutable())
    }

    /// Create a new test harness with the given options.
    fn new(fs: TestFileSystem, workers: u16, inject_prelude: bool, load_libraries: bool) -> Self {
        init_tracing();
        let root_directory = match &fs {
            TestFileSystem::Memory { .. } => PathBuf::new(),
            TestFileSystem::Physical { root_directory, .. } => root_directory.clone(),
        };

        // seed a default package name for in memory tests
        if let TestFileSystem::Memory { fs } = &fs {
            fs.create_dir(Path::new(""))
                .unwrap_or_else(|_| panic!("failed to initialize memory file system root"));
            fs.add_file("package.json", br#"{ "name": "test" }"#)
                .unwrap_or_else(|_| {
                    panic!("failed to add package.json to memory file system");
                });
        }

        let cache = fs.cache();
        let repository = Arc::new(
            Repository::open_root_from_fs(
                root_directory.clone(),
                fs.fs(),
                AmbientSnapshot::default(),
            )
            .expect("failed to import repository from compiler test file system")
            .with_cache(cache),
        );
        let program = Arc::new(TestWorkspaceView::new(repository.clone(), root_directory));

        let compiler_options = CompilerOptions {
            workers,
            inject_prelude,
            load_libraries,
            elaborate_parenthesize_casts: true,
            ..CompilerOptions::default()
        };
        let compiler = Arc::new(Compiler::new(repository.clone(), compiler_options));
        let workspace_reference = Ref::for_workspace_root(program.root_directory());
        let linter = Arc::new(Linter::new(repository.clone()));

        // track the seeded package manifest in the initial revision
        repository
            .apply(
                &workspace_reference,
                Change::from([Edit::set_text("package.json", r#"{ "name": "test" }"#)]),
            )
            .unwrap_or_else(|error| panic!("failed to publish initial package manifest: {error}"));
        Self {
            fs,
            repository,
            program,
            compiler,
            linter,
            latest_diagnostics: Mutex::new(DiagnosticCollection::new()),
            pending_artifact_keys: Mutex::new(Vec::new()),
            default_profile_override: None,
        }
    }

    /// In-memory, parallel, without prelude injection.
    pub fn memory_parallel() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            test_parallel_workers(),
            false,
            false,
        )
    }

    /// In-memory, parallel, with prelude injection.
    pub fn memory_parallel_with_builtins() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            test_parallel_workers(),
            true,
            false,
        )
    }

    /// In-memory, parallel, with prelude injection and libs.
    pub fn memory_parallel_with_prelude_and_libs() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            test_parallel_workers(),
            true,
            true,
        )
    }

    /// In-memory, sequential, without prelude injection.
    pub fn memory_sequential() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            1,
            false,
            false,
        )
    }

    /// In-memory, sequential, with prelude injection.
    pub fn memory_sequential_with_prelude() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            1,
            true,
            false,
        )
    }

    /// In-memory, sequential, with prelude injection and libs.
    pub fn memory_sequential_with_prelude_and_libs() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            1,
            true,
            true,
        )
    }

    /// Load a library module set (builder pattern).
    pub fn with_lib(self, name: &str) -> Self {
        let profile_id = self.default_profile_id_for_root();
        let profile_key = self.profile(profile_id).key.clone();
        self.repository
            .load_builtin_library(name, &profile_key)
            .unwrap_or_else(|| panic!("missing builtin library '{name}'"));
        self
    }

    /// Override the default profile with an explicit library set.
    pub fn with_profile_libs(mut self, libs: &[&str]) -> Self {
        let default_profile = self.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.lib = libs.iter().map(|lib| (*lib).to_string()).collect();
        let profile_id = Profile::id_for_key(&key);
        self.default_profile_override = Some(profile_id);
        self
    }

    /// Override the default profile with an explicit emit format.
    pub fn with_profile_emit(mut self, emit: EmitFormat) -> Self {
        let default_profile = self.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.emit = emit;
        let profile_id = Profile::id_for_key(&key);
        self.default_profile_override = Some(profile_id);
        self
    }

    /// Mutate compiler options for this test program.
    pub fn with_options_mut<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut CompilerOptions),
    {
        let compiler = Arc::get_mut(&mut self.compiler)
            .unwrap_or_else(|| panic!("compiler options are already shared"));
        f(&mut compiler.options);
        self
    }

    /// Add a file to the memory filesystem (without creating a Module).
    pub fn add_file(&self, path: &str, content: &str) {
        self.write_workspace_text_file(Path::new(path), content);
    }

    /// Add a package.json and optionally destack.json to the memory filesystem.
    ///
    /// `destack_config_compiler_options` is the raw JSON content for `compilerOptions`, e.g.:
    /// ```ignore
    /// test.add_package("my-pkg", Some(r#""noRedeclaredLocals": true"#));
    /// ```
    pub fn add_package(&self, name: &str, destack_config_compiler_options: Option<&str>) {
        self.write_workspace_text_file(
            Path::new("package.json"),
            &format!(r#"{{ "name": "{name}" }}"#),
        );

        if let Some(opts) = destack_config_compiler_options {
            self.write_workspace_text_file(
                Path::new("destack.json"),
                &format!(r#"{{ "compilerOptions": {{ {opts} }} }}"#),
            );
        }
    }

    /// Add a destack.json to the memory filesystem.
    pub fn add_destack_config(&self, content: &str) {
        self.write_workspace_text_file(Path::new("destack.json"), content);
    }

    /// Set the cache mode for this test program.
    pub fn with_cache_mode(self, mode: CacheMode) -> Self {
        // map cache mode to json value
        let mode_value = match mode {
            CacheMode::Off => "off",
            CacheMode::Disk => "disk",
        };

        // build config json value
        let config_value = json!({ "cache": { "mode": mode_value } });
        let config = config_value.to_string();

        // write config to memory fs
        self.add_destack_config(&config);

        self
    }

    /// Add a file and materialize one module id for it.
    pub fn add_module(&self, path: &str, content: &str) -> ModuleId {
        self.write_workspace_text_file(Path::new(path), content);
        let revision = self.program.current_revision();
        let path = PathBuf::from(path);

        self.compiler
            .resolve_path_to_module(revision, &path)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to register module at '{}': {error:?}",
                    path.display()
                )
            })
    }

    /// Get the default profile id for a module.
    pub fn default_profile_id(&self, module_id: ModuleId) -> ProfileId {
        let revision = self.program.current_revision();

        self.default_profile_override.unwrap_or_else(|| {
            self.compiler
                .context(revision)
                .unwrap_or_else(|error| panic!("{error}"))
                .default_profile_id_for_module(module_id)
        })
    }

    /// Get the default profile id for the root module.
    pub fn default_profile_id_for_root(&self) -> ProfileId {
        let revision = self.program.current_revision();
        let root_module_id = self.program.synthetic_root_module_id();

        self.default_profile_override.unwrap_or_else(|| {
            self.compiler
                .context(revision)
                .unwrap_or_else(|error| panic!("{error}"))
                .default_profile_id_for_module(root_module_id)
        })
    }

    /// Return one compiler context for the current workspace revision.
    pub fn context(&self) -> CompilerContext<'_> {
        let revision = self.program.current_revision();

        self.compiler
            .context(revision)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Return the current workspace revision.
    pub fn current_revision(&self) -> Revision {
        self.program.current_revision()
    }

    /// Return the active artifact revision for test reads.
    pub fn artifact_revision(&self) -> Revision {
        self.compiler
            .current_execution_revision()
            .unwrap_or_else(|| self.program.current_revision())
    }

    /// Return one profile value for the current revision.
    pub fn profile(&self, profile_id: ProfileId) -> Profile {
        self.compiler
            .profile_for_revision(self.current_revision(), profile_id)
    }

    /// Return one revision-scoped module graph artifact.
    pub fn module_graph(&self, profile_id: ProfileId) -> Arc<ModuleGraph> {
        let context = self
            .compiler
            .context(self.artifact_revision())
            .unwrap_or_else(|error| panic!("{error}"));

        context
            .module_graph(profile_id)
            .unwrap_or_else(|| panic!("missing module graph for profile {profile_id:?}"))
    }

    /// Return one revision-scoped parsed data artifact.
    pub fn data(&self, module_id: ModuleId) -> Arc<Data> {
        let context = self
            .compiler
            .context(self.artifact_revision())
            .unwrap_or_else(|error| panic!("{error}"));

        context
            .data(module_id)
            .unwrap_or_else(|| panic!("missing data payload for module {module_id:?}"))
    }

    /// Return one artifact dependency for the current revision.
    pub fn artifact_stamp_for_key(&self, artifact_key: &ArtifactKey) -> ArtifactStamp {
        self.compiler
            .artifact_stamp_for_revision(self.program.current_revision(), artifact_key)
    }

    /// Return the resolved module graph version for one module.
    pub fn module_version(&self, module_id: ModuleId) -> ModuleVersion {
        let profile = self.default_profile_id(module_id);
        let artifact_key = ArtifactKey::dir_resolved(module_id, profile);
        let artifact_stamp = self.artifact_stamp_for_key(&artifact_key);

        ModuleVersion::new(artifact_stamp.0)
    }

    /// Enqueue Import task for a module.
    pub fn import_module(&self, module: ModuleId) {
        self.enqueue(ArtifactKey::dir_base(module));
    }

    /// Enqueue Bind task for a module.
    pub fn bind_module(&self, module: ModuleId) {
        self.enqueue(ArtifactKey::dir_base(module));
    }

    /// Enqueue Resolve task for a module.
    pub fn resolve_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_resolved(module, profile));
    }

    /// Resolve the language environment for the default root profile.
    pub fn resolve_language_environment(&self) {
        let profile = self.default_profile_id_for_root();
        self.run(ArtifactKey::language_environment(profile));
    }

    /// Resolve builtin libraries for the default root profile.
    pub fn resolve_libs(&self) {
        let profile = self.default_profile_id_for_root();
        self.run(ArtifactKey::library_environment(profile));
    }

    /// Enqueue Analyze task for a module.
    pub fn analyze_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_analyzed(module, profile));
    }

    /// Drive declaration analysis for one module to completion.
    pub fn declare_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.run(ArtifactKey::dir_declared(module, profile));
    }

    /// Analyze a module and check no diagnostics.
    pub fn analyze_module_and_check_clean(&self, module: ModuleId) {
        // enqueue the analyze task
        self.analyze_module(module);

        // run compilation and diagnostics
        self.compile_check_clean();
    }

    /// Lint a module through the linter crate.
    pub fn lint_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.run(ArtifactKey::module_linted(module, profile));
    }

    /// Enqueue Elaborate task for a module.
    pub fn elaborate_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_elaborated(module, profile));
    }

    /// Enqueue Execute task for a module.
    pub fn execute_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_patched(module, profile));
    }

    /// Add a build target to the package containing the given module.
    ///
    /// Uses `Target::implicit_for_name()` for known target names like "native", "js", "wasm".
    pub fn add_target(&self, module: ModuleId, name: &str) {
        let target_value = Self::implicit_target_config_value(name)
            .unwrap_or_else(|| panic!("unknown implicit target '{name}'"));

        self.edit_destack_config(module, |config| {
            let targets = Self::destack_targets_value(config);
            targets.insert(name.to_string(), target_value);
        });
    }

    /// Configure a build target for the package containing the given module.
    ///
    /// Adds the target when missing.
    pub fn configure_target(
        &self,
        module: ModuleId,
        name: &str,
        configure: impl FnOnce(&mut Target),
    ) {
        let package_id = module.package_id;
        let target_id = self.target_id(package_id, name);
        let target = self
            .program
            .package_descriptor(package_id)
            .target(&target_id)
            .cloned()
            .or_else(|| Target::implicit_for_name(name))
            .unwrap_or_else(|| panic!("unknown implicit target '{name}'"));
        let mut target = target;
        configure(&mut target);
        let target_value = Self::target_config_value(&target);

        self.edit_destack_config(module, |config| {
            let targets = Self::destack_targets_value(config);
            targets.insert(name.to_string(), target_value);
        });
    }

    /// Apply a destack.json blob to the package containing the given module.
    pub fn apply_destack_config(&self, module: ModuleId, content: &str) {
        let patch: JsonValue = serde_json::from_str(content)
            .unwrap_or_else(|error| panic!("invalid destack.json: {error}"));

        self.edit_destack_config(module, |config| {
            Self::merge_json_value(config, patch);
        });
    }

    /// Enqueue Lower task for a module.
    pub fn lower_module(&self, module: ModuleId, target: &str) {
        let package_id = module.package_id;
        let target_id = self.target_id(package_id, target);
        let profile = match self.program.profile_id_for_target(module, &target_id) {
            Some(profile) => profile,
            None => {
                self.add_target(module, target);
                self.program
                    .profile_id_for_target(module, &target_id)
                    .unwrap_or_else(|| panic!("missing profile for target '{target}'"))
            }
        };
        self.enqueue(ArtifactKey::mir_base(module, profile, target_id));
    }

    /// Copy one target configuration onto another target name.
    pub fn copy_target(&self, module: ModuleId, from: &str, to: &str) {
        let target_value = self
            .destack_target_value(module, from)
            .unwrap_or_else(|| panic!("missing target '{from}'"));

        self.edit_destack_config(module, |config| {
            let targets = Self::destack_targets_value(config);
            targets.insert(to.to_string(), target_value);
        });
    }

    /// Remove one target configuration from the package containing the module.
    pub fn remove_target(&self, module: ModuleId, name: &str) {
        self.edit_destack_config(module, |config| {
            let targets = Self::destack_targets_value(config);
            targets.shift_remove(name);
        });
    }

    /// Enqueue Optimize task for a module.
    pub fn optimize_module(&self, module: ModuleId, target: &str) {
        let package_id = module.package_id;
        let target_id = self.target_id(package_id, target);
        let profile = self
            .program
            .profile_id_for_target(module, &target_id)
            .unwrap_or_else(|| panic!("missing profile for target '{target}'"));
        self.enqueue(ArtifactKey::mir_optimized(module, profile, target_id));
    }

    /// Enqueue one artifact key.
    pub fn enqueue(&self, artifact_key: ArtifactKey) {
        self.pending_artifact_keys
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(artifact_key);
    }

    /// Return the workspace reference for repository revision updates.
    fn workspace_reference(&self) -> Ref {
        Ref::for_workspace_root(self.program.root_directory())
    }

    /// Return one repository interned target id.
    pub(crate) fn target_id(&self, package_id: PackageId, name: &str) -> TargetId {
        self.repository.intern_target_id(package_id, name)
    }

    /// Return one absolute workspace path for one test file.
    fn workspace_absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            return path.to_path_buf();
        }

        self.program.root_directory().join(path)
    }

    /// Return one workspace relative path for one test file.
    fn workspace_relative_path(&self, path: &Path) -> PathBuf {
        if let Ok(path) = path.strip_prefix(self.program.root_directory()) {
            return path.to_path_buf();
        }

        path.to_path_buf()
    }

    /// Write one tracked workspace file and publish a new revision.
    fn write_workspace_text_file(&self, path: &Path, content: &str) {
        let relative_path = self.workspace_relative_path(path);

        // file system
        match &self.fs {
            TestFileSystem::Memory { fs } => {
                fs.add_file(relative_path.to_string_lossy().as_ref(), content.as_bytes())
                    .unwrap_or_else(|_| {
                        panic!(
                            "failed to write test file '{}'",
                            relative_path.to_string_lossy()
                        )
                    });
            }
            TestFileSystem::Physical { .. } => {
                let absolute_path = self.workspace_absolute_path(path);
                if let Some(parent) = absolute_path.parent() {
                    std::fs::create_dir_all(parent).unwrap_or_else(|error| {
                        panic!(
                            "failed to create parent directory '{}' for test file: {error}",
                            parent.display()
                        )
                    });
                }

                std::fs::write(&absolute_path, content).unwrap_or_else(|error| {
                    panic!(
                        "failed to write test file '{}' to disk: {error}",
                        absolute_path.display()
                    )
                });
            }
        }

        // revision
        self.repository
            .apply(
                &self.workspace_reference(),
                Change::from([Edit::set_text(relative_path.to_string_lossy(), content)]),
            )
            .unwrap_or_else(|error| panic!("failed to publish tracked test file: {error}"));
    }

    /// Return the package directory for one module.
    fn package_directory_for_module(&self, module: ModuleId) -> PathBuf {
        let module = self.program.module_descriptor(module);
        let package = self.program.package_descriptor(module.package_id);

        if let Some(path) = package.path.as_ref() {
            return self.workspace_absolute_path(path);
        }

        if let Some(path) = module.path.as_ref()
            && let Some(parent) = path.parent()
        {
            return self.workspace_absolute_path(parent);
        }

        self.program.root_directory().clone()
    }

    /// Return the absolute destack config path for one module.
    fn destack_config_path_for_module(&self, module: ModuleId) -> PathBuf {
        self.package_directory_for_module(module)
            .join("destack.json")
    }

    /// Return the current destack config JSON for one module.
    fn destack_config_value(&self, module: ModuleId) -> JsonValue {
        let path = self.destack_config_path_for_module(module);
        let Some(file_id) = self.program.source_file_id_for_path(&path) else {
            return json!({});
        };
        let file = self.program.source_file(file_id);

        match file.content.payload() {
            FileContent::Text { content } => serde_json::from_str(content)
                .unwrap_or_else(|error| panic!("invalid tracked destack.json: {error}")),
            FileContent::Binary { .. } => json!({}),
        }
    }

    /// Return the current target JSON for one module when present.
    fn destack_target_value(&self, module: ModuleId, name: &str) -> Option<JsonValue> {
        let config = self.destack_config_value(module);
        let targets = config.get("targets")?;
        let targets = targets.as_object()?;

        targets.get(name).cloned()
    }

    /// Edit one package destack config and publish the resulting revision.
    fn edit_destack_config(&self, module: ModuleId, edit: impl FnOnce(&mut JsonValue)) {
        let mut config = self.destack_config_value(module);
        if !config.is_object() {
            config = json!({});
        }

        edit(&mut config);

        let content = serde_json::to_string_pretty(&config)
            .unwrap_or_else(|error| panic!("failed to serialize destack.json: {error}"));
        let path = self.destack_config_path_for_module(module);
        self.write_workspace_text_file(&path, &content);
    }

    /// Return the `targets` object for one mutable destack config value.
    fn destack_targets_value(config: &mut JsonValue) -> &mut serde_json::Map<String, JsonValue> {
        let JsonValue::Object(config) = config else {
            panic!("destack config must be one object");
        };
        let targets = config.entry("targets").or_insert_with(|| json!({}));
        let JsonValue::Object(targets) = targets else {
            panic!("destack config targets must be one object");
        };

        targets
    }

    /// Merge one JSON patch into one existing config value.
    fn merge_json_value(base: &mut JsonValue, patch: JsonValue) {
        match (base, patch) {
            (JsonValue::Object(base), JsonValue::Object(patch)) => {
                for (key, value) in patch {
                    if let Some(base_value) = base.get_mut(&key) {
                        Self::merge_json_value(base_value, value);
                    } else {
                        base.insert(key, value);
                    }
                }
            }
            (base, patch) => *base = patch,
        }
    }

    /// Return the minimal implicit target config for one builtin target name.
    fn implicit_target_config_value(name: &str) -> Option<JsonValue> {
        match name {
            "default" | "js" => Some(json!({
                "emit": "js",
                "runtime": "node",
                "declaration": true,
            })),
            "ts" => Some(json!({
                "emit": "ts",
                "runtime": "node",
            })),
            "html" => Some(json!({
                "emit": "html",
                "runtime": "browser",
            })),
            "node" => Some(json!({
                "emit": "js",
                "runtime": "node",
                "platform": "universal",
                "declaration": true,
            })),
            "wasm" => Some(json!({
                "emit": "wasm",
                "runtime": "wasm-js",
                "optimize": true,
            })),
            "wasm-wasi" | "wasi" => Some(json!({
                "emit": "wasm",
                "runtime": "wasm-wasi",
                "platform": "wasi",
                "optimize": true,
            })),
            "native" => Some(json!({
                "emit": "native",
                "runtime": "native-managed",
                "platform": "universal",
                "optimize": true,
            })),
            _ => None,
        }
    }

    /// Convert one target snapshot into one file backed config value.
    fn target_config_value(target: &Target) -> JsonValue {
        let base = Target::implicit_for_name(&target.name).unwrap_or_default();
        let mut value =
            Self::implicit_target_config_value(&target.name).unwrap_or_else(|| json!({}));
        let mut patch = Self::target_config_patch(target, &base);

        Self::prune_json_patch(&mut patch);
        Self::merge_json_value(&mut value, patch);

        value
    }

    /// Build one structured config patch for one target override.
    fn target_config_patch(target: &Target, base: &Target) -> JsonValue {
        let entry = (target.entry != base.entry).then(|| {
            target
                .entry
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        });
        let out_file = (target.out_file != base.out_file).then(|| {
            target
                .out_file
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
        });

        json!({
            "entry": if target.discovery == TargetDiscovery::Entry && target.entry.is_empty() {
                Some(Vec::<String>::new())
            } else {
                entry
            },
            "emit": (target.emit != base.emit).then(|| Self::emit_format_name(target.emit)),
            "outDir": (target.out_dir != base.out_dir)
                .then(|| target.out_dir.to_string_lossy().into_owned()),
            "outFile": out_file,
            "target": (target.es_target != base.es_target)
                .then(|| Self::es_target_name(target.es_target)),
            "boundsChecks": (target.bounds_checks != base.bounds_checks)
                .then(|| Self::bounds_check_policy_name(target.bounds_checks)),
            "overflowChecks": (target.overflow_checks != base.overflow_checks)
                .then(|| Self::overflow_check_policy_name(target.overflow_checks)),
            "divisionChecks": (target.division_checks != base.division_checks)
                .then(|| Self::division_check_policy_name(target.division_checks)),
            "shiftChecks": (target.shift_checks != base.shift_checks)
                .then(|| Self::shift_check_policy_name(target.shift_checks)),
            "checkFailure": (target.check_failure != base.check_failure)
                .then(|| Self::check_failure_policy_name(target.check_failure)),
            "assembly": (target.assembly != base.assembly)
                .then(|| Self::bundle_mode_name(target.assembly)),
            "manualChunks": (!target.manual_chunks.is_empty()).then(|| target.manual_chunks.clone()),
            "onlyExplicitManualChunks": (target.only_explicit_manual_chunks != base.only_explicit_manual_chunks)
                .then_some(target.only_explicit_manual_chunks),
            "dependencies": Self::target_dependency_config_patch(target, base),
            "output": Self::target_output_config_patch(target, base),
            "minify": Self::target_minify_config_patch(target, base),
        })
    }

    /// Build one structured dependency patch for one target override.
    fn target_dependency_config_patch(target: &Target, _base: &Target) -> Option<JsonValue> {
        let never_bundle = (!target.bundle_dependencies.never_bundle.is_empty())
            .then(|| target.bundle_dependencies.never_bundle.clone());
        let only_bundle = (!target.bundle_dependencies.only_bundle.is_empty())
            .then(|| target.bundle_dependencies.only_bundle.clone());

        if never_bundle.is_none() && only_bundle.is_none() {
            return None;
        }

        Some(json!({
            "neverBundle": never_bundle,
            "onlyBundle": only_bundle,
        }))
    }

    /// Build one structured output patch for one target override.
    fn target_output_config_patch(target: &Target, base: &Target) -> Option<JsonValue> {
        let generated_code = (target.bundle_output.generated_code
            != base.bundle_output.generated_code)
            .then(|| Self::generated_code_value(target.bundle_output.generated_code.as_ref()))
            .flatten();

        let patch = json!({
            "format": (target.bundle_output.format != base.bundle_output.format)
                .then(|| target.bundle_output.format.map(Self::bundle_format_name))
                .flatten(),
            "entryFileNames": (target.bundle_output.entry_file_names != base.bundle_output.entry_file_names)
                .then(|| target.bundle_output.entry_file_names.clone())
                .flatten(),
            "chunkFileNames": (target.bundle_output.chunk_file_names != base.bundle_output.chunk_file_names)
                .then(|| target.bundle_output.chunk_file_names.clone())
                .flatten(),
            "assetFileNames": (target.bundle_output.asset_file_names != base.bundle_output.asset_file_names)
                .then(|| target.bundle_output.asset_file_names.clone())
                .flatten(),
            "publicPath": (target.bundle_output.public_path != base.bundle_output.public_path)
                .then(|| target.bundle_output.public_path.clone())
                .flatten(),
            "manifest": (target.bundle_output.manifest != base.bundle_output.manifest)
                .then_some(target.bundle_output.manifest),
            "sourcemap": (target.bundle_output.sourcemap != base.bundle_output.sourcemap)
                .then(|| target.bundle_output.sourcemap.map(Self::source_map_mode_name))
                .flatten(),
            "banner": (target.bundle_output.banner != base.bundle_output.banner)
                .then(|| target.bundle_output.banner.clone())
                .flatten(),
            "footer": (target.bundle_output.footer != base.bundle_output.footer)
                .then(|| target.bundle_output.footer.clone())
                .flatten(),
            "generatedCode": generated_code,
        });

        (!Self::json_patch_is_empty(&patch)).then_some(patch)
    }

    /// Build one structured minify patch for one target override.
    fn target_minify_config_patch(target: &Target, base: &Target) -> Option<JsonValue> {
        if target.minify == base.minify {
            return None;
        }

        Some(json!({
            "enabled": target.minify.enabled,
            "syntax": target.minify.syntax,
            "whitespace": target.minify.whitespace,
            "identifiers": target.minify.identifiers,
            "keepNames": target.minify.keep_names,
        }))
    }

    /// Remove `null` values and empty objects from one json patch.
    fn prune_json_patch(value: &mut JsonValue) {
        match value {
            JsonValue::Object(object) => {
                object.retain(|_, value| {
                    Self::prune_json_patch(value);
                    !value.is_null()
                        && !matches!(value, JsonValue::Object(object) if object.is_empty())
                });
            }
            JsonValue::Array(values) => {
                for value in values {
                    Self::prune_json_patch(value);
                }
            }
            _ => {}
        }
    }

    /// Return whether one json patch is empty after pruning.
    fn json_patch_is_empty(value: &JsonValue) -> bool {
        matches!(value, JsonValue::Object(object) if object.is_empty())
    }

    /// Return the JSON spelling for one bundle mode.
    fn bundle_mode_name(mode: BundleMode) -> &'static str {
        match mode {
            BundleMode::SingleFile => "singleFile",
            BundleMode::PreserveModules => "preserveModules",
            BundleMode::Chunked => "chunked",
        }
    }

    /// Return the JSON spelling for one emit format.
    fn emit_format_name(format: EmitFormat) -> &'static str {
        match format {
            EmitFormat::Js => "js",
            EmitFormat::Ts => "ts",
            EmitFormat::Html => "html",
            EmitFormat::Wasm => "wasm",
            EmitFormat::Native => "native",
        }
    }

    /// Return the JSON spelling for one bundle format.
    fn bundle_format_name(format: BundleFormat) -> &'static str {
        match format {
            BundleFormat::Esm => "esm",
            BundleFormat::Cjs => "cjs",
            BundleFormat::Iife => "iife",
        }
    }

    /// Return the JSON spelling for one ECMAScript target.
    fn es_target_name(target: EsTarget) -> &'static str {
        match target {
            EsTarget::Es5 => "es5",
            EsTarget::Es2015 => "es2015",
            EsTarget::Es2016 => "es2016",
            EsTarget::Es2017 => "es2017",
            EsTarget::Es2018 => "es2018",
            EsTarget::Es2019 => "es2019",
            EsTarget::Es2020 => "es2020",
            EsTarget::Es2021 => "es2021",
            EsTarget::Es2022 => "es2022",
            EsTarget::Es2023 => "es2023",
            EsTarget::Es2024 => "es2024",
            EsTarget::EsNext => "esnext",
        }
    }

    /// Return the JSON value for one generated code configuration.
    fn generated_code_value(
        generated_code: Option<&TargetGeneratedCodeOptions>,
    ) -> Option<JsonValue> {
        let generated_code = generated_code?;
        let mut value = json!({
            "preset": generated_code.preset.map(|preset| match preset {
                TargetGeneratedCodePreset::Es5 => "es5",
                TargetGeneratedCodePreset::Es2015 => "es2015",
            }),
            "arrowFunctions": generated_code.arrow_functions,
            "constBindings": generated_code.const_bindings,
            "objectShorthand": generated_code.object_shorthand,
            "reservedNamesAsProps": generated_code.reserved_names_as_props,
            "symbols": generated_code.symbols,
        });

        Self::prune_json_patch(&mut value);
        (!Self::json_patch_is_empty(&value)).then_some(value)
    }

    /// Return the JSON spelling for one source map mode.
    fn source_map_mode_name(mode: SourceMapMode) -> &'static str {
        match mode {
            SourceMapMode::External => "external",
            SourceMapMode::Inline => "inline",
            SourceMapMode::Hidden => "hidden",
        }
    }

    /// Return the JSON spelling for one overflow check policy.
    fn overflow_check_policy_name(policy: destack_workspace::OverflowCheckPolicy) -> &'static str {
        match policy {
            destack_workspace::OverflowCheckPolicy::Always => "always",
            destack_workspace::OverflowCheckPolicy::Debug => "debug",
            destack_workspace::OverflowCheckPolicy::Never => "never",
        }
    }

    /// Return the JSON spelling for one bounds check policy.
    fn bounds_check_policy_name(policy: BoundsCheckPolicy) -> &'static str {
        match policy {
            BoundsCheckPolicy::Always => "always",
            BoundsCheckPolicy::Debug => "debug",
            BoundsCheckPolicy::Never => "never",
        }
    }

    /// Return the JSON spelling for one division check policy.
    fn division_check_policy_name(policy: DivisionCheckPolicy) -> &'static str {
        match policy {
            DivisionCheckPolicy::Always => "always",
            DivisionCheckPolicy::Debug => "debug",
            DivisionCheckPolicy::Never => "never",
        }
    }

    /// Return the JSON spelling for one shift check policy.
    fn shift_check_policy_name(policy: ShiftCheckPolicy) -> &'static str {
        match policy {
            ShiftCheckPolicy::Always => "always",
            ShiftCheckPolicy::Debug => "debug",
            ShiftCheckPolicy::Never => "never",
        }
    }

    /// Return the JSON spelling for one check failure policy.
    fn check_failure_policy_name(policy: CheckFailurePolicy) -> &'static str {
        match policy {
            CheckFailurePolicy::Trap => "trap",
            CheckFailurePolicy::Panic => "panic",
            CheckFailurePolicy::Abort => "abort",
        }
    }

    /// Enqueue one artifact key (does not run it).
    pub fn enqueue_artifact<T: Into<ArtifactKey>>(&self, artifact_key: T) {
        self.pending_artifact_keys
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(artifact_key.into());
    }

    /// Enqueue one artifact key and run to completion.
    pub fn run<T: Into<ArtifactKey>>(&self, artifact_key: T) {
        self.enqueue_artifact(artifact_key);
        self.compile();
    }

    /// Run all queued tasks to completion (with configurable timeout).
    pub fn compile(&self) {
        let timeout = Duration::from_secs(test_timeout_seconds());
        self.compile_with_timeout(timeout);
    }

    /// Run all queued tasks to completion with a custom timeout.
    pub fn compile_with_timeout(&self, timeout: Duration) {
        let artifact_keys = {
            let mut pending_artifact_keys = self
                .pending_artifact_keys
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            std::mem::take(&mut *pending_artifact_keys)
        };

        if artifact_keys.is_empty() {
            return;
        }

        // only serialize tests that also run a parallel compiler
        let compile_guard = (self.compiler.options.workers > 1).then(|| {
            TEST_COMPILE_LOCK
                .lock()
                .unwrap_or_else(|error| error.into_inner())
        });

        // spawn the provide thread
        let compiler = self.compiler.clone();
        let revision = self.program.current_revision();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            provide_artifacts_to_completion(compiler.as_ref(), revision, &artifact_keys);
            let _ = tx.send(());
        });

        match rx.recv_timeout(timeout) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let snapshot = self.compiler.stats.snapshot_with_repository(
                    self.program.tracked_module_count(),
                    Some(self.program.as_ref()),
                );
                let summary = format_stats_snapshot(&snapshot);
                panic!("compile timed out after {timeout:?}\n{summary}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("compile thread panicked");
            }
        }

        // publish diagnostics from this compiler run into the test harness
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());

        drop(compile_guard);
    }

    /// Replace the latest compiler diagnostics for this test harness.
    fn replace_latest_diagnostics(&self, diagnostics: DiagnosticCollection) {
        let mut latest_diagnostics = self
            .latest_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *latest_diagnostics = diagnostics;
    }

    /// Return the latest compiler diagnostics for this test harness.
    pub fn diagnostics(&self) -> DiagnosticCollection {
        self.latest_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    /// Collect the current workspace diagnostics from module artifact families.
    fn current_workspace_diagnostics(&self) -> DiagnosticCollection {
        let revision = self.program.current_revision();
        let module_ids = self
            .program
            .workspace_module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read workspace modules: {error}"));
        let mut diagnostics = DiagnosticCollection::new();

        // current workspace families
        for module_id in module_ids {
            let profile_id = self.program.default_profile_id_for_module(module_id);
            diagnostics.merge_from(
                &self
                    .program
                    .module_artifact_diagnostics(revision, module_id, profile_id),
            );
        }

        diagnostics
    }

    /// Check no errors.
    pub fn check_clean(&self) {
        self.check_no_diagnostic(DiagnosticSeverity::Error);
    }

    /// Compile and check no diagnostics.
    pub fn compile_check_clean(&self) {
        self.compile();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
    }

    /// Get the file for a module.
    pub fn file(&self, module: ModuleId) -> Arc<File> {
        let module = self.program.module_descriptor(module);
        self.program.source_file(module.file_id)
    }

    /// Get the root expression ids for a module.
    pub fn module_dir_roots(&self, module_id: ModuleId) -> Vec<LocalNodeId<Expression>> {
        let profile = self.default_profile_id(module_id);
        self.artifact_roots(module_id, profile)
    }

    /// Run a closure with read access to one module's latest cloned DIR.
    pub(crate) fn with_dir_read<T>(
        &self,
        module_id: ModuleId,
        f: impl FnOnce(&Module, ProfileId, &TestDir, &Tree, &SymbolTable, &TypeTable) -> T,
    ) -> T {
        let profile = self.default_profile_id(module_id);
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let dir = self.artifact_dir(module_id, profile);

        f(module, profile, &dir, &dir.tree, &dir.symbols, &dir.types)
    }

    /// Run a closure with mutable access to one module's cloned type table.
    pub(crate) fn with_dir_types_mut<T>(
        &self,
        module_id: ModuleId,
        f: impl FnOnce(&Module, ProfileId, &TestDir, &Tree, &SymbolTable, &mut TypeTable) -> T,
    ) -> T {
        let profile = self.default_profile_id(module_id);
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let dir = self.artifact_dir(module_id, profile);
        let mut types = dir.types.clone();

        f(module, profile, &dir, &dir.tree, &dir.symbols, &mut types)
    }

    /// Get a module by URI.
    pub fn module(&self, module_uri: &str) -> Arc<Module> {
        let uri = Uri::from_string(module_uri);
        let module_id = self
            .program
            .module_id_for_uri(&uri)
            .unwrap_or_else(|| panic!("module not found for '{module_uri}'"));

        self.program.module_descriptor(module_id)
    }

    /// Get a module by file.
    pub fn module_for_file(&self, file: &File) -> Arc<Module> {
        let module_id = self
            .program
            .module_id_for_uri(&file.uri)
            .unwrap_or_else(|| panic!("module not found for file: '{}'", file.uri));

        self.program.module_descriptor(module_id)
    }

    /// Check no diagnostics of at least the given severity.
    pub fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.diagnostics();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            self.program
                .print_diagnostics(self.program.current_revision(), &diagnostics, 120);
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "program has {} unexpected {severity_name}s",
                diagnostics.len()
            );
        }
    }

    /// Check that no diagnostics with the given prefix are present.
    pub fn check_no_diagnostics_with_prefix(&self, prefix: &str) {
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let has_prefix = diagnostic_vec.iter().any(|d| d.code.starts_with(prefix));
        if has_prefix {
            // print the diagnostics
            let matching = DiagnosticCollection::from_diagnostics(
                diagnostic_vec
                    .iter()
                    .filter(|d| d.code.starts_with(prefix))
                    .cloned()
                    .collect(),
            );
            self.program
                .print_diagnostics(self.program.current_revision(), &matching, 120);
            panic!("unexpected diagnostics with prefix '{prefix}'");
        }
    }

    /// Check that no diagnostics for the given phases are present.
    pub fn check_no_diagnostics_for_phases(&self, phases: &[CompilePhase]) {
        // build prefixes for both errors and warnings for each phase
        let prefixes: Vec<String> = phases
            .iter()
            .flat_map(|phase| {
                let letter = phase.letter();
                [format!("E{letter}"), format!("W{letter}")]
            })
            .collect();

        // check for any matching diagnostics
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let has_matching = diagnostic_vec
            .iter()
            .any(|d| prefixes.iter().any(|prefix| d.code.starts_with(prefix)));

        if has_matching {
            // print only the matching diagnostics
            let matching_diagnostics = DiagnosticCollection::from_diagnostics(
                diagnostic_vec
                    .iter()
                    .filter(|d| prefixes.iter().any(|prefix| d.code.starts_with(prefix)))
                    .cloned()
                    .collect(),
            );
            self.program.print_diagnostics(
                self.program.current_revision(),
                &matching_diagnostics,
                120,
            );
            let phase_names: Vec<&str> = phases.iter().map(|p| p.name()).collect();
            panic!("unexpected diagnostics for phases: {phase_names:?}");
        }
    }

    /// Check that no diagnostics up to (but not including) the given phase are present.
    pub fn check_no_diagnostics_up_to_excluding_phase(&self, phase: CompilePhase) {
        let phases: Vec<CompilePhase> = CompilePhase::all().take_while(|p| *p != phase).collect();
        self.check_no_diagnostics_for_phases(&phases);
    }

    /// Check that no diagnostics up to and including the given phase are present.
    pub fn check_no_diagnostics_up_to_including_phase(&self, phase: CompilePhase) {
        let phases: Vec<CompilePhase> = CompilePhase::all()
            .take_while(|p| p.code() <= phase.code())
            .collect();
        self.check_no_diagnostics_for_phases(&phases);
    }

    /// Check that exactly the given diagnostics are present (by code).
    /// Panics if the actual diagnostics don't match.
    pub fn check_has_diagnostics(&self, expected_codes: &[&str]) {
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
        if actual_codes != expected_codes {
            self.program
                .print_diagnostics(self.program.current_revision(), &diagnostics, 120);
            panic!("diagnostic mismatch\nexpected: {expected_codes:?}\nactual: {actual_codes:?}");
        }
    }

    /// Check that exactly the given diagnostics are present.
    pub fn check_exact_diagnostics(&self, expected: &[ExpectedDiagnostic]) {
        let diagnostics = self.diagnostics();
        let actual = diagnostics
            .iter()
            .into_iter()
            .map(|diagnostic| ExpectedDiagnostic {
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
            })
            .collect::<Vec<_>>();

        if actual != expected {
            let expected = Self::format_expected_diagnostics(expected);
            let actual = Self::format_expected_diagnostics(&actual);

            print_diff(&expected, &actual, &DiffOptions::new());
            panic!("diagnostic mismatch");
        }
    }

    /// Check that a diagnostic with the given code is present.
    pub fn check_has_diagnostic(&self, code: &str) {
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let has_code = diagnostic_vec.iter().any(|d| d.code == code);
        if !has_code {
            self.program
                .print_diagnostics(self.program.current_revision(), &diagnostics, 120);
            let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
            panic!("expected diagnostic with code '{code}' but found: {actual_codes:?}");
        }
    }

    /// Check that a diagnostic code appears the expected number of times.
    pub fn check_diagnostic_count(&self, code: &str, expected: usize) {
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let count = diagnostic_vec.iter().filter(|d| d.code == code).count();
        if count != expected {
            self.program
                .print_diagnostics(self.program.current_revision(), &diagnostics, 120);
            panic!("expected {expected} diagnostics for '{code}', found {count}");
        }
    }

    /// Check that no diagnostic with the given code is present.
    pub fn check_no_diagnostic_code(&self, code: &str) {
        let diagnostics = self.diagnostics();
        let diagnostic_vec = diagnostics.iter();
        let has_code = diagnostic_vec.iter().any(|d| d.code == code);
        if has_code {
            self.program
                .print_diagnostics(self.program.current_revision(), &diagnostics, 120);
            panic!("unexpected diagnostic with code '{code}'");
        }
    }

    /// Format expected diagnostics for stable diff output.
    fn format_expected_diagnostics(diagnostics: &[ExpectedDiagnostic]) -> String {
        diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Unbind a module's DIR back to AST and format it to a string.
    pub fn unbind_to_string(&self, module_id: ModuleId) -> String {
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let tree = self.artifact_tree(module.id, profile);
        let symbols = self.artifact_symbols(module.id, profile);
        let roots = self.artifact_roots(module.id, profile);
        let types = self.artifact_types(module.id, profile);

        // unbind DIR to AST
        let unbound = self
            .compiler
            .unbind_module_from_parts(module, &tree, &symbols, &types, &roots);

        // create a synthetic file for formatting (no real source)
        let file = File::from_text(
            FileId::new(0),
            "<unbound>".to_string(),
            Uri::from_string("<unbound>"),
            None,
            FileType::Destack,
            String::new(),
        );

        // format the AST
        let strings = unbound.strings.into_immutable();
        let empty_tokens = Vec::new();
        let empty_side_tokens = Vec::new();
        let empty_side_span = MultiSpan::new(vec![]);
        let context = DestackFormatContext::new(
            DestackFormatOptions::default(),
            &file,
            &unbound.tree,
            &empty_tokens,
            &empty_side_tokens,
            &empty_side_span,
            &strings,
            NodeParentIndex::from_tree(&unbound.tree),
        );

        // format the module roots as one statement list
        let formatted = destack_fir::format!(context, [statement_list(&unbound.roots)]).unwrap();
        let printed = formatted.print().unwrap();

        printed.into_str()
    }

    /// Format a module's MIR to a string.
    pub fn mir_to_string(&self, module_id: ModuleId, target: &str) -> String {
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let target_id = self.target_id(module.package_id, target);
        let profile = self.default_profile_id(module_id);
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);
        format_mir(&tree, &strings, self.mir_format_options())
    }

    /// Format a module's MIR using the profile selected by the target.
    pub fn target_mir_to_string(&self, module_id: ModuleId, target: &str) -> String {
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let target_id = self.target_id(module.package_id, target);
        let profile = self
            .program
            .profile_id_for_target(module_id, &target_id)
            .unwrap_or_else(|| panic!("missing profile for target '{target}'"));
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);

        format_mir(&tree, &strings, self.mir_format_options())
    }

    /// Format MIR options for test output.
    fn mir_format_options(&self) -> MirFormatOptions {
        // enable type aliases in test output
        MirFormatOptions::default()
            .with_type_aliases(true)
            .with_local_names(true)
    }

    /// Return the linked package output for one package target.
    pub fn package_output(&self, package_id: PackageId, target: &str) -> PackageOutput {
        let target_id = self.target_id(package_id, target);
        let revision = self.artifact_revision();

        self.repository
            .package_output(revision, package_id, target_id)
            .unwrap_or_else(|| panic!("missing package output for target '{target}'"))
            .as_ref()
            .clone()
    }

    /// Return the generated module output for one target.
    pub fn module_output(&self, module_id: ModuleId, target: &str) -> ModuleOutput {
        let module = self.program.module_descriptor(module_id);
        let target_id = self.target_id(module.package_id, target);
        let revision = self.artifact_revision();

        self.repository
            .module_output(revision, module_id, target_id)
            .unwrap_or_else(|| panic!("missing module output for target '{target}'"))
            .as_ref()
            .clone()
    }

    /// Return the linked package output for the package containing one module.
    pub fn package_output_for_module(&self, module_id: ModuleId, target: &str) -> PackageOutput {
        let module = self.program.module_descriptor(module_id);

        self.package_output(module.package_id, target)
    }

    /// Normalize expected MIR text for comparison.
    fn normalize_mir_expected_with_options(
        &self,
        expected: &str,
        options: MirFormatOptions,
    ) -> String {
        normalize_mir_text_with_options("expected", expected, options)
    }

    /// Normalize expected MIR text for comparison.
    fn normalize_mir_expected(&self, expected: &str) -> String {
        self.normalize_mir_expected_with_options(expected, self.mir_format_options())
    }

    /// Normalize actual MIR text for comparison.
    fn normalize_mir_actual_with_options(&self, actual: &str, options: MirFormatOptions) -> String {
        normalize_mir_text_with_options("actual", actual, options)
    }

    /// Normalize actual MIR text for comparison.
    fn normalize_mir_actual(&self, actual: &str) -> String {
        self.normalize_mir_actual_with_options(actual, self.mir_format_options())
    }

    /// Run a callback with the MIR tree and strings for a lowered target.
    pub fn with_mir_tree<T>(
        &self,
        module_id: ModuleId,
        target: &str,
        f: impl FnOnce(&mir::Tree, &ImmutableStringPool) -> T,
    ) -> T {
        // load the module for the target
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();

        // build the target id
        let target_id = self.target_id(module.package_id, target);

        // load the mir module state
        let profile = self.default_profile_id(module_id);
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);

        // run the callback while the tree is held
        f(&tree, &strings)
    }

    /// Create a fresh MIR interpreter for the module and target.
    pub fn mir_isolate(&self, module_id: ModuleId, target: &str) -> TestIsolate {
        let module = self.program.module_descriptor(module_id);
        let module = module.as_ref();
        let target_id = self.target_id(module.package_id, target);
        let profile = self.default_profile_id(module_id);
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);
        let mut isolate =
            Isolate::build_with_options(IsolateId::new(1), tree, strings, IsolateOptions::test())
                .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
        let mut statics = StaticSpace::empty();
        let (heap, shared) = create_test_heaps();
        let shared_gc = shared.gc_worker(0);

        isolate
            .initialize(&heap, &shared, &mut statics)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        TestIsolate {
            isolate,
            statics,
            heap,
            shared,
            shared_gc,
        }
    }

    /// Create a reusable MIR interpreter for repeated calls.
    pub fn mir_isolate_runner(&self, module_id: ModuleId, target: &str) -> TestIsolate {
        self.mir_isolate(module_id, target)
    }

    /// Run a MIR function by name and return its output value.
    pub fn run_mir_function(
        &self,
        module_id: ModuleId,
        target: &str,
        function: &str,
        arguments: &[Value],
    ) -> Value {
        let mut interpreter = self.mir_isolate_runner(module_id, target);
        interpreter.run_function_by_name(function, arguments)
    }

    /// Assert a MIR function's output matches the expected value.
    pub fn assert_mir_function_output(
        &self,
        module_id: ModuleId,
        target: &str,
        function: &str,
        arguments: &[Value],
        expected: Value,
    ) {
        let actual = self.run_mir_function(module_id, target, function, arguments);
        if actual != expected {
            panic!("mir function output mismatch\nexpected: {expected:?}\nactual: {actual:?}");
        }
    }

    /// Assert that a module's DIR has been elaborated to the given AST.
    pub fn assert_elaborated(&self, module_id: ModuleId, expected: &str) {
        let unbound = self.unbind_to_string(module_id);
        let unbound = unbound.trim();
        let expected = expected.trim();
        if unbound != expected {
            print_diff(expected, unbound, &DiffOptions::new());
            panic!("elaborated code mismatch");
        }
    }

    /// Assert that a module's DIR has been bound (and desugared) to the given AST.
    pub fn assert_bound(&self, module_id: ModuleId, expected: &str) {
        let unbound = self.unbind_to_string(module_id);
        let unbound = unbound.trim();
        let expected = expected.trim();
        if unbound != expected {
            print_diff(expected, unbound, &DiffOptions::new());
            panic!("bound code mismatch");
        }
    }

    /// Assert that a module's MIR matches the expected text format.
    pub fn assert_mir(&self, module_id: ModuleId, target: &str, expected: &str) {
        // normalize the actual mir
        let actual = self.mir_to_string(module_id, target);
        let actual = self.normalize_mir_actual(&actual);

        // normalize the expected mir
        let expected = self.normalize_mir_expected(expected);

        // compare formatted output
        if actual != expected {
            print_diff(&expected, &actual, &DiffOptions::new());
            panic!("mir code mismatch");
        }
    }

    /// Assert that a target-profile MIR artifact matches the expected text format.
    pub fn assert_target_mir(&self, module_id: ModuleId, target: &str, expected: &str) {
        let actual = self.target_mir_to_string(module_id, target);
        let actual = self.normalize_mir_actual(&actual);
        let expected = self.normalize_mir_expected(expected);

        if actual != expected {
            print_diff(&expected, &actual, &DiffOptions::new());
            panic!("mir code mismatch");
        }
    }

    /// Assert that a module's DIR has been patched after Execute.
    pub fn assert_executed(&self, module_id: ModuleId, expected: &str) {
        let unbound = self.unbind_to_string(module_id);
        let unbound = unbound.trim();
        let expected = expected.trim();
        if unbound != expected {
            print_diff(expected, unbound, &DiffOptions::new());
            panic!("executed code mismatch");
        }
    }

    /// Get a symbol by id.
    pub fn symbol_by_id(&self, symbol_id: GlobalSymbolId) -> Symbol {
        let profile = self.default_profile_id(symbol_id.module_id);
        let symbols = self.artifact_symbols(symbol_id.module_id, profile);

        symbols.get_symbol(symbol_id.into_local()).clone()
    }

    /// Get the root expression for a bound module.
    pub fn expect_root_expression(&self, module_id: ModuleId) -> LocalNodeId<Expression> {
        // load bound module state
        let dir = self.dir_base(module_id);
        // require a single root expression
        let roots = &dir.roots;
        if roots.len() != 1 {
            panic!("expected single root expression");
        }

        roots[0]
    }

    /// Get the nth function symbol declared in a module.
    pub fn expect_nth_function_symbol(&self, module_id: ModuleId, index: usize) -> GlobalSymbolId {
        let profile = self.default_profile_id(module_id);
        let tree = self.artifact_tree(module_id, profile);
        let roots = self.artifact_roots(module_id, profile);

        // scan for the first function declaration
        let mut current_index = 0;
        for root_id in &roots {
            let expression = tree.get(*root_id);

            // select the declaration expression if present
            let declaration_id = match expression {
                Expression::Declaration(declaration) => Some(*declaration),
                _ => None,
            };

            // return the first matching function declaration
            if let Some(declaration_id) = declaration_id {
                let declaration = tree.get(declaration_id);
                if let Declaration::Function(declaration) = declaration {
                    if current_index == index {
                        return declaration.symbol.into_global(module_id);
                    }
                    current_index += 1;
                }
            }
        }

        panic!("expected function declaration at index {index}");
    }

    /// Get the nth let declarator declared in a module.
    pub fn expect_nth_let_declarator(
        &self,
        module_id: ModuleId,
        index: usize,
    ) -> LocalNodeId<Declarator> {
        let profile = self.default_profile_id(module_id);
        let tree = self.artifact_tree(module_id, profile);
        let roots = self.artifact_roots(module_id, profile);

        // scan for the first let expression
        let mut current_index = 0;
        for root_id in &roots {
            let expression = tree.get(*root_id);

            // select the let expression if present
            let let_expression_id = match expression {
                Expression::Let { .. } => Some(*root_id),
                _ => None,
            };

            let Some(let_expression_id) = let_expression_id else {
                continue;
            };

            let Expression::Let { declarators, .. } = tree.get(let_expression_id) else {
                continue;
            };

            let declarator_id = declarators.first().copied();

            // return the first declarator
            if let Some(declarator_id) = declarator_id {
                if current_index == index {
                    return declarator_id;
                }
                current_index += 1;
            }
        }

        panic!("expected let expression at index {index}");
    }

    /// Get the symbol for a named interface member in a module.
    pub fn expect_interface_member_symbol(
        &self,
        module_id: ModuleId,
        interface_symbol: GlobalSymbolId,
        member_name: StringId,
    ) -> GlobalSymbolId {
        // load the module tree and symbol table
        let profile = self.default_profile_id(module_id);
        let tree = self.artifact_tree(module_id, profile);
        let symbols = self.artifact_symbols(module_id, profile);

        // resolve the interface declaration and scan members
        let interface_entry = symbols.get_symbol(interface_symbol.local_id);
        let interface_declaration_id = interface_entry
            .primary_declaration
            .expect("expected interface declaration")
            .into_local_typed::<Declaration>();
        let declaration = tree.get(interface_declaration_id);
        let members = match declaration {
            Declaration::Interface(declaration) => &declaration.members,
            _ => panic!("expected interface declaration"),
        };

        // find the first matching member name
        for member_id in members {
            let member = tree.get(*member_id);
            let member_name_id = member.key().and_then(|key| match key {
                Key::Name(name) => Some(name.string()),
                Key::Private(_) | Key::Expression(_) => None,
            });

            if member_name_id.is_some_and(|name| name == member_name) {
                return member.symbol().into_global(module_id);
            }
        }

        panic!("expected member symbol");
    }

    /// Get the nth lambda function symbol declared in a module.
    pub fn expect_nth_lambda_symbol(&self, module_id: ModuleId, index: usize) -> GlobalSymbolId {
        let profile = self.default_profile_id(module_id);
        let tree = self.artifact_tree(module_id, profile);

        // scan for the requested lambda declaration
        let mut current_index = 0;
        for (_, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            let Declaration::Function(declaration) = declaration else {
                continue;
            };

            if declaration.signature.kind == FunctionKind::Lambda {
                if current_index == index {
                    return declaration.symbol.into_global(module_id);
                }
                current_index += 1;
            }
        }

        panic!("expected lambda declaration at index {index}");
    }

    /// Get a capture set for a named function in a module.
    pub fn capture_set_for_function_name(&self, module_uri: &str, name: &str) -> CaptureSet {
        let symbol = self
            .function_symbol_by_name(module_uri, name)
            .unwrap_or_else(|| panic!("expected {name} symbol"));
        let module = self.module(module_uri);
        let module = module.as_ref();
        self.capture_set_for_symbol(module.id, symbol)
    }

    /// Get a capture set for a function symbol in a module.
    pub fn capture_set_for_symbol(
        &self,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
    ) -> CaptureSet {
        // load capture data for the module
        let profile = self.default_profile_id(module_id);
        let captures = self.artifact_captures(module_id, profile);

        // resolve the capture set
        captures
            .capture_set(symbol)
            .cloned()
            .unwrap_or_else(|| panic!("expected capture set for {symbol:?}"))
    }

    /// Get capture names and kinds for a capture set.
    pub fn capture_names_and_kinds(
        &self,
        module_id: ModuleId,
        capture_set: &CaptureSet,
    ) -> Vec<(String, CaptureKind)> {
        // load the module symbol table
        let profile = self.default_profile_id(module_id);
        let symbols = self.artifact_symbols(module_id, profile);

        // resolve capture names
        capture_set
            .captures
            .iter()
            .map(|binding| {
                let name = symbols
                    .get_symbol(binding.symbol.into_local())
                    .name()
                    .unwrap_or_else(|| panic!("expected named capture"));
                let name = self.program.strings.get(name);
                (name.to_string(), binding.kind)
            })
            .collect()
    }

    /// Get capture names for a capture set.
    pub fn capture_names(&self, module_id: ModuleId, capture_set: &CaptureSet) -> Vec<String> {
        // load the module symbol table
        let profile = self.default_profile_id(module_id);
        let symbols = self.artifact_symbols(module_id, profile);

        // resolve capture names
        capture_set
            .captures
            .iter()
            .map(|binding| {
                let name = symbols
                    .get_symbol(binding.symbol.into_local())
                    .name()
                    .unwrap_or_else(|| panic!("expected named capture"));
                self.program.strings.get(name).to_string()
            })
            .collect()
    }

    /// Get names for by-reference locals captured by a function.
    pub fn reference_local_names(
        &self,
        module_id: ModuleId,
        owner_symbol: GlobalSymbolId,
    ) -> Vec<String> {
        // load the module symbol table
        let profile = self.default_profile_id(module_id);
        let symbols = self.artifact_symbols(module_id, profile);
        let captures = self.artifact_captures(module_id, profile);

        // resolve captured local names
        let reference_locals = captures.reference_locals(owner_symbol).unwrap_or(&[]);
        reference_locals
            .iter()
            .map(|symbol| {
                let name = symbols
                    .get_symbol(symbol.into_local())
                    .name()
                    .unwrap_or_else(|| panic!("expected named symbol"));
                self.program.strings.get(name).to_string()
            })
            .collect()
    }

    /// Resolve the first decorator for a named declaration.
    pub fn decorator_info_for_declaration(&self, module_uri: &str, name: &str) -> DecoratorInfo {
        // resolve the declaration symbol
        let declaration_symbol = self
            .declaration_symbol_by_name(module_uri, name)
            .unwrap_or_else(|| panic!("expected {name} symbol"));

        // load the module tree and symbols
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let tree = self.artifact_tree(module.id, profile);
        let symbols = self.artifact_symbols(module.id, profile);

        // resolve the declaration
        let declaration_entry = symbols.get_symbol(declaration_symbol.into_local());
        let Some(declaration) = declaration_entry.primary_declaration else {
            panic!("expected declaration for {name}");
        };

        // locate the decorator annotation
        let mut decorators = tree.get_decorators(declaration.local_id.id);
        if decorators.is_empty() {
            let wrapper_id = tree
                .iter_nodes_of_type::<Expression>()
                .find_map(|(expression_id, expression)| match expression {
                    Expression::Declaration(inner) if inner.id == declaration.local_id.id => {
                        Some(expression_id)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("expected declaration wrapper for {name}"));
            decorators = tree.get_decorators(wrapper_id.id);
        }
        let decorator_id = decorators
            .iter()
            .copied()
            .next()
            .unwrap_or_else(|| panic!("expected decorator annotation for {name}"));

        // resolve decorator metadata
        let decorator = tree.get(decorator_id);
        let call = self.compiler.decorator_call(&tree, decorator.expression);

        // resolve the decorator target symbol
        let target_symbol = match tree.get(call.callee) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            other => panic!("expected resolved decorator reference, got {other:?}"),
        };

        // resolve the first argument kind
        let first_argument_is_string_literal = call
            .arguments
            .and_then(|arguments| arguments.first().copied())
            .and_then(|argument_id| match tree.get(argument_id) {
                Argument::Positional { value, .. }
                | Argument::Named { value, .. }
                | Argument::Labeled { value, .. } => Some(*value),
                Argument::Error { .. } => None,
                Argument::Spread { .. } => {
                    panic!("unexpected spread decorator argument for {name}");
                }
            })
            .is_some_and(|value_id| {
                matches!(
                    tree.get(value_id),
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::String(_)
                    }
                )
            });

        DecoratorInfo {
            target_symbol,
            first_argument_is_string_literal,
        }
    }
}

/// Create local and shared test heaps over one allocator.
fn create_test_heaps() -> (Heap, SharedHeap) {
    let local_options = HeapOptions::local();
    let shared_options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(
            local_options.page_bytes,
            local_options.allocator_chunk_bytes,
        )
        .expect("test allocator should build"),
    );
    let heap = Heap::with_allocator_limits_and_options(
        allocator.clone(),
        HeapLimits::default(),
        local_options,
    )
    .expect("test heap should build");
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        shared_options,
    )
    .expect("test shared heap should build");

    (heap, shared)
}

/// Format one compact compiler stats snapshot for timeout diagnostics.
fn format_stats_snapshot(snapshot: &crate::StatsSnapshot) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "stats: elapsed={:?}", snapshot.elapsed);
    let _ = writeln!(
        output,
        "modules: parsed={} bound={} resolved={} analyzed={} elaborated={} executed={} lowered={} optimized={} generated={}",
        snapshot.modules.parsed,
        snapshot.modules.bound,
        snapshot.modules.resolved,
        snapshot.modules.analyzed,
        snapshot.modules.elaborated,
        snapshot.modules.executed,
        snapshot.modules.lowered,
        snapshot.modules.optimized,
        snapshot.modules.generated
    );
    let _ = writeln!(
        output,
        "cache: ast_hits(memory={}) dir_hits(memory={}) mir_hits(memory={}) misses(ast={},dir={},mir={}) writes(ast_memory={},dir_memory={},mir_memory={})",
        snapshot.cache.ast_hits_memory,
        snapshot.cache.dir_hits_memory,
        snapshot.cache.mir_hits_memory,
        snapshot.cache.ast_misses,
        snapshot.cache.dir_misses,
        snapshot.cache.mir_misses,
        snapshot.cache.ast_writes_memory,
        snapshot.cache.dir_writes_memory,
        snapshot.cache.mir_writes_memory
    );

    output
}

/// Normalize MIR text for comparison.
fn normalize_mir_text_with_options(
    label: &str,
    mir_text: &str,
    options: MirFormatOptions,
) -> String {
    let mir_text = mir_text.trim();

    let (tree, strings) = mir::parse::Parser::parse(
        destack_source::FileId::new(0),
        mir_text,
        mir::parse::ParseOptions::default(),
    )
    .validate()
    .unwrap_or_else(|error| panic!("{label} mir parse failed: {error}"));

    format_mir(&tree, &strings, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reject malformed emitted MIR during actual normalization.
    #[test]
    #[should_panic(expected = "actual mir parse failed")]
    fn test_rejects_invalid_actual_mir_during_normalization() {
        normalize_mir_text_with_options(
            "actual",
            r#"
function broken(): void {
b0():
    v0: ref<usize1], raw, space(stack)> = stack.alloc usize[1]
    return
}
            "#,
            MirFormatOptions::default(),
        );
    }
}
