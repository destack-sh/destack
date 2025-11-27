#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use dyst_dir::{Module, Program};
use dyst_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem,
    PhysicalFileSystem, Uri,
};
use parking_lot::RwLock;

use crate::{CompileOptions, Compiler, Task};

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
}

impl TestProgram {
    /// Create a new blank TestProgram.
    pub fn memory() -> Self {
        let fs = TestFileSystem::Memory {
            fs: Arc::new(MemoryFileSystem::new()),
        };
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            PathBuf::new(),
            fs.fs(),
            Arc::new(FileRegistry::new()),
        ));
        let compiler = Arc::new(Compiler::new(program.clone(), CompileOptions::default()));
        Self {
            fs,
            program,
            compiler,
        }
    }

    /// Create a new TestProgram from a physical fixture.
    pub fn physical(root_directory: &str) -> Self {
        let root_directory = PathBuf::from(root_directory);
        let fs = TestFileSystem::Physical {
            root_directory: root_directory.clone(),
            fs: Arc::new(PhysicalFileSystem::new()),
        };
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            root_directory.clone(),
            fs.fs(),
            Arc::new(FileRegistry::new()),
        ));
        let compiler = Arc::new(Compiler::new(program.clone(), CompileOptions::default()));
        Self {
            fs,
            program,
            compiler,
        }
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

    /// Get a module by URI.
    pub fn module(&self, module_uri: &str) -> Arc<RwLock<Module>> {
        self.program
            .modules
            .get_by_uri(&Uri::from_string(module_uri))
            .unwrap()
    }

    /// Get a module by file id.
    pub fn module_for_file(&self, file: &File) -> Arc<RwLock<Module>> {
        self.program.modules.get_by_uri(&file.uri).unwrap()
    }
}
