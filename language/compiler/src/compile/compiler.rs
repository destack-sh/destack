use std::sync::Arc;

use dashmap::DashMap;

use destack_base::ImmutableStringPool;
use destack_resolver::Resolver;
use destack_source::{DiagnosticCollector, DiagnosticOptions, DiagnosticSeverity, ModuleId, Uri};
use destack_workspace::{Builtins, Program, Session, Target};
use parking_lot::Mutex;

use crate::{
    CacheRegistry, CompileDiagnostic, CompilerEvent, CompilerEventHandler, CompilerStats,
    DiagnosticAnchor, Task, TaskDependency, TaskDependencyError, TaskError, TaskQueue,
    TaskResultCollector, TaskStatus, TaskWarning,
};

use super::parallel::default_workers as resolve_default_workers;

/// Get the default number of worker threads.
pub fn default_workers() -> u16 {
    resolve_default_workers()
}

/// How unresolved imports should be handled during resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    /// Emit errors for unresolved imports.
    Strict,
    /// Emit warnings for unresolved imports and continue.
    Lenient,
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
    /// How unresolved imports should be handled during resolve.
    pub resolve_mode: ResolveMode,
    /// Whether to disallow ambiguous tree literal syntax.
    pub disallow_ambiguous_tree_literal: bool,

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
    /// Whether to wrap implicit casts inserted during elaborate in parentheses.
    pub elaborate_parenthesize_casts: bool,

    /// Whether to retain comptime expressions as comments after execution.
    pub retain_comptime_as_comment: bool,
    /// Maximum length of retained comptime comments (after compaction).
    /// A value of 0 disables comment retention entirely.
    pub retain_comptime_comment_max_length: usize,

    /// Whether to overwrite existing files.
    pub emit_overwrite: bool,
    /// Whether to create parent directories if they don't exist.
    pub emit_create_dirs: bool,
    /// Dry run: report what would be written without actually writing.
    pub emit_dry_run: bool,

    /// Optional event handler for progress reporting.
    /// Called for task start/complete/fail events during compilation.
    pub event_handler: Option<CompilerEventHandler>,

    /// Whether to collect detailed timing tags.
    pub timings: bool,
    /// Whether to validate builtin declaration libs eagerly.
    pub validate_builtin_libs: bool,
    /// Whether to verify MIR after building it (internal debug/test builds).
    pub verify_mir: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: resolve_default_workers(),

            follow_imports: true,
            import_resolve: destack_resolver::ResolveOptions::default(),
            resolve_mode: ResolveMode::Strict,
            disallow_ambiguous_tree_literal: false,

            default_int_width: 32,
            default_float_width: 64,
            inject_prelude: true,
            load_libs: true,

            source_map: true,

            elaborate_with_ternary: true,
            elaborate_split_declarators: true,
            elaborate_explicit_return: true,
            elaborate_parenthesize_casts: false,
            retain_comptime_as_comment: false,
            retain_comptime_comment_max_length: 120,

            emit_overwrite: true,
            emit_create_dirs: true,
            emit_dry_run: false,

            event_handler: None,
            timings: false,
            validate_builtin_libs: false,
            verify_mir: true,
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
            .field("resolve_mode", &self.resolve_mode)
            .field(
                "disallow_ambiguous_tree_literal",
                &self.disallow_ambiguous_tree_literal,
            )
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
            .field(
                "elaborate_parenthesize_casts",
                &self.elaborate_parenthesize_casts,
            )
            .field(
                "retain_comptime_as_comment",
                &self.retain_comptime_as_comment,
            )
            .field(
                "retain_comptime_comment_max_length",
                &self.retain_comptime_comment_max_length,
            )
            .field("emit_overwrite", &self.emit_overwrite)
            .field("emit_create_dirs", &self.emit_create_dirs)
            .field("emit_dry_run", &self.emit_dry_run)
            .field("event_handler", &self.event_handler.is_some())
            .field("timings", &self.timings)
            .field("validate_builtin_libs", &self.validate_builtin_libs)
            .field("verify_mir", &self.verify_mir)
            .finish()
    }
}

/// Compile files and sources into something (via DIR).
/// #Architecture: should Compiler be per-target? what about comptime though?
#[allow(clippy::type_complexity)]
pub struct Compiler {
    /// The session (shared state).
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The options for compiling.
    pub options: CompilerOptions,
    /// Base resolver reused for import resolution option variants.
    pub(crate) base_resolver: Resolver,

    /// Seen errors for deduplication.
    seen_errors: Mutex<Vec<TaskError>>,
    /// Seen warnings for deduplication.
    seen_warnings: Mutex<Vec<TaskWarning>>,
    /// The pending compiler diagnostics (transient).
    pub pending_diagnostics: DiagnosticCollector,

    /// The shared comptime target configuration.
    pub comptime_target: Target,

    /// The queue of compiler tasks.
    pub(super) queue: TaskQueue,
    /// Compilation statistics.
    pub stats: Arc<CompilerStats>,
    /// Cache registry for compiler artifacts.
    pub cache: CacheRegistry,
    /// Snapshot of strings for signature hashing.
    signature_strings: Mutex<Option<CompilerStringSnapshot>>,

    /// Locks for serializing module creation per (URI, loader) pair.
    /// The loader salt distinguishes imports with non-default loaders.
    import_locks: DashMap<(Uri, Option<String>), Arc<Mutex<Option<ModuleId>>>>,
}

#[derive(Debug)]
struct CompilerStringSnapshot {
    /// The string pool size used for this snapshot.
    string_count: usize,
    /// The immutable string snapshot.
    strings: Arc<ImmutableStringPool>,
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
        let timings = options.timings;
        let base_resolver = Resolver::from_program(&program, options.import_resolve.clone());

        let compiler = Self {
            session,
            program,
            options,
            base_resolver,
            seen_errors: Mutex::new(Vec::new()),
            seen_warnings: Mutex::new(Vec::new()),
            pending_diagnostics: DiagnosticCollector::new(),
            comptime_target,
            queue: TaskQueue::new(),
            import_locks: DashMap::new(),
            stats: Arc::new(CompilerStats::new_with_timings(timings)),
            cache: CacheRegistry::new(),
            signature_strings: Mutex::new(None),
        };

        // load workspace index snapshot when available
        if let Err(error) = compiler.load_workspace_index() {
            tracing::warn!(?error, "compiler.cache.workspace_index.load_failed");
        }

        compiler
    }

    /// Emit a compiler event to the event handler (if configured).
    #[inline]
    pub fn emit_event(&self, event: CompilerEvent) {
        if let Some(handler) = &self.options.event_handler {
            handler(event);
        }
    }

    /// Get an immutable string snapshot for signature hashing.
    pub(crate) fn signature_strings(&self) -> Arc<ImmutableStringPool> {
        // snapshot when the pool grows
        let string_count = self.program.strings.len();
        let mut snapshot = self.signature_strings.lock();
        if let Some(snapshot) = snapshot.as_ref()
            && snapshot.string_count == string_count
        {
            return snapshot.strings.clone();
        }

        let strings = Arc::new(self.program.strings.as_ref().clone().into_immutable());
        *snapshot = Some(CompilerStringSnapshot {
            string_count,
            strings: strings.clone(),
        });
        strings
    }

    /// Get the builtins (if loaded in program).
    pub fn builtins(&self) -> Option<&Arc<Builtins>> {
        self.program.builtins.as_ref()
    }

    /// Clone the base resolver with one request specific option set.
    pub(crate) fn resolver_with_options(
        &self,
        options: destack_resolver::ResolveOptions,
    ) -> Resolver {
        self.base_resolver.with_options(options)
    }

    /// Check if a module is a code module (vs data/text/binary).
    /// Non-code modules skip most compiler phases.
    #[inline]
    pub fn is_code_module(&self, module_id: ModuleId) -> bool {
        self.program.modules.get(module_id).read().is_code()
    }

    /// Get the import lock for a (URI, loader) pair.
    /// Used to serialize module creation and prevent race conditions when multiple import tasks
    /// resolve to the same file with the same loader.
    ///
    /// The `loader_salt` parameter distinguishes imports with non-default loaders
    /// (e.g., `with { type: "text" }`). Default loaders use `None`.
    pub(crate) fn get_import_lock(
        &self,
        uri: &Uri,
        loader_salt: Option<&str>,
    ) -> Arc<Mutex<Option<ModuleId>>> {
        let key = (uri.clone(), loader_salt.map(String::from));
        self.import_locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    /// Add an error to the compiler (deduplicated).
    pub fn error<T: Into<TaskError>>(&self, error: T) {
        let error: TaskError = error.into();
        if !self.should_emit_error(&error) {
            return;
        }
        let mut seen = self.seen_errors.lock();
        if seen.contains(&error) {
            return;
        }
        seen.push(error);
    }

    /// Add a warning to the compiler (deduplicated).
    pub fn warning<T: Into<TaskWarning>>(&self, warning: T) {
        let warning: TaskWarning = warning.into();
        if !self.should_emit_warning(&warning) {
            return;
        }
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
            // skip yielded dependency failures
            // (this is internal; they are already reported by other user-facing diagnostics)
            if error.is_yield_failed() {
                continue;
            }

            // apply diagnostic directive overrides
            let Some(severity) = self.error_effective_severity(&error) else {
                continue;
            };

            // build the diagnostic for the error
            let mut diagnostic = CompileDiagnostic::Error(error).to_diagnostic(&self.program);
            if severity != DiagnosticSeverity::Error {
                diagnostic.original_severity = Some(DiagnosticSeverity::Error);
                diagnostic.severity = severity;
            }

            // store the diagnostic
            self.pending_diagnostics.insert(diagnostic);
        }

        // convert warnings to diagnostics
        let warnings = {
            let mut seen = self.seen_warnings.lock();
            std::mem::take(&mut *seen)
        };
        for warning in warnings {
            // apply warning directive overrides
            // NOTE #Architecture: is Compiler.flush_diagnostics the right place for directive overrides?
            let Some(severity) = self.warning_effective_severity(&warning) else {
                continue;
            };

            // build the diagnostic for the warning
            let mut diagnostic = CompileDiagnostic::Warning(warning).to_diagnostic(&self.program);
            if severity != DiagnosticSeverity::Warning {
                diagnostic.original_severity = Some(DiagnosticSeverity::Warning);
                diagnostic.severity = severity;
            }

            // store the diagnostic
            self.pending_diagnostics.insert(diagnostic);
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
            Some(TaskStatus::Skipped { .. }) => Ok(()),
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
