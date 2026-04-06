use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock};
#[cfg(feature = "parallel")]
use std::thread;
use std::time::{Duration, Instant};

use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactPinSet, ArtifactVersion};
use destack_workspace::{Ref, Repository, RepositoryError, RepositorySnapshot, Revision};

#[cfg(test)]
use crate::tests::scenario::CompilerScenarioEvent;
use crate::{
    AnalyzeError, ArtifactRequirement, ArtifactTaskKeyExt, Compiler, CompilerContext,
    CompilerEvent, ElaborateError, ExecuteError, GenerateError, ImportError, InternalError,
    LinkError, LowerError, OptimizeError, RequirementSet, ResolveError, TaskError, TaskHandle,
    TaskId, TaskOutcome, TaskPhase, TaskStatus,
};

#[cfg(feature = "parallel")]
use super::parallel::compiler_stack_bytes;
use super::parallel::effective_worker_count;

/// Maximum number of yields allowed per task before treating it as an (internal) bug.
const MAX_TOTAL_YIELD_COUNT: u32 = 100;

/// The compiler state for one active execution scope.
#[derive(Debug)]
struct TaskContext {
    /// The running artifact key, when this scope belongs to one queued task.
    artifact_key: Option<ArtifactKey>,
    /// The pinned repository snapshot active for this execution scope.
    snapshot: RepositorySnapshot,
    /// The exact live artifact versions retained for this execution scope.
    retained_artifacts: ArtifactPinSet,
    /// The artifact requirements satisfied during this execution scope.
    requirements: Vec<ArtifactRequirement>,
}

impl TaskContext {
    /// Create one revision-only execution scope.
    fn for_revision(repository: &Arc<Repository>, revision: Revision) -> Self {
        Self {
            artifact_key: None,
            snapshot: repository
                .snapshot(revision)
                .unwrap_or_else(|error| panic!("failed to pin compiler revision: {error}")),
            retained_artifacts: ArtifactPinSet::new(repository.artifact_store().clone()),
            requirements: Vec::new(),
        }
    }

    /// Create one task execution scope.
    fn for_task(repository: &Arc<Repository>, handle: &TaskHandle) -> Self {
        Self {
            artifact_key: Some(handle.artifact_key),
            snapshot: repository
                .snapshot(handle.revision)
                .unwrap_or_else(|error| panic!("failed to pin compiler task revision: {error}")),
            retained_artifacts: ArtifactPinSet::new(repository.artifact_store().clone()),
            requirements: Vec::new(),
        }
    }

    /// Build one compiler context view from this task scope.
    fn compiler_context<'a>(&self, compiler: &'a Compiler) -> CompilerContext<'a> {
        CompilerContext::new(compiler, self.snapshot.clone(), self.artifact_key)
    }
}

thread_local! {
    /// Flag to detect if we're inside a worker loop (to prevent nested run_task calls).
    static IN_WORKER_LOOP: Cell<bool> = const { Cell::new(false) };
    /// The current compiler execution scope for this thread.
    static CURRENT_TASK_CONTEXT: RefCell<Option<TaskContext>> = const { RefCell::new(None) };
}

impl Compiler {
    /// Return the active compiler context for this execution scope when it exists.
    pub(crate) fn current_context_maybe(&self) -> Option<CompilerContext<'_>> {
        CURRENT_TASK_CONTEXT.with(|current| {
            current
                .borrow()
                .as_ref()
                .map(|context| context.compiler_context(self))
        })
    }

    /// Return the active compiler context for this execution scope.
    #[track_caller]
    pub(crate) fn current_context(&self) -> CompilerContext<'_> {
        let caller = std::panic::Location::caller();

        self.current_context_maybe().unwrap_or_else(|| {
            panic!(
                "missing compiler execution context at {}:{}:{}",
                caller.file(),
                caller.line(),
                caller.column()
            )
        })
    }

    /// Return one compiler context pinned to one explicit revision.
    pub fn context(&self, revision: Revision) -> Result<CompilerContext<'_>, RepositoryError> {
        let snapshot = self.repository.snapshot(revision)?;

        Ok(CompilerContext::new(self, snapshot, None))
    }

    /// Return the task currently executing on this worker thread.
    pub(crate) fn current_task(&self) -> Option<ArtifactKey> {
        CURRENT_TASK_CONTEXT.with(|current| {
            current
                .borrow()
                .as_ref()
                .and_then(|context| context.artifact_key)
        })
    }

    /// Return the artifact key currently executing on this worker thread.
    pub(crate) fn current_artifact_key(&self) -> Option<ArtifactKey> {
        self.current_task()
    }

    /// Return the revision currently active for this execution scope.
    ///
    /// NOTE #Architecture: compiler TLS is valid only inside one explicit
    /// execution scope per thread. Worker threads process at most one task
    /// at a time, and nested revision scopes are rejected below.
    #[cfg(test)]
    pub(crate) fn current_execution_revision(&self) -> Option<Revision> {
        self.current_context_maybe()
            .map(|context| context.revision())
    }

    /// Run one action inside one explicit compiler execution scope.
    fn with_current_task_context<T>(
        &self,
        context: TaskContext,
        action: impl FnOnce(&CompilerContext<'_>) -> T,
    ) -> T {
        let compiler_context = context.compiler_context(self);

        CURRENT_TASK_CONTEXT.with(|current| {
            let mut current = current.borrow_mut();
            assert!(
                current.is_none(),
                "nested compiler execution scopes are not allowed",
            );
            assert!(
                context.retained_artifacts.is_empty(),
                "compiler retained artifact set must be empty before entering a task scope",
            );
            assert!(
                context.requirements.is_empty(),
                "compiler requirement log must be empty before entering a task scope",
            );

            *current = Some(context);
        });

        let result = action(&compiler_context);

        CURRENT_TASK_CONTEXT.with(|current| {
            let context = current
                .borrow_mut()
                .take()
                .expect("compiler execution scope should be active on exit");

            assert!(
                context.requirements.is_empty(),
                "compiler requirement log must be empty after leaving a task scope",
            );

            drop(context);
        });

        result
    }

    /// Run one action inside an explicit revision scope.
    pub fn with_revision_scope<T>(
        &self,
        revision: Revision,
        action: impl FnOnce(&CompilerContext<'_>) -> T,
    ) -> T {
        self.with_current_task_context(
            TaskContext::for_revision(&self.repository, revision),
            action,
        )
    }

    /// Run one action inside one concrete task execution scope.
    ///
    /// The compiler relies on the invariant that one worker thread executes at
    /// most one task at a time. This helper makes that invariant explicit and
    /// rejects nested task scopes.
    fn with_task_scope<T>(
        &self,
        handle: &TaskHandle,
        action: impl FnOnce(&CompilerContext<'_>) -> T,
    ) -> T {
        self.with_current_task_context(TaskContext::for_task(&self.repository, handle), action)
    }

    /// Record one satisfied artifact requirement for the current task attempt.
    pub(crate) fn record_current_requirement(&self, requirement: ArtifactRequirement) {
        if !self.should_record_current_requirement(&requirement) {
            return;
        }

        CURRENT_TASK_CONTEXT.with(|current| {
            let mut current = current.borrow_mut();
            let context = current
                .as_mut()
                .expect("compiler requirement log requires one active task context");

            if context.requirements.iter().any(|existing| {
                existing.key == requirement.key && existing.stamp == requirement.stamp
            }) {
                return;
            }

            context.requirements.push(requirement);
        });
    }

    /// Return whether one exact requirement was already satisfied in the current task attempt.
    pub(crate) fn current_requirement_is_recorded(
        &self,
        requirement: &ArtifactRequirement,
    ) -> bool {
        CURRENT_TASK_CONTEXT.with(|current| {
            current.borrow().as_ref().is_some_and(|context| {
                context.requirements.iter().any(|existing| {
                    existing.key == requirement.key && existing.stamp == requirement.stamp
                })
            })
        })
    }

    /// Return whether one satisfied requirement should persist past task completion.
    fn should_record_current_requirement(&self, requirement: &ArtifactRequirement) -> bool {
        let _ = requirement;
        CURRENT_TASK_CONTEXT.with(|current| {
            current
                .borrow()
                .as_ref()
                .is_some_and(|context| context.artifact_key.is_some())
        })
    }

    /// Clear the current task requirement log.
    fn clear_current_requirements(&self) {
        CURRENT_TASK_CONTEXT.with(|current| {
            if let Some(context) = current.borrow_mut().as_mut() {
                context.requirements.clear();
            }
        });
    }

    /// Take the current task requirement log.
    fn take_current_requirements(&self) -> Vec<ArtifactRequirement> {
        CURRENT_TASK_CONTEXT.with(|current| {
            let mut current = current.borrow_mut();
            let Some(context) = current.as_mut() else {
                return Vec::new();
            };

            std::mem::take(&mut context.requirements)
        })
    }

    /// Clone the current task requirement log.
    pub(crate) fn current_requirements(&self) -> Vec<ArtifactRequirement> {
        CURRENT_TASK_CONTEXT.with(|current| {
            current
                .borrow()
                .as_ref()
                .map(|context| context.requirements.clone())
                .unwrap_or_default()
        })
    }

    /// Retain one exact live artifact version for the current execution scope.
    pub(crate) fn retain_current_artifact_version(&self, version: &ArtifactVersion) {
        CURRENT_TASK_CONTEXT.with(|current| {
            let mut current = current.borrow_mut();
            let Some(context) = current.as_mut() else {
                return;
            };

            context.retained_artifacts.pin(*version);
        });
    }

    /// Enqueue a task to the compiler.
    /// Noop if we already have the same task queued, returns the existing TaskId.
    pub fn enqueue<T: Into<ArtifactKey>>(
        &self,
        revision: Revision,
        artifact_key: T,
    ) -> (TaskId, bool) {
        let artifact_key: ArtifactKey = artifact_key.into();
        let event = format!(
            "{}.{}.enqueue",
            artifact_key.phase().name(),
            artifact_key.name()
        );
        let args = artifact_key.trace_args(revision, &self.repository, &self.artifacts);

        // reuse existing queued or running tasks directly
        if let Some(handle) = self.queue.find_task_handle(revision, &artifact_key) {
            if !handle.status.is_final() {
                self.stats.record_enqueue();
                tracing::trace!(%event, %args, task_id = ?handle.id, "compile.enqueue");
                return (handle.id, false);
            }

            // rerun stale final tasks when the current artifact requirement is no longer satisfied
            if !self.artifact_key_is_available(revision, &artifact_key) {
                let task_id = self
                    .queue
                    .try_requeue_final(revision, &artifact_key)
                    .expect("existing final task should requeue");
                self.stats.record_enqueue();
                tracing::trace!(%event, %args, ?task_id, "compile.enqueue");
                return (task_id, false);
            }
        }

        let (task_id, is_new) = self.queue.enqueue(revision, artifact_key);
        if is_new {
            self.stats.record_enqueue();
            tracing::trace!(%event, %args, ?task_id, "compile.enqueue");
        }
        (task_id, is_new)
    }

    /// Get the status of an artifact key.
    pub fn get_status(&self, revision: Revision, artifact_key: &ArtifactKey) -> Option<TaskStatus> {
        self.queue.find_task_status(revision, artifact_key)
    }

    /// Get the outcome of an artifact key.
    pub fn get_outcome(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskOutcome> {
        self.queue.find_task_outcome(revision, artifact_key)
    }

    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
        let workspace_reference = Ref::for_workspace_root(self.repository.workspace_root());
        let revision = self.repository.current(&workspace_reference).ok();

        let is_blocked_on_files = self.compile_queued(revision);

        assert!(
            !is_blocked_on_files,
            "compiler.compile() cannot expand the repository file world in place",
        );
    }

    /// Run the compiler loop without clearing previously accumulated diagnostics.
    ///
    /// Returns true when the pass stopped because it is blocked on file requirements.
    pub(crate) fn compile_queued(&self, revision: Option<Revision>) -> bool {
        self.queue.clear_blocked_on_files();

        // resolve worker count for this build mode
        let worker_count = effective_worker_count(self.options.workers);

        // start stats tracking
        self.stats.start();

        // emit compilation started event
        self.emit_event(CompilerEvent::CompilationStarted { worker_count });

        // parallel build: spawn worker threads
        #[cfg(feature = "parallel")]
        thread::scope(|scope| {
            let stack_bytes = compiler_stack_bytes();
            for worker_index in 0..worker_count {
                thread::Builder::new()
                    .name(format!("compiler-worker-{worker_index}"))
                    .stack_size(stack_bytes)
                    .spawn_scoped(scope, || self.run_loop())
                    .expect("failed to spawn compiler worker thread");
            }
        });

        // non-parallel build: run a single local worker loop
        #[cfg(not(feature = "parallel"))]
        self.run_loop();

        let is_blocked_on_files = self.queue.is_blocked_on_files();

        // flush remaining diagnostics for one completed pass
        if !is_blocked_on_files {
            self.flush_diagnostics();
        }

        // emit compilation finished event with stats snapshot
        let module_count = revision
            .and_then(|revision| self.repository.workspace_module_ids(revision).ok())
            .map(|module_ids| module_ids.len())
            .unwrap_or(0);
        self.emit_event(CompilerEvent::CompilationFinished {
            stats: self.stats.snapshot_with_modules(module_count),
        });

        is_blocked_on_files
    }

    /// Worker loop that processes tasks from the ready queue.
    fn run_loop(&self) -> bool {
        IN_WORKER_LOOP.set(true);
        loop {
            // mark this worker active before claiming work
            // this avoids transient "done" observations between pop and execution
            self.queue.begin_work();

            // try to pop a task from the ready queue
            let Some(task_id) = self.queue.pop_ready() else {
                self.queue.end_work();

                // no task available, wait for work or completion
                if !self.queue.wait_for_work() {
                    // if tasks are still pending, fail stalled yields instead of exiting silently
                    if self.queue.has_pending_non_final_tasks() {
                        let yielded_tasks = self.queue.yielded_tasks_with_requirements();

                        // a fixed-snapshot pass stops when any yielded task needs file expansion
                        if yielded_tasks
                            .iter()
                            .any(|(_, requirement)| requirement.has_file_requirements())
                        {
                            self.queue.mark_blocked_on_files();
                            break;
                        }

                        self.fail_stalled_yielded_tasks();
                        continue;
                    }
                    break;
                }
                continue;
            };

            // skip stale ready entries left behind by superseded passes
            if !matches!(self.queue.get_status(task_id), Some(TaskStatus::Queued)) {
                continue;
            }

            self.step_task(task_id);
            self.queue.end_work();
        }
        IN_WORKER_LOOP.set(false);

        self.queue.is_blocked_on_files()
    }

    /// Fail yielded tasks when the scheduler has no ready work but tasks are still pending.
    fn fail_stalled_yielded_tasks(&self) {
        let yielded_tasks = self.queue.yielded_tasks_with_requirements();

        for (task_id, requirement) in yielded_tasks {
            let handle = self.queue.get_task(task_id);
            let error = self.get_yield_failed_error(task_id, &requirement);
            let dependencies =
                self.live_dependencies_for_requirement_set(&handle.artifact_key, &requirement);

            self.publish_failed_artifact_version(
                handle.revision,
                handle.artifact_key,
                dependencies,
            );
            self.queue.set_status(
                task_id,
                TaskStatus::Failed {
                    error: error.clone(),
                },
            );

            // report the yielded task failure under its execution revision
            self.error_for_artifact(handle.revision, Some(handle.artifact_key), error);

            self.fail_waiters(task_id);
        }
    }

    /// Run a task and its dependencies until it is final (Complete, Skipped, or Error).
    /// If the task would yield with no progress possible, converts to Error.
    ///
    /// # Panics
    /// Panics if called from within `worker_loop`. Use yielding instead.
    pub fn run_task<T: Into<ArtifactKey>>(
        &self,
        revision: Revision,
        artifact_key: T,
    ) -> TaskOutcome {
        assert!(
            !IN_WORKER_LOOP.get(),
            "run_task_loop cannot be called from within worker_loop"
        );
        let (target_id, _) = self.enqueue(revision, artifact_key);
        loop {
            // check if target reached a final state
            if let Some(status) = self.queue.get_status(target_id) {
                match status {
                    TaskStatus::Complete => {
                        return TaskOutcome::Complete;
                    }
                    TaskStatus::Skipped => {
                        return TaskOutcome::Skipped;
                    }
                    TaskStatus::Failed { error } => {
                        return TaskOutcome::Error { error };
                    }
                    _ => {}
                }
            }

            // try to pop and run a ready task (non-blocking)
            self.queue.begin_work();
            let Some(next_id) = self.queue.pop_ready() else {
                self.queue.end_work();

                // no ready tasks; if target is yielded, convert to error (deadlock)
                if let Some(TaskStatus::Yielded { requirement }) = self.queue.get_status(target_id)
                {
                    return TaskOutcome::Error {
                        error: self.get_yield_failed_error(target_id, &requirement),
                    };
                }
                panic!("task {target_id:?} did not reach a final state");
            };
            self.step_task(next_id);
            self.queue.end_work();
        }
    }

    /// Process a single task 'step' (run until outcome, not final state).
    fn step_task(&self, task_id: TaskId) {
        let handle = self.queue.get_task(task_id);

        // get task description for events
        let description =
            handle
                .artifact_key
                .trace_args(handle.revision, &self.repository, &self.artifacts);

        // emit task started event
        self.emit_event(CompilerEvent::TaskStarted {
            task_id,
            artifact_key: handle.artifact_key,
            phase: handle.phase(),
            description: description.clone(),
        });

        // process task
        let started_at = Instant::now();
        self.queue.set_status(task_id, TaskStatus::Running);
        self.with_task_scope(&handle, |context| {
            self.clear_current_requirements();
            let outcome = self.process_task(context, &handle);

            // handle outcome
            let elapsed = started_at.elapsed();
            if let Some(threshold) = slow_task_threshold()
                && elapsed >= threshold
            {
                let event = format!(
                    "{}.{}.slow",
                    handle.phase().name(),
                    handle.artifact_key.name()
                );
                tracing::info!(%event, %description, ?task_id, ?elapsed, "compile.task.slow");
                self.stats.record_slow_task();

                // emit slow task event
                self.emit_event(CompilerEvent::TaskSlow {
                    task_id,
                    artifact_key: handle.artifact_key,
                    phase: handle.phase(),
                    elapsed,
                    description: description.clone(),
                });
            }
            self.handle_outcome(task_id, &handle, outcome, elapsed, description);
        });
    }

    /// Process a compiler task and return the outcome.
    fn process_task(&self, context: &CompilerContext<'_>, handle: &TaskHandle) -> TaskOutcome {
        // trace
        let artifact_key = &handle.artifact_key;
        let task_id = handle.id;
        let args = artifact_key.trace_args(handle.revision, &self.repository, &self.artifacts);
        let event_name = if handle.last_outcome.is_some() {
            "resume"
        } else {
            "start"
        };
        let event = format!(
            "{}.{}.{}",
            artifact_key.phase().name(),
            artifact_key.name(),
            event_name
        );
        tracing::debug!(%event, %args, ?task_id);

        // process

        match artifact_key {
            ArtifactKey::ModuleGraph { profile } => {
                self.process_module_graph(*profile, &context).into()
            }
            ArtifactKey::Ast { module } => self.process_ast(*module, &context).into(),
            ArtifactKey::DirBase { module } => self.process_dir_base(*module, &context).into(),
            ArtifactKey::DirPrepared { module, profile } => self
                .process_dir_prepared(*module, *profile, &context)
                .into(),
            ArtifactKey::LanguageEnvironment { profile } => {
                self.process_language_environment(*profile, &context).into()
            }
            ArtifactKey::IntrinsicEnvironment { profile } => self
                .process_intrinsic_environment(*profile, &context)
                .into(),
            ArtifactKey::LibraryEnvironment { profile } => {
                self.process_library_environment(*profile, &context).into()
            }
            ArtifactKey::DirResolved { module, profile } => self
                .process_dir_resolved(*module, *profile, &context)
                .into(),
            ArtifactKey::DirDeclared { module, profile } => self
                .process_dir_declared(*module, *profile, &context)
                .into(),
            ArtifactKey::DirInterface { module, profile } => self
                .process_dir_interface(*module, *profile, &context)
                .into(),
            ArtifactKey::DirAnalyzed { module, profile } => self
                .process_dir_analyzed(*module, *profile, &context)
                .into(),
            ArtifactKey::DirElaborated { module, profile } => self
                .process_dir_elaborated(*module, *profile, &context)
                .into(),
            ArtifactKey::DirPatched { module, profile } => {
                self.process_dir_patched(*module, *profile, &context).into()
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            } => self
                .process_mir(*module, *profile, *target, &context)
                .into(),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .process_mir_optimized(*module, *profile, *target, &context)
                .into(),
            ArtifactKey::ModuleOutput { module, target } => {
                let profile = context
                    .profile_id_for_target(*module, target)
                    .unwrap_or_else(|| {
                        panic!("missing profile for module {module:?} target {target:?}")
                    });

                self.process_module_output(*module, profile, *target, &context)
                    .into()
            }
            ArtifactKey::PackageOutput { package, target } => self
                .process_package_output(*package, *target, &context)
                .into(),
        }
    }

    /// Handle the outcome of a processed task.
    fn handle_outcome(
        &self,
        task_id: TaskId,
        handle: &TaskHandle,
        outcome: TaskOutcome,
        elapsed: Duration,
        description: String,
    ) {
        // record per-task timing
        let task_name = format!("{}.{}", handle.phase().name(), handle.artifact_key.name());
        self.stats.record_task_name_time(&task_name, elapsed);

        let mut requeued = false;
        match &outcome {
            // complete and wake waiters
            TaskOutcome::Complete => {
                let event = format!(
                    "{}.{}.complete",
                    handle.phase().name(),
                    handle.artifact_key.name()
                );
                tracing::debug!(%event, %description, ?task_id);

                // test interleavings
                #[cfg(test)]
                self.emit_scenario_event(CompilerScenarioEvent::BeforeTaskCommit {
                    artifact_key: handle.artifact_key,
                });

                let _ = self.take_current_requirements();
                self.queue.set_status(task_id, TaskStatus::Complete);
                self.wake_waiters(task_id);
                self.stats.record_complete();
                self.stats.record_phase_time(handle.phase(), elapsed);

                // record per-package time from task anchor
                let anchor = handle.artifact_key.anchor();
                if let Some(module_id) = anchor.module_id() {
                    if let Some(module) = self
                        .repository
                        .module(handle.revision, module_id)
                        .ok()
                        .flatten()
                    {
                        self.stats.record_package_time(module.package_id, elapsed);
                    }
                } else if let Some(package_id) = anchor.package_id() {
                    self.stats.record_package_time(package_id, elapsed);
                }

                // emit task completed event
                self.emit_event(CompilerEvent::TaskCompleted {
                    task_id,
                    artifact_key: handle.artifact_key,
                    phase: handle.phase(),
                    elapsed,
                    description,
                });
            }
            TaskOutcome::Skipped => {
                let event = format!(
                    "{}.{}.skip",
                    handle.phase().name(),
                    handle.artifact_key.name()
                );
                tracing::debug!(%event, %description, ?task_id);
                self.clear_current_requirements();
                self.queue.set_status(task_id, TaskStatus::Skipped);
                self.stats.record_skip();
                self.stats.record_phase_time(handle.phase(), elapsed);
                self.wake_waiters(task_id);

                // record per-package time from task anchor
                let anchor = handle.artifact_key.anchor();
                if let Some(module_id) = anchor.module_id() {
                    if let Some(module) = self
                        .repository
                        .module(handle.revision, module_id)
                        .ok()
                        .flatten()
                    {
                        self.stats.record_package_time(module.package_id, elapsed);
                    }
                } else if let Some(package_id) = anchor.package_id() {
                    self.stats.record_package_time(package_id, elapsed);
                }

                // emit task skipped event
                self.emit_event(CompilerEvent::TaskSkipped {
                    task_id,
                    artifact_key: handle.artifact_key,
                    phase: handle.phase(),
                    description,
                });
            }
            // error and fail waiters
            TaskOutcome::Error { error } => {
                let event = format!(
                    "{}.{}.error",
                    handle.phase().name(),
                    handle.artifact_key.name()
                );
                let mut dependencies =
                    self.live_dependencies_for_current_artifact(&handle.artifact_key);
                if let Some(requirement) = error.blocking_requirement() {
                    dependencies.extend(
                        self.live_dependencies_for_requirement_set(
                            &handle.artifact_key,
                            requirement,
                        ),
                    );
                }
                tracing::debug!(%event, %description, ?task_id);
                self.stats.record_fail();
                self.clear_current_requirements();
                self.publish_failed_artifact_version(
                    handle.revision,
                    handle.artifact_key,
                    dependencies,
                );
                self.queue.set_status(
                    task_id,
                    TaskStatus::Failed {
                        error: error.clone(),
                    },
                );
                self.error(error.clone());
                self.fail_waiters(task_id);

                // emit task failed event
                self.emit_event(CompilerEvent::TaskFailed {
                    task_id,
                    artifact_key: handle.artifact_key,
                    phase: handle.phase(),
                    error: error.clone(),
                });
            }
            // yield if possible
            TaskOutcome::Yield { requirement } => {
                // check for yield errors
                if let Some(internal_error) = self.check_yield(task_id, handle, requirement) {
                    let event = format!(
                        "{}.{}.circuit",
                        handle.phase().name(),
                        handle.artifact_key.name()
                    );
                    let mut dependencies =
                        self.live_dependencies_for_current_artifact(&handle.artifact_key);
                    dependencies.extend(
                        self.live_dependencies_for_requirement_set(
                            &handle.artifact_key,
                            requirement,
                        ),
                    );
                    tracing::error!(%event, %description, ?task_id);
                    self.publish_failed_artifact_version(
                        handle.revision,
                        handle.artifact_key,
                        dependencies,
                    );
                    self.queue.set_status(
                        task_id,
                        TaskStatus::Failed {
                            error: internal_error.clone().into(),
                        },
                    );
                    self.error(internal_error.clone());
                    self.fail_waiters(task_id);

                    // emit task failed event for circuit breaker
                    self.emit_event(CompilerEvent::TaskFailed {
                        task_id,
                        artifact_key: handle.artifact_key,
                        phase: handle.phase(),
                        error: internal_error.into(),
                    });
                    self.clear_current_requirements();
                    return;
                }

                self.clear_current_requirements();

                // mark yielded and register requirement waits
                self.queue.set_status(
                    task_id,
                    TaskStatus::Yielded {
                        requirement: requirement.clone(),
                    },
                );
                requeued = self.yield_requirement(task_id, requirement);

                // count only yields that actually wait: immediate requeues do not block
                if !requeued {
                    self.stats.record_yield();
                    self.queue.increment_yield_count(task_id);
                }

                // emit task yielded event
                self.emit_event(CompilerEvent::TaskYielded {
                    task_id,
                    artifact_key: handle.artifact_key,
                    phase: handle.phase(),
                });
            }
        }
        // remember outcome (skip if already requeued, since that clears last_outcome)
        if !requeued {
            self.queue.set_last_outcome(task_id, outcome);
        }
    }

    /// Check if a yield should trigger an internal error (i.e. circuit break).
    fn check_yield(
        &self,
        task_id: TaskId,
        handle: &TaskHandle,
        requirement: &RequirementSet,
    ) -> Option<InternalError> {
        // check for repeated yield to the same requirement
        if let Some(TaskOutcome::Yield {
            requirement: previous_requirement,
        }) = &handle.last_outcome
            && *previous_requirement == *requirement
        {
            return Some(InternalError::SuspiciousYield {
                task_id,
                requirement: requirement.clone(),
            });
        }

        // check for excessive yields
        if handle.yield_count >= MAX_TOTAL_YIELD_COUNT {
            return Some(InternalError::ExcessiveYield {
                task_id,
                artifact_key: handle.artifact_key,
                requirement: requirement.clone(),
                yield_count: handle.yield_count,
            });
        }

        None
    }

    /// Yield to a requirement, register waiters, and check if already satisfied.
    /// Returns true if the task was immediately requeued.
    fn yield_requirement(&self, waiter_id: TaskId, requirement: &RequirementSet) -> bool {
        // first, register all sub-requirements
        self.register_requirement(waiter_id, requirement);

        // then check if the full requirement is already satisfied
        if self.is_requirement_satisfied(requirement) {
            self.queue.try_requeue_yielded(waiter_id);
            return true;
        }

        false
    }

    /// Register waiters for all sub-requirements without re-queuing.
    fn register_requirement(&self, waiter_id: TaskId, requirement: &RequirementSet) {
        let waiter = self.queue.get_task(waiter_id);

        requirement.for_each_artifact(|requirement| {
            let (required_id, _) = self.enqueue(waiter.revision, requirement.key);

            if let Some(status) = self.queue.get_status(required_id)
                && status.is_final()
            {
                return;
            }

            self.queue
                .add_waiter(waiter.revision, requirement.key, waiter_id);
        });
    }

    /// Check if a requirement is satisfied.
    fn is_requirement_satisfied(&self, requirement: &RequirementSet) -> bool {
        requirement.all(|requirement| match requirement {
            crate::Requirement::Artifact(requirement) => {
                let waiter = self.current_context();
                self.artifact_satisfies_stamp_for_revision(
                    waiter.revision(),
                    &requirement.key,
                    requirement.stamp,
                )
            }
            crate::Requirement::File(..) => false,
        })
    }

    /// Wake all tasks waiting for the completed artifact key.
    fn wake_waiters(&self, completed_id: TaskId) {
        let completed_handle = self.queue.get_task(completed_id);
        let completed_key = completed_handle.artifact_key;
        let waiters = self
            .queue
            .take_waiters(completed_handle.revision, &completed_key);
        for waiter_id in waiters {
            // a completed dependency means the waiter must recompute its requirements
            if matches!(
                self.queue.get_status(waiter_id),
                Some(TaskStatus::Yielded { .. })
            ) {
                self.queue.try_requeue_yielded(waiter_id);
            }
        }
    }

    /// Fail all tasks waiting for the failed artifact key.
    fn fail_waiters(&self, failed_id: TaskId) {
        let failed_handle = self.queue.get_task(failed_id);
        let failed_key = failed_handle.artifact_key;
        let waiters = self.queue.take_waiters(failed_handle.revision, &failed_key);
        for waiter_id in waiters {
            if let Some(TaskStatus::Yielded { requirement }) = self.queue.get_status(waiter_id) {
                self.fail_waiter(waiter_id, &requirement);
            }
        }
    }

    /// Publish one failed exact artifact version with its blocking dependencies.
    fn publish_failed_artifact_version(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        dependencies: Vec<ArtifactDependency>,
    ) {
        let version = self.repository.artifact_version(revision, &artifact_key);

        self.artifacts.publish_failure(version, dependencies);
    }

    /// Fail a waiter using the fallback error from its requirement.
    fn fail_waiter(&self, waiter_id: TaskId, requirement: &RequirementSet) {
        let error: TaskError = Self::get_fallback_error(requirement)
            .unwrap_or_else(|| self.get_yield_failed_error(waiter_id, requirement));
        let handle = self.queue.get_task(waiter_id);
        let dependencies =
            self.live_dependencies_for_requirement_set(&handle.artifact_key, requirement);

        self.publish_failed_artifact_version(handle.revision, handle.artifact_key, dependencies);
        self.queue.set_status(
            waiter_id,
            TaskStatus::Failed {
                error: error.clone(),
            },
        );

        // yielded waiter diagnostics still belong to the waiter's revision
        self.error_for_artifact(handle.revision, Some(handle.artifact_key), error);
        self.fail_waiters(waiter_id);
    }

    /// Get the fallback error from a requirement.
    fn get_fallback_error(requirement: &RequirementSet) -> Option<TaskError> {
        requirement.find_map(|requirement| requirement.fallback_error().cloned())
    }

    /// Create one unsatisfied requirement error for the given waiter.
    fn get_yield_failed_error(&self, waiter_id: TaskId, requirement: &RequirementSet) -> TaskError {
        let handle = self.queue.get_task(waiter_id);
        match handle.phase() {
            TaskPhase::Import => ImportError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Resolve => ResolveError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Analyze => AnalyzeError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Elaborate => ElaborateError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Execute => ExecuteError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Lower => LowerError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Optimize => OptimizeError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Generate => GenerateError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
            TaskPhase::Link => LinkError::UnsatisfiedRequirement {
                requirement: requirement.clone(),
            }
            .into(),
        }
    }
}

/// Read the slow task logging threshold from the environment.
fn slow_task_threshold() -> Option<Duration> {
    static THRESHOLD: OnceLock<Option<Duration>> = OnceLock::new();
    *THRESHOLD.get_or_init(|| {
        let value = std::env::var("DESTACK_SLOW_TASK_MS").ok()?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        let ms: u64 = trimmed.parse().ok()?;
        if ms == 0 {
            None
        } else {
            Some(Duration::from_millis(ms))
        }
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use destack_workspace::{Change, Ref, Repository};

    use crate::{Compiler, CompilerOptions};

    /// Keep anonymous revisions alive while one compiler revision scope is active.
    #[test]
    fn test_with_revision_scope_roots_anonymous_revision() {
        let root = unique_test_root("compiler-revision-scope");
        fs::create_dir_all(&root).expect("compiler revision scope test root should exist");

        let repository = Arc::new(Repository::open_root(root.clone()));
        let compiler = Compiler::new(Arc::clone(&repository), CompilerOptions::default());
        let reference = Ref::for_workspace_root(&root);
        let base_revision = repository
            .current(&reference)
            .expect("workspace root ref should exist");
        let anonymous_revision = repository
            .apply_to_revision(
                base_revision,
                Change::add_text("src/example.ts", "export const value = 1"),
            )
            .expect("anonymous revision should publish");

        // pinned compiler scope
        compiler.with_revision_scope(anonymous_revision, |_context| {
            repository
                .apply(
                    &reference,
                    Change::add_text("src/other.ts", "export const other = 2"),
                )
                .expect("workspace ref should advance");
            repository.prune_unreachable_file_state();

            assert!(repository.revision(anonymous_revision).is_ok());
        });

        // scope dropped
        repository.prune_unreachable_file_state();
        assert!(repository.revision(anonymous_revision).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Build one unique workspace root for one compiler test.
    fn unique_test_root(prefix: &str) -> std::path::PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let process_id = std::process::id();

        std::env::temp_dir().join(format!("destack-{prefix}-{process_id}-{timestamp}"))
    }
}
