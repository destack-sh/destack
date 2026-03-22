#![allow(dead_code)]

use std::env::current_dir;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use destack_ast::NodeParentIndex;
use destack_core::ImmutableStringPool;
use destack_dir::{
    Annotation, Argument, CaptureKind, CaptureSet, CaptureTable, Declaration, Declarator,
    DumperOptions, DynamicKey, Expression, FunctionKind, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, NodeTree, Pattern, ScalarLiteral, StringId, Symbol, SymbolTable,
    TypeTable,
};
use destack_formatter::{DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions};
use destack_linter::Linter;
use destack_mir as mir;
use destack_mir::{MirFormatOptions, format_mir};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileId, FileSystem, FileType,
    MemoryFileSystem, ModuleId, ModuleStamp, ModuleVersion, MultiSpan, PackageId, PackageStamp,
    PackageVersion, PhysicalFileSystem, PrintOptions, ProfileStamp, ProfileVersion, Uri,
    print_diagnostics, print_diff,
};
use destack_vm::{Heap, Isolate, IsolateOptions, MemoryContext, SharedSpace, Value};
use destack_workspace::{
    ArtifactKey, CacheMode, CacheStore, Destack, DestackJson, DestackOptions, DirAnalyzed, DirBase,
    DirDeclared, DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved, DiskCacheStore,
    EmitFormat, ExportedSymbolTable, MemoryCacheStore, Module, ProfileId, Program, Session, Target,
    TargetId,
};
use serde_json::json;

use crate::{
    AnalyzeOptions, ArtifactTaskKeyExt, Compiler, CompilerOptions, TaskPhase, default_workers,
};

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

    /// Return a cache store suited to this file system.
    pub fn cache_store(&self) -> Arc<dyn CacheStore> {
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
    /// The cloned node tree.
    pub(crate) tree: NodeTree,
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

/// A test wrapper for a Program.
#[derive(Debug)]
pub struct TestProgram {
    /// The file system.
    pub fs: TestFileSystem,
    /// The session (holds shared state like builtins).
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    pub compiler: Arc<Compiler>,
    /// The dumper options.
    pub dumper_options: DumperOptions,
    /// Optional override for the default profile in tests.
    pub default_profile_override: Option<ProfileId>,
}

/// Resolve a root expression id, unwrapping statement wrappers.
pub fn root_expression_id(
    roots: &[LocalNodeId<Expression>],
    tree: &NodeTree,
    index: usize,
) -> LocalNodeId<Expression> {
    // select the requested root
    let root_id = roots
        .get(index)
        .copied()
        .unwrap_or_else(|| panic!("missing root at index {index}"));

    // unwrap statement roots
    let expression = tree.get(root_id);
    match expression {
        Expression::Statement { statement } => *statement,
        _ => root_id,
    }
}

/// Find a let declarator by binding name.
pub fn expect_let_declarator_by_name(
    roots: &[LocalNodeId<Expression>],
    tree: &NodeTree,
    name: StringId,
) -> LocalNodeId<Declarator> {
    // scan for a matching let declarator
    for root_id in roots {
        let let_expression_id = match tree.get(*root_id) {
            Expression::Let { .. } => Some(*root_id),
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::Let { .. } => Some(*statement),
                _ => None,
            },
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
    /// The authoritative heap for the isolate.
    heap: Heap,
    /// The world-shared memory for the isolate.
    shared: SharedSpace,
}

impl TestIsolate {
    /// Run a MIR function by name and return the full execution output.
    pub fn run_function_by_name_output(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> destack_vm::RuntimeResult<destack_vm::ExecutionOutput> {
        let mut memory = MemoryContext::new(&mut self.heap, &mut self.shared);

        self.isolate
            .run_function_by_name(&mut memory, function, arguments)
    }

    /// Run a MIR function by name and return its output value.
    pub fn run_function_by_name(&mut self, function: &str, arguments: &[Value]) -> Value {
        let output = self
            .run_function_by_name_output(function, arguments)
            .expect("execution failed");

        output.value
    }

    /// Read one VM string value through the authoritative heap.
    pub fn string_value(&self, value: Value) -> Result<String, destack_vm::Error> {
        self.isolate.string_value(&self.heap, value)
    }
}

impl TestProgram {
    /// Return the declared DIR data for one module and profile.
    pub(crate) fn artifact_dir_declared_data(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> DirDeclared {
        self.program
            .artifacts
            .dir_declared(module_id, profile)
            .map(|dir| dir.as_ref().clone())
            .unwrap_or_else(|| panic!("missing declared dir for module {module_id:?}"))
    }

    /// Clone the latest published DIR forward into one patched artifact.
    fn artifact_dir_exact_maybe(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Option<DirPatched> {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return Some(dir.as_ref().clone());
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return Some(DirPatched::from_elaborated_with(
                dir.as_ref(),
                dir.tree.as_ref().clone(),
            ));
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
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

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            let declared = self
                .program
                .artifacts
                .dir_declared(module_id, profile)
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

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            let resolved = self
                .program
                .artifacts
                .dir_resolved(module_id, profile)
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

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
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

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            let resolved = DirResolved::from_prepared_with(
                dir.as_ref(),
                dir.tree.as_ref().clone(),
                dir.symbols.as_ref().clone(),
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

        self.program.artifacts.dir_base(module_id).map(|dir| {
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

    /// Return one cloned test DIR with its stable source id and analyze options.
    pub(crate) fn artifact_dir_context(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> (Arc<Module>, TestDir, LocalNodeIdAny, AnalyzeOptions) {
        let module = self.program.modules.get(module_id);
        let dir = self.artifact_dir(module_id, profile);
        let source_id = dir.roots[0].into_any();
        let options = self.compiler.analyze_context_options_for_module(module.id);

        (module, dir, source_id, options)
    }

    /// Clone the latest published DIR tree for one module and profile.
    pub(crate) fn artifact_tree(&self, module_id: ModuleId, profile: ProfileId) -> NodeTree {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.tree.as_ref().clone();
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.tree.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact tree for module {module_id:?}"))
    }

    /// Clone the latest published DIR symbols for one module and profile.
    pub(crate) fn artifact_symbols(&self, module_id: ModuleId, profile: ProfileId) -> SymbolTable {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.symbols.as_ref().clone();
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.symbols.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact symbols for module {module_id:?}"))
    }

    /// Clone the latest published DIR types for one module and profile.
    pub(crate) fn artifact_types(&self, module_id: ModuleId, profile: ProfileId) -> TypeTable {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.types.as_ref().clone();
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.types.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact types for module {module_id:?}"))
    }

    /// Clone the latest published DIR roots for one module and profile.
    pub(crate) fn artifact_roots(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Vec<LocalNodeId<Expression>> {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.roots.as_ref().clone();
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.roots.as_ref().clone())
            .unwrap_or_else(|| panic!("missing artifact roots for module {module_id:?}"))
    }

    /// Clone the latest published DIR captures for one module and profile.
    pub(crate) fn artifact_captures(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> CaptureTable {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.captures.as_ref().clone();
        }

        if self
            .program
            .artifacts
            .dir_interface(module_id, profile)
            .is_some()
            && let Some(dir) = self.program.artifacts.dir_declared(module_id, profile)
        {
            return dir.captures.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
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
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.namespace_scope;
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.namespace_scope;
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.namespace_scope)
            .unwrap_or_else(|| panic!("missing artifact namespace scope for module {module_id:?}"))
    }

    /// Return the latest published anchor node for one module and profile.
    pub(crate) fn artifact_anchor_node(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> LocalNodeIdAny {
        if let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.anchor_node;
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.anchor_node;
        }

        self.program
            .artifacts
            .dir_base(module_id)
            .map(|dir| dir.anchor_node)
            .unwrap_or_else(|| panic!("missing artifact anchor node for module {module_id:?}"))
    }

    /// Clone the latest exported-symbol table for one module and profile.
    pub(crate) fn artifact_exported_symbols(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ExportedSymbolTable {
        if let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) {
            return dir.exported_symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) {
            return dir.exported_symbols.as_ref().clone();
        }

        if let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) {
            return dir.exported_symbols.as_ref().clone();
        }

        ExportedSymbolTable::new()
    }

    /// Return the resolved DIR artifact for one module's default profile.
    pub(crate) fn dir_resolved(&self, module_id: ModuleId) -> DirResolved {
        let profile = self.default_profile_id(module_id);
        let dir = self
            .program
            .artifacts
            .dir_resolved(module_id, profile)
            .unwrap_or_else(|| panic!("missing resolved dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Return the base DIR artifact for one module.
    pub(crate) fn dir_base(&self, module_id: ModuleId) -> DirBase {
        let dir = self
            .program
            .artifacts
            .dir_base(module_id)
            .unwrap_or_else(|| panic!("missing base dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Return the declared DIR artifact for one module's default profile.
    pub(crate) fn dir_declared(&self, module_id: ModuleId) -> DirDeclared {
        let profile = self.default_profile_id(module_id);
        let dir = self
            .program
            .artifacts
            .dir_declared(module_id, profile)
            .unwrap_or_else(|| panic!("missing declared dir for module {module_id:?}"));

        dir.as_ref().clone()
    }

    /// Clone the published MIR tree and strings for one module, profile, and target.
    pub(crate) fn artifact_mir_parts(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
    ) -> (mir::NodeTree, ImmutableStringPool) {
        if let Some(mir) = self
            .program
            .artifacts
            .mir_optimized(module_id, profile, target_id)
        {
            return (mir.tree.clone(), mir.strings.clone().into_immutable());
        }

        let mir = self
            .program
            .artifacts
            .mir_base(module_id, profile, target_id)
            .unwrap_or_else(|| panic!("missing artifact mir for module {module_id:?}"));

        (mir.tree.clone(), mir.strings.clone().into_immutable())
    }

    /// Create a new TestProgram with the given options.
    fn new(fs: TestFileSystem, workers: u16, inject_prelude: bool, load_libraries: bool) -> Self {
        init_tracing();
        let root_directory = match &fs {
            TestFileSystem::Memory { .. } => current_dir().unwrap(),
            TestFileSystem::Physical { root_directory, .. } => root_directory.clone(),
        };

        // seed a default package name for in memory tests
        if let TestFileSystem::Memory { fs } = &fs {
            fs.add_file("package.json", br#"{ "name": "test" }"#)
                .unwrap_or_else(|_| {
                    panic!("failed to add package.json to memory file system");
                });
        }

        let cache_store = fs.cache_store();
        let session = Arc::new(
            Session::new(root_directory.clone())
                .with_fs(fs.fs())
                .with_cache_store(cache_store),
        );
        let program = session.add_root(root_directory);

        let compiler_options = CompilerOptions {
            workers,
            inject_prelude,
            load_libraries,
            elaborate_parenthesize_casts: true,
            ..CompilerOptions::default()
        };
        let compiler = Arc::new(Compiler::new(
            session.clone(),
            program.clone(),
            compiler_options,
        ));

        Self {
            fs,
            session,
            program,
            compiler,
            dumper_options: DumperOptions::default(),
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
        let profile_key = self.program.profile(profile_id).key.clone();
        self.session
            .load_library(name, &profile_key)
            .unwrap_or_else(|| panic!("missing builtin library '{name}'"));
        self
    }

    /// Override the default profile with an explicit library set.
    pub fn with_profile_libs(mut self, libs: &[&str]) -> Self {
        let default_profile = self.program.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.lib = libs.iter().map(|lib| (*lib).to_string()).collect();
        let profile_id = self.program.profiles.get_or_create(key);
        self.default_profile_override = Some(profile_id);
        self
    }

    /// Override the default profile with an explicit emit format.
    pub fn with_profile_emit(mut self, emit: EmitFormat) -> Self {
        let default_profile = self.program.profile(self.default_profile_id_for_root());
        let mut key = default_profile.key.clone();
        key.emit = emit;
        let profile_id = self.program.profiles.get_or_create(key);
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
        match &self.fs {
            TestFileSystem::Memory { fs } => {
                fs.add_file(path, content.as_bytes()).unwrap_or_else(|_| {
                    panic!("failed to add test file '{path}' to memory file system")
                });
            }
            TestFileSystem::Physical { .. } => {
                panic!("cannot add test file '{path}' to physical file system");
            }
        }
    }

    /// Add a package.json and optionally destack.json to the memory filesystem.
    ///
    /// `destack_config_compiler_options` is the raw JSON content for `compilerOptions`, e.g.:
    /// ```ignore
    /// test.add_package("my-pkg", Some(r#""noRedeclaredLocals": true"#));
    /// ```
    pub fn add_package(&self, name: &str, destack_config_compiler_options: Option<&str>) {
        self.add_file("package.json", &format!(r#"{{ "name": "{name}" }}"#));
        if let Some(opts) = destack_config_compiler_options {
            self.add_file(
                "destack.json",
                &format!(r#"{{ "compilerOptions": {{ {opts} }} }}"#),
            );
        }
    }

    /// Add a destack.json to the memory filesystem.
    pub fn add_destack_config(&self, content: &str) {
        self.add_file("destack.json", content);
    }

    /// Set the cache mode for this test program.
    pub fn with_cache_mode(self, mode: CacheMode, dir: Option<&str>) -> Self {
        // map cache mode to json value
        let mode_value = match mode {
            CacheMode::Off => "off",
            CacheMode::Memory => "memory",
            CacheMode::Disk => "disk",
        };

        // build config json value
        let config_value = if let Some(dir) = dir {
            json!({ "cache": { "mode": mode_value, "dir": dir } })
        } else {
            json!({ "cache": { "mode": mode_value } })
        };
        let config = config_value.to_string();

        // write config to memory fs
        self.add_destack_config(&config);

        self
    }

    /// Add a file and register a blank module for it (no import/parsing yet).
    pub fn add_module(&self, path: &str, content: &str) -> ModuleId {
        self.add_file(path, content);
        self.compiler
            .resolve_path_to_module(&PathBuf::from(path))
            .unwrap_or_else(|e| panic!("failed to register module: {e:?}"))
    }

    /// Get the default profile id for a module.
    pub fn default_profile_id(&self, module_id: ModuleId) -> ProfileId {
        self.default_profile_override
            .unwrap_or_else(|| self.program.default_profile_id_for_module(module_id))
    }

    /// Get the default profile id for the root module.
    pub fn default_profile_id_for_root(&self) -> ProfileId {
        self.default_profile_override.unwrap_or_else(|| {
            self.program
                .default_profile_id_for_module(self.program.root_module_id)
        })
    }

    /// Get the current module version.
    pub fn module_version(&self, module_id: ModuleId) -> ModuleVersion {
        self.program.modules.version(module_id)
    }

    /// Get the current module stamp.
    pub fn module_stamp(&self, module_id: ModuleId) -> ModuleStamp {
        ModuleStamp::new(module_id, self.module_version(module_id))
    }

    /// Get the current profile version.
    pub fn profile_version(&self, profile_id: ProfileId) -> ProfileVersion {
        self.program
            .profiles
            .get(profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {profile_id:?}"))
            .version
    }

    /// Get the current profile stamp.
    pub fn profile_stamp(&self, profile_id: ProfileId) -> ProfileStamp {
        ProfileStamp::new(profile_id, self.profile_version(profile_id))
    }

    /// Get the current package version.
    pub fn package_version(&self, package_id: PackageId) -> PackageVersion {
        self.program.packages.version(package_id)
    }

    /// Get the current package stamp.
    pub fn package_stamp(&self, package_id: PackageId) -> PackageStamp {
        PackageStamp::new(package_id, self.package_version(package_id))
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
        self.compiler
            .run_to_completion(|compiler| compiler.require_language_environment(profile))
            .unwrap_or_else(|error| panic!("failed to resolve language environment: {error:?}"));
    }

    /// Resolve builtin libraries for the default root profile.
    pub fn resolve_libs(&self) {
        let profile = self.default_profile_id_for_root();
        self.compiler
            .run_to_completion(|compiler| compiler.require_library_environment(profile))
            .unwrap_or_else(|error| panic!("failed to resolve libs: {error:?}"));
    }

    /// Enqueue Analyze task for a module.
    pub fn analyze_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.enqueue(ArtifactKey::dir_analyzed(module, profile));
    }

    /// Drive declaration analysis for one module to completion.
    pub fn declare_module(&self, module: ModuleId) {
        let profile = self.default_profile_id(module);
        self.compiler
            .run_to_completion(|compiler| compiler.require_dir_declared(module, profile))
            .unwrap_or_else(|error| panic!("failed to declare module {module:?}: {error:?}"));
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
        self.enqueue(ArtifactKey::dir_analyzed(module, profile));
        self.compile();

        let linter = Linter::new(self.program.clone());
        linter
            .lint_module(module, profile)
            .unwrap_or_else(|error| panic!("failed to lint module {module:?}: {error}"));
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
        let module_ref = self.program.modules.get(module);
        let package_id = module_ref.package_id;
        let target_id = TargetId::new(package_id, name);
        let target_config = Target::implicit_for_name(name)
            .unwrap_or_else(|| panic!("unknown implicit target '{name}'"));

        let package = self.program.packages.get(package_id);
        let mut package = package.write();
        package.targets.insert(target_id, target_config);

        // bump package version for target updates
        drop(package);
        let _ = self.program.packages.bump_version(package_id);
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
        let module_ref = self.program.modules.get(module);
        let package_id = module_ref.package_id;
        let target_id = TargetId::new(package_id, name);
        let target_config = Target::implicit_for_name(name)
            .unwrap_or_else(|| panic!("unknown implicit target '{name}'"));

        let package = self.program.packages.get(package_id);
        let mut package = package.write();
        let target = package.targets.entry(target_id).or_insert(target_config);
        configure(target);

        // bump package version for target updates
        drop(package);
        let _ = self.program.packages.bump_version(package_id);
    }

    /// Apply a destack.json blob to the package containing the given module.
    pub fn apply_destack_config(&self, module: ModuleId, content: &str) {
        let module_ref = self.program.modules.get(module);
        let package_id = module_ref.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let directory = package
            .path
            .clone()
            .or_else(|| {
                module_ref
                    .path
                    .as_ref()
                    .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            })
            .unwrap_or_else(|| self.program.cwd.clone());
        drop(package);

        let path = directory.join("destack.json");
        self.add_file(path.to_string_lossy().as_ref(), content);

        let config_json: DestackJson = serde_json::from_str(content)
            .unwrap_or_else(|error| panic!("invalid destack.json: {error}"));
        let config = self
            .session
            .load_destack_for_path(&path)
            .unwrap_or_else(|| {
                let options = DestackOptions::from(&config_json);
                let file_id = self.program.files.next_id();
                Destack {
                    file_id,
                    path: path.clone(),
                    directory: directory.clone(),
                    options,
                    content: config_json,
                }
            });

        let package = self.program.packages.get(package_id);
        let mut package = package.write();
        package.config = Some(config);

        // bump package version for config updates
        drop(package);
        let _ = self.program.packages.bump_version(package_id);
    }

    /// Enqueue Lower task for a module.
    pub fn lower_module(&self, module: ModuleId, target: &str) {
        let module_ref = self.program.modules.get(module);
        let package_id = module_ref.package_id;
        let target_id = TargetId::new(package_id, target);
        let target_config = Target::implicit_for_name(target)
            .unwrap_or_else(|| panic!("unknown implicit target '{target}'"));

        let package = self.program.packages.get(package_id);
        {
            let mut package = package.write();
            package
                .targets
                .entry(target_id.clone())
                .or_insert(target_config);
        }

        let profile = self
            .program
            .profile_id_for_target(module, &target_id)
            .unwrap_or_else(|| panic!("missing profile for target '{target}'"));
        self.enqueue(ArtifactKey::mir_base(module, profile, target_id));
    }

    /// Enqueue Optimize task for a module.
    pub fn optimize_module(&self, module: ModuleId, target: &str) {
        let module_ref = self.program.modules.get(module);
        let package_id = module_ref.package_id;
        let target_id = TargetId::new(package_id, target);
        let profile = self
            .program
            .profile_id_for_target(module, &target_id)
            .unwrap_or_else(|| panic!("missing profile for target '{target}'"));
        self.enqueue(ArtifactKey::mir_optimized(module, profile, target_id));
    }

    /// Enqueue one artifact key.
    pub fn enqueue(&self, artifact_key: ArtifactKey) {
        self.compiler.enqueue(artifact_key);
    }

    /// Enqueue one artifact key (does not run it).
    pub fn enqueue_artifact<T: Into<ArtifactKey>>(&self, artifact_key: T) {
        self.compiler.enqueue(artifact_key);
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
        // only serialize tests that also run a parallel compiler
        let compile_guard = (self.compiler.options.workers > 1).then(|| {
            TEST_COMPILE_LOCK
                .lock()
                .unwrap_or_else(|error| error.into_inner())
        });

        // spawn the compile thread
        let compiler = self.compiler.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            compiler.compile();
            let _ = tx.send(());
        });

        match rx.recv_timeout(timeout) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let snapshot = self
                    .compiler
                    .stats
                    .snapshot_with_program(self.program.modules.len(), Some(&self.program));
                let summary = format_stats_snapshot(&snapshot);
                let tasks = self
                    .compiler
                    .task_handles()
                    .into_iter()
                    .map(|handle| {
                        let description = handle.artifact_key.trace_args(&self.program);
                        format!(
                            "  {:?} {:?} {} yields={} final_requirements={:?} last_outcome={:?}",
                            handle.id,
                            handle.status,
                            description,
                            handle.yield_count,
                            handle.final_requirements,
                            handle.last_outcome,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!("compile timed out after {timeout:?}\n{summary}\ntasks:\n{tasks}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("compile thread panicked");
            }
        }

        drop(compile_guard);
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

    /// Compile, dump and check no diagnostics.
    pub fn compile_dump_clean(&self) {
        self.compile();
        self.dump();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
    }

    /// Compile, check no diagnostics and dump output.
    pub fn compile_dump_check(&self) {
        self.compile();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
        self.dump();
    }

    /// Compile, dump and ignore diagnostics.
    pub fn compile_dump(&self) {
        self.compile();
        self.dump();
    }

    /// Get the file for a module.
    pub fn file(&self, module: ModuleId) -> Arc<File> {
        let module = self.program.modules.get(module);
        self.program.files.get(module.file_id)
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
        f: impl FnOnce(&Module, ProfileId, &TestDir, &NodeTree, &SymbolTable, &TypeTable) -> T,
    ) -> T {
        let profile = self.default_profile_id(module_id);
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let dir = self.artifact_dir(module_id, profile);

        f(module, profile, &dir, &dir.tree, &dir.symbols, &dir.types)
    }

    /// Run a closure with mutable access to one module's cloned type table.
    pub(crate) fn with_dir_types_mut<T>(
        &self,
        module_id: ModuleId,
        f: impl FnOnce(&Module, ProfileId, &TestDir, &NodeTree, &SymbolTable, &mut TypeTable) -> T,
    ) -> T {
        let profile = self.default_profile_id(module_id);
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let dir = self.artifact_dir(module_id, profile);
        let mut types = dir.types.clone();

        f(module, profile, &dir, &dir.tree, &dir.symbols, &mut types)
    }

    /// Get a module by URI.
    pub fn module(&self, module_uri: &str) -> Arc<Module> {
        self.program
            .modules
            .get_by_uri(&Uri::from_string(module_uri))
            .unwrap_or_else(|| panic!("module not found for '{module_uri}'"))
    }

    /// Get a module by file.
    pub fn module_for_file(&self, file: &File) -> Arc<Module> {
        self.program
            .modules
            .get_by_uri(&file.uri)
            .unwrap_or_else(|| panic!("module not found for file: '{}'", file.uri))
    }

    /// Check no diagnostics of at least the given severity.
    pub fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.program.diagnostics.collect();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "program has {} unexpected {severity_name}s",
                diagnostics.len()
            );
        }
    }

    /// Check that no diagnostics with the given prefix are present.
    pub fn check_no_diagnostics_with_prefix(&self, prefix: &str) {
        let diagnostics = self.program.diagnostics.collect();
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
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &matching, options);
            panic!("unexpected diagnostics with prefix '{prefix}'");
        }
    }

    /// Check that no diagnostics for the given phases are present.
    pub fn check_no_diagnostics_for_phases(&self, phases: &[TaskPhase]) {
        // build prefixes for both errors and warnings for each phase
        let prefixes: Vec<String> = phases
            .iter()
            .flat_map(|phase| {
                let letter = phase.letter();
                [format!("E{letter}"), format!("W{letter}")]
            })
            .collect();

        // check for any matching diagnostics
        let diagnostics = self.program.diagnostics.collect();
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
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &matching_diagnostics, options);
            let phase_names: Vec<&str> = phases.iter().map(|p| p.name()).collect();
            panic!("unexpected diagnostics for phases: {phase_names:?}");
        }
    }

    /// Check that no diagnostics up to (but not including) the given phase are present.
    pub fn check_no_diagnostics_up_to_excluding_phase(&self, phase: TaskPhase) {
        let phases: Vec<TaskPhase> = TaskPhase::all().take_while(|p| *p != phase).collect();
        self.check_no_diagnostics_for_phases(&phases);
    }

    /// Check that no diagnostics up to and including the given phase are present.
    pub fn check_no_diagnostics_up_to_including_phase(&self, phase: TaskPhase) {
        let phases: Vec<TaskPhase> = TaskPhase::all()
            .take_while(|p| p.code() <= phase.code())
            .collect();
        self.check_no_diagnostics_for_phases(&phases);
    }

    /// Check that exactly the given diagnostics are present (by code).
    /// Panics if the actual diagnostics don't match.
    pub fn check_has_diagnostics(&self, expected_codes: &[&str]) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
        if actual_codes != expected_codes {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            panic!("diagnostic mismatch\nexpected: {expected_codes:?}\nactual: {actual_codes:?}");
        }
    }

    /// Check that a diagnostic with the given code is present.
    pub fn check_has_diagnostic(&self, code: &str) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let has_code = diagnostic_vec.iter().any(|d| d.code == code);
        if !has_code {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
            panic!("expected diagnostic with code '{code}' but found: {actual_codes:?}");
        }
    }

    /// Check that a diagnostic code appears the expected number of times.
    pub fn check_diagnostic_count(&self, code: &str, expected: usize) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let count = diagnostic_vec.iter().filter(|d| d.code == code).count();
        if count != expected {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            panic!("expected {expected} diagnostics for '{code}', found {count}");
        }
    }

    /// Check that no diagnostic with the given code is present.
    pub fn check_no_diagnostic_code(&self, code: &str) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let has_code = diagnostic_vec.iter().any(|d| d.code == code);
        if has_code {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            panic!("unexpected diagnostic with code '{code}'");
        }
    }

    /// Unbind a module's DIR back to AST and format it to a string.
    pub fn unbind_to_string(&self, module_id: ModuleId) -> String {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let tree = self.artifact_tree(module.id, profile);
        let symbols = self.artifact_symbols(module.id, profile);
        let roots = self.artifact_roots(module.id, profile);
        let anchor_node = self.artifact_anchor_node(module.id, profile);

        // unbind DIR to AST
        let fallback_node = roots
            .first()
            .copied()
            .map(LocalNodeId::into_any)
            .unwrap_or(anchor_node);
        let unbound = self.compiler.unbind_module_from_parts(
            &module,
            &tree,
            &symbols,
            &roots,
            fallback_node,
            profile,
        );

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
            DestackFormatArtifacts {
                file: &file,
                tree: &unbound.tree,
                tokens: &empty_tokens,
                side_tokens: &empty_side_tokens,
                side_span: &empty_side_span,
                strings: &strings,
                parents: NodeParentIndex::from_tree(&unbound.tree),
            },
        );

        // format each root expression and join with blank lines
        let mut results = Vec::new();
        for root_id in &unbound.roots {
            let formatted = destack_fir::format!(context.clone(), [root_id]).unwrap();
            let printed = formatted.print().unwrap();
            results.push(printed.into_str());
        }
        results.join("\n\n")
    }

    /// Format a module's MIR to a string.
    pub fn mir_to_string(&self, module_id: ModuleId, target: &str) -> String {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let target_id = TargetId::new(module.package_id, target);
        let profile = self.default_profile_id(module_id);
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

    /// Normalize expected MIR text for comparison.
    fn normalize_mir_expected_with_options(
        &self,
        expected: &str,
        options: MirFormatOptions,
    ) -> String {
        // trim the expected input
        let expected = expected.trim();

        // parse and reformat to the canonical layout
        let (tree, strings) = mir::parse::Parser::parse(
            destack_source::FileId::new(0),
            expected,
            mir::parse::ParseOptions::default(),
        )
        .unwrap_or_else(|error| panic!("expected mir parse failed: {error}"));
        format_mir(&tree, &strings, options)
    }

    /// Normalize expected MIR text for comparison.
    fn normalize_mir_expected(&self, expected: &str) -> String {
        self.normalize_mir_expected_with_options(expected, self.mir_format_options())
    }

    /// Run a callback with the MIR tree and strings for a lowered target.
    pub fn with_mir_tree<T>(
        &self,
        module_id: ModuleId,
        target: &str,
        f: impl FnOnce(&mir::NodeTree, &ImmutableStringPool) -> T,
    ) -> T {
        // load the module for the target
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();

        // build the target id
        let target_id = TargetId::new(module.package_id, target);

        // load the mir module state
        let profile = self.default_profile_id(module_id);
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);

        // run the callback while the tree is held
        f(&tree, &strings)
    }

    /// Create a fresh MIR interpreter for the module and target.
    pub fn mir_isolate(&self, module_id: ModuleId, target: &str) -> TestIsolate {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let target_id = TargetId::new(module.package_id, target);
        let profile = self.default_profile_id(module_id);
        let (tree, strings) = self.artifact_mir_parts(module_id, profile, &target_id);
        let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));

        let heap = Heap::new();
        let mut heap = heap;
        let shared = SharedSpace::new();
        let mut shared = shared;
        let mut memory = MemoryContext::new(&mut heap, &mut shared);

        isolate
            .initialize(&mut memory)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        TestIsolate {
            isolate,
            heap,
            shared,
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
        // format the actual mir
        let actual = self.mir_to_string(module_id, target);
        let actual = actual.trim().to_string();

        // normalize the expected mir
        let expected = self.normalize_mir_expected(expected);

        // compare formatted output
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

        // unwrap statement roots to their inner expression
        let root_id = roots[0];
        let expression = dir.tree.get(root_id);
        match expression {
            Expression::Statement { statement } => *statement,
            _ => root_id,
        }
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
                Expression::Declaration { declaration } => Some(*declaration),
                Expression::Statement { statement } => match tree.get(*statement) {
                    Expression::Declaration { declaration } => Some(*declaration),
                    _ => None,
                },
                _ => None,
            };

            // return the first matching function declaration
            if let Some(declaration_id) = declaration_id {
                let declaration = tree.get(declaration_id);
                if let Declaration::Function { descriptor, .. } = declaration {
                    if current_index == index {
                        return descriptor.symbol.into_global(module_id);
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
                Expression::Statement { statement } => match tree.get(*statement) {
                    Expression::Let { .. } => Some(*statement),
                    _ => None,
                },
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
            Declaration::Interface { members, .. } => members,
            _ => panic!("expected interface declaration"),
        };

        // find the first matching member name
        for member_id in members {
            let member = tree.get(*member_id);
            let member_name_id = member.key().and_then(|key| match key {
                DynamicKey::Name(name) => Some(name),
                DynamicKey::Number(name) => Some(name),
                DynamicKey::Private(_) => None,
                DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
            });

            if member_name_id.is_some_and(|name| *name == member_name) {
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
            let Declaration::Function {
                descriptor,
                signature,
                ..
            } = declaration
            else {
                continue;
            };

            if signature.kind == FunctionKind::Lambda {
                if current_index == index {
                    return descriptor.symbol.into_global(module_id);
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
        let mut annotations = tree.get_annotations(declaration.local_id.id);
        if annotations.is_empty() {
            let wrapper_id = tree
                .iter_nodes_of_type::<Expression>()
                .find_map(|(expression_id, expression)| match expression {
                    Expression::Declaration { declaration: inner }
                        if inner.id == declaration.local_id.id =>
                    {
                        Some(expression_id)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("expected declaration wrapper for {name}"));
            annotations = tree.get_annotations(wrapper_id.id);
        }
        let decorator_id = annotations
            .iter()
            .find(|annotation_id| matches!(tree.get(**annotation_id), Annotation::Decorator { .. }))
            .copied()
            .unwrap_or_else(|| panic!("expected decorator annotation for {name}"));

        // resolve decorator metadata
        let Annotation::Decorator { expression, .. } = tree.get(decorator_id) else {
            panic!("expected decorator annotation for {name}");
        };

        let call = self.compiler.decorator_call(&tree, *expression);

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
            .map(|argument_id| match tree.get(argument_id) {
                Argument::Positional { value, .. }
                | Argument::Named { value, .. }
                | Argument::Labeled { value, .. } => *value,
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

/// Format one compact compiler stats snapshot for timeout diagnostics.
fn format_stats_snapshot(snapshot: &crate::StatsSnapshot) -> String {
    let mut output = String::new();

    let _ = writeln!(
        output,
        "stats: elapsed={:?} enqueued={} completed={} yielded={} failed={} skipped={}",
        snapshot.elapsed,
        snapshot.tasks.enqueued,
        snapshot.tasks.completed,
        snapshot.tasks.yielded,
        snapshot.tasks.failed,
        snapshot.tasks.skipped
    );
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

    let _ = writeln!(output, "phases:");
    for phase in &snapshot.phases {
        let _ = writeln!(
            output,
            "  {} {:?} x{}",
            phase.phase.name(),
            phase.duration,
            phase.task_count
        );
    }

    let _ = writeln!(output, "top tasks:");
    for task in snapshot.task_names.iter().take(10) {
        let _ = writeln!(
            output,
            "  {} {:?} x{}",
            task.name, task.duration, task.task_count
        );
    }

    output
}
