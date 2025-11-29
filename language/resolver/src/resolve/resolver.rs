use std::fmt;
use std::sync::Arc;

use destack_dir::Program;
use destack_source::{FileRegistry, FileSystem, LanguageOptions, PhysicalFileSystem};

use crate::ResolveOptions;

/// Module resolver implementing Node.js-style resolution.
pub struct Resolver {
    /// The program containing file registries and filesystem access.
    pub program: Arc<Program>,
    /// Configuration options controlling resolution behavior.
    pub options: ResolveOptions,
}

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

#[allow(dead_code)]
impl Resolver {
    /// Create a new resolver with options in an existing program.
    pub fn new(program: Arc<Program>, options: ResolveOptions) -> Self {
        Self {
            program,
            options: options.sanitize(),
        }
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolveOptions) -> Self {
        Self {
            program: self.program.clone(),
            options: options.sanitize(),
        }
    }

    /// Create a new resolver with physical file system in an empty program (for testing).
    pub(crate) fn blank(options: ResolveOptions) -> Self {
        let cwd = options
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            cwd,
            fs.clone(),
            files.clone(),
        ));
        Self::new(program, options)
    }

    /// Create a new resolver with a custom file system in an empty program (for testing).
    pub(crate) fn blank_with_fs(fs: Arc<dyn FileSystem>, options: ResolveOptions) -> Self {
        let cwd = options
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        let files = Arc::new(FileRegistry::new());
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            cwd,
            fs.clone(),
            files.clone(),
        ));
        Self::new(program, options)
    }
}
