use destack_heap as heap;
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::EventLoop;
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Eager asynchronous tasks indexed by stable task identities.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TaskTable {
    /// Dense reusable task slots.
    slots: Vec<TaskSlot>,
    /// Vacant slot indices.
    vacant: Vec<u32>,
    /// Number of live tasks.
    len: usize,
    /// Number of tasks with execution still pending.
    pending: usize,
}

/// One generation-checked task slot.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TaskSlot {
    /// Generation issued by this slot.
    generation: u32,
    /// Live task state when this slot is occupied.
    state: Option<TaskState>,
}

/// One live task execution and result state.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum TaskState {
    /// The task is executing synchronously.
    Running {
        /// Whether cancellation must enter at the next suspension.
        is_cancelled: bool,
        /// The destination of the eventual task result.
        consumer: TaskConsumer,
    },
    /// The task is suspended on one asynchronous waiter.
    Suspended {
        /// Waiter that resumes the task execution.
        waiter: program::Waiter,
        /// The destination of the eventual task result.
        consumer: TaskConsumer,
    },
    /// The task continuation is queued for execution.
    Ready {
        /// Whether the queued continuation must enter cancellation instead.
        is_cancelled: bool,
        /// The destination of the eventual task result.
        consumer: TaskConsumer,
    },
    /// The task completed while its result handle remained live.
    Completed {
        /// The completed task value.
        value: program::Value,
    },
    /// The task was cancelled while its result handle remained live.
    Cancelled,
}

/// The destination of one pending task result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum TaskConsumer {
    /// The source task handle remains live.
    Handle,
    /// One waiter consumes the eventual result.
    Waiter(program::Waiter),
    /// No caller observes the eventual result.
    Detached,
}

/// One waiter action produced by a task state transition.
enum TaskWake {
    /// Resume one waiter with the completed task value.
    Queue {
        /// The waiter to resume.
        waiter: program::Waiter,
        /// The completed value to deliver.
        value: program::Value,
    },
    /// Cancel one waiter without producing a value.
    Cancel {
        /// The waiter to cancel.
        waiter: program::Waiter,
    },
}

impl TaskTable {
    /// Fork this task table for one forked World.
    pub(super) fn fork(&self) -> Self {
        Self {
            slots: self.slots.iter().map(TaskSlot::fork).collect(),
            vacant: self.vacant.clone(),
            len: self.len,
            pending: self.pending,
        }
    }

    /// Insert one live task state.
    fn insert(&mut self, state: TaskState) -> program::Task {
        let is_pending = state.is_pending();
        let index = if let Some(index) = self.vacant.pop() {
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(TaskSlot {
                generation: 1,
                state: None,
            });

            index
        };
        let slot = &mut self.slots[index as usize];
        slot.state = Some(state);
        self.len += 1;
        self.pending += usize::from(is_pending);

        program::Task::new(index, slot.generation)
    }

    /// Borrow one live task state mutably.
    fn state_mut(&mut self, task: program::Task) -> program::Result<&mut TaskState> {
        let Some(slot) = self.slots.get_mut(task.index() as usize) else {
            return Err(program::Error::UndefinedTask { task });
        };
        if slot.generation != task.generation() {
            return Err(program::Error::UndefinedTask { task });
        }

        slot.state
            .as_mut()
            .ok_or(program::Error::UndefinedTask { task })
    }

    /// Take one live task state for one immediate transition.
    fn take(&mut self, task: program::Task) -> program::Result<TaskState> {
        let Some(slot) = self.slots.get_mut(task.index() as usize) else {
            return Err(program::Error::UndefinedTask { task });
        };
        if slot.generation != task.generation() {
            return Err(program::Error::UndefinedTask { task });
        }
        slot.state
            .take()
            .ok_or(program::Error::UndefinedTask { task })
    }

    /// Replace one task state after an immediate transition.
    fn replace(&mut self, task: program::Task, state: TaskState) -> program::Result<()> {
        let Some(slot) = self.slots.get_mut(task.index() as usize) else {
            return Err(program::Error::UndefinedTask { task });
        };
        if slot.generation != task.generation() {
            return Err(program::Error::UndefinedTask { task });
        }
        if slot.state.is_some() {
            return Err(program::Error::InvalidTaskState { task });
        }

        slot.state = Some(state);

        Ok(())
    }

    /// Release one task slot after its state was taken.
    fn release(&mut self, task: program::Task) -> program::Result<()> {
        let Some(slot) = self.slots.get_mut(task.index() as usize) else {
            return Err(program::Error::UndefinedTask { task });
        };
        if slot.generation != task.generation() {
            return Err(program::Error::UndefinedTask { task });
        }
        if slot.state.is_some() {
            return Err(program::Error::InvalidTaskState { task });
        }

        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.vacant.push(task.index());
        self.len -= 1;

        Ok(())
    }

    /// Mark one running task as suspended.
    pub(super) fn suspend(
        &mut self,
        task: program::Task,
        waiter: program::Waiter,
    ) -> program::Result<()> {
        let state = self.state_mut(task)?;
        let TaskState::Running {
            is_cancelled,
            consumer,
        } = state
        else {
            return Err(program::Error::InvalidTaskState { task });
        };
        if *is_cancelled {
            return Err(program::Error::InvalidTaskState { task });
        }

        *state = TaskState::Suspended {
            waiter,
            consumer: *consumer,
        };

        Ok(())
    }

    /// Mark one suspended task as ready.
    pub(super) fn ready(&mut self, task: program::Task, is_cancelled: bool) -> program::Result<()> {
        let state = self.state_mut(task)?;
        let TaskState::Suspended { consumer, .. } = state else {
            return Err(program::Error::InvalidTaskState { task });
        };

        *state = TaskState::Ready {
            is_cancelled,
            consumer: *consumer,
        };

        Ok(())
    }

    /// Begin one queued task continuation.
    pub(super) fn begin(&mut self, task: program::Task) -> program::Result<bool> {
        let state = self.state_mut(task)?;
        let TaskState::Ready {
            is_cancelled,
            consumer,
        } = state
        else {
            return Err(program::Error::InvalidTaskState { task });
        };
        let is_cancelled = *is_cancelled;
        *state = TaskState::Running {
            is_cancelled,
            consumer: *consumer,
        };

        Ok(is_cancelled)
    }

    /// Park one result waiter on a task.
    fn park(
        &mut self,
        task: program::Task,
        waiter: program::Waiter,
    ) -> program::Result<Option<TaskWake>> {
        let state = self.take(task)?;
        match state {
            TaskState::Running {
                is_cancelled,
                consumer: TaskConsumer::Handle,
            } => self.replace(
                task,
                TaskState::Running {
                    is_cancelled,
                    consumer: TaskConsumer::Waiter(waiter),
                },
            ),
            TaskState::Suspended {
                waiter: suspension,
                consumer: TaskConsumer::Handle,
            } => self.replace(
                task,
                TaskState::Suspended {
                    waiter: suspension,
                    consumer: TaskConsumer::Waiter(waiter),
                },
            ),
            TaskState::Ready {
                is_cancelled,
                consumer: TaskConsumer::Handle,
            } => self.replace(
                task,
                TaskState::Ready {
                    is_cancelled,
                    consumer: TaskConsumer::Waiter(waiter),
                },
            ),
            TaskState::Completed { value } => {
                self.release(task)?;

                return Ok(Some(TaskWake::Queue { waiter, value }));
            }
            TaskState::Cancelled => {
                self.release(task)?;

                return Ok(Some(TaskWake::Cancel { waiter }));
            }
            state => {
                self.replace(task, state)?;

                return Err(program::Error::InvalidTaskState { task });
            }
        }?;

        Ok(None)
    }

    /// Finish one running task.
    fn finish(
        &mut self,
        task: program::Task,
        outcome: program::TaskOutcome,
    ) -> program::Result<(Option<TaskWake>, Option<program::Value>)> {
        let state = self.take(task)?;
        let TaskState::Running { consumer, .. } = state else {
            self.replace(task, state)?;

            return Err(program::Error::InvalidTaskState { task });
        };

        let (wake, drop) = match (consumer, outcome) {
            (TaskConsumer::Handle, program::TaskOutcome::Completed(value)) => {
                self.replace(task, TaskState::Completed { value })?;

                (None, None)
            }
            (TaskConsumer::Handle, program::TaskOutcome::Cancelled) => {
                self.replace(task, TaskState::Cancelled)?;

                (None, None)
            }
            (TaskConsumer::Waiter(waiter), program::TaskOutcome::Completed(value)) => {
                self.release(task)?;

                (Some(TaskWake::Queue { waiter, value }), None)
            }
            (TaskConsumer::Waiter(waiter), program::TaskOutcome::Cancelled) => {
                self.release(task)?;

                (Some(TaskWake::Cancel { waiter }), None)
            }
            (TaskConsumer::Detached, program::TaskOutcome::Completed(value)) => {
                self.release(task)?;

                (None, Some(value))
            }
            (TaskConsumer::Detached, program::TaskOutcome::Cancelled) => {
                self.release(task)?;

                (None, None)
            }
        };
        self.pending -= 1;

        Ok((wake, drop))
    }

    /// Request cancellation and return one suspended waiter to cancel.
    fn request_cancel(&mut self, task: program::Task) -> program::Result<Option<program::Waiter>> {
        let state = self.state_mut(task)?;
        match state {
            TaskState::Running { is_cancelled, .. } | TaskState::Ready { is_cancelled, .. } => {
                *is_cancelled = true;

                Ok(None)
            }
            TaskState::Suspended { waiter, .. } => Ok(Some(*waiter)),
            TaskState::Completed { .. } | TaskState::Cancelled => Ok(None),
        }
    }

    /// Return whether cancellation was requested for one running task.
    fn is_cancelled(&mut self, task: program::Task) -> program::Result<bool> {
        let state = self.state_mut(task)?;
        match state {
            TaskState::Running { is_cancelled, .. } => Ok(*is_cancelled),
            _ => Err(program::Error::InvalidTaskState { task }),
        }
    }

    /// Detach one task result.
    fn detach(&mut self, task: program::Task) -> program::Result<Option<program::Value>> {
        let state = self.take(task)?;
        let drop = match state {
            TaskState::Running {
                is_cancelled,
                consumer: TaskConsumer::Handle,
            } => {
                self.replace(
                    task,
                    TaskState::Running {
                        is_cancelled,
                        consumer: TaskConsumer::Detached,
                    },
                )?;

                None
            }
            TaskState::Suspended {
                waiter,
                consumer: TaskConsumer::Handle,
            } => {
                self.replace(
                    task,
                    TaskState::Suspended {
                        waiter,
                        consumer: TaskConsumer::Detached,
                    },
                )?;

                None
            }
            TaskState::Ready {
                is_cancelled,
                consumer: TaskConsumer::Handle,
            } => {
                self.replace(
                    task,
                    TaskState::Ready {
                        is_cancelled,
                        consumer: TaskConsumer::Detached,
                    },
                )?;

                None
            }
            TaskState::Completed { value } => {
                self.release(task)?;

                Some(value)
            }
            TaskState::Cancelled => {
                self.release(task)?;

                None
            }
            state => {
                self.replace(task, state)?;

                return Err(program::Error::InvalidTaskState { task });
            }
        };

        Ok(drop)
    }

    /// Return whether no task is live.
    pub(super) const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return whether any task still has execution to run.
    pub(super) const fn has_pending_execution(&self) -> bool {
        self.pending != 0
    }

    /// Visit mutable heap roots retained by completed task values.
    pub(super) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        for slot in &mut self.slots {
            let Some(TaskState::Completed { value }) = &mut slot.state else {
                continue;
            };
            program
                .visit_value_root_slots(value, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(())
    }
}

impl TaskState {
    /// Fork this task state for one forked World.
    fn fork(&self) -> Self {
        match self {
            Self::Running {
                is_cancelled,
                consumer,
            } => Self::Running {
                is_cancelled: *is_cancelled,
                consumer: *consumer,
            },
            Self::Suspended { waiter, consumer } => Self::Suspended {
                waiter: *waiter,
                consumer: *consumer,
            },
            Self::Ready {
                is_cancelled,
                consumer,
            } => Self::Ready {
                is_cancelled: *is_cancelled,
                consumer: *consumer,
            },
            Self::Completed { value } => Self::Completed {
                value: value.fork(),
            },
            Self::Cancelled => Self::Cancelled,
        }
    }

    /// Return whether this task still has execution to run.
    const fn is_pending(&self) -> bool {
        matches!(
            self,
            Self::Running { .. } | Self::Suspended { .. } | Self::Ready { .. }
        )
    }
}

impl TaskSlot {
    /// Fork this task slot for one forked World.
    fn fork(&self) -> Self {
        Self {
            generation: self.generation,
            state: self.state.as_ref().map(TaskState::fork),
        }
    }
}

impl EventLoop {
    /// Create one already completed task.
    pub(crate) fn resolve_task(&mut self, value: program::Value) -> program::Task {
        self.task_table.insert(TaskState::Completed { value })
    }

    /// Start one running task.
    pub(crate) fn start_task(&mut self) -> program::Task {
        self.task_table.insert(TaskState::Running {
            is_cancelled: false,
            consumer: TaskConsumer::Handle,
        })
    }

    /// Suspend one running task and return its runtime waiter.
    pub(crate) fn suspend_task(
        &mut self,
        task: program::Task,
        continuation: program::Continuation,
    ) -> program::Result<program::Waiter> {
        let waiter = self.waiters.insert(continuation, Some(task));
        if let Err(error) = self.task_table.suspend(task, waiter) {
            let Some(_) = self.waiters.take(waiter) else {
                return Err(program::Error::UndefinedWaiter { waiter });
            };

            return Err(error);
        }

        Ok(waiter)
    }

    /// Park one result waiter on a task.
    pub(crate) fn park_task(
        &mut self,
        task: program::Task,
        waiter: program::Waiter,
    ) -> program::Result<()> {
        if !self.waiters.contains(waiter) {
            return Err(program::Error::UndefinedWaiter { waiter });
        }

        let wake = self.task_table.park(task, waiter)?;
        self.dispatch_task_wake(wake)
    }

    /// Request cooperative cancellation of one task.
    pub(crate) fn cancel_task(&mut self, task: program::Task) -> program::Result<()> {
        let waiter = self.task_table.request_cancel(task)?;
        if let Some(waiter) = waiter {
            let is_cancelled = self.cancel_waiter(waiter)?;
            if !is_cancelled {
                return Err(program::Error::UndefinedWaiter { waiter });
            }
        }

        Ok(())
    }

    /// Return whether cancellation was requested for one running task.
    pub(crate) fn is_task_cancelled(&mut self, task: program::Task) -> program::Result<bool> {
        self.task_table.is_cancelled(task)
    }

    /// Detach one task result.
    pub(crate) fn detach_task(&mut self, task: program::Task) -> program::Result<()> {
        if let Some(value) = self.task_table.detach(task)? {
            self.release(value);
        }

        Ok(())
    }

    /// Finish one running task.
    pub(crate) fn finish_task(
        &mut self,
        task: program::Task,
        outcome: program::TaskOutcome,
    ) -> program::Result<()> {
        let (wake, drop) = self.task_table.finish(task, outcome)?;
        if let Some(value) = drop {
            self.release(value);
        }

        self.dispatch_task_wake(wake)
    }

    /// Begin one queued task continuation.
    pub(crate) fn begin_task(&mut self, task: program::Task) -> program::Result<bool> {
        self.task_table.begin(task)
    }

    /// Dispatch one task result waiter action.
    fn dispatch_task_wake(&mut self, wake: Option<TaskWake>) -> program::Result<()> {
        let (waiter, is_settled) = match wake {
            Some(TaskWake::Queue { waiter, value }) => (waiter, self.queue_waiter(waiter, value)?),
            Some(TaskWake::Cancel { waiter }) => (waiter, self.cancel_waiter(waiter)?),
            None => return Ok(()),
        };
        if !is_settled {
            return Err(program::Error::UndefinedWaiter { waiter });
        }

        Ok(())
    }
}
