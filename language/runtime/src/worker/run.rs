use std::sync::Arc;

use super::{Activation, RunnableProgress, RunnableScope, Worker};
use crate::diagnostic::{MachineError, RuntimeError, RuntimeResult};
use crate::host::{Host, HostQueue};
use crate::machine::Entry;
use crate::runtime::SharedHeap;
use crate::worker::scheduler::{Invocation, Runnable, RunnableId, StoppedRunnable};
use crate::world::WorldState;
use crate::world::time::Nanos;
use destack_heap as heap;
use destack_program as program;
use program::{Outcome, Value};

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

/// Machine outcome and the task settled by its terminal state.
struct InvocationOutcome {
    /// Task settled by this execution when present.
    task: Option<program::Task>,
    /// Outcome produced by the worker machine.
    outcome: Outcome<Value>,
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

    /// Run one entrypoint through this worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        self.refresh_debugger(world);

        // execute the entrypoint
        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            RunnableScope::empty(),
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };
        let outcome = self.machine.run(
            activation,
            entry,
            args,
            Some(&self.stop_points),
            Some(&self.watch_points),
            self.profile.as_mut(),
        )?;

        // handle the entry outcome
        let output = match outcome {
            Outcome::Completed { value } => value,
            Outcome::Cancelled => {
                return Err(RuntimeError::Internal {
                    message: "entry execution completed through cancellation".to_string(),
                }
                .boxed());
            }
            Outcome::Stopped { .. } => {
                self.machine.clear();

                return Err(RuntimeError::execution_stopped().boxed());
            }
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => {
                return Err(RuntimeError::machine(
                    self.machine.kind(),
                    MachineError::UnownedSuspension,
                )
                .boxed());
            }
        };

        Ok(output)
    }

    /// Run one pending microtask.
    pub(crate) fn run_microtask(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
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

        let outcome = self.select_task(
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
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
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
        shared: &Arc<SharedHeap>,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // active pass
        let Some(epoch) = shared.pending_root_epoch(self.id) else {
            return Ok(None);
        };

        // owner-local root publication
        let roots = self.collect_shared_roots()?;
        let work_bytes = roots.len() * std::mem::size_of::<heap::SharedHeapReference>();
        shared.replace_direct_roots(&self.program, epoch, self.id, roots);

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
        shared: &Arc<SharedHeap>,
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
        shared.push_edge_roots(&self.program, self.id, &roots);

        let is_idle = self.shared_edge_scan_idle();
        if is_idle {
            shared.leave_edge_scan(&self.program, self.id);
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
        runtime_shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        let shared = &runtime_shared.shared;
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
                self.program.trace_view(),
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
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<heap::GcAdvance> {
        let budget_bytes = self.heap.take_collection_budget_bytes();
        let event_loop = &mut self.event_loop;
        let local_static = &mut self.local_static;
        let machine = &mut self.machine;
        let program = &self.program;
        let mut roots = |visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>| {
            program
                .visit_static_root_slots(program::GlobalLocation::LocalStatic, local_static, visit)
                .map_err(Box::<RuntimeError>::from)?;
            event_loop.visit_root_slots(program, visit)?;
            machine.visit_root_slots(visit)?;
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
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        drop: heap::GcDrop,
    ) -> RuntimeResult<()> {
        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            RunnableScope::empty(),
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };

        self.machine.drop_value(activation, drop)
    }

    /// Select and run one queued task or wake.
    #[inline(never)]
    fn select_task(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // run one queued task before pulling external wakes
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
        if let Some(wake) = self.event_loop.next_wake(wall_now, mono_now)? {
            self.event_loop.dispatch(wake);
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

    /// Run one task runnable and retain stopped execution.
    fn run_task_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let Runnable { id, invocation } = runnable;
        let scope = RunnableScope::task(id);
        let outcome = self.execute_invocation(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            invocation,
            scope,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Run one microtask runnable and retain stopped execution.
    fn run_microtask_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // run the microtask runnable
        let Runnable { id, invocation } = runnable;
        let scope = RunnableScope::microtask(id);
        let outcome = self.execute_invocation(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            invocation,
            scope,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Resume one stopped runnable under its retained runnable scope.
    fn execute_stopped_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        stop: StoppedRunnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let scope = stop.scope;
        let id = stop.id;
        let task = stop.task;
        let outcome = self.continue_runnable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            scope,
            task,
            stop.reason,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Handle one run outcome and retain stopped runnable state.
    fn handle_run_outcome(
        &mut self,
        id: RunnableId,
        scope: RunnableScope,
        invocation: InvocationOutcome,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let InvocationOutcome { task, outcome } = invocation;

        match outcome {
            Outcome::Completed { value } => {
                if let Some(task) = task {
                    self.event_loop
                        .finish_task(task, program::TaskOutcome::Completed(value))?;
                } else {
                    self.event_loop.release(value);
                }
                let Some(progress) = scope.progress() else {
                    return Err(RuntimeError::Internal {
                        message: "runnable completed without an active runnable scope".to_string(),
                    }
                    .boxed());
                };

                Ok(WorkerRunOutcome::Progressed { progress })
            }
            Outcome::Cancelled => {
                if let Some(task) = task {
                    self.event_loop
                        .finish_task(task, program::TaskOutcome::Cancelled)?;
                }
                let Some(progress) = scope.progress() else {
                    return Err(RuntimeError::Internal {
                        message: "runnable completed without an active runnable scope".to_string(),
                    }
                    .boxed());
                };

                Ok(WorkerRunOutcome::Progressed { progress })
            }
            Outcome::Stopped { reason } => {
                self.stop = Some(StoppedRunnable::new(id, scope, task, reason));

                Ok(WorkerRunOutcome::Stopped { reason })
            }
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => Err(RuntimeError::machine(
                self.machine.kind(),
                MachineError::UnownedSuspension,
            )
            .boxed()),
        }
    }

    /// Execute one queued program invocation.
    fn execute_invocation(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        invocation: Invocation,
        scope: RunnableScope,
    ) -> RuntimeResult<InvocationOutcome> {
        self.refresh_debugger(world);

        // begin one queued task continuation before entering the machine
        let task = invocation.task();
        let is_cancelled = if let Some(task) = task {
            self.event_loop.begin_task(task)?
        } else {
            false
        };

        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            scope,
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };

        let outcome = match invocation {
            Invocation::Function {
                function,
                environment,
                arguments,
            } => self.machine.run_function(
                activation,
                function,
                environment.as_ref(),
                &arguments,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
            Invocation::Resume {
                continuation,
                value,
                ..
            } => {
                if is_cancelled {
                    self.machine.cancel(
                        activation,
                        continuation,
                        Some(&self.stop_points),
                        Some(&self.watch_points),
                        self.profile.as_mut(),
                    )
                } else {
                    self.machine.resume(
                        activation,
                        continuation,
                        &value,
                        Some(&self.stop_points),
                        Some(&self.watch_points),
                        self.profile.as_mut(),
                    )
                }
            }
            Invocation::Complete {
                continuation,
                value,
            } => self.machine.complete(
                activation,
                continuation,
                &value,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
            Invocation::Cancel { continuation, .. } => self.machine.cancel(
                activation,
                continuation,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
        }?;

        // park asynchronous execution through its concrete Awaitable implementation
        let Outcome::Awaited {
            park,
            awaitable,
            continuation,
        } = outcome
        else {
            return Ok(InvocationOutcome { task, outcome });
        };
        self.park_awaitable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            scope,
            task,
            park,
            awaitable,
            continuation,
        )
    }

    /// Continue the physical execution retained by this worker machine.
    fn continue_runnable(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        scope: RunnableScope,
        task: Option<program::Task>,
        stop_reason: program::StopReason,
    ) -> RuntimeResult<InvocationOutcome> {
        self.refresh_debugger(world);

        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            scope,
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };

        let resume_skip = stop_reason.resume_skip();

        let outcome = self.machine.continue_execution(
            activation,
            Some(&self.stop_points),
            Some(&self.watch_points),
            self.profile.as_mut(),
            resume_skip,
        )?;

        let Outcome::Awaited {
            park,
            awaitable,
            continuation,
        } = outcome
        else {
            return Ok(InvocationOutcome { task, outcome });
        };
        self.park_awaitable(
            world,
            shared,
            shared_static,
            constant_space,
            host,
            host_queue,
            scope,
            task,
            park,
            awaitable,
            continuation,
        )
    }

    /// Park one suspended continuation through its concrete Awaitable implementation.
    fn park_awaitable(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        scope: RunnableScope,
        task: Option<program::Task>,
        park: program::FunctionId,
        awaitable: Value,
        continuation: program::Continuation,
    ) -> RuntimeResult<InvocationOutcome> {
        let waiter = if let Some(task) = task {
            self.event_loop.suspend_task(task, continuation)?
        } else {
            self.event_loop.park(continuation)
        };
        let arguments = Self::park_arguments(self.machine.program(), park, awaitable, waiter)?;

        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            scope,
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };
        let outcome = self.machine.run_function(
            activation,
            park,
            None,
            &arguments,
            Some(&self.stop_points),
            Some(&self.watch_points),
            self.profile.as_mut(),
        )?;

        // awaitable park is synchronous and never settles the suspended task
        match outcome {
            Outcome::Completed { .. } | Outcome::Stopped { .. } => Ok(InvocationOutcome {
                task: None,
                outcome,
            }),
            Outcome::Cancelled => Err(RuntimeError::Internal {
                message: "awaitable park completed through cancellation".to_string(),
            }
            .boxed()),
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => Err(RuntimeError::machine(
                self.machine.kind(),
                MachineError::UnownedSuspension,
            )
            .boxed()),
        }
    }

    /// Build the exact arguments for one concrete Awaitable park call.
    fn park_arguments(
        program: &program::Program,
        park: program::FunctionId,
        awaitable: Value,
        waiter: program::Waiter,
    ) -> RuntimeResult<[Value; 2]> {
        let waiter_type = program
            .function_parameters(park)
            .and_then(|parameters| parameters.get(1))
            .copied()
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("awaitable park {} has no waiter parameter", park.index()),
                }
                .boxed()
            })?;
        let waiter = program
            .value(waiter_type, [program::Word::from_bits(waiter.bits())])
            .map_err(Box::<RuntimeError>::from)?;

        Ok([awaitable, waiter])
    }

    /// Publish root changes and donate GC work after one bounded run.
    fn finish_run(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        outcome: WorkerRunOutcome,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // destroy released values only after physical execution has completed
        if matches!(outcome, WorkerRunOutcome::Progressed { .. }) {
            self.destroy_released_values(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
        }

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

    /// Destroy every value released by completed scheduler transitions.
    fn destroy_released_values(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<()> {
        while let Some(value) = self.event_loop.pop_drop() {
            self.destroy_value(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
                value,
            )?;
        }

        Ok(())
    }

    /// Destroy one scheduler-owned runtime value.
    fn destroy_value(
        &mut self,
        world: &mut WorldState,
        shared: &Arc<SharedHeap>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticImage,
        host: &dyn Host,
        host_queue: &HostQueue,
        value: program::Value,
    ) -> RuntimeResult<()> {
        let mut activation = Activation::new(
            self.runtime_id,
            self.id,
            self.environment.as_ref(),
            self.conditions.as_ref(),
            self.program.as_ref(),
            self.diagnostics.as_ref(),
            &self.binding_access,
            self.binding_table.as_ref(),
            host,
            host_queue,
            world,
            RunnableScope::empty(),
            &mut self.event_loop,
        );
        let activation = program::Activation {
            runtime: &mut activation,
            memory: program::Memory {
                allocation_plans: shared.allocation_plans(),
                heap: &mut self.heap,
                shared_heap: &shared.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static,
                constant_space,
            },
        };

        self.machine.destroy_value(activation, value)
    }
}
