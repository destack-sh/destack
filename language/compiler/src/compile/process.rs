use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
#[cfg(feature = "parallel")]
use std::thread;
use std::time::{Duration, Instant};

use destack_workspace::ArtifactKey;

use crate::{
    AnalyzeError, ArtifactRequirement, ArtifactRequirementSet, ArtifactTaskKeyExt, Compiler,
    CompilerEvent, ElaborateError, ExecuteError, GenerateError, ImportError, InternalError,
    LinkError, LowerError, OptimizeError, ResolveError, TaskError, TaskHandle, TaskId, TaskOutcome,
    TaskPhase, TaskStatus,
};

#[cfg(feature = "parallel")]
use super::parallel::compiler_stack_bytes;
use super::parallel::effective_worker_count;

/// Maximum number of yields allowed per task before treating it as an (internal) bug.
const MAX_TOTAL_YIELD_COUNT: u32 = 100;

thread_local! {
    /// Flag to detect if we're inside a worker loop (to prevent nested run_task calls).
    static IN_WORKER_LOOP: Cell<bool> = const { Cell::new(false) };
    /// The task currently being executed on this worker.
    static CURRENT_TASK: RefCell<Option<ArtifactKey>> = const { RefCell::new(None) };
    /// The exact artifact requirements satisfied by the current task attempt.
    static CURRENT_REQUIREMENTS: RefCell<Vec<ArtifactRequirement>> = const { RefCell::new(Vec::new()) };
}

impl Compiler {
    /// Return the task currently executing on this worker thread.
    pub(crate) fn current_task(&self) -> Option<ArtifactKey> {
        CURRENT_TASK.with(|current| current.borrow().clone())
    }

    /// Return the artifact key currently executing on this worker thread.
    pub(crate) fn current_artifact_key(&self) -> Option<ArtifactKey> {
        self.current_task()
    }

    /// Record one satisfied artifact requirement for the current task attempt.
    pub(crate) fn record_current_requirement(&self, requirement: ArtifactRequirement) {
        if !self.should_record_current_requirement(&requirement) {
            return;
        }

        CURRENT_REQUIREMENTS.with(|requirements| {
            let mut requirements = requirements.borrow_mut();
            if requirements.iter().any(|existing| {
                existing.key == requirement.key && existing.dependency == requirement.dependency
            }) {
                return;
            }

            requirements.push(requirement);
        });
    }

    /// Return whether one exact requirement was already satisfied in the current task attempt.
    pub(crate) fn current_requirement_is_recorded(
        &self,
        requirement: &ArtifactRequirement,
    ) -> bool {
        CURRENT_REQUIREMENTS.with(|requirements| {
            requirements.borrow().iter().any(|existing| {
                existing.key == requirement.key && existing.dependency == requirement.dependency
            })
        })
    }

    /// Return whether one satisfied requirement should persist past task completion.
    fn should_record_current_requirement(&self, requirement: &ArtifactRequirement) -> bool {
        let _ = requirement;
        self.current_artifact_key().is_some()
    }

    /// Clear the current task requirement log.
    fn clear_current_requirements(&self) {
        CURRENT_REQUIREMENTS.with(|requirements| {
            requirements.borrow_mut().clear();
        });
    }

    /// Take the current task requirement log.
    fn take_current_requirements(&self) -> Vec<ArtifactRequirement> {
        CURRENT_REQUIREMENTS.with(|requirements| std::mem::take(&mut *requirements.borrow_mut()))
    }

    /// Enqueue a task to the compiler.
    /// Noop if we already have the same task queued, returns the existing TaskId.
    pub fn enqueue<T: Into<ArtifactKey>>(&self, artifact_key: T) -> (TaskId, bool) {
        let artifact_key: ArtifactKey = artifact_key.into();
        let event = format!(
            "{}.{}.enqueue",
            artifact_key.phase().name(),
            artifact_key.name()
        );
        let args = artifact_key.trace_args(&self.program);

        // reuse existing queued or running tasks directly
        if let Some(handle) = self.queue.find_task_handle(&artifact_key) {
            if !handle.status.is_final() {
                self.stats.record_enqueue();
                tracing::trace!(%event, %args, task_id = ?handle.id, "compile.enqueue");
                return (handle.id, false);
            }

            // rerun stale final tasks when the current artifact requirement is no longer satisfied
            if !self.artifact_key_is_available(&artifact_key) {
                let task_id = self
                    .queue
                    .try_requeue_final(&artifact_key)
                    .expect("existing final task should requeue");
                self.stats.record_enqueue();
                tracing::trace!(%event, %args, ?task_id, "compile.enqueue");
                return (task_id, false);
            }
        }

        let (task_id, is_new) = self.queue.enqueue(artifact_key);
        if is_new {
            self.stats.record_enqueue();
            tracing::trace!(%event, %args, ?task_id, "compile.enqueue");
        }
        (task_id, is_new)
    }

    /// Get the status of an artifact key.
    pub fn get_status(&self, artifact_key: &ArtifactKey) -> Option<TaskStatus> {
        self.queue.find_task_status(artifact_key)
    }

    /// Get the outcome of an artifact key.
    pub fn get_outcome(&self, artifact_key: &ArtifactKey) -> Option<TaskOutcome> {
        self.queue.find_task_outcome(artifact_key)
    }

    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
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

        // flush remaining diagnostics
        self.flush_diagnostics();

        // flush workspace index
        if let Err(error) = self.flush_workspace_index() {
            tracing::warn!(?error, "compile.cache.workspace_index.flush_failed");
        }

        // emit compilation finished event with stats snapshot
        self.emit_event(CompilerEvent::CompilationFinished {
            stats: self.stats.snapshot_with_modules(self.program.modules.len()),
        });
    }

    /// Worker loop that processes tasks from the ready queue.
    fn run_loop(&self) {
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
                        self.fail_stalled_yielded_tasks();
                        continue;
                    }
                    break;
                }
                continue;
            };

            self.step_task(task_id);
            self.queue.end_work();
        }
        IN_WORKER_LOOP.set(false);
    }

    /// Fail yielded tasks when the scheduler has no ready work but tasks are still pending.
    fn fail_stalled_yielded_tasks(&self) {
        let yielded_tasks = self.queue.yielded_tasks_with_requirements();

        for (task_id, requirement) in yielded_tasks {
            let _handle = self.queue.get_task(task_id);
            let error = self.get_yield_failed_error(task_id, &requirement);
            self.queue.set_status(
                task_id,
                TaskStatus::Failed {
                    error: error.clone(),
                },
            );
            self.error(error);
            self.fail_waiters(task_id);
        }
    }

    /// Run a task and its dependencies until it is final (Complete, Skipped, or Error).
    /// If the task would yield with no progress possible, converts to Error.
    ///
    /// # Panics
    /// Panics if called from within `worker_loop`. Use yielding instead.
    pub fn run_task<T: Into<ArtifactKey>>(&self, artifact_key: T) -> TaskOutcome {
        assert!(
            !IN_WORKER_LOOP.get(),
            "run_task_loop cannot be called from within worker_loop"
        );
        let (target_id, _) = self.enqueue(artifact_key);
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
        let description = handle.artifact_key.trace_args(&self.program);

        // emit task started event
        self.emit_event(CompilerEvent::TaskStarted {
            task_id,
            artifact_key: handle.artifact_key.clone(),
            phase: handle.phase(),
            description: description.clone(),
        });

        // process task
        let started_at = Instant::now();
        self.queue.set_status(task_id, TaskStatus::Running);
        self.clear_current_requirements();
        CURRENT_TASK.with(|current| {
            *current.borrow_mut() = Some(handle.artifact_key.clone());
        });
        let outcome = self.process_task(&handle);
        CURRENT_TASK.with(|current| {
            current.borrow_mut().take();
        });

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
                artifact_key: handle.artifact_key.clone(),
                phase: handle.phase(),
                elapsed,
                description: description.clone(),
            });
        }
        self.handle_outcome(task_id, &handle, outcome, elapsed, description);
    }

    /// Process a compiler task and return the outcome.
    fn process_task(&self, handle: &TaskHandle) -> TaskOutcome {
        // trace
        let artifact_key = &handle.artifact_key;
        let task_id = handle.id;
        let args = artifact_key.trace_args(&self.program);
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
        let outcome = match artifact_key {
            ArtifactKey::ModuleGraph { profile } => self.process_module_graph(*profile).into(),
            ArtifactKey::Ast { module } => self.process_ast(*module).into(),
            ArtifactKey::DirBase { module } => self.process_dir_base(*module).into(),
            ArtifactKey::DirPrepared { module, profile } => {
                self.process_dir_prepared(*module, *profile).into()
            }
            ArtifactKey::LanguageEnvironment { profile } => {
                self.process_language_environment(*profile).into()
            }
            ArtifactKey::IntrinsicEnvironment { profile } => {
                self.process_intrinsic_environment(*profile).into()
            }
            ArtifactKey::LibraryEnvironment { profile } => {
                self.process_library_environment(*profile).into()
            }
            ArtifactKey::DirResolved { module, profile } => {
                self.process_dir_resolved(*module, *profile).into()
            }
            ArtifactKey::DirDeclared { module, profile } => {
                self.process_dir_declared(*module, *profile).into()
            }
            ArtifactKey::DirInterface { module, profile } => {
                self.process_dir_interface(*module, *profile).into()
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                self.process_dir_analyzed(*module, *profile).into()
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.process_dir_elaborated(*module, *profile).into()
            }
            ArtifactKey::DirPatched { module, profile } => {
                self.process_dir_patched(*module, *profile).into()
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            } => self.process_mir(*module, *profile, target.clone()).into(),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .process_mir_optimized(*module, *profile, target.clone())
                .into(),
            ArtifactKey::ModuleOutput { module, target } => {
                let profile = self
                    .program
                    .profile_id_for_target(*module, target)
                    .unwrap_or_else(|| {
                        panic!(
                            "missing profile for module {:?} target {:?}",
                            module, target
                        )
                    });

                self.process_module_output(*module, profile, target.clone())
                    .into()
            }
            ArtifactKey::PackageOutput { package, target } => {
                self.process_package_output(*package, target.clone()).into()
            }
        };

        outcome
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

                // publish the current dependency stamp before waking waiters
                self.commit_completed_artifact(&handle.artifact_key);
                self.queue
                    .set_final_requirements(task_id, self.take_current_requirements());
                self.queue.set_status(task_id, TaskStatus::Complete);
                self.wake_waiters(task_id);
                self.stats.record_complete();
                self.stats.record_phase_time(handle.phase(), elapsed);

                // record per-package time from task anchor
                let anchor = handle.artifact_key.anchor();
                if let Some(module_id) = anchor.module_id() {
                    let package_id = self.program.modules.get(module_id).package_id;
                    self.stats.record_package_time(package_id, elapsed);
                } else if let Some(package_id) = anchor.package_id() {
                    self.stats.record_package_time(package_id, elapsed);
                }

                // emit task completed event
                self.emit_event(CompilerEvent::TaskCompleted {
                    task_id,
                    artifact_key: handle.artifact_key.clone(),
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
                self.queue.clear_final_requirements(task_id);
                self.clear_current_requirements();
                self.queue.set_status(task_id, TaskStatus::Skipped);
                self.stats.record_skip();
                self.stats.record_phase_time(handle.phase(), elapsed);
                self.wake_waiters(task_id);

                // record per-package time from task anchor
                let anchor = handle.artifact_key.anchor();
                if let Some(module_id) = anchor.module_id() {
                    let package_id = self.program.modules.get(module_id).package_id;
                    self.stats.record_package_time(package_id, elapsed);
                } else if let Some(package_id) = anchor.package_id() {
                    self.stats.record_package_time(package_id, elapsed);
                }

                // emit task skipped event
                self.emit_event(CompilerEvent::TaskSkipped {
                    task_id,
                    artifact_key: handle.artifact_key.clone(),
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
                tracing::debug!(%event, %description, ?task_id);
                self.stats.record_fail();
                self.queue.clear_final_requirements(task_id);
                self.clear_current_requirements();
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
                    artifact_key: handle.artifact_key.clone(),
                    phase: handle.phase(),
                    error: error.clone(),
                });
            }
            // yield if possible
            TaskOutcome::Yield { requirement } => {
                self.clear_current_requirements();
                // check for yield errors
                if let Some(internal_error) = self.check_yield(task_id, handle, requirement) {
                    let event = format!(
                        "{}.{}.circuit",
                        handle.phase().name(),
                        handle.artifact_key.name()
                    );
                    tracing::error!(%event, %description, ?task_id);
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
                        artifact_key: handle.artifact_key.clone(),
                        phase: handle.phase(),
                        error: internal_error.into(),
                    });
                    return;
                }

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
                    artifact_key: handle.artifact_key.clone(),
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
        requirement: &ArtifactRequirementSet,
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
                artifact_key: handle.artifact_key.clone(),
                requirement: requirement.clone(),
                yield_count: handle.yield_count,
            });
        }

        None
    }

    /// Yield to a requirement, register waiters, and check if already satisfied.
    /// Returns true if the task was immediately requeued.
    fn yield_requirement(&self, waiter_id: TaskId, requirement: &ArtifactRequirementSet) -> bool {
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
    fn register_requirement(&self, waiter_id: TaskId, requirement: &ArtifactRequirementSet) {
        requirement.for_each(|requirement| {
            let (required_id, _) = self.enqueue(requirement.key.clone());

            if let Some(status) = self.queue.get_status(required_id)
                && status.is_final()
            {
                return;
            }

            self.queue.add_waiter(requirement.key.clone(), waiter_id);
        });
    }

    /// Check if a requirement is satisfied.
    fn is_requirement_satisfied(&self, requirement: &ArtifactRequirementSet) -> bool {
        requirement.all(|requirement| {
            self.artifact_satisfies_dependency(&requirement.key, requirement.dependency)
                && self.artifact_requirements_are_satisfied(&requirement.key)
        })
    }

    /// Wake all tasks waiting for the completed artifact key.
    fn wake_waiters(&self, completed_id: TaskId) {
        let completed_handle = self.queue.get_task(completed_id);
        let completed_key = completed_handle.artifact_key.clone();
        let waiters = self.queue.take_waiters(&completed_key);
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
        let failed_key = failed_handle.artifact_key.clone();
        let waiters = self.queue.take_waiters(&failed_key);
        for waiter_id in waiters {
            if let Some(TaskStatus::Yielded { requirement }) = self.queue.get_status(waiter_id) {
                self.fail_waiter(waiter_id, &requirement);
            }
        }
    }
    /// Fail a waiter using the fallback error from its requirement.
    fn fail_waiter(&self, waiter_id: TaskId, requirement: &ArtifactRequirementSet) {
        let error: TaskError = Self::get_fallback_error(requirement)
            .unwrap_or_else(|| self.get_yield_failed_error(waiter_id, requirement));
        self.queue.set_status(
            waiter_id,
            TaskStatus::Failed {
                error: error.clone(),
            },
        );
        self.error(error);
    }

    /// Get the fallback error from a requirement.
    fn get_fallback_error(requirement: &ArtifactRequirementSet) -> Option<TaskError> {
        requirement.find_map(|requirement| requirement.error.as_ref().map(|error| *error.clone()))
    }

    /// Create one unsatisfied requirement error for the given waiter.
    fn get_yield_failed_error(
        &self,
        waiter_id: TaskId,
        requirement: &ArtifactRequirementSet,
    ) -> TaskError {
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
