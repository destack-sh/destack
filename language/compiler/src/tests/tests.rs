#![allow(dead_code)]

use std::env::current_dir;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use destack_ast::NodeParentIndex;
use destack_dir::{DumperOptions, GlobalSymbolId, Symbol};
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_linter::print_diff;
use destack_source::{
    DiagnosticSeverity, File, FileId, FileSystem, FileType, MemoryFileSystem, ModuleId, MultiSpan,
    PhysicalFileSystem, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{Module, Program, Session};
use parking_lot::RwLock;

use crate::{
    AnalyzeTask, BindTask, CompileOptions, Compiler, ElaborateTask, ImportTask, LintTask,
    ResolveTask, Task, default_workers,
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
    /// The session (holds shared state like builtins).
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    pub compiler: Arc<Compiler>,
    /// The dumper options.
    pub dumper_options: DumperOptions,
}

impl TestProgram {
    /// Create a new TestProgram with the given options.
    fn new(fs: TestFileSystem, workers: u16, inject_prelude: bool) -> Self {
        init_tracing();
        let root_directory = match &fs {
            TestFileSystem::Memory { .. } => current_dir().unwrap(),
            TestFileSystem::Physical { root_directory, .. } => root_directory.clone(),
        };

        let session = Arc::new(Session::new(root_directory.clone()).with_fs(fs.fs()));
        let program = session.add_root(root_directory);

        let compiler_options = CompileOptions {
            workers,
            inject_prelude,
            ..CompileOptions::default()
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
        }
    }

    /// In-memory, parallel, without prelude injection.
    pub fn memory_parallel() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            default_workers(),
            false,
        )
    }

    /// In-memory, parallel, with prelude injection.
    pub fn memory_parallel_with_builtins() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            default_workers(),
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
        )
    }

    /// In-memory, sequential, with prelude injection.
    pub fn memory_sequential_with_builtins() -> Self {
        Self::new(
            TestFileSystem::Memory {
                fs: Arc::new(MemoryFileSystem::new()),
            },
            1,
            true,
        )
    }

    /// Load a lib module set (builder pattern).
    pub fn with_lib(self, name: &str) -> Self {
        self.session.load_lib(name);
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

    /// Add a package.json and optionally dsconfig.json to the memory filesystem.
    ///
    /// `dsconfig_compiler_options` is the raw JSON content for `compilerOptions`, e.g.:
    /// ```ignore
    /// test.add_package("my-pkg", Some(r#""noRedeclaredLocals": true"#));
    /// ```
    pub fn add_package(&self, name: &str, dsconfig_compiler_options: Option<&str>) {
        self.add_file("package.json", &format!(r#"{{ "name": "{name}" }}"#));
        if let Some(opts) = dsconfig_compiler_options {
            self.add_file(
                "dsconfig.json",
                &format!(r#"{{ "compilerOptions": {{ {opts} }} }}"#),
            );
        }
    }

    /// Add a file and register a blank module for it (no import/parsing yet).
    pub fn add_module(&self, path: &str, content: &str) -> ModuleId {
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
        self.enqueue(BindTask::BindModule { module });
    }

    /// Enqueue Resolve task for a module.
    pub fn resolve_module(&self, module: ModuleId) {
        self.enqueue(ResolveTask::ResolveModule { module });
    }

    /// Enqueue ResolveBuiltins task.
    pub fn resolve_builtins(&self) {
        self.enqueue(ResolveTask::ResolveBuiltins);
    }

    /// Enqueue Analyze task for a module.
    pub fn analyze_module(&self, module: ModuleId) {
        self.enqueue(AnalyzeTask::AnalyzeModule { module });
    }

    /// Enqueue Lint task for a module.
    pub fn lint_module(&self, module: ModuleId) {
        self.enqueue(LintTask::LintModule { module });
    }

    /// Enqueue Elaborate task for a module.
    pub fn elaborate_module(&self, module: ModuleId) {
        self.enqueue(ElaborateTask::ElaborateModule { module });
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

    /// Check that exactly the given diagnostics are present (by code).
    /// Panics if the actual diagnostics don't match.
    pub fn check_diagnostics(&self, expected_codes: &[&str]) {
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
        let module = module.read();

        // unbind DIR to AST
        let unbound = self.compiler.unbind_module(&module);

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
        let context = DestackFormatContext {
            options: DestackFormatOptions::default(),
            file: &file,
            tree: &unbound.tree,
            source_map: &unbound.tree.source_map,
            parents: NodeParentIndex::from_tree(&unbound.tree),
            tokens: &vec![],
            side_tokens: &vec![],
            side_span: &MultiSpan::new(vec![]),
            strings: &strings,
        };

        // format each root expression and join with newlines
        let mut results = Vec::new();
        for root_id in &unbound.roots {
            let formatted = destack_fir::format!(context.clone(), [root_id]).unwrap();
            let printed = formatted.print().unwrap();
            results.push(printed.into_str());
        }
        results.join("\n")
    }

    /// Assert that a module's DIR has been elaborated to the given AST.
    pub fn assert_elaborated(&self, module_id: ModuleId, expected: &str) {
        let unbound = self.unbind_to_string(module_id);
        let unbound = unbound.trim();
        let expected = expected.trim();
        if unbound != expected {
            print_diff(expected, unbound);
            panic!("elaborated code mismatch");
        }
    }
}
