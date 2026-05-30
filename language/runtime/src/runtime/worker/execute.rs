use std::ptr::NonNull;

use super::{
    BindingCallContext, ExecutionContext, RunnableScope, Worker, current_runnable_scope,
    enter_runnable_scope,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Host;
use crate::host::core::{HostQueue, poll_host_events};
use crate::host::poller::HostPoller;
use crate::runtime::RuntimeHeap;
use crate::runtime::engine::{Continuation, Entry, Outcome};
use crate::runtime::scheduler::{Microtask, Task, TaskId, Wake};
use crate::runtime::time::{ClockSource, Nanos};
use crate::world::WorldState;
use destack_engine as engine;
use destack_heap as heap;

/// The default maximum nested microtask depth.
const DEFAULT_MAX_MICROTASK_DEPTH: usize = usize::MAX;

impl Worker {
    /// Build one runtime-owned binding call context.
    fn binding_call_context<'host>(
        &mut self,
        world: &mut WorldState,
        host: &'host dyn Host,
        host_queue: &'host HostQueue,
    ) -> BindingCallContext<'host> {
        BindingCallContext {
            runtime_id: self.runtime_id,
            worker_id: self.id,
            environment: self.environment.clone(),
            options: self.options.clone(),
            diagnostics: self.diagnostics.clone(),
            scenario: self.scenario.clone(),
            bindings: &self.bindings,
            host,
            host_queue,
            world,
            scope: current_runnable_scope(),
            execution_context: ExecutionContext::new(host.is_process_main_context()),
        }
    }

    /// Run one entrypoint through this worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        entry: &Entry,
        args: &[engine::Value],
        poller: &mut dyn HostPoller,
    ) -> RuntimeResult<engine::Value> {
        // execute the entrypoint with yielding enabled
        let _guard = enter_runnable_scope(RunnableScope::empty());
        let mut call_context = self.binding_call_context(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_gc_worker,
            statics,
            engine,
            ..
        } = self;
        let context = engine::CallContext {
            runtime: NonNull::from(&mut call_context).cast(),
            memory: engine::MemoryContext {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
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
                let task_id = self.event_loop.next_task_id()?;
                self.enqueue_task(world, task_id, continuation, value)?;

                let output = self.run_event_loop(
                    world,
                    shared,
                    runtime_static,
                    host,
                    host_queue,
                    Some(task_id),
                    None,
                    poller,
                )?;
                let Some(output) = output else {
                    return Err(RuntimeError::event_loop_idle(task_id.get()).boxed());
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
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
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
                self.tick_loop(world, shared, runtime_static, host, host_queue, target_task)?;
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
                && self.wait_for_next_turn(
                    world,
                    host,
                    host_queue,
                    poller,
                    remaining_timeout_nanos,
                )?
            {
                continue;
            }

            // exit when the target task cannot make further progress
            if !progressed {
                let Some(target_task) = target_task else {
                    return Ok(None);
                };

                return Err(RuntimeError::event_loop_idle(target_task.get()).boxed());
            }
        }
    }

    /// Execute one local worker tick.
    pub(crate) fn tick(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<bool> {
        // run one event loop tick and capture progress
        let (progressed, _) =
            self.tick_loop(world, shared, runtime_static, host, host_queue, None)?;
        let mut progressed = progressed;

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
    fn collect_at_safepoint(&mut self, shared: &RuntimeHeap, is_idle: bool) -> RuntimeResult<bool> {
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
    fn collect_shared_priority_step(&mut self, shared: &RuntimeHeap) -> RuntimeResult<bool> {
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
    fn collect_local_priority_step(&mut self, shared: &RuntimeHeap) -> RuntimeResult<bool> {
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
    fn assist_shared_root_scan(&mut self, shared: &RuntimeHeap) -> RuntimeResult<bool> {
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
    fn assist_shared_edge_scan(&mut self, shared: &RuntimeHeap) -> RuntimeResult<bool> {
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
    fn assist_shared_gc(&mut self, runtime_shared: &RuntimeHeap) -> RuntimeResult<bool> {
        let shared = runtime_shared.shared.as_ref();
        let budget_bytes = shared.take_assist_budget_bytes();
        if budget_bytes == 0 || shared.gc_phase() == heap::GcPhase::Idle {
            return Ok(false);
        }

        let shared_roots = runtime_shared.roots();
        let roots = shared_roots.roots_snapshot();
        let roots_complete = shared_roots.roots_complete();
        let progress = shared
            .collect_step_for_worker(
                Some(&self.shared_gc_worker),
                roots.as_ref(),
                roots_complete,
                budget_bytes,
                runtime_shared.trace_table(),
            )
            .map_err(Box::<RuntimeError>::from)?;

        Ok(progress.made_progress())
    }

    /// Tick the loop once and return a target task output when requested.
    #[inline(never)]
    fn tick_loop(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        statics: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<(bool, Option<engine::Value>)> {
        // track whether this tick processed any event loop work
        let mut progressed = false;
        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let drained = self.drain_microtasks(world, shared, statics, host, host_queue)?;
            if drained > 0 {
                progressed = true;
            }
        }

        // run one queued macrotask before pulling external wakes
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_dequeued_task(
                world,
                shared,
                statics,
                host,
                host_queue,
                task,
                target_task,
            )? {
                return Ok((true, Some(output)));
            }

            return Ok((true, None));
        }

        // dispatch one wake into the task queue
        let wall_now = Nanos::new(world.wall_nanos());
        let mono_now = Nanos::new(world.mono_nanos());
        if let Some(wake) = self.event_loop.next_wake(wall_now, mono_now)? {
            progressed = true;
            if matches!(&wake, Wake::Timer(_)) {
                self.scenario.on_timer_fire(world)?;
            }
            if let Some(task) = self.event_loop.task_for_wake(wake, &mut self.engine)? {
                self.enqueue_prepared_task(world, task)?;
            }
        }

        // run one task produced by the dispatched wake
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_dequeued_task(
                world,
                shared,
                statics,
                host,
                host_queue,
                task,
                target_task,
            )? {
                return Ok((true, Some(output)));
            }

            return Ok((true, None));
        }

        Ok((progressed, None))
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

    /// Execute one dequeued task and return output when it completes the target task.
    fn execute_dequeued_task(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        task: Task,
        target_task: Option<TaskId>,
    ) -> RuntimeResult<Option<engine::Value>> {
        self.scenario.on_task_start(world)?;
        let _guard = enter_runnable_scope(RunnableScope::for_task(task.id));
        let outcome = self.execute_runnable(
            world,
            shared,
            runtime_static,
            host,
            host_queue,
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

        self.drain_microtasks(world, shared, runtime_static, host, host_queue)?;

        Ok(None)
    }

    /// Enqueue one prepared task and record scenario events.
    fn enqueue_prepared_task(&mut self, world: &mut WorldState, task: Task) -> RuntimeResult<()> {
        // enqueue the task into the event loop
        self.event_loop.enqueue_task(task);
        self.scenario.on_task_ready(world)?;

        Ok(())
    }

    /// Execute one microtask to completion.
    fn execute_microtask(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        microtask: Microtask,
        max_microtask_depth: usize,
    ) -> RuntimeResult<()> {
        // enforce true microtask nesting depth
        let parent_scope = current_runnable_scope();
        let next_depth = parent_scope
            .microtask_depth()
            .checked_add(1)
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: "microtask depth space exhausted".to_string(),
                }
                .boxed()
            })?;
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
            host_queue,
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

    /// Drain all pending microtasks.
    fn drain_microtasks(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<usize> {
        // drain microtasks until the queue is exhausted
        let mut num_drained_microtasks = 0usize;
        loop {
            let Some(microtask) = self.event_loop.pop_microtask() else {
                break;
            };
            self.scenario.on_task_start(world)?;
            self.execute_microtask(
                world,
                shared,
                runtime_static,
                host,
                host_queue,
                microtask,
                DEFAULT_MAX_MICROTASK_DEPTH,
            )?;
            num_drained_microtasks = num_drained_microtasks.checked_add(1).ok_or_else(|| {
                RuntimeError::Internal {
                    message: "microtask drain counter space exhausted".to_string(),
                }
                .boxed()
            })?;
        }

        Ok(num_drained_microtasks)
    }

    /// Resume one engine continuation with one runtime value.
    fn execute_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        runtime_static: &engine::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        let mut call_context = self.binding_call_context(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_gc_worker,
            statics,
            engine,
            ..
        } = self;
        let context = engine::CallContext {
            runtime: NonNull::from(&mut call_context).cast(),
            memory: engine::MemoryContext {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
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
        host: &dyn Host,
        host_queue: &HostQueue,
        poller: &mut dyn HostPoller,
        remaining_timeout_nanos: Option<u64>,
    ) -> RuntimeResult<bool> {
        // virtual mode never blocks: callers must advance virtual time explicitly
        if world.clock.source() == ClockSource::Runtime {
            return Ok(false);
        }

        // compute one timeout from the next scheduled timer deadline
        let (wall_now, mono_now) = match world.clock.source() {
            ClockSource::Host => (Nanos::new(host.wall_nanos()), Nanos::new(host.mono_nanos())),
            ClockSource::Runtime => (world.wall(), world.mono()),
        };
        let timeout_nanos = self.event_loop.timeout_until_next_timer(wall_now, mono_now);
        let timeout_nanos = match (timeout_nanos, remaining_timeout_nanos.map(Nanos::new)) {
            (Some(timer_timeout), Some(loop_timeout)) => Some(timer_timeout.min(loop_timeout)),
            (Some(timer_timeout), None) => Some(timer_timeout),
            (None, Some(loop_timeout)) => Some(loop_timeout),
            (None, None) => None,
        };

        // drain host ingress before blocking or sleeping
        let host_event_count = self.drain_host_wakes(host, host_queue, Some(0))?;
        if host_event_count > 0 {
            for _ in 0..host_event_count {
                self.scenario.on_ingress_ready(world)?;
            }

            return Ok(true);
        }

        // block on poller work and timer wakeups together
        let event_count = self
            .event_loop
            .poll_poller(poller, timeout_nanos.map(|timeout| timeout.get()))?;
        if event_count > 0 {
            for _ in 0..event_count {
                self.scenario.on_ingress_ready(world)?;
            }

            return Ok(true);
        }

        Ok(timeout_nanos.is_some())
    }

    /// Poll host events and enqueue host wakes.
    fn drain_host_wakes(
        &mut self,
        host: &dyn Host,
        host_queue: &HostQueue,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        // drain host events
        let poll_result = poll_host_events(host, host_queue, timeout_nanos)?;
        let host_events = poll_result.events;

        // enqueue host wakes
        let host_event_count = host_events.len();
        if !host_events.is_empty() {
            self.event_loop.enqueue_host_wakes(host_events);
        }

        Ok(host_event_count)
    }
}
