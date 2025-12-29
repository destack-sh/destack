use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use dashmap::DashMap;

use destack_source::{DiagnosticCollector, DiagnosticOptions, ModuleId, Uri};
use destack_workspace::{LanguageBuiltins, Program, Session};
use parking_lot::Mutex;

use crate::{
    CompileDiagnostic, CompilerEvent, CompilerEventHandler, CompilerStats, DiagnosticAnchor, Task,
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
#[derive(Clone)]
pub struct CompilerOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,

    /// Whether to follow imports automatically.
    pub follow_imports: bool,
    /// Options for resolving imports.
    pub import_resolve: destack_resolver::ResolveOptions,

    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
    /// Whether to make prelude items (Add, Type, deprecated, etc.) available.
    /// When true, prelude items resolve without explicit imports.
    pub inject_prelude: bool,
    /// Whether to load profile libraries (es*, dom, std, etc.) by default.
    pub load_libs: bool,

    /// Whether to generate source maps.
    pub source_map: bool,

    /// Whether to use ternary expressions for simple if-else value expressions.
    pub elaborate_with_ternary: bool,
    /// Whether to split multi-declarator let statements into individual lets.
    /// e.g., `let a = 1, b = 2` → `let a = 1; let b = 2;`
    pub elaborate_split_declarators: bool,
    /// Whether to make implicit returns explicit.
    /// e.g., `function f() { 42 }` → `function f() { return 42; }`
    pub elaborate_explicit_return: bool,

    /// Whether to overwrite existing files.
    pub emit_overwrite: bool,
    /// Whether to create parent directories if they don't exist.
    pub emit_create_dirs: bool,
    /// Dry run: report what would be written without actually writing.
    pub emit_dry_run: bool,

    /// Optional event handler for progress reporting.
    /// Called for task start/complete/fail events during compilation.
    pub event_handler: Option<CompilerEventHandler>,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: default_workers(),

            follow_imports: true,
            import_resolve: destack_resolver::ResolveOptions::default(),

            default_int_width: 32,
            default_float_width: 64,
            inject_prelude: true,
            load_libs: true,

            source_map: true,

            elaborate_with_ternary: true,
            elaborate_split_declarators: true,
            elaborate_explicit_return: true,

            emit_overwrite: true,
            emit_create_dirs: true,
            emit_dry_run: false,

            event_handler: None,
        }
    }
}

impl std::fmt::Debug for CompilerOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilerOptions")
            .field("diagnostic", &self.diagnostic)
            .field("workers", &self.workers)
            .field("follow_imports", &self.follow_imports)
            .field("import_resolve", &self.import_resolve)
            .field("default_int_width", &self.default_int_width)
            .field("default_float_width", &self.default_float_width)
            .field("inject_prelude", &self.inject_prelude)
            .field("load_libs", &self.load_libs)
            .field("source_map", &self.source_map)
            .field("elaborate_with_ternary", &self.elaborate_with_ternary)
            .field(
                "elaborate_split_declarators",
                &self.elaborate_split_declarators,
            )
            .field("elaborate_explicit_return", &self.elaborate_explicit_return)
            .field("emit_overwrite", &self.emit_overwrite)
            .field("emit_create_dirs", &self.emit_create_dirs)
            .field("emit_dry_run", &self.emit_dry_run)
            .field("event_handler", &self.event_handler.is_some())
            .finish()
    }
}

/// Compile files and sources into something (via DIR).
/// #Architecture: should Compiler be per-target? what about comptime though?
pub struct Compiler {
    /// The session (shared state).
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The options for compiling.
    pub options: CompilerOptions,
    /// Seen errors for deduplication.
    seen_errors: Mutex<Vec<TaskError>>,
    /// Seen warnings for deduplication.
    seen_warnings: Mutex<Vec<TaskWarning>>,
    /// The pending compiler diagnostics (transient).
    pub pending_diagnostics: DiagnosticCollector,
    /// The queue of compiler tasks.
    pub(super) queue: TaskQueue,
    /// The shared comptime target configuration.
    pub comptime_target: destack_workspace::Target,
    /// Locks for serializing module creation per URI (to lock the File->Module import/bind race).
    import_locks: DashMap<Uri, Arc<Mutex<Option<ModuleId>>>>,
    /// Compilation statistics.
    pub stats: Arc<CompilerStats>,
}

impl std::fmt::Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiler")
            .field("session", &"...")
            .field("program", &self.program)
            .field("options", &self.options)
            .field("queue", &self.queue)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Create a new Compiler.
    pub fn new(session: Arc<Session>, program: Arc<Program>, options: CompilerOptions) -> Self {
        let comptime_target = destack_workspace::Target::comptime("comptime");

        Self {
            session,
            program,
            options,
            seen_errors: Mutex::new(Vec::new()),
            seen_warnings: Mutex::new(Vec::new()),
            pending_diagnostics: DiagnosticCollector::new(),
            queue: TaskQueue::new(),
            import_locks: DashMap::new(),
            stats: Arc::new(CompilerStats::new()),
            comptime_target,
        }
    }

    /// Emit a compiler event to the event handler (if configured).
    #[inline]
    pub fn emit_event(&self, event: CompilerEvent) {
        if let Some(handler) = &self.options.event_handler {
            handler(event);
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
    /// This function should only be called directly by each phase's main process logic.
    pub(crate) fn do_require_task_internal_only<T: Into<Task> + Clone>(
        &self,
        task: T,
    ) -> Result<(), TaskDependencyError> {
        let t: Task = task.clone().into();
        match self.queue.find_task_status(&t) {
            Some(TaskStatus::Complete) => Ok(()),
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
