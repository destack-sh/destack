#![allow(dead_code)]

use std::env::current_dir;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use destack_dir::{DumperOptions, GlobalSymbolId, Symbol};
use destack_source::{
    DiagnosticSeverity, File, FileRegistry, FileSystem, LanguageOptions, MemoryFileSystem,
    ModuleId, PhysicalFileSystem, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{Module, Program};
use parking_lot::RwLock;

use crate::{
    AnalyzeTask, BindTask, CompileOptions, Compiler, ImportTask, ResolveTask, Task, default_workers,
};

use super::tracing::init_tracing;

const TEST_TIMEOUT_SECONDS: u64 = 1;

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
}

/// A test wrapper for a Program.
#[derive(Debug)]
pub struct TestProgram {
    /// The file system.
    pub fs: TestFileSystem,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    pub compiler: Arc<Compiler>,
    /// The dumper options.
    pub dumper_options: DumperOptions,
}

impl TestProgram {
    /// Create a new TestProgram with the given TestFileSystem and worker count.
    fn new(fs: TestFileSystem, workers: u16) -> Self {
        init_tracing();
        let root_directory = match &fs {
            TestFileSystem::Memory { .. } => current_dir().unwrap(),
            TestFileSystem::Physical { root_directory, .. } => root_directory.clone(),
        };

        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            root_directory,
            fs.fs(),
            Arc::new(FileRegistry::new()),
        ));

        let compiler_options = CompileOptions {
            workers,
            ..CompileOptions::default()
        };
        let compiler = Arc::new(Compiler::new(program.clone(), compiler_options));

        Self {
            fs,
            program,
            compiler,
            dumper_options: DumperOptions::default(),
        }
    }

    /// Create a new blank TestProgram with in-memory file system and multiple workers.
    pub fn memory_parallel() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            default_workers(),
        )
    }

    /// Create a new blank TestProgram with in-memory file system and a single worker.
    pub fn memory_sequential() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            1,
        )
    }

    /// Create a new blank TestProgram with physical file system and multiple workers.
    pub fn physical_parallel(root_directory: &str) -> Self {
        Self::new(
            TestFileSystem::Physical {
                root_directory: PathBuf::from(root_directory),
                fs: Arc::new(PhysicalFileSystem::new()),
            },
            default_workers(),
        )
    }

    /// Create a new blank TestProgram with physical file system and a single worker.
    pub fn physical_sequential(root_directory: &str) -> Self {
        Self::new(
            TestFileSystem::Physical {
                root_directory: PathBuf::from(root_directory),
                fs: Arc::new(PhysicalFileSystem::new()),
            },
            1,
        )
    }

    // -------------------------------------------------------------------------
    // File helpers
    // -------------------------------------------------------------------------

    /// Add a file to the memory filesystem (without creating a Module).
    ///
    /// Use this for files that will be transitively imported.
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

    /// Add a file and register a blank module for it (no import/parsing yet).
    pub fn register_module(&self, path: &str, content: &str) -> ModuleId {
        self.add_file(path, content);
        self.compiler
            .resolve_path_to_module(&PathBuf::from(path))
            .unwrap_or_else(|e| panic!("failed to register module: {e:?}"))
    }

    /// Enqueue Import task for a module.
    pub fn import_module(&self, module: ModuleId) {
        self.enqueue(ImportTask::ImportModule { module });
    }

    /// Enqueue Bind task for a module.
    pub fn bind_module(&self, module: ModuleId) {
        self.enqueue(BindTask::BindModuleValidate { module });
    }

    /// Enqueue Resolve task for a module.
    pub fn resolve_module(&self, module: ModuleId) {
        self.enqueue(ResolveTask::ResolveModuleCanonical { module });
    }

    /// Enqueue Analyze task for a module (runs all phases: Declare, Infer, Check).
    pub fn analyze_module(&self, module: ModuleId) {
        self.enqueue(AnalyzeTask::AnalyzeModuleValidate { module });
    }

    /// Enqueue a task (does not run it).
    pub fn enqueue<T: Into<Task>>(&self, task: T) {
        self.compiler.enqueue(task);
    }

    /// Run all queued tasks to completion (with 5s timeout).
    pub fn compile(&self) {
        let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);
        let compiler = self.compiler.clone();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            compiler.compile();
            let _ = tx.send(());
        });

        match rx.recv_timeout(timeout) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("compile timed out after {TEST_TIMEOUT_SECONDS}s");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("compile thread panicked");
            }
        }
    }

    /// Enqueue a task and run to completion.
    pub fn run<T: Into<Task>>(&self, task: T) {
        self.enqueue(task);
        self.compile();
    }

    /// Check no errors.
    pub fn check_clean(&self) {
        self.check_no_diagnostic(DiagnosticSeverity::Error);
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
        self.program.files.get(module.read().file_id)
    }

    /// Get a module by URI.
    pub fn module(&self, module_uri: &str) -> Arc<RwLock<Module>> {
        self.program
            .modules
            .get_by_uri(&Uri::from_string(module_uri))
            .unwrap_or_else(|| panic!("module not found for '{module_uri}'"))
    }

    /// Get a module by file.
    pub fn module_for_file(&self, file: &File) -> Arc<RwLock<Module>> {
        self.program
            .modules
            .get_by_uri(&file.uri)
            .unwrap_or_else(|| panic!("module not found for file: '{}'", file.uri))
    }

    /// Get a symbol by id.
    pub fn symbol_by_id(&self, symbol_id: GlobalSymbolId) -> Symbol {
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();
        symbols.get_symbol(symbol_id.into_local()).clone()
    }

    /// Check no diagnostics of at least the given severity.
    pub fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.program.diagnostics.collect();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let options = PrintOptions::new()
                .with_line_width(self.program.language.formatting.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "program has {} unexpected {severity_name}s",
                diagnostics.len()
            );
        }
    }

    /// Check that exactly the given diagnostics are present (by code).
    /// Panics if the actual diagnostics don't match.
    pub fn check_diagnostics(&self, expected_codes: &[&str]) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
        if actual_codes != expected_codes {
            let options = PrintOptions::new()
                .with_line_width(self.program.language.formatting.line_width as u32)
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
                .with_line_width(self.program.language.formatting.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            let actual_codes: Vec<&str> = diagnostic_vec.iter().map(|d| d.code.as_str()).collect();
            panic!("expected diagnostic with code '{code}' but found: {actual_codes:?}");
        }
    }

    /// Check that no diagnostic with the given code is present.
    pub fn check_no_diagnostic_code(&self, code: &str) {
        let diagnostics = self.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let has_code = diagnostic_vec.iter().any(|d| d.code == code);

        if has_code {
            let options = PrintOptions::new()
                .with_line_width(self.program.language.formatting.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            panic!("unexpected diagnostic with code '{code}'");
        }
    }
}
