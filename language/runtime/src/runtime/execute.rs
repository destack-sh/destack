use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::engine::{Engine, EngineOutcome};
use crate::replay::{
    ReplayEvent, SchedulerEvent, SchedulerEventKind, SchedulerQueue, SchedulerSubject,
};
use crate::runtime::{ExecutionContext, enter_execution_context};
use crate::scheduler::{Microtask, Runnable, ScheduledItem, Task, TaskId, TaskState};

use super::Runtime;

impl Runtime {
    /// Run an entrypoint through the scheduler.
    pub fn run_entry<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        entry: &E::Entry,
        args: &[vm::Value],
    ) -> RuntimeResult<vm::ExecutionOutput> {
        // execute the entrypoint with yielding enabled
        let _guard = enter_execution_context(ExecutionContext::empty());
        let outcome = engine.run(entry, args)?;

        // handle the entry outcome
        match outcome {
            EngineOutcome::Completed { output } => Ok(output),
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                // enqueue the yielded continuation
                let task_id = self.scheduler.next_task_id();
                self.enqueue_task(task_id, Runnable::Vm(continuation), value)?;

                self.run_until(engine, task_id)
            }
        }
    }

    /// Drive the scheduler until the specified task completes.
    pub fn run_until<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<vm::ExecutionOutput> {
        // drive the scheduler until the target task completes
        loop {
            // run a single scheduler tick for the engine
            if let Some(output) = self.tick_engine(engine, target_task)? {
                return Ok(output);
            }

            // exit if nothing is left to do
            if !self.scheduler.has_pending_work() {
                return Err(RuntimeError::SchedulerIdle {
                    task_id: target_task.get(),
                }
                .boxed());
            }
        }
    }

    /// Execute one scheduler item and return output for the target task.
    pub fn tick_engine<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<Option<vm::ExecutionOutput>> {
        // poll platform events if a poller is installed
        let now = self.context.time().wall_nanos();
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.scheduler.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                // NOTE #Incomplete: wire events into tasks
            }
        }

        // drain microtasks before selecting other work
        if self.scheduler.has_microtasks() {
            self.drain_microtasks(engine)?;
        }

        // run the next scheduled item if available
        if let Some(item) = self.scheduler.next_runnable(now)? {
            match item {
                ScheduledItem::Task(task) => {
                    // record the dequeue event
                    self.record_scheduler_event(
                        SchedulerSubject::Task(task.id),
                        SchedulerQueue::Macrotask,
                        SchedulerEventKind::Dequeue,
                    )?;

                    if let Some(output) = self.execute_task(engine, task, target_task)? {
                        return Ok(Some(output));
                    }
                }
                ScheduledItem::Microtask(microtask) => {
                    // run the microtask to completion
                    self.execute_microtask(engine, microtask)?;
                }
                ScheduledItem::Timer(_timer) => {
                    // NOTE #Incomplete: wire timer callbacks into tasks
                }
                ScheduledItem::Event(_event) => {
                    // NOTE #Incomplete: wire external events into tasks
                }
            }
        }

        Ok(None)
    }

    fn enqueue_task(
        &mut self,
        task_id: TaskId,
        runnable: Runnable,
        resume_value: vm::Value,
    ) -> RuntimeResult<()> {
        // record the enqueue event for replay
        self.record_scheduler_event(
            SchedulerSubject::Task(task_id),
            SchedulerQueue::Macrotask,
            SchedulerEventKind::Enqueue,
        )?;

        // build the task metadata
        let task = Task {
            id: task_id,
            runnable,
            resume_value,
            state: TaskState::Ready,
            priority: 0,
        };

        // enqueue the task into the scheduler
        self.scheduler.enqueue_task(task);

        Ok(())
    }

    fn execute_task<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        task: Task,
        target_task: TaskId,
    ) -> RuntimeResult<Option<vm::ExecutionOutput>> {
        // run the task runnable
        let _guard = enter_execution_context(ExecutionContext::for_task(task.id));
        let outcome = self.execute_runnable(engine, task.runnable, task.resume_value)?;

        // handle the task outcome
        match outcome {
            EngineOutcome::Completed { output } => {
                self.record_scheduler_event(
                    SchedulerSubject::Task(task.id),
                    SchedulerQueue::Macrotask,
                    SchedulerEventKind::Complete,
                )?;
                if task.id == target_task {
                    return Ok(Some(output));
                }
            }
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                self.record_scheduler_event(
                    SchedulerSubject::Task(task.id),
                    SchedulerQueue::Macrotask,
                    SchedulerEventKind::Yield,
                )?;
                self.enqueue_task(task.id, Runnable::Vm(continuation), value)?;
            }
        }

        self.drain_microtasks(engine)?;

        Ok(None)
    }

    fn execute_microtask<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        microtask: Microtask,
    ) -> RuntimeResult<()> {
        // run the microtask runnable
        let _guard = enter_execution_context(ExecutionContext::for_microtask(microtask.id));
        let outcome = self.execute_runnable(engine, microtask.runnable, microtask.resume_value)?;

        // ensure microtasks run to completion
        match outcome {
            EngineOutcome::Completed { .. } => {
                self.record_scheduler_event(
                    SchedulerSubject::Microtask(microtask.id),
                    SchedulerQueue::Microtask,
                    SchedulerEventKind::Complete,
                )?;
                Ok(())
            }
            EngineOutcome::Yielded { .. } => Err(RuntimeError::Internal {
                message: "microtask yielded while running to completion".to_string(),
            }
            .boxed()),
        }
    }

    fn drain_microtasks<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
    ) -> RuntimeResult<()> {
        loop {
            let Some(microtask) = self.scheduler.pop_microtask() else {
                break;
            };
            self.record_scheduler_event(
                SchedulerSubject::Microtask(microtask.id),
                SchedulerQueue::Microtask,
                SchedulerEventKind::Dequeue,
            )?;
            self.execute_microtask(engine, microtask)?;
        }

        Ok(())
    }

    fn execute_runnable<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        runnable: Runnable,
        resume_value: vm::Value,
    ) -> RuntimeResult<EngineOutcome<vm::ExecutionOutput, vm::Continuation, vm::Value>> {
        // select the runnable implementation
        match runnable {
            Runnable::Vm(continuation) => engine.resume(continuation, resume_value),
            Runnable::Native(_) => Err(RuntimeError::Internal {
                message: "native runnable execution is not wired yet".to_string(),
            }
            .boxed()),
        }
    }

    fn record_scheduler_event(
        &mut self,
        subject: SchedulerSubject,
        queue: SchedulerQueue,
        kind: SchedulerEventKind,
    ) -> RuntimeResult<()> {
        // record the scheduler event for replay
        let sequence = self.scheduler.next_sequence();
        let event = ReplayEvent::SchedulerEvent(SchedulerEvent {
            subject,
            queue,
            kind,
            sequence,
        });

        self.context.replay().record_event(event)?;

        Ok(())
    }
}
