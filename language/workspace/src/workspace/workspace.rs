use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{FileRegistry, FileSystem, PhysicalFileSystem};

use crate::{FormatterOptions, LanguageOptions, LinterOptions, Program};

/// A workspace containing multiple program roots.
#[derive(Debug)]
pub struct Workspace {
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the workspace.
    pub files: Arc<FileRegistry>,
    /// The language options.
    pub language: LanguageOptions,
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The programs in the workspace, keyed by root path.
    pub programs: DashMap<PathBuf, Arc<Program>>,
}

impl Workspace {
    /// Create a new Workspace with physical file system.
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            fs: Arc::new(PhysicalFileSystem::new()),
            files: Arc::new(FileRegistry::new()),
            language: LanguageOptions::default(),
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            programs: DashMap::new(),
        }
    }

    /// Create a new Workspace with a custom file system.
    pub fn with_fs(cwd: PathBuf, fs: Arc<dyn FileSystem>) -> Self {
        Self {
            cwd,
            fs,
            files: Arc::new(FileRegistry::new()),
            language: LanguageOptions::default(),
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            programs: DashMap::new(),
        }
    }

    /// Create a new Workspace with language options.
    pub fn with_language(mut self, language: LanguageOptions) -> Self {
        self.language = language;
        self
    }

    /// Create a new Workspace with formatter options.
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.formatter = formatter;
        self
    }

    /// Create a new Workspace with linter options.
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.linter = linter;
        self
    }

    /// Add a root to the workspace.
    /// Creates a new Program for the given root path.
    pub fn add_root(&self, root: PathBuf) -> Arc<Program> {
        let program = Arc::new(Program::new(
            self.language,
            self.formatter,
            self.linter.clone(),
            root.clone(),
            self.fs.clone(),
            self.files.clone(),
        ));
        self.programs.insert(root.clone(), program.clone());
        program
    }

    /// Get a program by root path.
    pub fn get_program(&self, root: &Path) -> Option<Arc<Program>> {
        self.programs.get(root).map(|p| p.clone())
    }

    /// Get or create a program for the given root path.
    pub fn get_or_create_program(&self, root: PathBuf) -> Arc<Program> {
        if let Some(program) = self.programs.get(&root) {
            return program.clone();
        }
        self.add_root(root)
    }

    /// Find the program that contains the given path.
    /// Falls back to the cwd-based program if no match is found.
    pub fn find_program_for_path(&self, path: &Path) -> Arc<Program> {
        // look for a program whose root is a prefix of the path
        for entry in self.programs.iter() {
            if path.starts_with(entry.key()) {
                return entry.value().clone();
            }
        }

        // fallback: create/get a program for the cwd
        self.get_or_create_program(self.cwd.clone())
    }
}
