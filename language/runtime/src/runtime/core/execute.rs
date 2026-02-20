use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::RuntimeHookState;
use crate::runtime::engine::{Engine, EngineContinuation, EngineOutcome};
use crate::runtime::replay::{QueueEventKind, ReplayEvent, TaskQueue, TaskQueueEvent, TaskSubject};
use crate::runtime::scheduler::{
    EventLoopScope, Microtask, Runnable, Task, TaskId, TaskState, enter_event_loop_scope,
};

use super::Runtime;

impl Runtime {
    /// Run an entrypoint through the event loop.
    pub fn run_entrypoint<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        entry: &E::Entry,
        args: &[vm::Value],
    ) -> RuntimeResult<vm::ExecutionOutput> {
        // execute the entrypoint with yielding enabled
        let _guard = enter_event_loop_scope(EventLoopScope::empty());
        let outcome = engine.run(entry, args)?;

        // handle the entry outcome
        match outcome {
            EngineOutcome::Completed { output } => Ok(output),
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                // enqueue the yielded continuation
                let task_id = self.event_loop.next_task_id();
                self.enqueue_task(task_id, EngineContinuation::Vm(continuation), value)?;

                self.run_event_loop_until_task_complete(engine, task_id)
            }
        }
    }

    /// Run the event loop until the specified task completes.
    pub fn run_event_loop_until_task_complete<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<vm::ExecutionOutput> {
        // run the event loop until the target task completes
        loop {
            // run one event loop tick for the engine
            if let Some(output) = self.tick_event_loop_once(engine, target_task)? {
                return Ok(output);
            }

            // exit if nothing is left to do
            if !self.event_loop.has_pending_work() {
                return Err(RuntimeError::EventLoopIdle {
                    task_id: target_task.get(),
                }
                .boxed());
            }
        }
    }

    /// Tick the event loop once and return output for the target task.
    pub fn tick_event_loop_once<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<Option<vm::ExecutionOutput>> {
        // poll platform events if a poller is installed
        let now = self.state.time.wall_nanos();
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.event_loop.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                self.state.rules.on_scheduler_event_wake(RuntimeHookState {
                    external_event_count: Some(event_count),
                    ..RuntimeHookState::empty()
                });
                // NOTE #Incomplete: wire events into tasks
            }
        }

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            self.drain_microtasks(engine)?;
        }

        // run the next scheduled item if available
        if let Some(item) = self.event_loop.next_runnable(now)? {
            match item {
                Runnable::Task(task) => {
                    // record the dequeue event
                    self.record_task_queue_event(
                        TaskSubject::Task(task.id),
                        TaskQueue::Macrotask,
                        QueueEventKind::Dequeue,
                    )?;
                    self.state.rules.on_scheduler_dequeue(RuntimeHookState {
                        task_id: Some(task.id),
                        ..RuntimeHookState::empty()
                    });

                    if let Some(output) = self.execute_task(engine, task, target_task)? {
                        return Ok(Some(output));
                    }
                }
                Runnable::Microtask(microtask) => {
                    self.state.rules.on_scheduler_dequeue(RuntimeHookState {
                        microtask_id: Some(microtask.id),
                        ..RuntimeHookState::empty()
                    });
                    // run the microtask to completion
                    self.execute_microtask(engine, microtask)?;
                }
                Runnable::Timer(_timer) => {
                    self.state
                        .rules
                        .on_scheduler_timer_fire(RuntimeHookState::empty());
                    // NOTE #Incomplete: wire timer callbacks into tasks
                }
                Runnable::Event(_event) => {
                    // NOTE #Incomplete: wire external events into tasks
                }
            }
        }

        Ok(None)
    }

    fn enqueue_task(
        &mut self,
        task_id: TaskId,
        runnable: EngineContinuation,
        resume_value: vm::Value,
    ) -> RuntimeResult<()> {
        // record the enqueue event for replay
        self.record_task_queue_event(
            TaskSubject::Task(task_id),
            TaskQueue::Macrotask,
            QueueEventKind::Enqueue,
        )?;

        // build the task metadata
        let task = Task {
            id: task_id,
            runnable,
            resume_value,
            state: TaskState::Ready,
            priority: 0,
        };

        // enqueue the task into the event loop
        self.event_loop.enqueue_task(task);
        self.state.rules.on_scheduler_enqueue(RuntimeHookState {
            task_id: Some(task_id),
            ..RuntimeHookState::empty()
        });

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
        let _guard = enter_event_loop_scope(EventLoopScope::for_task(task.id));
        let outcome = self.execute_runnable(engine, task.runnable, task.resume_value)?;

        // handle the task outcome
        match outcome {
            EngineOutcome::Completed { output } => {
                self.record_task_queue_event(
                    TaskSubject::Task(task.id),
                    TaskQueue::Macrotask,
                    QueueEventKind::Complete,
                )?;
                if task.id == target_task {
                    return Ok(Some(output));
                }
            }
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                self.record_task_queue_event(
                    TaskSubject::Task(task.id),
                    TaskQueue::Macrotask,
                    QueueEventKind::Yield,
                )?;
                self.enqueue_task(task.id, EngineContinuation::Vm(continuation), value)?;
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
        let _guard = enter_event_loop_scope(EventLoopScope::for_microtask(microtask.id));
        let outcome = self.execute_runnable(engine, microtask.runnable, microtask.resume_value)?;

        // ensure microtasks run to completion
        match outcome {
            EngineOutcome::Completed { .. } => {
                self.record_task_queue_event(
                    TaskSubject::Microtask(microtask.id),
                    TaskQueue::Microtask,
                    QueueEventKind::Complete,
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
            let Some(microtask) = self.event_loop.pop_microtask() else {
                break;
            };
            self.record_task_queue_event(
                TaskSubject::Microtask(microtask.id),
                TaskQueue::Microtask,
                QueueEventKind::Dequeue,
            )?;
            self.state.rules.on_scheduler_dequeue(RuntimeHookState {
                microtask_id: Some(microtask.id),
                ..RuntimeHookState::empty()
            });
            self.execute_microtask(engine, microtask)?;
        }

        Ok(())
    }

    fn execute_runnable<
        E: Engine<Output = vm::ExecutionOutput, Continuation = vm::Continuation, Value = vm::Value>,
    >(
        &mut self,
        engine: &mut E,
        runnable: EngineContinuation,
        resume_value: vm::Value,
    ) -> RuntimeResult<EngineOutcome<vm::ExecutionOutput, vm::Continuation, vm::Value>> {
        // select the runnable implementation
        match runnable {
            EngineContinuation::Vm(continuation) => engine.resume(continuation, resume_value),
            EngineContinuation::Native(_) => Err(RuntimeError::Internal {
                message: "native runnable execution is not wired yet".to_string(),
            }
            .boxed()),
        }
    }

    fn record_task_queue_event(
        &mut self,
        subject: TaskSubject,
        queue: TaskQueue,
        kind: QueueEventKind,
    ) -> RuntimeResult<()> {
        // record the task queue event for replay
        let sequence = self.event_loop.next_sequence();
        let event = ReplayEvent::TaskQueueEvent(TaskQueueEvent {
            subject,
            queue,
            kind,
            sequence,
        });

        self.state.replay.record_event(event)?;

        Ok(())
    }
}
