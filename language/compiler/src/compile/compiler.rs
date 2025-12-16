use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use dashmap::DashMap;

use destack_source::{DiagnosticCollector, DiagnosticOptions, ModuleId, Uri};
use destack_workspace::{LanguageBuiltins, Program};
use parking_lot::Mutex;

use crate::{
    AnalyzeOptions, BindOptions, CompileDiagnostic, DiagnosticAnchor, EmitOptions, GenerateOptions,
    ImportOptions, LinkOptions, LowerOptions, OptimizeOptions, ResolveOptions, Task,
    TaskDependency, TaskDependencyError, TaskError, TaskQueue, TaskResultCollector, TaskStatus,
    TaskWarning,
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
    /// The options for binding.
    pub bind: BindOptions,
    /// The options for evaluating.
    pub resolve: ResolveOptions,
    /// The options for validating.
    pub analyze: AnalyzeOptions,
    /// The options for lowering.
    pub lower: LowerOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for code generation.
    pub generate: GenerateOptions,
    /// The options for linking.
    pub link: LinkOptions,
    /// The options for emitting.
    pub emit: EmitOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: default_workers(),
            import: ImportOptions::default(),
            bind: BindOptions::default(),
            resolve: ResolveOptions::default(),
            analyze: AnalyzeOptions::default(),
            lower: LowerOptions::default(),
            optimize: OptimizeOptions::default(),
            generate: GenerateOptions::default(),
            link: LinkOptions::default(),
            emit: EmitOptions::default(),
        }
    }
}

/// Compile files and sources into something (via DIR).
/// NOTE #Architecture: should Compiler be per-target? what about comptime though?
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

    /// Get the builtins (if loaded in program).
    pub fn builtins(&self) -> Option<&Arc<LanguageBuiltins>> {
        self.program.builtins.as_ref()
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
        seen.push(error);
    }

    /// Add a warning to the compiler (deduplicated).
    pub fn warning<T: Into<TaskWarning>>(&self, warning: T) {
        let warning: TaskWarning = warning.into();
        let mut seen = self.seen_warnings.lock();
        if seen.contains(&warning) {
            return;
        }
        seen.push(warning);
    }

    /// Collect a result into a TaskResultCollector, reporting non-yield errors.
    ///
    /// Returns `Some(value)` on success, `None` on error (yield or hard error).
    /// Yields are collected into the collector, hard errors are reported via `self.error()`.
    pub fn collect<T, E>(
        &self,
        collector: &mut TaskResultCollector,
        result: Result<T, E>,
    ) -> Option<T>
    where
        E: TryInto<TaskDependency, Error = E> + Into<TaskError>,
    {
        match &result {
            Ok(_) => {}
            Err(_) => {
                if let Some(error) = collector.try_collect(result) {
                    self.error(error);
                }
                return None;
            }
        }
        result.ok()
    }

    /// Flush pending diagnostics into the program.
    /// Converts all stored errors/warnings to diagnostics.
    pub fn flush_diagnostics(&self) {
        // convert errors to diagnostics
        let errors = {
            let mut seen = self.seen_errors.lock();
            std::mem::take(&mut *seen)
        };
        for error in errors {
            let diagnostic: CompileDiagnostic = error.into();
            self.pending_diagnostics
                .insert(diagnostic.to_diagnostic(&self.program));
        }

        // convert warnings to diagnostics
        let warnings = {
            let mut seen = self.seen_warnings.lock();
            std::mem::take(&mut *seen)
        };
        for warning in warnings {
            let diagnostic: CompileDiagnostic = warning.into();
            self.pending_diagnostics
                .insert(diagnostic.to_diagnostic(&self.program));
        }

        // flush to program
        self.program
            .diagnostics
            .take_from(&self.pending_diagnostics);
    }

    /// Require a task to be complete, returning an error if it's not ready or has failed.
    /// Each phase defines its own tasks, and its own higher level require_* helper functions.
    ///
    /// This function should only be called directly by each phase's main process.
    pub(crate) fn do_require_task_internal_only<T: Into<Task> + Clone>(
        &self,
        task: T,
    ) -> Result<(), TaskDependencyError> {
        let t: Task = task.clone().into();
        match self.queue.find_task_status(&t) {
            Some(TaskStatus::Complete { .. }) => Ok(()),
            Some(TaskStatus::Failed { error }) => Err(TaskDependencyError::Failed {
                dependency: TaskDependency::Complete {
                    anchor: DiagnosticAnchor::Global,
                    task: t,
                    error: Some(Box::new(error)),
                },
            }),
            _ => Err(TaskDependencyError::NotReady {
                dependency: TaskDependency::Complete {
                    anchor: DiagnosticAnchor::Global,
                    task: t,
                    error: None,
                },
            }),
        }
    }
}
