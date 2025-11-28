#![allow(dead_code)]

use std::env::current_dir;
use std::path::PathBuf;
use std::sync::Arc;

use dyst_dir::{DumperOptions, Module, Program};
use dyst_source::{
    DiagnosticSeverity, File, FileRegistry, FileSystem, FileType, LanguageOptions,
    MemoryFileSystem, PhysicalFileSystem, PrintOptions, Uri, print_diagnostics,
};
use parking_lot::RwLock;

use crate::{CompileOptions, Compiler, Task, default_workers};

use super::tracing::init_tracing;

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
        // context
        init_tracing();
        let root_directory = match &fs {
            TestFileSystem::Memory { .. } => current_dir().unwrap(),
            TestFileSystem::Physical { root_directory, .. } => root_directory.clone(),
        };

        // program
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            root_directory,
            fs.fs(),
            Arc::new(FileRegistry::new()),
        ));

        // compiler options
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

    /// Create a new source file.
    pub fn file(&self, path: &str, content: &str) -> Arc<File> {
        let file_id = self.program.files.next_id();
        let uri = Uri::from_string(path);
        let file = File::from_text(
            file_id,
            path.to_string(),
            uri.clone(),
            uri.to_path_buf(),
            FileType::Dyst,
            content.to_string(),
        );
        self.program.files.insert(file);
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
        self.program.files.get(file_id)
    }

    /// Enqueue a compile task.
    pub fn enqueue<T: Into<Task>>(&self, task: T) {
        self.compiler.enqueue(task);
    }

    /// Compile the program.
    pub fn compile(&self) {
        self.compiler.compile();
    }

    /// Check no diagnostics of at least the given severity.
    ///
    /// If any diagnostics meet the threshold, all diagnostics are printed before panicking.
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
                "program has {} unexpected {}s",
                diagnostics.len(),
                severity_name
            );
        }
    }

    /// Check no errors (convenience wrapper for check_no_diagnostic).
    pub fn check_no_errors(&self) {
        self.check_no_diagnostic(DiagnosticSeverity::Error);
    }

    /// Helper to compile, dump and check no diagnostics.
    pub fn compile_dump_clean(&self) {
        self.compile();
        self.dump();
        self.check_no_diagnostic(DiagnosticSeverity::Note);
    }

    /// Get a module by URI.
    pub fn module(&self, module_uri: &str) -> Arc<RwLock<Module>> {
        self.program
            .modules
            .get_by_uri(&Uri::from_string(module_uri))
            .unwrap_or_else(|| panic!("module not found for '{module_uri}'"))
    }

    /// Get a module by file id.
    pub fn module_for_file(&self, file: &File) -> Arc<RwLock<Module>> {
        self.program
            .modules
            .get_by_uri(&file.uri)
            .unwrap_or_else(|| panic!("module not found for file: '{}'", file.uri))
    }
}
