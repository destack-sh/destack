use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostSession;
use crate::platform::resource;
use crate::runtime::DropReason;
use crate::runtime::engine::{
    EngineContinuation, EngineOutcome, EngineOutput, Entry, EntryReference,
};
use crate::runtime::poller::HostPoller;
use crate::runtime::scheduler::{
    EventLoopScope, Microtask, Runnable, Task, TaskId, TaskStatus, current_event_loop_scope,
    enter_event_loop_scope,
};
use crate::runtime::world::World;
use destack_heap as heap;
use destack_workspace::TimeMode;

use super::{Agent, enter_current_agent_context};

impl Agent {
    /// Run an entrypoint through the event loop.
    pub fn run_entrypoint(
        &mut self,
        world: &World,
        host: &HostSession,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let mut poller: Option<Box<dyn HostPoller>> = None;
        self.run_entrypoint_with_host_and_poller(world, host, entry, args, &mut poller)
    }

    /// Run an entrypoint through the event loop with one external poller.
    pub(crate) fn run_entrypoint_with_host_and_poller(
        &mut self,
        world: &World,
        host: &HostSession,
        entry: &Entry,
        args: &[heap::Value],
        poller: &mut Option<Box<dyn HostPoller>>,
    ) -> RuntimeResult<EngineOutput> {
        let agent_ptr = self as *const Agent;
        let event_loop = self.event_loop.as_ref() as *const _;
        let host_ptr = host as *const HostSession;
        let world_ptr = world as *const World;
        let _context_guard = enter_current_agent_context(
            agent_ptr,
            event_loop,
            host_ptr,
            world_ptr,
            host.is_process_main_context(),
        );

        // execute the entrypoint with yielding enabled
        let _guard = enter_event_loop_scope(EventLoopScope::empty());
        let mut shared = world.shared.borrow_mut();
        let mut memory = heap::MemoryContext::with_shared_limits(
            &mut self.heap,
            &mut shared,
            world.shared_limits,
        );
        let outcome = self.engine.run(&mut memory, entry, args)?;

        // handle the entry outcome
        let output = match outcome {
            EngineOutcome::Completed { output } => Ok(output),
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                // enqueue the yielded continuation
                let task_id = self.event_loop.next_task_id();
                self.enqueue_task(world, task_id, continuation, value)?;

                let output = self.run_until_task_complete(world, host, task_id, None, poller)?;
                output.ok_or_else(|| {
                    RuntimeError::EventLoopIdle {
                        task_id: task_id.get(),
                    }
                    .boxed()
                })
            }
        }?;

        Ok(output)
    }

    /// Run one replayable entrypoint through the event loop with one external poller.
    pub(crate) fn run_replayable_entrypoint_with_host_and_poller(
        &mut self,
        world: &World,
        host: &HostSession,
        entry: &EntryReference,
        args: &[heap::Value],
        poller: &mut Option<Box<dyn HostPoller>>,
    ) -> RuntimeResult<EngineOutput> {
        let agent_ptr = self as *const Agent;
        let event_loop = self.event_loop.as_ref() as *const _;
        let host_ptr = host as *const HostSession;
        let world_ptr = world as *const World;
        let _context_guard = enter_current_agent_context(
            agent_ptr,
            event_loop,
            host_ptr,
            world_ptr,
            host.is_process_main_context(),
        );

        // execute the entrypoint with yielding enabled
        let _guard = enter_event_loop_scope(EventLoopScope::empty());
        let mut shared = world.shared.borrow_mut();
        let mut memory = heap::MemoryContext::with_shared_limits(
            &mut self.heap,
            &mut shared,
            world.shared_limits,
        );
        let outcome = self.engine.run_replayable_entry(&mut memory, entry, args)?;

        // handle the entry outcome
        let output = match outcome {
            EngineOutcome::Completed { output } => Ok(output),
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                let task_id = self.event_loop.next_task_id();
                self.enqueue_task(world, task_id, continuation, value)?;

                let output = self.run_until_task_complete(world, host, task_id, None, poller)?;
                output.ok_or_else(|| {
                    RuntimeError::EventLoopIdle {
                        task_id: task_id.get(),
                    }
                    .boxed()
                })
            }
        }?;

        Ok(output)
    }

    /// Run the loop until the specified task completes.
    pub fn run_loop_until_task_complete(
        &mut self,
        world: &World,
        host: &HostSession,
        target_task: TaskId,
    ) -> RuntimeResult<EngineOutput> {
        let mut poller: Option<Box<dyn HostPoller>> = None;
        self.run_loop_until_task_complete_with_host_and_poller(
            world,
            host,
            target_task,
            &mut poller,
        )
    }

    /// Run the loop until the specified task completes with one external poller.
    pub(crate) fn run_loop_until_task_complete_with_host_and_poller(
        &mut self,
        world: &World,
        host: &HostSession,
        target_task: TaskId,
        poller: &mut Option<Box<dyn HostPoller>>,
    ) -> RuntimeResult<EngineOutput> {
        let output = self.run_until_task_complete(world, host, target_task, None, poller)?;
        output.ok_or_else(|| {
            RuntimeError::EventLoopIdle {
                task_id: target_task.get(),
            }
            .boxed()
        })
    }

    /// Run the loop until the specified task completes or one timeout elapses.
    pub fn run_loop_until_task_complete_with_timeout(
        &mut self,
        world: &World,
        host: &HostSession,
        target_task: TaskId,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<EngineOutput>> {
        let mut poller: Option<Box<dyn HostPoller>> = None;
        self.run_until_task_complete(world, host, target_task, timeout_nanos, &mut poller)
    }

    /// Run the loop until one task completes or one timeout elapses with one external poller.
    fn run_until_task_complete(
        &mut self,
        world: &World,
        host: &HostSession,
        target_task: TaskId,
        timeout_nanos: Option<u64>,
        poller: &mut Option<Box<dyn HostPoller>>,
    ) -> RuntimeResult<Option<EngineOutput>> {
        // capture one monotonic start timestamp for timeout accounting
        let start_mono_nanos = world.mono_nanos();

        // run the loop until the target task completes
        loop {
            // stop once the configured timeout elapses
            if let Some(timeout_nanos) = timeout_nanos {
                let elapsed = world.mono_nanos().saturating_sub(start_mono_nanos);
                if elapsed >= timeout_nanos {
                    return Ok(None);
                }
            }

            // run one loop tick for the engine
            let (progressed, output) = self.tick_loop(world, host, Some(target_task))?;
            if let Some(output) = output {
                return Ok(Some(output));
            }

            // wait for the next wakeup when no work progressed this tick
            if !progressed
                && self.event_loop.has_pending_work()
                && self.wait_for_next_turn(world, host, poller)?
            {
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

    /// Execute one local agent tick.
    pub fn tick(&mut self, world: &World, host: &HostSession) -> RuntimeResult<bool> {
        self.tick_once(world, host)
    }

    /// Run runtime ticks until no work remains.
    pub fn tick_until_idle(&mut self, world: &World, host: &HostSession) -> RuntimeResult<()> {
        loop {
            let progressed = self.tick_once(world, host)?;
            if !progressed {
                break;
            }
        }

        Ok(())
    }

    /// Execute one local agent tick.
    fn tick_once(&mut self, world: &World, host: &HostSession) -> RuntimeResult<bool> {
        // run one event loop tick and capture progress
        let (mut progressed, _) = self.tick_loop(world, host, None)?;

        // run one gc cycle when pacing says a cycle is due
        if self.should_collect() {
            let _stats = self.collect();
            progressed = true;
        }

        Ok(progressed)
    }
    /// Tick the loop once and return progress and optional target output.
    fn tick_loop(
        &mut self,
        world: &World,
        host: &HostSession,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<(bool, Option<EngineOutput>)> {
        let agent_ptr = self as *const Agent;
        let event_loop = self.event_loop.as_ref() as *const _;
        let host_ptr = host as *const HostSession;
        let world_ptr = world as *const World;
        let _context_guard = enter_current_agent_context(
            agent_ptr,
            event_loop,
            host_ptr,
            world_ptr,
            host.is_process_main_context(),
        );

        // track whether this tick processed any event loop work
        let mut progressed = false;
        let tick_start_mono_nanos = world.mono_nanos();

        // service host owned ingress before consuming runtime work
        host.service_ingress()?;

        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let (drained, budget_exhausted) = self.drain_microtasks(world)?;
            if drained > 0 {
                progressed = true;
            }
            if budget_exhausted {
                return Ok((progressed, None));
            }
        }
        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // run the next scheduled item if available
        let wall_now = world.wall_nanos();
        let mono_now = world.mono_nanos();
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
                    if let Some(output) = self.execute_dequeued_task(world, task, target_task)? {
                        return Ok((true, Some(output)));
                    }
                }
                Runnable::Microtask(microtask) => {
                    self.hooks.on_scheduler_dequeue(world);
                    // run the microtask to completion
                    self.execute_microtask(world, microtask, max_microtask_depth)?;
                }
                Runnable::Timer(timer) => {
                    self.hooks.on_scheduler_timer_fire(world);
                    self.deliver_timer_wake(world, timer)?;
                }
                Runnable::PollerEvent(event) => {
                    // dispatch an external-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_event(event) {
                        self.enqueue_prepared_task(world, task)?;
                    }
                    // drop stale and unregistered events without crashing the loop
                    else {
                        self.event_loop
                            .record_drop(DropReason::UnwatchedDispatch, 1);
                    }
                }
                Runnable::HostEvent(event) => {
                    // dispatch one host-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_host_event(event) {
                        self.enqueue_prepared_task(world, task)?;
                    }
                    // account for unconsumed host semantic events
                    else {
                        self.event_loop
                            .record_drop(DropReason::UnwatchedDispatch, 1);
                    }
                }
            }
        }

        // run one queued macrotask after routing timer and event watches
        if !ran_macrotask
            && let Some(task) = self.event_loop.pop_task()
            && let Some(output) = self.execute_dequeued_task(world, task, target_task)?
        {
            return Ok((true, Some(output)));
        }

        Ok((progressed, None))
    }

    /// Deliver one fired timer into the watched task queue.
    pub(crate) fn deliver_timer_wake(
        &mut self,
        world: &World,
        timer: crate::runtime::scheduler::Timer,
    ) -> RuntimeResult<()> {
        let should_dispatch = crate::runtime::time::timer::on_event_loop_timer_fire(
            &self.resources,
            world.clock(),
            world.time_mode(),
            resource::TimerHandle(timer.handle),
        )?;
        if should_dispatch {
            // dispatch a timer watch task when one is registered
            if let Some(task) = self.event_loop.task_for_timer(timer) {
                self.enqueue_prepared_task(world, task)?;
            }

            // one-shot timers no longer need a dispatch watch after firing
            if timer.interval.is_none() {
                self.event_loop.unwatch_timer(timer.handle);
            }
        }
        // stale and inactive timer fires must not dispatch callbacks
        else {
            self.event_loop.unwatch_timer(timer.handle);
        }

        Ok(())
    }

    /// Enqueue one yielded continuation as a task.
    fn enqueue_task(
        &mut self,
        world: &World,
        task_id: TaskId,
        runnable: EngineContinuation,
        resume_value: heap::Value,
    ) -> RuntimeResult<()> {
        // build the task metadata
        let task = Task {
            id: task_id,
            runnable,
            resume_value,
            status: TaskStatus::Ready,
            priority: 0,
        };

        self.enqueue_prepared_task(world, task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_dequeued_task(
        &mut self,
        world: &World,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<EngineOutput>> {
        self.hooks.on_scheduler_dequeue(world);
        self.execute_task(world, task, target_task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_task(
        &mut self,
        world: &World,
        mut task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<EngineOutput>> {
        // run the task runnable
        task.status = TaskStatus::Waiting;
        let _guard = enter_event_loop_scope(EventLoopScope::for_task(task.id));
        let outcome = self.execute_runnable(world, task.runnable, task.resume_value)?;

        // handle the task outcome
        match outcome {
            EngineOutcome::Completed { output } => {
                task.status = TaskStatus::Completed;
                if target_task == Some(task.id) {
                    return Ok(Some(output));
                }
            }
            EngineOutcome::Yielded {
                continuation,
                value,
            } => {
                task.status = TaskStatus::Waiting;
                self.enqueue_task(world, task.id, continuation, value)?;
            }
        }

        self.drain_microtasks(world)?;

        Ok(None)
    }

    /// Enqueue one prepared task and record enqueue hooks.
    fn enqueue_prepared_task(&mut self, world: &World, task: Task) -> RuntimeResult<()> {
        // enqueue the task into the event loop
        self.event_loop.enqueue_task(task);
        self.hooks.on_scheduler_enqueue(world);

        Ok(())
    }

    /// Execute one microtask to completion.
    fn execute_microtask(
        &mut self,
        world: &World,
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
        let outcome =
            self.execute_runnable(world, microtask.continuation, microtask.resume_value)?;

        // ensure microtasks run to completion
        match outcome {
            EngineOutcome::Completed { .. } => Ok(()),
            EngineOutcome::Yielded { .. } => Err(RuntimeError::Internal {
                message: "microtask yielded while running to completion".to_string(),
            }
            .boxed()),
        }
    }

    /// Drain all pending microtasks. Returns (drained_microtasks, budget_exhausted)
    fn drain_microtasks(&mut self, world: &World) -> RuntimeResult<(usize, bool)> {
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
        let mut num_drained_microtasks = 0usize;
        let mut budget_exhausted = false;
        loop {
            // stop when the configured budget is consumed
            if num_drained_microtasks >= microtask_budget {
                budget_exhausted = self.event_loop.has_microtasks();
                break;
            }

            let Some(microtask) = self.event_loop.pop_microtask() else {
                break;
            };
            self.hooks.on_scheduler_dequeue(world);
            self.execute_microtask(world, microtask, max_microtask_depth)?;
            num_drained_microtasks = num_drained_microtasks.saturating_add(1);
        }

        Ok((num_drained_microtasks, budget_exhausted))
    }

    /// Resume one engine continuation with one runtime value.
    fn execute_runnable(
        &mut self,
        world: &World,
        runnable: EngineContinuation,
        resume_value: heap::Value,
    ) -> RuntimeResult<EngineOutcome> {
        let mut shared = world.shared.borrow_mut();
        let mut memory = heap::MemoryContext::with_shared_limits(
            &mut self.heap,
            &mut shared,
            world.shared_limits,
        );
        self.engine.resume(&mut memory, runnable, resume_value)
    }

    /// Wait for one scheduler wakeup when the loop has pending but not-ready work.
    fn wait_for_next_turn(
        &mut self,
        world: &World,
        host: &HostSession,
        poller: &mut Option<Box<dyn HostPoller>>,
    ) -> RuntimeResult<bool> {
        // virtual mode never blocks: callers must advance virtual time explicitly
        if world.time_mode() == TimeMode::Virtual {
            return Ok(false);
        }

        // compute one timeout from the next scheduled timer deadline
        let wall_now = world.wall();
        let mono_now = world.mono();
        let timeout_nanos = self.event_loop.timeout_until_next_timer(wall_now, mono_now);

        // poll host events before blocking or sleeping
        host.service_ingress()?;
        let host_event_count = self.poll_host_events(host, Some(0))?;
        if host_event_count > 0 {
            for _ in 0..host_event_count {
                self.hooks.on_ingress_enqueue(world);
            }

            return Ok(true);
        }

        // block on the poller when available
        if let Some(poller) = poller.as_mut() {
            let event_count = self
                .event_loop
                .poll_poller(poller.as_mut(), timeout_nanos.map(|timeout| timeout.get()))?;
            if event_count > 0 {
                for _ in 0..event_count {
                    self.hooks.on_ingress_enqueue(world);
                }
            }

            return Ok(true);
        }

        // otherwise wait for the next timer deadline when one is scheduled
        if let Some(timeout_nanos) = timeout_nanos {
            if timeout_nanos.get() > 0 {
                world.clock().host_sleep_nanos(timeout_nanos.get());
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Poll host events and enqueue host semantic events.
    fn poll_host_events(
        &mut self,
        host: &HostSession,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        // drain host events for this tick
        let poll_result = host.poll_events(timeout_nanos)?;
        let host_events = poll_result.events;

        // record dropped host queue events from host-side queue policy
        let dropped_host_events = poll_result.dropped_event_count;
        if dropped_host_events > 0 {
            self.record_drop(DropReason::QueuePressure, dropped_host_events);
        }

        // enqueue host semantic events for watch-based dispatch
        let host_event_count = host_events.len();
        if !host_events.is_empty() {
            self.event_loop.enqueue_host_events(host_events);
        }

        Ok(host_event_count)
    }

    /// Return whether the current tick exhausted the configured budget.
    fn is_tick_budget_exhausted(&self, world: &World, tick_start_mono_nanos: u64) -> bool {
        let Some(tick_budget_nanos) = self.event_loop.options().tick_budget_ns else {
            return false;
        };

        let now = world.mono_nanos();
        now.saturating_sub(tick_start_mono_nanos) >= tick_budget_nanos
    }
}
