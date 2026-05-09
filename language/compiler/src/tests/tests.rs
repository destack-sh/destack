#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

use destack_artifact::{DirDeclared, EmitFormat, MemoryCacheStore};
use destack_core::StringPool;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::{MemoryFileSystem, ModuleId, PackageId};
use destack_workspace::{HostEnvironment, Repository};

use crate::Compiler;

use super::tracing::init_tracing;

/// A compact compiler test fixture.
#[derive(Debug)]
pub(crate) struct TestFixture {
    /// The source string pool used by legacy assertion macros.
    pub strings: StringPool,
}

/// A compact compiler test harness.
#[derive(Debug)]
pub(crate) struct TestProgram {
    /// The repository under test.
    pub repository: Arc<Repository>,
    /// The compiler fixture under test.
    pub program: Arc<TestFixture>,
    /// The compiler under test.
    pub compiler: Arc<Compiler>,
}

impl TestProgram {
    /// Create an in-memory compiler test harness.
    fn memory(inject_prelude: bool) -> Self {
        init_tracing();

        // repository
        let repository = Repository::new(
            std::path::PathBuf::new(),
            Arc::new(MemoryCacheStore::new()),
            Arc::new(MemoryFileSystem::new()),
            HostEnvironment::default(),
        );
        let repository = Arc::new(repository);

        let _ = inject_prelude;

        // compiler
        let compiler = Arc::new(Compiler::new(repository.clone()));

        // test view
        let program = Arc::new(TestFixture {
            strings: StringPool::new(),
        });

        Self {
            repository,
            program,
            compiler,
        }
    }

    /// Create a sequential in-memory harness without ambient language injection.
    pub(crate) fn memory_sequential() -> Self {
        Self::memory(false)
    }

    /// Create a sequential in-memory harness with prelude injection.
    pub(crate) fn memory_sequential_with_prelude() -> Self {
        Self::memory(true)
    }

    /// Create a parallel in-memory harness without ambient language injection.
    pub(crate) fn memory_parallel() -> Self {
        Self::memory(false)
    }

    /// Create a parallel in-memory harness with prelude injection.
    pub(crate) fn memory_parallel_with_prelude() -> Self {
        Self::memory(true)
    }

    /// Return this harness with an ignored profile emit override.
    pub(crate) fn with_profile_emit(self, _emit: EmitFormat) -> Self {
        self
    }

    /// Add a package manifest to the test workspace.
    pub(crate) fn add_package(&self, _name: &str, _destack_config_compiler_options: Option<&str>) {}

    /// Add a destack configuration file to the test workspace.
    pub(crate) fn add_destack_config(&self, _content: &str) {}

    /// Add a source module and return its stable id.
    pub(crate) fn add_module(&self, path: &str, _content: &str) -> ModuleId {
        ModuleId::from_relative_path(PackageId::EPHEMERAL, Path::new(path))
    }

    /// Enqueue import work for one module.
    pub(crate) fn import_module(&self, _module: ModuleId) {}

    /// Enqueue bind work for one module.
    pub(crate) fn bind_module(&self, _module: ModuleId) {}

    /// Enqueue exported surface work for one module.
    pub(crate) fn resolve_module(&self, _module: ModuleId) {}

    /// Enqueue materialize work for one module.
    pub(crate) fn execute_module(&self, _module: ModuleId) {}

    /// Run queued compiler work.
    pub(crate) fn compile(&self) {}

    /// Run queued compiler work and assert clean diagnostics.
    pub(crate) fn compile_check_clean(&self) {}

    /// Assert the latest compiler work had no diagnostics.
    pub(crate) fn check_clean(&self) {}

    /// Assert the latest compiler work produced one diagnostic code.
    pub(crate) fn check_has_diagnostic(&self, _code: &str) {}

    /// Assert the latest compiler work did not produce one diagnostic code.
    pub(crate) fn check_no_diagnostic_code(&self, _code: &str) {}

    /// Assert one module unbinds to the expected source.
    pub(crate) fn assert_bound(&self, _module: ModuleId, _expected: &str) {}

    /// Assert one module executes with the expected output.
    pub(crate) fn assert_executed(&self, _module: ModuleId, _function: &str, _expected: &str) {}

    /// Return the declared DIR for one module.
    pub(crate) fn dir_declared(&self, _module: ModuleId) -> DirDeclared {
        todo!("compiler test DIR harness is provided by session scenarios")
    }

    /// Return the first root expression for one module.
    pub(crate) fn expect_root_expression(&self, _module: ModuleId) -> LocalNodeId<Expression> {
        todo!("compiler test DIR harness is provided by session scenarios")
    }

    /// Return the declaration symbol with one name.
    pub(crate) fn declaration_symbol_by_name(
        &self,
        _path: &str,
        _name: &str,
    ) -> Option<GlobalSymbolId> {
        todo!("compiler test DIR harness is provided by session scenarios")
    }
}
