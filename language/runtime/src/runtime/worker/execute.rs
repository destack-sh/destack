use std::ptr::NonNull;

use super::{
    BindingCallContext, ExecutionContext, RunnableScope, Worker, current_runnable_scope,
    enter_runnable_scope,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostSession;
use crate::host::poller::HostPoller;
use crate::runtime::SharedHeap;
use crate::runtime::engine::{Continuation, Entry, Outcome};
use crate::runtime::scheduler::{Microtask, Task, TaskId, Wake};
use crate::runtime::time::Nanos;
use crate::world::WorldState;
use destack_workspace::ClockSource;
use {destack_engine as engine, destack_heap as heap};

impl Worker {
    /// Build one runtime-owned binding call context.
    fn binding_call_context(
        &mut self,
        world: &mut WorldState,
        host: &HostSession,
    ) -> BindingCallContext {
        BindingCallContext {
            worker: self as *mut Worker,
            event_loop: self.event_loop.as_ref(),
            host,
            world,
            scope: current_runnable_scope(),
            execution_context: ExecutionContext::new(host.is_process_main_context()),
        }
    }

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
        let mut call_context = self.binding_call_context(world, host);
        let Worker {
            heap,
            shared_allocator,
            shared_gc_worker,
            statics,
            engine,
            ..
        } = self;
        let context = engine::CallContext {
            runtime: NonNull::from(&mut call_context).cast(),
            memory: engine::MemoryContext {
                heap,
                shared_heap: shared.heap(),
                shared_allocator,
                shared_gc_worker,
                worker_static: statics,
                runtime_static,
            },
        };
        let outcome = engine.run(context, entry, args)?;

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
            let mut remaining_timeout_nanos = None;

            // stop once the configured timeout elapses
            if let Some(timeout_nanos) = timeout_nanos {
                let elapsed = world.mono_nanos().saturating_sub(start_mono_nanos);
                if elapsed >= timeout_nanos {
                    return Ok(None);
                }

                remaining_timeout_nanos = Some(timeout_nanos.saturating_sub(elapsed));
            }

            // run one loop tick for the engine
            let (progressed, output) =
                self.tick_loop_for_target(world, shared, runtime_static, host, target_task)?;
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
                && self.wait_for_next_turn(world, host, poller, remaining_timeout_nanos)?
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
        let mut progressed = self.tick_loop(world, shared, runtime_static, host)?;

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

    /// Tick the loop once and return whether work progressed.
    #[inline(never)]
    fn tick_loop(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
    ) -> RuntimeResult<bool> {
        // track whether this tick processed any event loop work
        let mut progressed = false;
        let tick_start_mono_nanos = world.mono_nanos();

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let (drained, budget_exhausted) =
                self.drain_microtasks(world, shared, runtime_static, host)?;
            if drained > 0 {
                progressed = true;
            }
            if budget_exhausted {
                return Ok(progressed);
            }
        }
        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok(progressed);
        }

        // run one queued macrotask before pulling external wakes
        if self.run_ready_task(world, shared, runtime_static, host)? {
            return Ok(true);
        }

        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok(progressed);
        }

        // dispatch one wake into the task queue
        if self.dispatch_one_wake(world)? {
            progressed = true;
        }

        // run one task produced by the dispatched wake
        if self.run_ready_task(world, shared, runtime_static, host)? {
            return Ok(true);
        }

        Ok(progressed)
    }

    /// Tick the loop once and return output when the target task completes.
    #[inline(never)]
    fn tick_loop_for_target(
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

        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let (drained, budget_exhausted) =
                self.drain_microtasks(world, shared, runtime_static, host)?;
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

        // run one queued macrotask before pulling external wakes
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_dequeued_task_for_target(
                world,
                shared,
                runtime_static,
                host,
                task,
                target_task,
            )? {
                return Ok((true, Some(output)));
            }

            return Ok((true, None));
        }

        if self.is_tick_budget_exhausted(world, tick_start_mono_nanos) {
            return Ok((progressed, None));
        }

        // dispatch one wake into the task queue
        let wall_now = Nanos::new(world.wall_nanos());
        let mono_now = Nanos::new(world.mono_nanos());
        if let Some(wake) = self.event_loop.next_wake(wall_now, mono_now)? {
            progressed = true;
            if matches!(&wake, Wake::Timer(_)) {
                self.hooks.on_scheduler_timer_fire(world);
            }
            if let Some(task) = self.event_loop.task_for_wake(wake, &mut self.engine) {
                self.enqueue_prepared_task(world, task)?;
            }
        }

        // run one task produced by the dispatched wake
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_dequeued_task_for_target(
                world,
                shared,
                runtime_static,
                host,
                task,
                target_task,
            )? {
                return Ok((true, Some(output)));
            }

            return Ok((true, None));
        }

        Ok((progressed, None))
    }

    /// Run one ready task when the queue is non-empty.
    #[inline(never)]
    fn run_ready_task(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
    ) -> RuntimeResult<bool> {
        let Some(task) = self.event_loop.pop_task() else {
            return Ok(false);
        };

        self.execute_dequeued_task(world, shared, runtime_static, host, task)?;

        Ok(true)
    }

    /// Dispatch one wake into the task queue.
    #[inline(never)]
    fn dispatch_one_wake(&mut self, world: &mut WorldState) -> RuntimeResult<bool> {
        let wall_now = Nanos::new(world.wall_nanos());
        let mono_now = Nanos::new(world.mono_nanos());
        let Some(wake) = self.event_loop.next_wake(wall_now, mono_now)? else {
            return Ok(false);
        };

        if matches!(&wake, Wake::Timer(_)) {
            self.hooks.on_scheduler_timer_fire(world);
        }
        if let Some(task) = self.event_loop.task_for_wake(wake, &mut self.engine) {
            self.enqueue_prepared_task(world, task)?;
        }

        Ok(true)
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
            priority: 0,
        };

        self.enqueue_prepared_task(world, task)
    }

    /// Execute one task.
    fn execute_dequeued_task(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        task: Task,
    ) -> RuntimeResult<()> {
        self.hooks.on_scheduler_dequeue(world);
        self.execute_task(world, shared, runtime_static, host, task)
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_dequeued_task_for_target(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<engine::Value>> {
        self.hooks.on_scheduler_dequeue(world);
        self.execute_task_for_target(world, shared, runtime_static, host, task, target_task)
    }

    /// Execute one task.
    fn execute_task(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        task: Task,
    ) -> RuntimeResult<()> {
        // run the task runnable
        let task_id = task.id;
        let _guard = enter_runnable_scope(RunnableScope::for_task(task_id));
        let outcome = self.execute_runnable(
            world,
            shared,
            runtime_static,
            host,
            task.runnable,
            task.resume_value,
        )?;

        // handle the task outcome
        if let Outcome::Yielded {
            continuation,
            value,
        } = outcome
        {
            self.enqueue_task(world, task_id, continuation, value)?;
        }

        self.drain_microtasks(world, shared, runtime_static, host)?;

        Ok(())
    }

    /// Execute one task and return output when it completes the target task.
    fn execute_task_for_target(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<engine::Value>> {
        // run the task runnable
        let _guard = enter_runnable_scope(RunnableScope::for_task(task.id));
        let outcome = self.execute_runnable(
            world,
            shared,
            runtime_static,
            host,
            task.runnable,
            task.resume_value,
        )?;

        // handle the task outcome
        match outcome {
            Outcome::Completed { value } => {
                if target_task == Some(task.id) {
                    return Ok(Some(value));
                }
            }
            Outcome::Yielded {
                continuation,
                value,
            } => {
                self.enqueue_task(world, task.id, continuation, value)?;
            }
        }

        self.drain_microtasks(world, shared, runtime_static, host)?;

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
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
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
            world,
            shared,
            runtime_static,
            host,
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
        host: &HostSession,
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
            self.execute_microtask(
                world,
                shared,
                runtime_static,
                host,
                microtask,
                max_microtask_depth,
            )?;
            num_drained_microtasks = num_drained_microtasks.saturating_add(1);
        }

        Ok((num_drained_microtasks, budget_exhausted))
    }

    /// Resume one engine continuation with one runtime value.
    fn execute_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &SharedHeap,
        runtime_static: &engine::StaticSpace,
        host: &HostSession,
        runnable: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        let mut call_context = self.binding_call_context(world, host);
        let Worker {
            heap,
            shared_allocator,
            shared_gc_worker,
            statics,
            engine,
            ..
        } = self;
        let context = engine::CallContext {
            runtime: NonNull::from(&mut call_context).cast(),
            memory: engine::MemoryContext {
                heap,
                shared_heap: shared.heap(),
                shared_allocator,
                shared_gc_worker,
                worker_static: statics,
                runtime_static,
            },
        };

        engine.resume(context, runnable, resume_value)
    }

    /// Wait for one scheduler wakeup when the loop has pending but not-ready work.
    fn wait_for_next_turn(
        &mut self,
        world: &mut WorldState,
        host: &HostSession,
        poller: &mut dyn HostPoller,
        remaining_timeout_nanos: Option<u64>,
    ) -> RuntimeResult<bool> {
        // virtual mode never blocks: callers must advance virtual time explicitly
        if world.clock().source() == ClockSource::Virtual {
            return Ok(false);
        }

        // compute one timeout from the next scheduled timer deadline
        let wall_now = world.wall();
        let mono_now = world.mono();
        let timeout_nanos = self.event_loop.timeout_until_next_timer(wall_now, mono_now);
        let timeout_nanos = match (timeout_nanos, remaining_timeout_nanos.map(Nanos::new)) {
            (Some(timer_timeout), Some(loop_timeout)) => Some(timer_timeout.min(loop_timeout)),
            (Some(timer_timeout), None) => Some(timer_timeout),
            (None, Some(loop_timeout)) => Some(loop_timeout),
            (None, None) => None,
        };

        // drain host ingress before blocking or sleeping
        let host_event_count = self.drain_host_wakes(host, Some(0))?;
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

    /// Poll host events and enqueue host wakes.
    fn drain_host_wakes(
        &mut self,
        host: &HostSession,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        // drain host events
        let poll_result = host.poll(timeout_nanos)?;
        let host_events = poll_result.events;

        // enqueue host wakes
        let host_event_count = host_events.len();
        if !host_events.is_empty() {
            self.event_loop.enqueue_host_wakes(host_events);
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
