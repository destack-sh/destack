use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource;
use crate::runtime::RuntimeHookState;
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, RuntimeOutput, RuntimeValue,
};
use crate::runtime::host::HostEvent;
use crate::runtime::replay::{QueueEventKind, ReplayEvent, TaskQueue, TaskQueueEvent, TaskSubject};
use crate::runtime::scheduler::{
    EventLoopScope, Microtask, Runnable, Task, TaskId, TaskStatus, current_event_loop_scope,
    enter_event_loop_scope,
};
use destack_workspace::TimeMode;

use super::Runtime;

impl Runtime {
    /// Run an entrypoint through the event loop.
    pub fn run_entrypoint<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        entry: &E::Entry,
        args: &[RuntimeValue],
    ) -> RuntimeResult<RuntimeOutput> {
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
                self.enqueue_task(task_id, continuation, value)?;

                self.run_loop_until_task_complete(engine, task_id)
            }
        }
    }

    /// Run the loop until the specified task completes.
    pub fn run_loop_until_task_complete<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<RuntimeOutput> {
        let output = self.run_loop_until_task_complete_with_timeout(engine, target_task, None)?;
        output.ok_or_else(|| {
            RuntimeError::EventLoopIdle {
                task_id: target_task.get(),
            }
            .boxed()
        })
    }

    /// Run the loop until the specified task completes or one timeout elapses.
    pub fn run_loop_until_task_complete_with_timeout<
        E: Engine<Output = RuntimeOutput, Value = RuntimeValue>,
    >(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<RuntimeOutput>> {
        // capture one monotonic start timestamp for timeout accounting
        let start_mono_nanos = self.state.time.mono_nanos();

        // run the loop until the target task completes
        loop {
            // stop once the configured timeout elapses
            if let Some(timeout_nanos) = timeout_nanos {
                let elapsed = self
                    .state
                    .time
                    .mono_nanos()
                    .saturating_sub(start_mono_nanos);
                if elapsed >= timeout_nanos {
                    return Ok(None);
                }
            }

            // run one loop tick for the engine
            let (progressed, output) = self.tick_loop(engine, Some(target_task))?;
            if let Some(output) = output {
                return Ok(Some(output));
            }

            // wait for the next wakeup when no work progressed this tick
            if !progressed && self.event_loop.has_pending_work() && self.wait_for_next_turn()? {
                continue;
            }

            // exit when the target task cannot make further progress
            if !progressed {
                return Err(RuntimeError::EventLoopIdle {
                    task_id: target_task.get(),
                }
                .boxed());
            }
        }
    }

    /// Run runtime ticks until no work remains.
    pub fn tick_until_idle<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
    ) -> RuntimeResult<()> {
        loop {
            let progressed = self.tick_once(engine)?;
            if !progressed {
                break;
            }
        }

        Ok(())
    }

    /// Execute one runtime tick.
    pub fn tick_once<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
    ) -> RuntimeResult<bool> {
        // run one event loop tick and capture progress
        let (mut progressed, _) = self.tick_loop(engine, None)?;

        // run one gc cycle when pacing says a cycle is due
        if self.heap.should_collect() {
            let _stats = self.heap.collect();
            progressed = true;
        }

        Ok(progressed)
    }

    /// Tick the loop once and return output for the target task.
    pub fn tick_loop_once<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<Option<RuntimeOutput>> {
        let (_, output) = self.tick_loop(engine, Some(target_task))?;
        Ok(output)
    }

    /// Tick the loop once and return progress and optional target output.
    fn tick_loop<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<(bool, Option<RuntimeOutput>)> {
        // track whether this tick processed any event loop work
        let mut progressed = false;
        let tick_start_mono_nanos = self.state.time.mono_nanos();

        // poll host adapter events before poller events
        let host_event_count = self.poll_host_events(Some(0))?;
        if host_event_count > 0 {
            progressed = true;
            self.state.hooks.on_scheduler_event_wake(RuntimeHookState {
                external_event_count: Some(host_event_count),
                ..RuntimeHookState::empty()
            });
        }

        // poll platform events if a poller is installed
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.event_loop.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                progressed = true;
                self.state.hooks.on_scheduler_event_wake(RuntimeHookState {
                    external_event_count: Some(event_count),
                    ..RuntimeHookState::empty()
                });
            }
        }
        if self.is_tick_budget_exhausted(tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let (drained, budget_exhausted) = self.drain_microtasks(engine)?;
            if drained > 0 {
                progressed = true;
            }
            if budget_exhausted {
                return Ok((progressed, None));
            }
        }
        if self.is_tick_budget_exhausted(tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // run the next scheduled item if available
        let wall_now = self.state.time.wall_nanos();
        let mono_now = self.state.time.mono_nanos();
        let max_microtask_depth = self
            .event_loop
            .options()
            .max_microtask_depth
            .map(|depth| usize::try_from(depth).unwrap_or(usize::MAX))
            .unwrap_or(usize::MAX);
        let mut ran_macrotask = false;
        if let Some(item) = self.event_loop.next_runnable(wall_now, mono_now)? {
            progressed = true;
            match item {
                Runnable::Task(task) => {
                    ran_macrotask = true;
                    if let Some(output) = self.execute_dequeued_task(engine, task, target_task)? {
                        return Ok((true, Some(output)));
                    }
                }
                Runnable::Microtask(microtask) => {
                    self.state.hooks.on_scheduler_dequeue(RuntimeHookState {
                        microtask_id: Some(microtask.id),
                        ..RuntimeHookState::empty()
                    });
                    // run the microtask to completion
                    self.execute_microtask(engine, microtask, max_microtask_depth)?;
                }
                Runnable::Timer(timer) => {
                    self.state.hooks.on_scheduler_timer_fire(RuntimeHookState {
                        resource_id: Some(timer.handle),
                        ..RuntimeHookState::empty()
                    });
                    let should_dispatch = crate::runtime::time::timer::on_event_loop_timer_fire(
                        &self.state.resources,
                        &self.state.time,
                        resource::TimerHandle(timer.handle),
                    )?;
                    if should_dispatch {
                        // dispatch a timer watch task when one is registered
                        if let Some(task) = self.event_loop.task_for_timer(timer) {
                            self.enqueue_prepared_task(task)?;
                        }

                        // one-shot timers no longer need a dispatch watch after firing
                        if timer.interval_nanos.is_none() {
                            let _ = self.event_loop.unwatch_timer(timer.handle);
                        }
                    } else {
                        // stale and inactive timer fires must not dispatch callbacks
                        let _ = self.event_loop.unwatch_timer(timer.handle);
                    }
                }
                Runnable::PollerEvent(event) => {
                    // dispatch an external-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_event(event) {
                        self.enqueue_prepared_task(task)?;
                    }
                    // drop stale and unregistered events without crashing the loop
                    else {
                        self.event_loop.record_dropped_unwatched_dispatch_event();
                    }
                }
                Runnable::HostEvent(event) => {
                    // dispatch one host-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_host_event(event) {
                        self.enqueue_prepared_task(task)?;
                    }
                    // account for unconsumed host semantic events
                    else {
                        self.event_loop.record_dropped_unwatched_dispatch_event();
                    }
                }
            }
        }

        // run one queued macrotask after routing timer and event watches
        if !ran_macrotask
            && let Some(task) = self.event_loop.pop_task()
            && let Some(output) = self.execute_dequeued_task(engine, task, target_task)?
        {
            return Ok((true, Some(output)));
        }

        Ok((progressed, None))
    }

    /// Enqueue one yielded continuation as a task.
    fn enqueue_task(
        &mut self,
        task_id: TaskId,
        runnable: EngineContinuation,
        resume_value: RuntimeValue,
    ) -> RuntimeResult<()> {
        // build the task metadata
        let task = Task {
            id: task_id,
            runnable,
            resume_value,
            status: TaskStatus::Ready,
            priority: 0,
        };

        self.enqueue_prepared_task(task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_dequeued_task<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<RuntimeOutput>> {
        // record dequeue state for one task
        self.record_task_queue_event(
            TaskSubject::Task(task.id),
            TaskQueue::Macrotask,
            QueueEventKind::Dequeue,
        )?;
        self.state.hooks.on_scheduler_dequeue(RuntimeHookState {
            task_id: Some(task.id),
            ..RuntimeHookState::empty()
        });

        self.execute_task(engine, task, target_task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_task<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        mut task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<RuntimeOutput>> {
        // run the task runnable
        task.status = TaskStatus::Waiting;
        let _guard = enter_event_loop_scope(EventLoopScope::for_task(task.id));
        let outcome = self.execute_runnable(engine, task.runnable, task.resume_value)?;

        // handle the task outcome
        match outcome {
            EngineOutcome::Completed { output } => {
                task.status = TaskStatus::Completed;
                self.record_task_queue_event(
                    TaskSubject::Task(task.id),
                    TaskQueue::Macrotask,
                    QueueEventKind::Complete,
                )?;
                if target_task == Some(task.id) {
                    return Ok(Some(output));
                }
            }
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                task.status = TaskStatus::Waiting;
                self.record_task_queue_event(
                    TaskSubject::Task(task.id),
                    TaskQueue::Macrotask,
                    QueueEventKind::Yield,
                )?;
                self.enqueue_task(task.id, continuation, value)?;
            }
        }

        let _ = self.drain_microtasks(engine)?;

        Ok(None)
    }

    /// Enqueue one prepared task and record enqueue hooks.
    fn enqueue_prepared_task(&mut self, task: Task) -> RuntimeResult<()> {
        // record enqueue for replay
        self.record_task_queue_event(
            TaskSubject::Task(task.id),
            TaskQueue::Macrotask,
            QueueEventKind::Enqueue,
        )?;

        // enqueue the task into the event loop
        let task_id = task.id;
        self.event_loop.enqueue_task(task);
        self.state.hooks.on_scheduler_enqueue(RuntimeHookState {
            task_id: Some(task_id),
            ..RuntimeHookState::empty()
        });

        Ok(())
    }

    /// Execute one microtask to completion.
    fn execute_microtask<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        microtask: Microtask,
        max_microtask_depth: usize,
    ) -> RuntimeResult<()> {
        // enforce true nesting depth against the current event loop scope
        let parent_scope = current_event_loop_scope();
        let next_depth = parent_scope.microtask_depth().saturating_add(1);
        if next_depth > max_microtask_depth {
            return Err(RuntimeError::Internal {
                message: "microtask depth exceeded max_microtask_depth".to_string(),
            }
            .boxed());
        }

        // run the microtask runnable
        let _guard =
            enter_event_loop_scope(EventLoopScope::for_microtask(microtask.id, next_depth));
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

    /// Drain all pending microtasks.
    fn drain_microtasks<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
    ) -> RuntimeResult<(usize, bool)> {
        // resolve the microtask safety limits for this drain cycle
        let microtask_budget = self
            .event_loop
            .options()
            .microtask_budget
            .map(|budget| usize::try_from(budget).unwrap_or(usize::MAX))
            .unwrap_or(usize::MAX);
        let max_microtask_depth = self
            .event_loop
            .options()
            .max_microtask_depth
            .map(|depth| usize::try_from(depth).unwrap_or(usize::MAX))
            .unwrap_or(usize::MAX);

        // drain microtasks until the queue or budget is exhausted
        let mut drained_microtasks = 0usize;
        let mut budget_exhausted = false;
        loop {
            // stop when the configured budget is consumed
            if drained_microtasks >= microtask_budget {
                budget_exhausted = self.event_loop.has_microtasks();
                break;
            }

            let Some(microtask) = self.event_loop.pop_microtask() else {
                break;
            };
            self.record_task_queue_event(
                TaskSubject::Microtask(microtask.id),
                TaskQueue::Microtask,
                QueueEventKind::Dequeue,
            )?;
            self.state.hooks.on_scheduler_dequeue(RuntimeHookState {
                microtask_id: Some(microtask.id),
                ..RuntimeHookState::empty()
            });
            self.execute_microtask(engine, microtask, max_microtask_depth)?;
            drained_microtasks = drained_microtasks.saturating_add(1);
        }

        Ok((drained_microtasks, budget_exhausted))
    }

    /// Resume one engine continuation with one runtime value.
    fn execute_runnable<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        runnable: EngineContinuation,
        resume_value: RuntimeValue,
    ) -> RuntimeResult<EngineOutcome<RuntimeOutput, RuntimeValue>> {
        engine.resume(runnable, resume_value)
    }

    /// Record one task queue event in replay state.
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

    /// Wait for one scheduler wakeup when the loop has pending but not-ready work.
    fn wait_for_next_turn(&mut self) -> RuntimeResult<bool> {
        // virtual mode never blocks: callers must advance virtual time explicitly
        if self.state.time.mode() == TimeMode::Virtual {
            return Ok(false);
        }

        // compute one timeout from the next scheduled timer deadline
        let wall_now_nanos = self.state.time.wall_nanos();
        let mono_now_nanos = self.state.time.mono_nanos();
        let timeout_nanos = self
            .event_loop
            .timeout_until_next_timer(wall_now_nanos, mono_now_nanos);

        // poll host adapter events before blocking or sleeping
        let host_event_count = self.poll_host_events(Some(0))?;
        if host_event_count > 0 {
            self.state.hooks.on_scheduler_event_wake(RuntimeHookState {
                external_event_count: Some(host_event_count),
                ..RuntimeHookState::empty()
            });

            return Ok(true);
        }

        // block on the poller when available
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self
                .event_loop
                .poll_poller(poller.as_mut(), timeout_nanos)?;
            if event_count > 0 {
                self.state.hooks.on_scheduler_event_wake(RuntimeHookState {
                    external_event_count: Some(event_count),
                    ..RuntimeHookState::empty()
                });
            }

            return Ok(true);
        }

        // otherwise wait for the next timer deadline when one is scheduled
        if let Some(timeout_nanos) = timeout_nanos {
            if timeout_nanos > 0 {
                self.state.time.sleep_nanos(timeout_nanos);
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Poll host adapter events and enqueue runtime poller events.
    fn poll_host_events(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<usize> {
        // drain host adapter events for this tick
        let events = self.host.poll_events(timeout_nanos)?;
        let mut poller_events = Vec::new();
        let mut host_events = Vec::new();
        for event in events {
            // route poller-compatible host events into the scheduler queue
            match event {
                HostEvent::Poller(event) => {
                    poller_events.push(event);
                }
                event => {
                    host_events.push(event);
                }
            }
        }

        // record dropped host queue events from adapter-side queue policy
        let dropped_host_events = self.host.take_dropped_event_count();
        if dropped_host_events > 0 {
            self.event_loop
                .record_dropped_host_queue_events(dropped_host_events);
        }

        let host_event_count = host_events.len();
        let poller_event_count = poller_events.len();

        // enqueue host semantic events for watch-based dispatch
        if !host_events.is_empty() {
            self.event_loop.enqueue_host_events(host_events);
        }

        // enqueue poller events for token-based dispatch
        if !poller_events.is_empty() {
            self.event_loop.enqueue_events(poller_events);
        }

        Ok(host_event_count.saturating_add(poller_event_count))
    }

    /// Return whether the current tick exhausted the configured budget.
    fn is_tick_budget_exhausted(&self, tick_start_mono_nanos: u64) -> bool {
        let Some(tick_budget_nanos) = self.event_loop.options().tick_budget_ns else {
            return false;
        };

        let now = self.state.time.mono_nanos();
        now.saturating_sub(tick_start_mono_nanos) >= tick_budget_nanos
    }
}
