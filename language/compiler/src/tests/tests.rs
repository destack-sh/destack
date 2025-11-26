#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use dyst_dir::Program;
use dyst_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem,
    PhysicalFileSystem, Uri,
};

use crate::{CompileOptions, CompileTask, Compiler};

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
        let file = File::from_text(
            file_id,
            path.to_string(),
            Uri::from_string(path),
            None,
            FileType::Dyst,
            content.to_string(),
        );
        self.program.files.insert(file);
        self.program.files.get(file_id)
    }

    /// Enqueue a compile task.
    pub fn enqueue<T: Into<CompileTask>>(&self, task: T) {
        self.compiler.enqueue(task);
    }

    /// Compile the program.
    pub fn compile(&self) {
        self.compiler.compile();
    }
}

/// Assert that `tree.get(id)` matches `$pat`.
/// If a body is provided (`=> { ... }`), it runs with the pattern bindings.
///
/// Examples:
/// ```
/// assert_node!(tree, id, Pattern::Wildcard);
/// assert_node!(tree, id, Pattern::Pointer { mutability, target } => {
///     assert_eq!(*mutability, Mutability::Mutable);
///     assert_node!(tree, *target, Pattern::Wildcard);
/// });
/// ```
#[macro_export]
macro_rules! assert_node {
    // `tree.get(id)` matches a pattern, no body.
    // e.g., `assert_node!(tree, id, Pattern::Wildcard);`
    ($tree:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // `tree.get(id)` matches a pattern, then run a block with the bindings.
    // e.g., `assert_node!(tree, id, Pattern::Pointer { mutability, target } => { /* ... */ });`
    ($tree:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, run a block.
    // e.g., `assert_node!(node_ref, Pattern::Tuple { fields } => { /* ... */ });`
    ($node:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, no body.
    // e.g., `assert_node!(node_ref, Pattern::Rest);`
    ($node:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}

/// Assert a `StringId` directly against an expected string.
#[macro_export]
macro_rules! assert_string {
    ($program:expr, $id:expr, $expected:expr) => {{
        let got = $program.strings.get($id).to_string();
        assert_eq!(got, $expected, "expected string");
    }};
}

/// Assert a `Name` directly against an expected string.
#[macro_export]
macro_rules! assert_name {
    ($program:expr, $name:expr, $expected:expr) => {{
        let got = $program.strings.get($name.string()).to_string();
        assert_eq!(got, $expected, "expected name");
    }};
}

/// Assert a `Path` directly against an expected string.
#[macro_export]
macro_rules! assert_path {
    ($program:expr, $path:expr, $expected:expr) => {{
        let path_str = $path
            .segments
            .iter()
            .map(|s| $program.strings.get(*s).to_string())
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(path_str, $expected, "expected path");
    }};
}

/// Assert an "Expression::Path(path)" directly against an expected string.
#[macro_export]
macro_rules! assert_expression_path {
    ($program:expr, $expr:expr, $expected:expr) => {{
        match $expr {
            ::dyst_dir::Expression::Path {
                path,
                static_arguments: _,
            } => {
                assert_path!($program, *path, $expected);
            }
            other => panic!("expected Expression::Path, got {other:?}"),
        }
    }};
}
