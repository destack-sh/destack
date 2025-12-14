use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{FileRegistry, FileSystem, PhysicalFileSystem};

use crate::{FormatterOptions, LinterOptions, Program, Workspace};

/// A session is the persistent state for a workspace.
#[derive(Debug)]
pub struct Session {
    /// The workspace this session is for.
    pub workspace: Arc<Workspace>,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the session.
    pub files: Arc<FileRegistry>,
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The programs in the session, keyed by root path.
    pub programs: DashMap<PathBuf, Arc<Program>>,
}

impl Session {
    /// Create a new session for a workspace.
    pub fn new(workspace: Arc<Workspace>, cwd: PathBuf) -> Self {
        Self {
            workspace,
            cwd,
            fs: Arc::new(PhysicalFileSystem::new()),
            files: Arc::new(FileRegistry::new()),
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            programs: DashMap::new(),
        }
    }

    /// Create a new session with a custom file system.
    pub fn with_fs(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs = fs;
        self
    }

    /// Create a new session with formatter options.
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.formatter = formatter;
        self
    }

    /// Create a new session with linter options.
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.linter = linter;
        self
    }

    /// Add a root to the session.
    /// Creates a new Program for the given root path.
    pub fn add_root(&self, root: PathBuf) -> Arc<Program> {
        let program = Arc::new(Program::new(
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
