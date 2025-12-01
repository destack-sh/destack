use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use dashmap::DashMap;
use destack_dir::{ModuleId, Program};
use destack_source::{DiagnosticCollector, DiagnosticOptions, Uri};
use parking_lot::Mutex;

use crate::{
    GenerateOptions, CompileDiagnostic, ExecuteOptions, ImportOptions, LinkOptions, LowerOptions,
    OptimizeOptions, ResolveOptions, TaskDependency, TaskError, TaskQueue, TaskResultCollector,
    TaskWarning, AnalyzeOptions,
};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
}

/// The options for compiling a Workspace.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
    /// The options for importing.
    pub import: ImportOptions,
    /// The options for evaluating.
    pub resolve: ResolveOptions,
    /// The options for validating.
    pub analyze: AnalyzeOptions,
    /// The options for lowering.
    pub lower: LowerOptions,
    /// The options for executing.
    pub execute: ExecuteOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for code generation.
    pub generate: GenerateOptions,
    /// The options for linking.
    pub link: LinkOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: default_workers(),
            import: ImportOptions::default(),
            resolve: ResolveOptions::default(),
            analyze: AnalyzeOptions::default(),
            lower: LowerOptions::default(),
            execute: ExecuteOptions::default(),
            optimize: OptimizeOptions::default(),
            generate: GenerateOptions::default(),
            link: LinkOptions::default(),
        }
    }
}

/// Compile files and sources into something (via DIR).
pub struct Compiler {
    /// The program.
    pub program: Arc<Program>,
    /// The options for compiling.
    pub options: CompileOptions,
    /// Seen errors for deduplication.
    seen_errors: Mutex<Vec<TaskError>>,
    /// Seen warnings for deduplication.
    seen_warnings: Mutex<Vec<TaskWarning>>,
    /// The pending compiler diagnostics (transient).
    pub pending_diagnostics: DiagnosticCollector,
    /// The queue of compiler tasks.
    pub(super) queue: TaskQueue,
    /// Locks for serializing module creation per URI (to lock the File->Module import/bind race).
    import_locks: DashMap<Uri, Arc<Mutex<Option<ModuleId>>>>,
}

impl std::fmt::Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiler")
            .field("program", &self.program)
            .field("options", &self.options)
            .field("queue", &self.queue)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Create a new Compiler.
    pub fn new(program: Arc<Program>, options: CompileOptions) -> Self {
        Self {
            program,
            options,
            seen_errors: Mutex::new(Vec::new()),
            seen_warnings: Mutex::new(Vec::new()),
            pending_diagnostics: DiagnosticCollector::new(),
            queue: TaskQueue::new(),
            import_locks: DashMap::new(),
        }
    }

    /// Get the import lock for a URI.
    /// Used to serialize module creation and prevent race conditions when multiple import tasks
    /// resolve to the same file.
    ///
    /// Usage pattern:
    /// 1. Acquire lock: `let lock = compiler.get_import_lock(&uri); let mut guard = lock.lock();`
    /// 2. Check value: if `Some(module_id)`, module already exists, use it
    /// 3. If `None`: create module, set `*guard = Some(module_id)`, then drop guard
    pub(crate) fn get_import_lock(&self, uri: &Uri) -> Arc<Mutex<Option<ModuleId>>> {
        self.import_locks
            .entry(uri.clone())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    /// Add an error to the compiler (deduplicated).
    pub fn error<T: Into<TaskError>>(&self, error: T) {
        let error: TaskError = error.into();
        let mut seen = self.seen_errors.lock();
        if seen.contains(&error) {
            return;
        }
        seen.push(error.clone());
        drop(seen);
        let diagnostic: CompileDiagnostic = error.into();
        let diagnostic = diagnostic.to_diagnostic(&self.program);
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Add a warning to the compiler (deduplicated).
    pub fn warning<T: Into<TaskWarning>>(&self, warning: T) {
        let warning: TaskWarning = warning.into();
        let mut seen = self.seen_warnings.lock();
        if seen.contains(&warning) {
            return;
        }
        seen.push(warning.clone());
        drop(seen);
        let diagnostic: CompileDiagnostic = warning.into();
        let diagnostic = diagnostic.to_diagnostic(&self.program);
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Collect a result into a TaskResultCollector, reporting non-yield errors.
    pub fn collect<T, E>(&self, collector: &mut TaskResultCollector, result: Result<T, E>)
    where
        E: TryInto<TaskDependency, Error = E> + Into<TaskError>,
    {
        if let Some(error) = collector.try_collect(result) {
            self.error(error);
        }
    }

    /// Flush pending diagnostics into the program.
    pub fn flush_diagnostics(&self) {
        self.program
            .diagnostics
            .take_from(&self.pending_diagnostics);
    }
}
