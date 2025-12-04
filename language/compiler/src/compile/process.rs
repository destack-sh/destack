use std::cell::Cell;
use std::thread;

use crate::{
    AnalyzeError, Compiler, ElaborateError, ExecuteError, GenerateError, InternalError, LinkError,
    LowerError, OptimizeError, ResolveError, Task, TaskDebug, TaskDependency, TaskError,
    TaskHandle, TaskId, TaskOutcome, TaskOutput, TaskPhase, TaskStatus, VerifyError,
};

/// Maximum number of yields allowed per task before treating it as an (internal) bug.
const MAX_TOTAL_YIELD_COUNT: u32 = 100;

thread_local! {
    /// Flag to detect if we're inside a worker loop (to prevent nested run_task calls).
    static IN_WORKER_LOOP: Cell<bool> = const { Cell::new(false) };
}

impl Compiler {
    /// Enqueue a task to the compiler.
    /// Noop if we already have the same task queued, returns the existing TaskId.
    pub fn enqueue<T: Into<Task>>(&self, task: T) -> (TaskId, bool) {
        let task: Task = task.into();
        let event = format!("{}.{}.enqueue", task.phase().name(), task.name());
        let args = task.trace_args(&self.program);
        let (task_id, is_new) = self.queue.enqueue(task);
        if is_new {
            tracing::trace!(%event, %args, ?task_id, "compile.enqueue");
        }
        (task_id, is_new)
    }

    /// Get the status of a task.
    pub fn get_status<T: Into<Task>>(&self, task: T) -> Option<TaskStatus> {
        let task: Task = task.into();
        self.queue.find_task_status(&task)
    }

    /// Get the outcome of a task.
    pub fn get_outcome<T: Into<Task>>(&self, task: T) -> Option<TaskOutcome> {
        let task: Task = task.into();
        self.queue.find_task_outcome(&task)
    }

    /// Get the output of a task
    pub fn get_output<T: Into<Task>>(&self, task: T) -> Option<TaskOutput> {
        let task: Task = task.into();
        self.queue.find_task_output(&task)
    }

    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
        // spawn worker threads
        thread::scope(|scope| {
            for _ in 0..self.options.workers {
                scope.spawn(|| self.run_loop());
            }
        });

        // flush remaining diagnostics
        self.flush_diagnostics();
    }

    /// Worker loop that processes tasks from the ready queue.
    fn run_loop(&self) {
        IN_WORKER_LOOP.set(true);
        loop {
            // try to pop a task from the ready queue
            let Some(task_id) = self.queue.pop_ready() else {
                // no task available, wait for work or completion
                if !self.queue.wait_for_work() {
                    break;
                }
                continue;
            };
            self.step_task(task_id);
        }
        IN_WORKER_LOOP.set(false);
    }

    /// Run a task and its dependencies until it is final (Complete or Error).
    /// If the task would yield with no progress possible, converts to Error.
    ///
    /// # Panics
    /// Panics if called from within `worker_loop`. Use yielding instead.
    pub fn run_task<T: Into<Task>>(&self, task: T) -> TaskOutcome {
        assert!(
            !IN_WORKER_LOOP.get(),
            "run_task_loop cannot be called from within worker_loop"
        );
        let (target_id, _) = self.enqueue(task);
        loop {
            // check if target reached a final state
            if let Some(status) = self.queue.get_status(target_id) {
                match status {
                    TaskStatus::Complete { output } => {
                        return TaskOutcome::Complete { output };
                    }
                    TaskStatus::Failed { error } => {
                        return TaskOutcome::Error { error };
                    }
                    _ => {}
                }
            }

            // try to pop and run a ready task (non-blocking)
            let Some(next_id) = self.queue.pop_ready() else {
                // no ready tasks; if target is yielded, convert to error (deadlock)
                if let Some(TaskStatus::Yielded { dependency }) = self.queue.get_status(target_id) {
                    return TaskOutcome::Error {
                        error: self.get_yield_failed_error(target_id, &dependency),
                    };
                }
                panic!("task {target_id:?} did not reach a final state");
            };
            self.step_task(next_id);
        }
    }

    /// Process a single task 'step' (run until outcome, not final state).
    fn step_task(&self, task_id: TaskId) {
        let handle = self.queue.get_task(task_id);
        self.queue.begin_work();
        self.queue.set_status(task_id, TaskStatus::Running);
        let outcome = self.process_task(&handle);
        self.handle_outcome(task_id, outcome);
        self.queue.end_work();
    }

    /// Process a compiler task and return the outcome.
    fn process_task(&self, handle: &TaskHandle) -> TaskOutcome {
        // trace
        let task = &handle.task;
        let task_id = handle.id;
        let args = task.trace_args(&self.program);
        let event_name = if handle.last_outcome.is_some() {
            "resume"
        } else {
            "start"
        };
        let event = format!("{}.{}.{}", task.phase().name(), task.name(), event_name);
        tracing::debug!(%event, %args, ?task_id);

        // process
        match task.clone() {
            Task::Import(import_task) => self.process_import(import_task).into(),
            Task::Bind(bind_task) => self.process_bind(bind_task).into(),
            Task::Resolve(resolve_task) => self.process_resolve(resolve_task).into(),
            Task::Analyze(analyze_task) => self.process_analyze(analyze_task).into(),
            Task::Elaborate(elaborate_task) => self.process_elaborate(elaborate_task).into(),
            Task::Lower(lower_task) => self.process_lower(lower_task).into(),
            Task::Verify(verify_task) => self.process_verify(verify_task).into(),
            Task::Execute(execute_task) => self.process_execute(execute_task).into(),
            Task::Optimize(optimize_task) => self.process_optimize(optimize_task).into(),
            Task::Generate(generate_task) => self.process_generate(generate_task).into(),
            Task::Link(link_task) => self.process_link(link_task).into(),
        }
    }

    /// Handle the outcome of a processed task.
    fn handle_outcome(&self, task_id: TaskId, outcome: TaskOutcome) {
        let handle = self.queue.get_task(task_id);
        let args = handle.task.trace_args(&self.program);
        match &outcome {
            // complete and wake waiters
            TaskOutcome::Complete { output } => {
                let event = format!("{}.{}.complete", handle.phase().name(), handle.task.name());
                tracing::debug!(%event, %args, ?task_id);
                self.queue.set_status(
                    task_id,
                    TaskStatus::Complete {
                        output: output.clone(),
                    },
                );
                self.wake_waiters(task_id);
            }
            // error and fail waiters
            TaskOutcome::Error { error } => {
                let event = format!("{}.{}.error", handle.phase().name(), handle.task.name());
                tracing::debug!(%event, %args, ?task_id);
                self.queue.set_status(
                    task_id,
                    TaskStatus::Failed {
                        error: error.clone(),
                    },
                );
                self.error(error.clone());
                self.fail_waiters(task_id);
            }
            // yield if possible
            TaskOutcome::Yield { dependency } => {
                // check for yield errors
                if let Some(internal_error) = self.check_yield(task_id, &handle, dependency) {
                    let event = format!("{}.{}.circuit", handle.phase().name(), handle.task.name());
                    tracing::error!(%event, %args, ?task_id);
                    self.queue.set_status(
                        task_id,
                        TaskStatus::Failed {
                            error: internal_error.clone().into(),
                        },
                    );
                    self.error(internal_error);
                    self.fail_waiters(task_id);
                    return;
                }

                // yield
                let event = format!("{}.{}.yield", handle.phase().name(), handle.task.name());
                tracing::debug!(%event, %args, ?task_id);
                self.queue.set_status(
                    task_id,
                    TaskStatus::Yielded {
                        dependency: dependency.clone(),
                    },
                );
                self.yield_dependency(task_id, dependency);
            }
        }
        // remember outcome
        self.queue.set_last_outcome(task_id, outcome);
    }

    /// Check if a yield should trigger an internal error (i.e. circuit break).
    fn check_yield(
        &self,
        task_id: TaskId,
        handle: &TaskHandle,
        dependency: &TaskDependency,
    ) -> Option<InternalError> {
        // check for repeated yield to the same dependency
        if let Some(TaskOutcome::Yield {
            dependency: previous_dependency,
        }) = &handle.last_outcome
            && *previous_dependency == *dependency
        {
            return Some(InternalError::SuspiciousYield {
                node: dependency.node(),
                task_id,
                dependency: dependency.clone(),
            });
        }

        // check for excessive yields
        if handle.yield_count >= MAX_TOTAL_YIELD_COUNT {
            return Some(InternalError::ExcessiveYield {
                node: dependency.node(),
                task_id,
                yield_count: handle.yield_count,
            });
        }

        None
    }

    /// Yield to a dependency, register waiters for all sub-dependencies, and check if already satisfied.
    fn yield_dependency(&self, waiter_id: TaskId, dependency: &TaskDependency) {
        // first, register all sub-dependencies (recursively)
        self.register_dependency(waiter_id, dependency);

        // then check if the full dependency is already satisfied
        if self.is_dependency_satisfied(dependency) {
            self.queue.try_requeue_yielded(waiter_id);
            return;
        }

        // check if any dependency has failed
        if self.is_dependency_failed(dependency) {
            self.fail_waiter(waiter_id, dependency);
        }
    }

    /// Register waiters for all sub-dependencies without re-queuing.
    fn register_dependency(&self, waiter_id: TaskId, dependency: &TaskDependency) {
        match dependency {
            // register for a single dependency
            TaskDependency::Complete { task, .. } => {
                // enqueue the dependency task (might already exist)
                let (dependency_id, _) = self.enqueue(task.clone());

                // only register as waiter if not already complete or failed
                if let Some(status) = self.queue.get_status(dependency_id)
                    && status.is_final()
                {
                    return;
                }

                // register as waiter
                self.queue.add_waiter(dependency_id, waiter_id);
            }
            // register for all dependencies
            TaskDependency::CompleteAll { dependencies } => {
                for dependency in dependencies {
                    self.register_dependency(waiter_id, dependency);
                }
            }
            // register for any dependency (first to complete will wake)
            TaskDependency::CompleteAny { dependencies } => {
                for dependency in dependencies {
                    self.register_dependency(waiter_id, dependency);
                }
            }
        }
    }

    /// Check if any dependency in the tree has failed.
    fn is_dependency_failed(&self, dependency: &TaskDependency) -> bool {
        match dependency {
            TaskDependency::Complete { task, .. } => {
                matches!(
                    self.queue.find_task_status(task),
                    Some(TaskStatus::Failed { .. })
                )
            }
            TaskDependency::CompleteAll { dependencies } => dependencies
                .iter()
                .any(|dependency| self.is_dependency_failed(dependency)),
            TaskDependency::CompleteAny { dependencies } => {
                // for CompleteAny, only fail if ALL have failed
                dependencies
                    .iter()
                    .all(|dependency| self.is_dependency_failed(dependency))
            }
        }
    }

    /// Check if a dependency is satisfied.
    fn is_dependency_satisfied(&self, dependency: &TaskDependency) -> bool {
        match dependency {
            TaskDependency::Complete { task, .. } => {
                matches!(
                    self.queue.find_task_status(task),
                    Some(TaskStatus::Complete { .. })
                )
            }
            TaskDependency::CompleteAll { dependencies } => dependencies
                .iter()
                .all(|dependency| self.is_dependency_satisfied(dependency)),
            TaskDependency::CompleteAny { dependencies } => dependencies
                .iter()
                .any(|dependency| self.is_dependency_satisfied(dependency)),
        }
    }

    /// Wake all tasks waiting for the completed task.
    fn wake_waiters(&self, completed_id: TaskId) {
        let waiters = self.queue.take_waiters(completed_id);
        for waiter_id in waiters {
            if let Some(TaskStatus::Yielded { dependency }) = self.queue.get_status(waiter_id)
                && self.is_dependency_satisfied(&dependency)
            {
                self.queue.try_requeue_yielded(waiter_id);
            }
        }
    }

    /// Fail all tasks waiting for the failed task.
    fn fail_waiters(&self, failed_id: TaskId) {
        let waiters = self.queue.take_waiters(failed_id);
        for waiter_id in waiters {
            if let Some(TaskStatus::Yielded { dependency }) = self.queue.get_status(waiter_id) {
                self.fail_waiter(waiter_id, &dependency);
            }
        }
    }
    /// Fail a waiter using the fallback error from its dependency.
    fn fail_waiter(&self, waiter_id: TaskId, dependency: &TaskDependency) {
        let error: TaskError = Self::get_fallback_error(dependency)
            .unwrap_or_else(|| self.get_yield_failed_error(waiter_id, dependency));
        self.queue.set_status(
            waiter_id,
            TaskStatus::Failed {
                error: error.clone(),
            },
        );
        self.error(error);
    }

    /// Get the fallback error from a dependency.
    fn get_fallback_error(dependency: &TaskDependency) -> Option<TaskError> {
        match dependency {
            TaskDependency::Complete { error, .. } => error.as_ref().map(|e| *e.clone()),
            TaskDependency::CompleteAll { dependencies } => {
                // return first fallback error found
                dependencies
                    .iter()
                    .find_map(|dependency| Self::get_fallback_error(dependency))
            }
            TaskDependency::CompleteAny { dependencies } => {
                // return first fallback error found
                dependencies
                    .iter()
                    .find_map(|dependency| Self::get_fallback_error(dependency))
            }
        }
    }

    /// Create a UnsatisfiedDependency variant of TaskError for the given waiter and dependency.
    fn get_yield_failed_error(&self, waiter_id: TaskId, dependency: &TaskDependency) -> TaskError {
        let task = self.queue.get_task(waiter_id);
        match task.phase() {
            TaskPhase::Import => {
                panic!("import tasks cannot yield {waiter_id} for dependency {dependency:?}")
            }
            TaskPhase::Bind => {
                panic!("bind tasks cannot yield {waiter_id} for dependency {dependency:?}")
            }
            TaskPhase::Resolve => ResolveError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Analyze => AnalyzeError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Elaborate => ElaborateError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Lower => LowerError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Verify => VerifyError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Optimize => OptimizeError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Execute => ExecuteError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Generate => GenerateError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
            TaskPhase::Link => LinkError::UnsatisfiedDependency {
                dependency: dependency.clone(),
            }
            .into(),
        }
    }
}
