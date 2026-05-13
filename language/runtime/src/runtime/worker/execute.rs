use super::{RunnableScope, Worker, current_runnable_scope, enter_runnable_scope};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::poller::HostPoller;
use crate::host::{HostSession, resource};
use crate::runtime::SharedHeap;
use crate::runtime::engine::{Context, Continuation, Entry, Outcome};
use crate::runtime::scheduler::{
    Microtask, Runnable, Task, TaskId, TaskStatus, Timer, TimerHandle,
};
use crate::runtime::time::timer::on_event_loop_timer_fire;
use crate::world::WorldState;
use destack_workspace::TimeMode;
use {destack_engine as engine, destack_heap as heap};

impl Worker {
    /// Run an entrypoint through the event loop with one external poller.
    pub(crate) fn run_entrypoint_with_host_and_poller(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        entry: &Entry,
        args: &[engine::Value],
        poller: &mut dyn HostPoller,
    ) -> RuntimeResult<engine::Value> {
        // execute the entrypoint with yielding enabled
        let _guard = enter_runnable_scope(RunnableScope::empty());
        let context = Context {
            heap: &mut self.heap,
            shared_heap: shared.heap(),
            shared_allocator: &mut self.shared_allocator,
            shared_gc: &self.shared_gc_worker,
            worker_static: &mut self.statics,
            runtime_static,
        };
        let outcome = self.engine.run(context, entry, args)?;

        // handle the entry outcome
        let output = match outcome {
            Outcome::Completed { value } => value,
            Outcome::Yielded {
                continuation,
                value,
            } => {
                let task_id = self.event_loop.next_task_id();
                self.enqueue_task(world, task_id, continuation, value)?;

                let output = self.run_event_loop(
                    world,
                    shared,
                    runtime_static,
                    host,
                    Some(task_id),
                    None,
                    poller,
                )?;
                let Some(output) = output else {
                    return Err(RuntimeError::EventLoopIdle {
                        task_id: task_id.get(),
                    }
                    .boxed());
                };

                output
            }
        };

        Ok(output)
    }

    /// Run the event loop until idle, timeout, or the target task completes.
    pub(crate) fn run_event_loop(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        target_task: Option<TaskId>,
        timeout_nanos: Option<u64>,
        poller: &mut dyn HostPoller,
    ) -> RuntimeResult<Option<engine::Value>> {
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
            let (progressed, output) =
                self.tick_loop(world, shared, runtime_static, host, target_task)?;
            if let Some(output) = output {
                return Ok(Some(output));
            }

            // direct roots: worker execution may reshuffle shared roots
            if progressed && shared.is_marking() {
                shared.queue_root_scan(self.id);
            }

            // donate GC work before sleeping or declaring idle
            let safepoint_progressed = self.collect_at_safepoint(shared, true)?;
            let progressed = progressed || safepoint_progressed;

            // wait for the next wakeup when no work progressed this tick
            if !progressed
                && self.event_loop.has_pending_work()
                && self.wait_for_next_turn(world, host, poller)?
            {
                continue;
            }

            // exit when the target task cannot make further progress
            if !progressed {
                let Some(target_task) = target_task else {
                    return Ok(None);
                };

                return Err(RuntimeError::EventLoopIdle {
                    task_id: target_task.get(),
                }
                .boxed());
            }
        }
    }

    /// Execute one local worker tick.
    pub(crate) fn tick(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
    ) -> RuntimeResult<bool> {
        self.tick_once(world, shared, runtime_static, host)
    }

    /// Execute one local worker tick.
    fn tick_once(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
    ) -> RuntimeResult<bool> {
        // run one event loop tick and capture progress
        let (mut progressed, _) = self.tick_loop(world, shared, runtime_static, host, None)?;

        // direct roots: worker execution may reshuffle shared roots
        if progressed && shared.is_marking() {
            shared.queue_root_scan(self.id);
        }

        // run one explicit GC safepoint after the ordinary tick
        if self.collect_at_safepoint(shared, !progressed)? {
            progressed = true;
        }

        Ok(progressed)
    }

    /// Run cooperative GC work at one worker safepoint.
    fn collect_at_safepoint(&mut self, shared: &SharedHeap, is_idle: bool) -> RuntimeResult<bool> {
        let prioritize_shared = shared.is_terminating()
            || shared.pending_root_epoch(self.id).is_some()
            || (shared.is_marking() && !self.shared_edge_scan_idle());
        let should_drain = is_idle || shared.is_terminating();
        let mut progressed = false;

        // cooperative GC work
        loop {
            let pass_progressed = if prioritize_shared {
                self.collect_shared_priority_step(shared)?
            } else {
                self.collect_local_priority_step(shared)?
            };

            if !pass_progressed {
                break;
            }

            progressed = true;
            if !should_drain {
                break;
            }
        }

        Ok(progressed)
    }

    /// Run one GC safepoint step with shared heap work first.
    fn collect_shared_priority_step(&mut self, shared: &SharedHeap) -> RuntimeResult<bool> {
        // direct shared roots
        if self.assist_shared_root_scan(shared)? {
            return Ok(true);
        }

        // local to shared edges
        if self.assist_shared_edge_scan(shared)? {
            return Ok(true);
        }

        // shared mark and sweep work
        if self.assist_shared_gc(shared)? {
            return Ok(true);
        }

        // local heap work
        if self.collect_local_step()?.made_progress() {
            return Ok(true);
        }

        Ok(false)
    }

    /// Run one GC safepoint step with local heap work first.
    fn collect_local_priority_step(&mut self, shared: &SharedHeap) -> RuntimeResult<bool> {
        // local heap work
        if self.collect_local_step()?.made_progress() {
            return Ok(true);
        }

        // direct shared roots
        if self.assist_shared_root_scan(shared)? {
            return Ok(true);
        }

        // local to shared edges
        if self.assist_shared_edge_scan(shared)? {
            return Ok(true);
        }

        // shared mark and sweep work
        if self.assist_shared_gc(shared)? {
            return Ok(true);
        }

        Ok(false)
    }

    /// Publish one pending direct shared-root scan from this worker safepoint.
    fn assist_shared_root_scan(&mut self, shared: &SharedHeap) -> RuntimeResult<bool> {
        // active pass
        let Some(epoch) = shared.pending_root_epoch(self.id) else {
            return Ok(false);
        };

        // owner-local root publication
        let roots = self.collect_shared_roots()?;
        shared.replace_direct_roots(epoch, self.id, roots);

        Ok(true)
    }

    /// Assist one active shared reference pass from this worker safepoint.
    fn assist_shared_edge_scan(&mut self, shared: &SharedHeap) -> RuntimeResult<bool> {
        if !shared.is_marking() || self.shared_edge_scan_idle() {
            return Ok(false);
        }

        let work_bytes = shared.edge_scan_work_bytes();
        if work_bytes == 0 {
            return Ok(false);
        }

        let mut roots = Vec::new();
        let work_done = self.scan_shared_references(&mut roots, work_bytes)?;
        shared.push_edge_roots(self.id, &roots);

        let is_idle = self.shared_edge_scan_idle();
        if is_idle {
            shared.leave_edge_scan(self.id);
        }

        Ok(work_done > 0 || is_idle)
    }

    /// Assist one active shared collection from this worker safepoint.
    fn assist_shared_gc(&mut self, runtime_heap: &SharedHeap) -> RuntimeResult<bool> {
        let shared = runtime_heap.heap();
        let budget_bytes = shared.take_assist_budget_bytes();
        if budget_bytes == 0 || shared.gc_phase() == heap::SharedGcPhase::Idle {
            return Ok(false);
        }

        let shared_roots = runtime_heap.roots();
        let roots = shared_roots.roots_snapshot();
        let roots_complete = shared_roots.roots_complete();
        let progress = shared
            .collect_step_for_worker(
                Some(&self.shared_gc_worker),
                roots.as_ref(),
                roots_complete,
                budget_bytes,
            )
            .map_err(Box::<RuntimeError>::from)?;

        Ok(progress.made_progress())
    }

    /// Tick the loop once and return progress and optional target output.
    fn tick_loop(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<(bool, Option<engine::Value>)> {
        // track whether this tick processed any event loop work
        let mut progressed = false;
        let tick_start_mono_nanos = world.mono_nanos();

        // service host ingress before consuming runtime work
        let host_event_count = self.poll_host_events(host, Some(0))?;
        if host_event_count > 0 {
            for _ in 0..host_event_count {
                self.hooks.on_ingress_enqueue(world);
            }

            progressed = true;
        }

        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let (drained, budget_exhausted) =
                self.drain_microtasks(world, shared, runtime_static)?;
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
            .map(|depth| runtime_limit_as_usize(depth, "max_microtask_depth"))
            .transpose()?
            .unwrap_or(usize::MAX);
        let mut ran_macrotask = false;
        if let Some(item) = self.event_loop.next_runnable(wall_now, mono_now)? {
            progressed = true;
            match item {
                Runnable::Task(task) => {
                    ran_macrotask = true;
                    if let Some(output) = self.execute_dequeued_task(
                        world,
                        shared,
                        runtime_static,
                        task,
                        target_task,
                    )? {
                        return Ok((true, Some(output)));
                    }
                }
                Runnable::Microtask(microtask) => {
                    self.hooks.on_scheduler_dequeue(world);
                    // run the microtask to completion
                    self.execute_microtask(shared, runtime_static, microtask, max_microtask_depth)?;
                }
                Runnable::Timer(timer) => {
                    self.hooks.on_scheduler_timer_fire(world);
                    self.deliver_timer_wake(world, timer)?;
                }
                Runnable::PollerEvent(event) => {
                    // dispatch an external-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_event(event, &mut self.engine) {
                        self.enqueue_prepared_task(world, task)?;
                    }
                }
                Runnable::HostEvent(event) => {
                    // dispatch one host-event watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_host_event(event, &mut self.engine)
                    {
                        self.enqueue_prepared_task(world, task)?;
                    }
                }
            }
        }

        // run one queued macrotask after routing timer and event watches
        if !ran_macrotask
            && let Some(task) = self.event_loop.pop_task()
            && let Some(output) =
                self.execute_dequeued_task(world, shared, runtime_static, task, target_task)?
        {
            return Ok((true, Some(output)));
        }

        Ok((progressed, None))
    }

    /// Deliver one fired timer into the watched task queue.
    pub(crate) fn deliver_timer_wake(
        &mut self,
        world: &mut WorldState,
        timer: Timer,
    ) -> RuntimeResult<()> {
        // worker-owned callbacks
        match timer.handle {
            TimerHandle::Internal(_) => Err(RuntimeError::Internal {
                message: "internal timer fired without an owner".to_string(),
            }
            .boxed()),
            TimerHandle::Resource(handle) => {
                let should_dispatch = on_event_loop_timer_fire(
                    &self.resources,
                    world.clock(),
                    world.time_mode(),
                    resource::TimerHandle(handle),
                )?;
                if should_dispatch {
                    // dispatch a timer watch task when one is registered
                    if let Some(task) = self.event_loop.task_for_timer(timer, &mut self.engine) {
                        self.enqueue_prepared_task(world, task)?;
                    }

                    // one-shot timers no longer need a dispatch watch after firing
                    if timer.interval.is_none() {
                        self.event_loop.unwatch_timer(handle);
                    }
                }
                // stale and inactive timer fires must not dispatch callbacks
                else {
                    self.event_loop.unwatch_timer(handle);
                }

                Ok(())
            }
        }
    }

    /// Enqueue one yielded continuation as a task.
    fn enqueue_task(
        &mut self,
        world: &mut WorldState,
        task_id: TaskId,
        runnable: Continuation,
        resume_value: engine::Value,
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
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<engine::Value>> {
        self.hooks.on_scheduler_dequeue(world);
        self.execute_task(world, shared, runtime_static, task, target_task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_task(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        mut task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<engine::Value>> {
        // run the task runnable
        task.status = TaskStatus::Waiting;
        let _guard = enter_runnable_scope(RunnableScope::for_task(task.id));
        let outcome =
            self.execute_runnable(shared, runtime_static, task.runnable, task.resume_value)?;

        // handle the task outcome
        match outcome {
            Outcome::Completed { value } => {
                task.status = TaskStatus::Completed;
                if target_task == Some(task.id) {
                    return Ok(Some(value));
                }
            }
            Outcome::Yielded {
                continuation,
                value,
            } => {
                task.status = TaskStatus::Waiting;
                self.enqueue_task(world, task.id, continuation, value)?;
            }
        }

        self.drain_microtasks(world, shared, runtime_static)?;

        Ok(None)
    }

    /// Enqueue one prepared task and record enqueue hooks.
    fn enqueue_prepared_task(&mut self, world: &mut WorldState, task: Task) -> RuntimeResult<()> {
        // enqueue the task into the event loop
        self.event_loop.enqueue_task(task);
        self.hooks.on_scheduler_enqueue(world);

        Ok(())
    }

    /// Execute one microtask to completion.
    fn execute_microtask(
        &mut self,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        microtask: Microtask,
        max_microtask_depth: usize,
    ) -> RuntimeResult<()> {
        // enforce true microtask nesting depth
        let parent_scope = current_runnable_scope();
        let next_depth = parent_scope.microtask_depth().saturating_add(1);
        if next_depth > max_microtask_depth {
            return Err(RuntimeError::Internal {
                message: "microtask depth exceeded max_microtask_depth".to_string(),
            }
            .boxed());
        }

        // run the microtask runnable
        let _guard = enter_runnable_scope(RunnableScope::for_microtask(microtask.id, next_depth));
        let outcome = self.execute_runnable(
            shared,
            runtime_static,
            microtask.continuation,
            microtask.resume_value,
        )?;

        // ensure microtasks run to completion
        match outcome {
            Outcome::Completed { .. } => Ok(()),
            Outcome::Yielded { .. } => Err(RuntimeError::Internal {
                message: "microtask yielded while running to completion".to_string(),
            }
            .boxed()),
        }
    }

    /// Drain all pending microtasks. Returns (drained_microtasks, budget_exhausted)
    fn drain_microtasks(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
    ) -> RuntimeResult<(usize, bool)> {
        // resolve the microtask safety limits for this drain cycle
        let microtask_budget = self
            .event_loop
            .options()
            .microtask_budget
            .map(|budget| runtime_limit_as_usize(budget, "microtask_budget"))
            .transpose()?
            .unwrap_or(usize::MAX);
        let max_microtask_depth = self
            .event_loop
            .options()
            .max_microtask_depth
            .map(|depth| runtime_limit_as_usize(depth, "max_microtask_depth"))
            .transpose()?
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
            self.execute_microtask(shared, runtime_static, microtask, max_microtask_depth)?;
            num_drained_microtasks = num_drained_microtasks.saturating_add(1);
        }

        Ok((num_drained_microtasks, budget_exhausted))
    }

    /// Resume one engine continuation with one runtime value.
    fn execute_runnable(
        &mut self,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        runnable: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        let context = Context {
            heap: &mut self.heap,
            shared_heap: shared.heap(),
            shared_allocator: &mut self.shared_allocator,
            shared_gc: &self.shared_gc_worker,
            worker_static: &mut self.statics,
            runtime_static,
        };

        self.engine.resume(context, runnable, resume_value)
    }

    /// Wait for one scheduler wakeup when the loop has pending but not-ready work.
    fn wait_for_next_turn(
        &mut self,
        world: &mut WorldState,
        host: &HostSession,
        poller: &mut dyn HostPoller,
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
        let host_event_count = self.poll_host_events(host, Some(0))?;
        if host_event_count > 0 {
            for _ in 0..host_event_count {
                self.hooks.on_ingress_enqueue(world);
            }

            return Ok(true);
        }

        // block on poller work and timer wakeups together
        let event_count = self
            .event_loop
            .poll_poller(poller, timeout_nanos.map(|timeout| timeout.get()))?;
        if event_count > 0 {
            for _ in 0..event_count {
                self.hooks.on_ingress_enqueue(world);
            }

            return Ok(true);
        }

        Ok(timeout_nanos.is_some())
    }

    /// Poll host events and enqueue host events.
    fn poll_host_events(
        &mut self,
        host: &HostSession,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        // drain host events for this tick
        let poll_result = host.poll(timeout_nanos)?;
        let host_events = poll_result.events;

        // enqueue host events for watch-based dispatch
        let host_event_count = host_events.len();
        if !host_events.is_empty() {
            self.event_loop.enqueue_host_events(host_events);
        }

        Ok(host_event_count)
    }

    /// Return whether the current tick exhausted the configured budget.
    fn is_tick_budget_exhausted(&self, world: &mut WorldState, tick_start_mono_nanos: u64) -> bool {
        let Some(tick_budget_nanos) = self.event_loop.options().tick_budget_ns else {
            return false;
        };

        let now = world.mono_nanos();
        now.saturating_sub(tick_start_mono_nanos) >= tick_budget_nanos
    }
}

/// Convert one configured runtime limit into a host usize.
fn runtime_limit_as_usize(value: u64, label: &str) -> RuntimeResult<usize> {
    usize::try_from(value)
        .map_err(|_| RuntimeError::Internal {
            message: format!("runtime {label} exceeds host usize: {value}"),
        })
        .map_err(Box::new)
}
