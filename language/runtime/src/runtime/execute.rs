use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::engine::{Engine, EngineOutcome, EntryPoint};
use crate::replay::{ReplayEvent, SchedulerEvent, SchedulerEventKind, SchedulerSubject};
use crate::scheduler::{Microtask, Runnable, ScheduledItem, Task, TaskId, TaskState};

use super::Runtime;

impl Runtime {
    /// Run an entrypoint through the scheduler.
    pub fn run_entry<E: Engine>(
        &mut self,
        engine: &mut E,
        entry: &EntryPoint,
        args: &[vm::Value],
    ) -> RuntimeResult<vm::ExecutionOutput> {
        // execute the entrypoint with yielding enabled
        let outcome = engine.run(entry, args)?;
        match outcome {
            EngineOutcome::Completed { output } => Ok(output),
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                let task_id = self.scheduler.next_task_id();
                self.enqueue_task(task_id, Runnable::Vm(continuation), value);
                self.run_until(engine, task_id)
            }
        }
    }

    /// Drive the scheduler until the specified task completes.
    pub fn run_until<E: Engine>(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<vm::ExecutionOutput> {
        loop {
            // run a single scheduler tick for the engine
            if let Some(output) = self.tick_engine(engine, target_task)? {
                return Ok(output);
            }

            // exit if nothing is left to do
            if !self.scheduler.has_pending_work() {
                return Err(RuntimeError::scheduler_idle(target_task.get()).boxed());
            }
        }
    }

    /// Execute one scheduler item and return output for the target task.
    pub fn tick_engine<E: Engine>(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<Option<vm::ExecutionOutput>> {
        // poll timers for ready callbacks
        let now = self.context.time().wall_nanos();
        let _ready_timers = self.scheduler.poll_timers(now)?;

        // TODO #Incomplete: wire timer callbacks into tasks yet

        // poll platform events if a poller is installed
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.scheduler.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                // TODO #Incomplete: wire events into tasks yet
            }
        }

        // run the next scheduled item if available
        if let Some(item) = self.scheduler.next_runnable() {
            match item {
                ScheduledItem::Task(task) => {
                    self.record_task_event(task.id, SchedulerEventKind::Dequeue);
                    if let Some(output) = self.execute_task(engine, task, target_task)? {
                        return Ok(Some(output));
                    }
                }
                ScheduledItem::Microtask(microtask) => {
                    self.execute_microtask(engine, microtask)?;
                }
            }
        }

        Ok(None)
    }

    fn enqueue_task(&mut self, task_id: TaskId, runnable: Runnable, resume_value: vm::Value) {
        // record the enqueue event for replay
        self.record_task_event(task_id, SchedulerEventKind::Enqueue);

        let task = Task {
            id: task_id,
            runnable,
            resume_value,
            state: TaskState::Ready,
            priority: 0,
        };

        self.scheduler.enqueue_task(task);
    }

    fn execute_task<E: Engine>(
        &mut self,
        engine: &mut E,
        task: Task,
        target_task: TaskId,
    ) -> RuntimeResult<Option<vm::ExecutionOutput>> {
        match task.runnable {
            Runnable::Vm(continuation) => {
                let outcome = engine.resume(continuation, task.resume_value)?;
                match outcome {
                    EngineOutcome::Completed { output } => {
                        self.record_task_event(task.id, SchedulerEventKind::Complete);
                        if task.id == target_task {
                            return Ok(Some(output));
                        }
                    }
                    EngineOutcome::Yielded {
                        continuation,
                        value,
                    } => {
                        self.record_task_event(task.id, SchedulerEventKind::Yield);
                        self.enqueue_task(task.id, Runnable::Vm(continuation), value);
                    }
                }
            }
            Runnable::Native(_) => {
                return Err(
                    RuntimeError::internal("native runnable execution is not wired yet").boxed(),
                );
            }
        }

        Ok(None)
    }

    fn execute_microtask<E: Engine>(
        &mut self,
        engine: &mut E,
        microtask: Microtask,
    ) -> RuntimeResult<()> {
        match microtask.runnable {
            Runnable::Vm(continuation) => {
                let outcome = engine.resume(continuation, microtask.resume_value)?;
                match outcome {
                    EngineOutcome::Completed { .. } => {}
                    EngineOutcome::Yielded { .. } => {
                        return Err(RuntimeError::internal(
                            "microtask yielded while running to completion",
                        )
                        .boxed());
                    }
                }
            }
            Runnable::Native(_) => {
                return Err(
                    RuntimeError::internal("native microtask execution is not wired yet").boxed(),
                );
            }
        }

        Ok(())
    }

    fn record_task_event(&self, task_id: TaskId, kind: SchedulerEventKind) {
        let event = ReplayEvent::SchedulerEvent(SchedulerEvent {
            subject: SchedulerSubject::Task(task_id),
            kind,
        });

        self.context.replay().record_event(event);
    }
}
