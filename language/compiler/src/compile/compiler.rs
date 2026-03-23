use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::ArtifactStore;
use destack_resolver::Resolver;
use destack_source::{DiagnosticCollector, DiagnosticSeverity, ModuleId, Uri};
use destack_workspace::{Program, Session, Target};
use parking_lot::Mutex;

use super::CompilerIndex;
#[cfg(test)]
use crate::TaskHandle;
#[cfg(test)]
use crate::tests::scenario::CompilerScenarioEvent;
use crate::{
    ArtifactRequirementCollector, ArtifactRequirementSet, CompileDiagnostic, CompilerEvent,
    CompilerOptions, CompilerStats, TaskError, TaskQueue, TaskWarning,
};

/// Compile files and sources into something (via DIR).
/// #Architecture: should Compiler be per-target? what about comptime though?
#[allow(clippy::type_complexity)]
pub struct Compiler {
    /// The session (shared state).
    pub session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The live artifact store for the program.
    pub artifacts: Arc<ArtifactStore>,
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
    /// Locks for serializing module creation per (URI, loader) pair.
    /// The loader salt distinguishes imports with non-default loaders.
    import_locks: DashMap<(Uri, Option<String>), Arc<Mutex<Option<ModuleId>>>>,
    /// Ephemeral compiler indices derived from artifacts.
    pub(crate) index: CompilerIndex,
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
        let comptime_target = Target::comptime("comptime");
        let timings = options.timings;
        let base_resolver = Resolver::from_program(&program, options.import_resolve.clone());
        let artifacts = session
            .get_artifacts_for_program(program.as_ref())
            .unwrap_or_else(|| Arc::new(ArtifactStore::new()));

        let compiler = Self {
            session,
            program,
            artifacts,
            options,
            base_resolver,
            seen_errors: Mutex::new(Vec::new()),
            seen_warnings: Mutex::new(Vec::new()),
            pending_diagnostics: DiagnosticCollector::new(),
            comptime_target,
            queue: TaskQueue::new(),
            import_locks: DashMap::new(),
            index: CompilerIndex::default(),
            stats: Arc::new(CompilerStats::new_with_timings(timings)),
        };

        // load workspace index when available
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

    /// Emit one internal compiler scenario event.
    #[cfg(test)]
    #[inline]
    pub(crate) fn emit_scenario_event(&self, event: CompilerScenarioEvent) {
        if let Some(handler) = &self.options.scenario_event_handler {
            handler(event);
        }
    }

    /// Snapshot all tracked task handles.
    #[cfg(test)]
    pub(crate) fn task_handles(&self) -> Vec<TaskHandle> {
        self.queue.task_handles()
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
        self.program.modules.get(module_id).is_code()
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

    /// Collect a result into a ArtifactRequirementCollector, reporting non-yield errors.
    ///
    /// Returns `Some(value)` on success, `None` on error (yield or hard error).
    /// Yields are collected into the collector, hard errors are reported via `self.error()`.
    pub fn collect<T, E>(
        &self,
        collector: &mut ArtifactRequirementCollector,
        result: Result<T, E>,
    ) -> Option<T>
    where
        E: TryInto<ArtifactRequirementSet, Error = E> + Into<TaskError>,
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

    /// Enqueue all artifact keys needed by a requirement set.
    pub fn enqueue_requirements(&self, requirement: &ArtifactRequirementSet) {
        requirement.for_each(|requirement| {
            self.enqueue(requirement.key.clone());
        });
    }

    /// Run one requirement-producing operation to completion.
    pub fn run_to_completion<T, E, F>(&self, mut action: F) -> Result<T, E>
    where
        F: FnMut(&Self) -> Result<T, E>,
        E: TryInto<ArtifactRequirementSet, Error = E>,
    {
        loop {
            let result = action(self);
            match result {
                Ok(value) => return Ok(value),
                Err(error) => match error.try_into() {
                    // unmet requirements: enqueue and keep building
                    Ok(requirement) => {
                        self.enqueue_requirements(&requirement);
                        self.compile();
                    }

                    // hard failure
                    Err(error) => return Err(error),
                },
            }
        }
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
            let mut diagnostic =
                CompileDiagnostic::Error(error).to_diagnostic(&self.program, &self.artifacts);
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
            let mut diagnostic =
                CompileDiagnostic::Warning(warning).to_diagnostic(&self.program, &self.artifacts);
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
}
