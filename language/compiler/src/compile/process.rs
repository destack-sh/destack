use std::num::NonZero;
use std::thread;

use crate::{
    AnalyzeError, BindError, BuildError, Compiler, ElaborateError, ExecuteError, ImportError,
    LinkError, LowerError, OptimizeError, Phase, ResolveError, Task, TaskDependency, TaskError,
    TaskId, TaskOutcome, TaskStatus, ValidateError,
};

impl Compiler {
    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
        // determine thread count
        let thread_count = self.options.workers.unwrap_or_else(|| {
            thread::available_parallelism()
                .unwrap_or(NonZero::new(1).unwrap())
                .get() as u16
        });

        // spawn worker threads
        thread::scope(|scope| {
            for _ in 0..thread_count {
                scope.spawn(|| self.worker_loop());
            }
        });

        // flush remaining diagnostics
        self.flush_diagnostics();
    }

    /// Worker loop that processes tasks from the ready queue.
    fn worker_loop(&self) {
        loop {
            // try to pop a task from the ready queue
            let Some(task_id) = self.queue.pop_ready() else {
                // no task available, wait for work or completion
                if !self.queue.wait_for_work() {
                    break;
                }
                continue;
            };

            // get the task handle and run it
            let handle = self.queue.get_task(task_id);
            self.queue.begin_work();
            self.queue.set_status(task_id, TaskStatus::Running);
            let outcome = self.process_task(&handle.task);
            self.handle_outcome(task_id, outcome);
            self.queue.end_work();
        }
    }

    /// Enqueue a task to the compiler.
    /// Noop if we already have the same task queued, returns the existing TaskId.
    pub fn enqueue<T: Into<Task>>(&self, task: T) -> TaskId {
        let task: Task = task.into();
        self.queue.enqueue(task)
    }

    /// Process a compiler task and return the outcome.
    fn process_task(&self, task: &Task) -> TaskOutcome {
        match task.clone() {
            Task::Import(import_task) => self.process_import(import_task).into(),
            Task::Bind(bind_task) => self.process_bind(bind_task).into(),
            Task::Resolve(resolve_task) => self.process_resolve(resolve_task).into(),
            Task::Validate(validate_task) => self.process_validate(validate_task).into(),
            Task::Elaborate(elaborate_task) => self.process_elaborate(elaborate_task).into(),
            Task::Lower(lower_task) => self.process_lower(lower_task).into(),
            Task::Analyze(analyze_task) => self.process_analyze(analyze_task).into(),
            Task::Execute(execute_task) => self.process_execute(execute_task).into(),
            Task::Optimize(optimize_task) => self.process_optimize(optimize_task).into(),
            Task::Build(build_task) => self.process_build(build_task).into(),
            Task::Link(link_task) => self.process_link(link_task).into(),
        }
    }

    /// Handle the outcome of a processed task.
    fn handle_outcome(&self, task_id: TaskId, outcome: TaskOutcome) {
        match outcome {
            TaskOutcome::Complete { output } => {
                self.queue
                    .set_status(task_id, TaskStatus::Complete { output });
                self.wake_waiters(task_id);
            }
            TaskOutcome::Error { error } => {
                self.queue.set_status(
                    task_id,
                    TaskStatus::Failed {
                        error: error.clone(),
                    },
                );
                self.error(error);
                self.fail_waiters(task_id);
            }
            TaskOutcome::Yield { dependency } => {
                self.queue.set_status(
                    task_id,
                    TaskStatus::Yielded {
                        dependency: dependency.clone(),
                    },
                );
                self.register_dependency(task_id, &dependency);
            }
        }
    }

    /// Register dependencies for a yielded task.
    fn register_dependency(&self, waiter_id: TaskId, dependency: &TaskDependency) {
        match dependency {
            TaskDependency::Complete { task, .. } => {
                // enqueue the dependency task (might already exist)
                let dependency_id = self.enqueue(task.clone());

                // check if already complete
                if let Some(status) = self.queue.get_status(dependency_id) {
                    match status {
                        TaskStatus::Complete { .. } => {
                            // dependency already done, re-queue the waiter
                            self.queue.set_status(waiter_id, TaskStatus::Queued);
                            self.queue.push_ready(waiter_id);
                            return;
                        }
                        TaskStatus::Failed { .. } => {
                            // dependency failed, fail the waiter with fallback error
                            self.fail_waiter_with_fallback(waiter_id, dependency);
                            return;
                        }
                        _ => {}
                    }
                }

                // register as waiter
                self.queue.add_waiter(dependency_id, waiter_id);
            }
            TaskDependency::CompleteAll { dependencies } => {
                // register for all dependencies
                for dependency in dependencies {
                    self.register_dependency(waiter_id, dependency);
                }
            }
            TaskDependency::CompleteAny { dependencies } => {
                // register for all dependencies (first to complete will wake)
                for dependency in dependencies {
                    self.register_dependency(waiter_id, dependency);
                }
            }
        }
    }

    /// Wake all tasks waiting for the completed task.
    fn wake_waiters(&self, completed_id: TaskId) {
        let waiters = self.queue.take_waiters(completed_id);
        for waiter_id in waiters {
            // check if waiter can be re-queued
            if let Some(TaskStatus::Yielded { dependency }) = self.queue.get_status(waiter_id)
                && self.is_dependency_satisfied(&dependency)
            {
                self.queue.set_status(waiter_id, TaskStatus::Queued);
                self.queue.push_ready(waiter_id);
            }
        }
    }

    /// Fail all tasks waiting for the failed task.
    fn fail_waiters(&self, failed_id: TaskId) {
        let waiters = self.queue.take_waiters(failed_id);
        for waiter_id in waiters {
            if let Some(TaskStatus::Yielded { dependency }) = self.queue.get_status(waiter_id) {
                self.fail_waiter_with_fallback(waiter_id, &dependency);
            }
        }
    }

    /// Fail a waiter using the fallback error from its dependency.
    fn fail_waiter_with_fallback(&self, waiter_id: TaskId, dependency: &TaskDependency) {
        let error: TaskError = Self::get_fallback_error(dependency).unwrap_or_else(|| {
            let task = self.queue.get_task(waiter_id);
            match task.phase() {
                Phase::Import => ImportError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Bind => BindError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Resolve => ResolveError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Validate => ValidateError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Elaborate => ElaborateError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Lower => LowerError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Analyze => AnalyzeError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Optimize => OptimizeError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Execute => ExecuteError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Build => BuildError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
                Phase::Link => LinkError::YieldFailed {
                    dependency: dependency.clone(),
                }
                .into(),
            }
        });
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

    /// Check if a dependency is satisfied.
    fn is_dependency_satisfied(&self, dependency: &TaskDependency) -> bool {
        match dependency {
            TaskDependency::Complete { task, .. } => {
                // find the task and check if complete
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
}
