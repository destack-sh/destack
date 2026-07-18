use std::ptr::NonNull;

use super::{
    BindingCall, RunnableProgress, RunnableScope, Worker, current_runnable_scope,
    enter_runnable_scope,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Host;
use crate::host::core::{HostQueue, poll_host_events};
use crate::host::poller::HostPoller;
use crate::runtime::RuntimeHeap;
use crate::runtime::machine::{Continuation, Entry, Outcome};
use crate::runtime::scheduler::{Runnable, RunnableId, StoppedRunnable};
use crate::runtime::time::{ClockSource, Nanos};
use crate::world::WorldState;
use destack_heap as heap;
use destack_program as program;

/// The default maximum nested microtask depth.
const DEFAULT_MAX_MICROTASK_DEPTH: usize = usize::MAX;

/// Outcome from one bounded worker run operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerRunOutcome {
    /// One task or microtask made progress.
    Progressed {
        /// Work that made progress.
        progress: RunnableProgress,
    },
    /// No work was runnable.
    Idle,
    /// Execution stopped at one runtime stop point.
    Stopped {
        /// Reason execution stopped.
        reason: program::StopReason,
    },
    /// Execution is paused at one previously reached stop point.
    Paused {
        /// Reason execution stopped.
        reason: program::StopReason,
    },
}

impl Worker {
    /// Refresh derived debug sets when debugger configuration changed.
    fn refresh_debugger(&mut self, world: &WorldState) {
        let generation = world.debugger.generation();
        if self.debug_generation == generation {
            return;
        }

        self.stop_points = world.debugger.stop_set(self.runtime_id, self.id);
        self.watch_points = world.debugger.watch_set(self.runtime_id, self.id);
        self.debug_generation = generation;
    }

    /// Build one runtime-owned binding call.
    fn binding_call<'host>(
        &mut self,
        world: &mut WorldState,
        host: &'host dyn Host,
        host_queue: &'host HostQueue,
    ) -> BindingCall<'host> {
        BindingCall {
            runtime_id: self.runtime_id,
            worker_id: self.id,
            environment: self.environment.clone(),
            conditions: self.conditions.clone(),
            diagnostics: self.diagnostics.clone(),
            binding_table: &self.binding_table,
            host,
            host_queue,
            world,
            scope: current_runnable_scope(),
            is_process_main: host.is_process_main_context(),
        }
    }

    /// Run one entrypoint through this worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        entry: &Entry,
        args: &[program::Value],
        poller: &mut dyn HostPoller,
    ) -> RuntimeResult<program::Value> {
        self.refresh_debugger(world);

        // execute the entrypoint with yielding enabled
        let _guard = enter_runnable_scope(RunnableScope::empty());
        let mut call_context = self.binding_call(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            machine,
            stop_points,
            watch_points,
            profile,
            ..
        } = self;
        let context = program::ProgramActivation {
            state: NonNull::from(&mut call_context).cast(),
            storage: program::ProgramStorage {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
                shared_mark_worker,
                local_static,
                shared_static,
                constant_space,
            },
        };
        let outcome = machine.run(
            context,
            entry,
            args,
            Some(stop_points),
            Some(watch_points),
            profile.as_mut(),
        )?;

        // handle the entry outcome
        let output = match outcome {
            Outcome::Completed { value } => value,
            Outcome::Yielded {
                continuation,
                value,
            } => {
                let task_id = self.event_loop.next_runnable_id()?;
                self.enqueue_task(task_id, continuation, value)?;

                let output = self.run_event_loop(
                    world,
                    shared,
                    shared_static,
                    constant_space,
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
            Outcome::Stopped { .. } => return Err(RuntimeError::execution_stopped().boxed()),
        };

        Ok(output)
    }

    /// Run the event loop until idle, timeout, or the target task completes.
    pub(crate) fn run_event_loop(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        target_task: Option<RunnableId>,
        timeout_nanos: Option<u64>,
        poller: &mut dyn HostPoller,
    ) -> RuntimeResult<Option<program::Value>> {
        self.reject_stopped_execution()?;

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

            // run one loop tick for the machine
            let (progressed, output) = self.tick_loop(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                target_task,
            )?;
            if let Some(output) = output {
                return Ok(Some(output));
            }

            // direct roots: worker execution may reshuffle shared roots
            if progressed && shared.is_marking() {
                shared.queue_root_scan(self.id);
            }

            // donate GC work before sleeping or declaring idle
            let safepoint_progressed = self.collect_at_safepoint(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                true,
            )?;
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

    /// Run one pending microtask.
    pub(crate) fn run_microtask(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        if let Some(stop) = &self.stop {
            return Ok(WorkerRunOutcome::Paused {
                reason: stop.reason,
            });
        }

        let Some(microtask) = self.event_loop.pop_microtask() else {
            return Ok(WorkerRunOutcome::Idle);
        };

        // run one microtask to completion
        let outcome = self.run_microtask_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            microtask,
            DEFAULT_MAX_MICROTASK_DEPTH,
        )?;

        self.finish_run(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            outcome,
        )
    }

    /// Continue this worker from a retained runtime stop point.
    pub(crate) fn continue_stop(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let Some(stop) = self.stop.take() else {
            return Ok(WorkerRunOutcome::Idle);
        };

        let outcome = self.execute_stopped_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            stop,
        )?;

        self.finish_run(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            outcome,
        )
    }

    /// Run one task or scheduler event and retain runtime stop points.
    pub(crate) fn run_task(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        if let Some(stop) = &self.stop {
            return Ok(WorkerRunOutcome::Paused {
                reason: stop.reason,
            });
        }

        let outcome = self.run_loop(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )?;

        self.finish_run(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            outcome,
        )
    }

    /// Run cooperative GC work at one worker safepoint.
    fn collect_at_safepoint(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        is_idle: bool,
    ) -> RuntimeResult<bool> {
        let prioritize_shared = shared.is_terminating()
            || shared.pending_root_epoch(self.id).is_some()
            || (shared.is_marking() && !self.shared_edge_scan_idle());
        let should_drain = is_idle || shared.is_terminating();
        let mut progressed = false;

        // cooperative GC work
        loop {
            let advance = if prioritize_shared {
                self.step_gc_with_shared_priority(
                    world,
                    shared,
                    shared_static,
                    constant_space,
                    host,
                    host_queue,
                )?
            } else {
                self.step_gc_with_local_priority(
                    world,
                    shared,
                    shared_static,
                    constant_space,
                    host,
                    host_queue,
                )?
            };

            if advance.is_none() {
                break;
            }

            progressed = true;
            if !should_drain {
                break;
            }
        }

        Ok(progressed)
    }

    /// Run cooperative GC work at one idle worker safepoint.
    pub(crate) fn run_safepoint(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        let prioritize_shared = shared.is_terminating()
            || shared.pending_root_epoch(self.id).is_some()
            || (shared.is_marking() && !self.shared_edge_scan_idle());

        if prioritize_shared {
            self.step_gc_with_shared_priority(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )
        } else {
            self.step_gc_with_local_priority(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )
        }
    }

    /// Run one GC safepoint step with shared heap work first.
    fn step_gc_with_shared_priority(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // direct shared roots
        if let Some(progress) = self.assist_shared_root_scan(shared)? {
            return Ok(Some(progress));
        }

        // local to shared edges
        if let Some(progress) = self.assist_shared_edge_scan(shared)? {
            return Ok(Some(progress));
        }

        // shared mark and sweep work
        if let Some(progress) = self.assist_shared_gc(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )? {
            return Ok(Some(progress));
        }

        // local heap work
        let progress = self.step_local_collection(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )?;
        if progress.advanced() {
            return Ok(Some(progress));
        }

        Ok(None)
    }

    /// Run one GC safepoint step with local heap work first.
    fn step_gc_with_local_priority(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // local heap work
        let progress = self.step_local_collection(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )?;
        if progress.advanced() {
            return Ok(Some(progress));
        }

        // direct shared roots
        if let Some(progress) = self.assist_shared_root_scan(shared)? {
            return Ok(Some(progress));
        }

        // local to shared edges
        if let Some(progress) = self.assist_shared_edge_scan(shared)? {
            return Ok(Some(progress));
        }

        // shared mark and sweep work
        if let Some(progress) = self.assist_shared_gc(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )? {
            return Ok(Some(progress));
        }

        Ok(None)
    }

    /// Publish one pending direct shared-root scan from this worker safepoint.
    fn assist_shared_root_scan(
        &mut self,
        shared: &RuntimeHeap,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // active pass
        let Some(epoch) = shared.pending_root_epoch(self.id) else {
            return Ok(None);
        };

        // owner-local root publication
        let roots = self.collect_shared_roots()?;
        let work_bytes = roots.len() * std::mem::size_of::<heap::SharedHeapReference>();
        shared.replace_direct_roots(epoch, self.id, roots);

        Ok(Some(heap::GcAdvance::stepped(
            heap::GcCollector::Shared,
            heap::GcPhase::PublishRoots,
            0,
            work_bytes,
        )))
    }

    /// Assist one active shared reference pass from this worker safepoint.
    fn assist_shared_edge_scan(
        &mut self,
        shared: &RuntimeHeap,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        if !shared.is_marking() || self.shared_edge_scan_idle() {
            return Ok(None);
        }

        let work_bytes = shared.edge_scan_work_bytes();
        if work_bytes == 0 {
            return Ok(None);
        }

        let mut roots = Vec::new();
        let work_done = self.trace_shared_roots(&mut roots, work_bytes)?;
        shared.push_edge_roots(self.id, &roots);

        let is_idle = self.shared_edge_scan_idle();
        if is_idle {
            shared.leave_edge_scan(self.id);
        }

        if work_done > 0 || is_idle {
            Ok(Some(heap::GcAdvance::stepped(
                heap::GcCollector::Shared,
                heap::GcPhase::ScanEdges,
                work_bytes,
                work_done,
            )))
        } else {
            Ok(None)
        }
    }

    /// Assist one active shared collection from this worker safepoint.
    fn assist_shared_gc(
        &mut self,
        world: &mut WorldState,
        runtime_shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        let shared = runtime_shared.shared.as_ref();
        let budget_bytes = if matches!(
            shared.gc_phase(),
            heap::GcPhase::Drop | heap::GcPhase::Sweep
        ) {
            shared.take_collection_budget_bytes(1)
        } else {
            shared.take_assist_budget_bytes()
        };
        if budget_bytes == 0 || shared.gc_phase() == heap::GcPhase::Idle {
            return Ok(None);
        }

        let shared_roots = runtime_shared.roots();
        let roots = shared_roots.roots_snapshot();
        let roots_complete = shared_roots.roots_complete();
        let progress = shared
            .step_collection_for_worker(
                Some(&self.shared_mark_worker),
                roots.as_ref(),
                roots_complete,
                budget_bytes,
                runtime_shared.program().trace_view(),
            )
            .map_err(Box::<RuntimeError>::from)?;

        if let heap::GcAdvance::Drop(drop) = progress {
            self.drop_value(
                world,
                runtime_shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                drop,
            )?;
            shared
                .complete_drop(drop.reference)
                .map_err(Box::<RuntimeError>::from)?;
        }

        if progress.advanced() {
            Ok(Some(progress))
        } else {
            Ok(None)
        }
    }

    /// Run one budgeted local collection step and any selected Drop callback.
    fn step_local_collection(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<heap::GcAdvance> {
        let budget_bytes = self.heap.take_collection_budget_bytes();
        let machine = &mut self.machine;
        let event_loop = &mut self.event_loop;
        let local_static = &mut self.local_static;
        let mut roots = |visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>| {
            machine.visit_root_slots(local_static, visit)?;
            event_loop.visit_root_slots(machine, visit)?;
            if let Some(stop) = &mut self.stop {
                stop.visit_root_slots(machine, visit)?;
            }

            Ok::<(), Box<RuntimeError>>(())
        };
        let progress =
            self.heap
                .step_collection(&mut roots, budget_bytes, self.program.trace_view())?;

        if let heap::GcAdvance::Drop(drop) = progress {
            self.drop_value(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                drop,
            )?;
            self.heap
                .complete_drop(drop.reference)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(progress)
    }

    /// Destroy one GC-selected value.
    fn drop_value(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        drop: heap::GcDrop,
    ) -> RuntimeResult<()> {
        let mut call_context = self.binding_call(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            machine,
            ..
        } = self;
        let context = program::ProgramActivation {
            state: NonNull::from(&mut call_context).cast(),
            storage: program::ProgramStorage {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
                shared_mark_worker,
                local_static,
                shared_static,
                constant_space,
            },
        };

        machine.drop_value(context, drop)
    }

    /// Tick the loop once and return a target task output when requested.
    #[inline(never)]
    fn tick_loop(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        target_task: Option<RunnableId>,
    ) -> RuntimeResult<(bool, Option<program::Value>)> {
        // track whether this tick processed any event loop work
        let mut progressed = false;
        // drain microtasks before selecting other work
        if self.event_loop.has_microtasks() {
            let drained = self.drain_microtasks(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            if drained > 0 {
                progressed = true;
            }
        }

        // run one queued macrotask before pulling external wakes
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_task_runnable(
                world,
                shared,
                shared_static,
                constant_space,
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
            if let Some(runnable) = self.event_loop.runnable_for_wake(wake)? {
                self.enqueue_task_runnable(runnable)?;
            }
        }

        // run one task produced by the dispatched wake
        if let Some(task) = self.event_loop.pop_task() {
            if let Some(output) = self.execute_task_runnable(
                world,
                shared,
                shared_static,
                constant_space,
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

    /// Run the loop once and retain stopped task state.
    #[inline(never)]
    fn run_loop(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // run one microtask before selecting other work
        if let Some(microtask) = self.event_loop.pop_microtask() {
            return self.run_microtask_runnable(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                microtask,
                DEFAULT_MAX_MICROTASK_DEPTH,
            );
        }

        // run one queued macrotask before pulling external wakes
        if let Some(task) = self.event_loop.pop_task() {
            return self.run_task_runnable(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                task,
            );
        }

        // dispatch one wake into the task queue
        let wall_now = Nanos::new(world.wall_nanos());
        let mono_now = Nanos::new(world.mono_nanos());
        if let Some(wake) = self.event_loop.next_wake(wall_now, mono_now)?
            && let Some(runnable) = self.event_loop.runnable_for_wake(wake)?
        {
            self.enqueue_task_runnable(runnable)?;
        }

        // run one task produced by the dispatched wake
        if let Some(task) = self.event_loop.pop_task() {
            return self.run_task_runnable(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                task,
            );
        }

        Ok(WorkerRunOutcome::Idle)
    }

    /// Enqueue one yielded continuation as a task.
    fn enqueue_task(
        &mut self,
        task_id: RunnableId,
        runnable: Continuation,
        resume_value: program::Value,
    ) -> RuntimeResult<()> {
        // build the runnable
        let runnable = Runnable {
            id: task_id,
            continuation: runnable,
            resume_value,
        };

        self.enqueue_task_runnable(runnable)
    }

    /// Execute one task runnable and return output when it completes the target task.
    fn execute_task_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
        target_task: Option<RunnableId>,
    ) -> RuntimeResult<Option<program::Value>> {
        let id = runnable.id;
        let _guard = enter_runnable_scope(RunnableScope::for_task(id));
        let outcome = self.execute_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            runnable.continuation,
            runnable.resume_value,
        )?;

        // handle the task outcome
        match outcome {
            Outcome::Completed { value } => {
                if target_task == Some(id) {
                    return Ok(Some(value));
                }
            }
            Outcome::Yielded {
                continuation,
                value,
            } => {
                self.enqueue_task(id, continuation, value)?;
            }
            Outcome::Stopped { .. } => return Err(RuntimeError::execution_stopped().boxed()),
        }

        self.drain_microtasks(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
        )?;

        Ok(None)
    }

    /// Run one task runnable and retain stopped continuations.
    fn run_task_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let id = runnable.id;
        let scope = RunnableScope::for_task(id);
        let _guard = enter_runnable_scope(scope);
        let outcome = self.execute_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            runnable.continuation,
            runnable.resume_value,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Enqueue one runnable as a macrotask.
    fn enqueue_task_runnable(&mut self, runnable: Runnable) -> RuntimeResult<()> {
        // enqueue the task into the event loop
        self.event_loop.enqueue_task(runnable);

        Ok(())
    }

    /// Execute one microtask runnable to completion.
    fn execute_microtask_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
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
        let _guard = enter_runnable_scope(RunnableScope::for_microtask(runnable.id, next_depth));
        let outcome = self.execute_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            runnable.continuation,
            runnable.resume_value,
        )?;

        // ensure microtasks run to completion
        match outcome {
            Outcome::Completed { .. } => Ok(()),
            Outcome::Yielded { .. } => Err(RuntimeError::Internal {
                message: "microtask yielded while running to completion".to_string(),
            }
            .boxed()),
            Outcome::Stopped { .. } => Err(RuntimeError::execution_stopped().boxed()),
        }
    }

    /// Run one microtask runnable and retain stopped continuations.
    fn run_microtask_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
        max_microtask_depth: usize,
    ) -> RuntimeResult<WorkerRunOutcome> {
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
        let scope = RunnableScope::for_microtask(runnable.id, next_depth);
        let _guard = enter_runnable_scope(scope);
        let outcome = self.execute_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            runnable.continuation,
            runnable.resume_value,
        )?;

        self.handle_run_outcome(runnable.id, scope, outcome)
    }

    /// Drain all pending microtasks.
    fn drain_microtasks(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<usize> {
        // drain microtasks until the queue is exhausted
        let mut num_drained_microtasks = 0usize;
        while let Some(microtask) = self.event_loop.pop_microtask() {
            self.execute_microtask_runnable(
                world,
                shared,
                shared_static,
                constant_space,
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

    /// Resume one stopped runnable under its retained runnable scope.
    fn execute_stopped_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        stop: StoppedRunnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let scope = stop.scope;
        let id = stop.id;
        let _guard = enter_runnable_scope(scope);
        let outcome = self.continue_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            stop.continuation,
            stop.reason,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Handle one run outcome and retain stopped runnable state.
    fn handle_run_outcome(
        &mut self,
        id: RunnableId,
        scope: RunnableScope,
        outcome: Outcome<Continuation>,
    ) -> RuntimeResult<WorkerRunOutcome> {
        match outcome {
            Outcome::Completed { .. } => {
                let Some(progress) = scope.progress() else {
                    return Err(RuntimeError::Internal {
                        message: "runnable completed without an active runnable scope".to_string(),
                    }
                    .boxed());
                };

                Ok(WorkerRunOutcome::Progressed { progress })
            }
            Outcome::Yielded {
                continuation,
                value,
            } => {
                if scope.microtask_id().is_some() {
                    return Err(RuntimeError::Internal {
                        message: "microtask yielded while running to completion".to_string(),
                    }
                    .boxed());
                }

                self.enqueue_task(id, continuation, value)?;

                let Some(progress) = scope.progress() else {
                    return Err(RuntimeError::Internal {
                        message: "runnable yielded without an active runnable scope".to_string(),
                    }
                    .boxed());
                };

                Ok(WorkerRunOutcome::Progressed { progress })
            }
            Outcome::Stopped {
                continuation,
                reason,
            } => {
                self.stop = Some(StoppedRunnable::new(id, continuation, scope, reason));

                Ok(WorkerRunOutcome::Stopped { reason })
            }
        }
    }

    /// Resume one machine continuation with one runtime value.
    fn execute_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Continuation,
        resume_value: program::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        self.refresh_debugger(world);

        let mut call_context = self.binding_call(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            machine,
            stop_points,
            watch_points,
            profile,
            ..
        } = self;
        let context = program::ProgramActivation {
            state: NonNull::from(&mut call_context).cast(),
            storage: program::ProgramStorage {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
                shared_mark_worker,
                local_static,
                shared_static,
                constant_space,
            },
        };

        machine.resume(
            context,
            runnable,
            resume_value,
            Some(stop_points),
            Some(watch_points),
            profile.as_mut(),
        )
    }

    /// Continue one stopped machine continuation.
    fn continue_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Continuation,
        stop_reason: program::StopReason,
    ) -> RuntimeResult<Outcome<Continuation>> {
        self.refresh_debugger(world);

        let mut call_context = self.binding_call(world, host, host_queue);
        let Worker {
            heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            machine,
            stop_points,
            watch_points,
            profile,
            ..
        } = self;
        let context = program::ProgramActivation {
            state: NonNull::from(&mut call_context).cast(),
            storage: program::ProgramStorage {
                heap,
                shared_heap: shared.shared.as_ref(),
                shared_cache,
                shared_mark_worker,
                local_static,
                shared_static,
                constant_space,
            },
        };

        let resume_skip = stop_reason.resume_skip();

        machine.continue_continuation(
            context,
            runnable,
            Some(stop_points),
            Some(watch_points),
            profile.as_mut(),
            resume_skip,
        )
    }

    /// Reject ordinary event-loop execution while this worker is stopped.
    fn reject_stopped_execution(&self) -> RuntimeResult<()> {
        if self.stop.is_some() {
            Err(RuntimeError::execution_stopped().boxed())
        } else {
            Ok(())
        }
    }

    /// Publish root changes and donate GC work after one bounded run.
    fn finish_run(
        &mut self,
        world: &mut WorldState,
        shared: &RuntimeHeap,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        outcome: WorkerRunOutcome,
    ) -> RuntimeResult<WorkerRunOutcome> {
        if outcome != WorkerRunOutcome::Idle {
            self.sequence = self.sequence.next()?;
        }

        if outcome != WorkerRunOutcome::Idle && shared.is_marking() {
            shared.queue_root_scan(self.id);
        }

        if matches!(outcome, WorkerRunOutcome::Progressed { .. }) {
            self.collect_at_safepoint(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                false,
            )?;
        }

        Ok(outcome)
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
            return Ok(true);
        }

        // block on poller work and timer wakeups together
        let event_count = self
            .event_loop
            .poll_poller(poller, timeout_nanos.map(|timeout| timeout.get()))?;
        if event_count > 0 {
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
