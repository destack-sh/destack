use std::sync::Arc;

use destack_heap as heap;
use destack_program as program;
use destack_vm as vm;
use program::{Outcome, Value};

use super::{Activation, Request, RunnableProgress, RunnableScope, Worker};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::SharedCollectionState;
use crate::host::{Host, HostQueue};
use crate::machine::Entry;
use crate::scheduler::{Invocation, RetainedRunnable, Runnable, RunnableId};
use crate::world::WorldState;
use crate::world::time::Nanos;

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
        /// Fiber that stopped.
        fiber_id: program::FiberId,
        /// Reason execution stopped.
        reason: program::StopReason,
    },
    /// Execution is paused at one previously reached stop point.
    Paused {
        /// Fiber retained at the stop.
        fiber_id: program::FiberId,
        /// Reason execution stopped.
        reason: program::StopReason,
    },
}

/// Machine outcome paired with the fiber that produced it.
struct FiberOutcome {
    /// Fiber identity in the scheduler table.
    fiber_id: program::FiberId,
    /// Physical execution owned while the fiber is mounted.
    execution: vm::Fiber,
    /// Outcome produced by the worker machine.
    outcome: Outcome<Value>,
}

/// Scheduler queue selected by one bounded worker run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunQueue {
    /// Select one task or external wake.
    Task,
    /// Select one queued microtask.
    Microtask,
}

/// What one worker run starts from.
enum EntryTarget<'a> {
    /// An entrypoint named by the caller.
    Entry(&'a Entry),
    /// A linked function selected by id.
    Function(program::FunctionId),
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
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        self.run_target(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
            EntryTarget::Entry(entry),
            args,
        )
    }

    /// Run one linked function through this worker event loop.
    pub(crate) fn run_function(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        function: program::FunctionId,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        self.run_target(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
            EntryTarget::Function(function),
            args,
        )
    }

    /// Run one entry target on a fresh fiber through this worker event loop.
    #[allow(clippy::too_many_arguments)]
    fn run_target(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        target: EntryTarget<'_>,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        self.refresh_debugger(world);

        // execute the entrypoint on one fresh fiber
        let fiber_id = self.event_loop.insert_fiber();
        let mut execution = self.machine.reserve_fiber()?;
        execution.mount(fiber_id);
        let result = self.drive_entry(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
            fiber_id,
            execution,
            target,
            args,
        );
        let retired = self.event_loop.retire_fiber(fiber_id);
        let value = result?;
        retired?;

        Ok(value)
    }

    /// Drive one entry fiber to completion, servicing runtime polls in place.
    #[allow(clippy::too_many_arguments)]
    fn drive_entry(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        fiber_id: program::FiberId,
        mut execution: vm::Fiber,
        target: EntryTarget<'_>,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        let mut context = program::Context::empty();
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
            Some(fiber_id),
            &mut self.event_loop,
            self.handshake.as_ref(),
        );
        let activation = program::Activation {
            runtime: &mut activation,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: self.allocation_plans.as_ref(),
                local_heap: &mut self.heap,
                shared_heap: self.shared_heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: shared_static,
                constants: constant_space,
                handshake: self.handshake.as_ref(),
            },
        };
        let outcome = match target {
            EntryTarget::Entry(entry) => self.machine.run(
                &mut execution,
                activation,
                entry,
                args,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
            EntryTarget::Function(function) => self.machine.run_function(
                &mut execution,
                activation,
                function,
                None,
                args,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
        };
        let mut outcome = outcome?;

        // service polls without interleaving another runnable
        loop {
            match outcome {
                Outcome::Completed { value } => return Ok(value),
                Outcome::Cancelled => {
                    return Err(RuntimeError::Internal {
                        message: "entry execution completed through cancellation".to_string(),
                    }
                    .boxed());
                }
                Outcome::Parked => {
                    return Err(RuntimeError::Internal {
                        message: "entry execution parked outside the event loop".to_string(),
                    }
                    .boxed());
                }
                Outcome::Stopped { .. } if self.handshake.is_pending() => {
                    // root the stopped fiber while runtime work runs
                    self.machine.retain_stopped(execution);
                    let reason = self.service_handshake(
                        world,
                        collection,
                        shared_static,
                        constant_space,
                        host,
                        host_queue,
                    )?;
                    execution = self.machine.take_stopped().ok_or_else(|| {
                        RuntimeError::Internal {
                            message: "runtime handshake lost the entry fiber".to_string(),
                        }
                        .boxed()
                    })?;
                    if reason.is_some() {
                        return Err(RuntimeError::execution_stopped().boxed());
                    }

                    let mut context = execution.context();
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
                        Some(fiber_id),
                        &mut self.event_loop,
                        self.handshake.as_ref(),
                    );
                    let activation = program::Activation {
                        runtime: &mut activation,
                        context: &mut context,
                        memory: program::Memory {
                            allocation_plans: self.allocation_plans.as_ref(),
                            local_heap: &mut self.heap,
                            shared_heap: self.shared_heap.as_ref(),
                            shared_cache: &mut self.shared_cache,
                            shared_mark_worker: &self.shared_mark_worker,
                            local_statics: &mut self.local_static,
                            shared_statics: shared_static,
                            constants: constant_space,
                            handshake: self.handshake.as_ref(),
                        },
                    };
                    let continued = self.machine.continue_execution(
                        &mut execution,
                        activation,
                        Some(&self.stop_points),
                        Some(&self.watch_points),
                        self.profile.as_mut(),
                        None,
                    );
                    outcome = continued?;
                }
                Outcome::Stopped { .. } => {
                    return Err(RuntimeError::execution_stopped().boxed());
                }
            }
        }
    }

    /// Run one pending microtask.
    pub(crate) fn run_microtask(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        self.run_queue(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
            RunQueue::Microtask,
        )
    }

    /// Resume this Worker from its retained debugger stop.
    pub(crate) fn resume(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let Some(retained) = self.retained.take() else {
            return Err(RuntimeError::worker_not_stopped(self.id.0).boxed());
        };
        let Some(reason) = retained.reason else {
            self.retained = Some(retained);

            return Err(RuntimeError::worker_not_stopped(self.id.0).boxed());
        };

        let outcome = self.execute_retained_runnable(
            world,
            shared_static,
            constant_space,
            host,
            host_queue,
            retained,
            Some(reason),
        )?;

        self.finish_run(
            world,
            collection,
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
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        self.run_queue(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
            RunQueue::Task,
        )
    }

    /// Run one scheduler queue without interleaving retained execution.
    #[allow(clippy::too_many_arguments)]
    fn run_queue(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        queue: RunQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        loop {
            // always resume retained execution before selecting another runnable
            let outcome = if let Some(retained) = self.retained.take() {
                if let Some(reason) = retained.reason {
                    let fiber_id = retained.fiber_id;
                    self.retained = Some(retained);

                    return Ok(WorkerRunOutcome::Paused { fiber_id, reason });
                }

                self.execute_retained_runnable(
                    world,
                    shared_static,
                    constant_space,
                    host,
                    host_queue,
                    retained,
                    None,
                )?
            } else {
                match queue {
                    RunQueue::Task => {
                        self.select_task(world, shared_static, constant_space, host, host_queue)?
                    }
                    RunQueue::Microtask => {
                        let Some(runnable) = self.event_loop.pop_microtask() else {
                            return Ok(WorkerRunOutcome::Idle);
                        };

                        self.run_microtask_runnable(
                            world,
                            shared_static,
                            constant_space,
                            host,
                            host_queue,
                            runnable,
                        )?
                    }
                }
            };
            let outcome = self.finish_run(
                world,
                collection,
                shared_static,
                constant_space,
                host,
                host_queue,
                outcome,
            )?;
            let is_handshake =
                matches!(outcome, WorkerRunOutcome::Stopped { .. }) && self.handshake.is_pending();
            if !is_handshake {
                return Ok(outcome);
            }

            // service runtime work before resuming the retained runnable
            let reason = self.service_handshake(
                world,
                collection,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            let retained = self.retained.as_mut().ok_or_else(|| {
                RuntimeError::Internal {
                    message: "runtime handshake lost its retained runnable".to_string(),
                }
                .boxed()
            })?;
            retained.reason = reason;
            if let Some(reason) = reason {
                return Ok(WorkerRunOutcome::Stopped {
                    fiber_id: retained.fiber_id,
                    reason,
                });
            }
        }
    }

    /// Advance cooperative GC work for one quiescent worker.
    fn advance_gc(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        is_idle: bool,
    ) -> RuntimeResult<bool> {
        let prioritize_shared = collection.is_terminating()
            || collection.pending_root_epoch(self.id).is_some()
            || (self.shared_heap.gc_phase() == heap::GcPhase::Mark
                && !self.shared_edge_scan_idle());
        let should_drain = is_idle || collection.is_terminating();
        let mut progressed = false;

        // cooperative GC work
        loop {
            let advance = if prioritize_shared {
                self.step_gc_with_shared_priority(
                    world,
                    collection,
                    shared_static,
                    constant_space,
                    host,
                    host_queue,
                )?
            } else {
                self.step_gc_with_local_priority(
                    world,
                    collection,
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

    /// Service every request consumed at one execution poll.
    fn service_handshake(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<program::StopReason>> {
        let requests = self.handshake.take();
        if requests.is_empty() {
            return Err(RuntimeError::Internal {
                message: "execution polled without a pending runtime request".to_string(),
            }
            .boxed());
        }
        if requests.contains(Request::Terminate) {
            self.terminate_execution()?;

            return Err(RuntimeError::execution_terminated().boxed());
        }

        // finish requested collection work before the runnable resumes
        if requests.contains(Request::Collect) {
            self.advance_gc(
                world,
                collection,
                shared_static,
                constant_space,
                host,
                host_queue,
                true,
            )?;
        }

        // retain inspection requests while deoptimization resumes through bytecode
        if requests.contains(Request::Pause) {
            let point = self.machine.activation_point().ok_or_else(|| {
                RuntimeError::Internal {
                    message: "inspection request has no retained program point".to_string(),
                }
                .boxed()
            })?;

            Ok(Some(program::StopReason::Pause { point }))
        } else {
            Ok(None)
        }
    }

    /// Terminate the retained runnable and release its machine state.
    fn terminate_execution(&mut self) -> RuntimeResult<()> {
        let retained = self.retained.take();
        self.machine.clear();
        if let Some(retained) = retained {
            self.event_loop.retire_fiber(retained.fiber_id)?;
        }

        Ok(())
    }

    /// Advance one cooperative GC operation for an idle worker.
    pub(crate) fn advance_gc_once(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        let prioritize_shared = collection.is_terminating()
            || collection.pending_root_epoch(self.id).is_some()
            || (self.shared_heap.gc_phase() == heap::GcPhase::Mark
                && !self.shared_edge_scan_idle());

        if prioritize_shared {
            self.step_gc_with_shared_priority(
                world,
                collection,
                shared_static,
                constant_space,
                host,
                host_queue,
            )
        } else {
            self.step_gc_with_local_priority(
                world,
                collection,
                shared_static,
                constant_space,
                host,
                host_queue,
            )
        }
    }

    /// Advance one GC operation with shared heap work first.
    fn step_gc_with_shared_priority(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // direct shared roots
        if let Some(progress) = self.assist_shared_root_scan(collection, shared_static)? {
            return Ok(Some(progress));
        }

        // local to shared edges
        if let Some(progress) = self.assist_shared_edge_scan(collection)? {
            return Ok(Some(progress));
        }

        // shared mark and sweep work
        if let Some(progress) = self.assist_shared_gc(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
        )? {
            return Ok(Some(progress));
        }

        // local heap work
        let progress =
            self.step_local_collection(world, shared_static, constant_space, host, host_queue)?;
        if progress.advanced() {
            return Ok(Some(progress));
        }

        Ok(None)
    }

    /// Advance one GC operation with local heap work first.
    fn step_gc_with_local_priority(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // local heap work
        let progress =
            self.step_local_collection(world, shared_static, constant_space, host, host_queue)?;
        if progress.advanced() {
            return Ok(Some(progress));
        }

        // direct shared roots
        if let Some(progress) = self.assist_shared_root_scan(collection, shared_static)? {
            return Ok(Some(progress));
        }

        // local to shared edges
        if let Some(progress) = self.assist_shared_edge_scan(collection)? {
            return Ok(Some(progress));
        }

        // shared mark and sweep work
        if let Some(progress) = self.assist_shared_gc(
            world,
            collection,
            shared_static,
            constant_space,
            host,
            host_queue,
        )? {
            return Ok(Some(progress));
        }

        Ok(None)
    }

    /// Publish one pending direct shared-root scan from this worker.
    fn assist_shared_root_scan(
        &mut self,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        // active pass
        let Some(epoch) = collection.pending_root_epoch(self.id) else {
            return Ok(None);
        };

        // owner-local root publication
        let roots = self.collect_shared_roots(shared_static)?;
        let work_bytes = roots.len() * std::mem::size_of::<heap::SharedHeapReference>();
        collection.replace_direct_roots(&self.shared_heap, &self.program, epoch, self.id, roots);

        Ok(Some(heap::GcAdvance::stepped(
            heap::GcCollector::Shared,
            heap::GcPhase::PublishRoots,
            0,
            work_bytes,
        )))
    }

    /// Assist one active shared reference pass from this worker.
    fn assist_shared_edge_scan(
        &mut self,
        collection: &Arc<SharedCollectionState>,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        if self.shared_heap.gc_phase() != heap::GcPhase::Mark || self.shared_edge_scan_idle() {
            return Ok(None);
        }

        let work_bytes = collection.edge_scan_work_bytes();
        if work_bytes == 0 {
            return Ok(None);
        }

        let mut roots = Vec::new();
        let work_done = self.trace_shared_roots(&mut roots, work_bytes)?;
        collection.push_edge_roots(&self.shared_heap, &self.program, self.id, &roots);

        let is_idle = self.shared_edge_scan_idle();
        if is_idle {
            collection.leave_edge_scan(&self.shared_heap, &self.program, self.id);
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

    /// Assist one active shared collection from this worker.
    fn assist_shared_gc(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<heap::GcAdvance>> {
        let shared_heap = self.shared_heap.clone();
        let budget_bytes = if matches!(
            shared_heap.gc_phase(),
            heap::GcPhase::Drop | heap::GcPhase::Sweep
        ) {
            shared_heap.take_collection_budget_bytes(1)
        } else {
            shared_heap.take_assist_budget_bytes()
        };
        if budget_bytes == 0 || shared_heap.gc_phase() == heap::GcPhase::Idle {
            return Ok(None);
        }

        let shared_roots = collection.roots();
        let roots = shared_roots.roots_snapshot();
        let roots_complete = shared_roots.roots_complete();
        let progress = shared_heap
            .step_collection_for_worker(
                Some(&self.shared_mark_worker),
                roots.as_ref(),
                roots_complete,
                budget_bytes,
                self.program.trace_view(),
            )
            .map_err(Box::<RuntimeError>::from)?;

        if let heap::GcAdvance::Drop(drop) = progress {
            self.drop_value(world, shared_static, constant_space, host, host_queue, drop)?;
            shared_heap
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
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
            for execution in event_loop.executions_mut() {
                machine.visit_fiber_root_slots(execution, visit)?;
            }
            Ok::<(), Box<RuntimeError>>(())
        };
        let progress =
            self.heap
                .step_collection(&mut roots, budget_bytes, self.program.trace_view())?;

        if let heap::GcAdvance::Drop(drop) = progress {
            self.drop_value(world, shared_static, constant_space, host, host_queue, drop)?;
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        drop: heap::GcDrop,
    ) -> RuntimeResult<()> {
        let mut context = program::Context::empty();
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
            None,
            &mut self.event_loop,
            self.handshake.as_ref(),
        );
        let activation = program::Activation {
            runtime: &mut activation,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: self.allocation_plans.as_ref(),
                local_heap: &mut self.heap,
                shared_heap: self.shared_heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: shared_static,
                constants: constant_space,
                handshake: self.handshake.as_ref(),
            },
        };

        self.machine.drop_value(activation, drop)
    }

    /// Select and run one queued task or wake.
    #[inline(never)]
    fn select_task(
        &mut self,
        world: &mut WorldState,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // run one queued task before pulling external wakes
        if let Some(task) = self.event_loop.pop_task() {
            return self.run_task_runnable(
                world,
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let Runnable { id, invocation } = runnable;
        let scope = RunnableScope::task(id);
        let outcome = self.execute_invocation(
            world,
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        runnable: Runnable,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // run the microtask runnable
        let Runnable { id, invocation } = runnable;
        let scope = RunnableScope::microtask(id);
        let outcome = self.execute_invocation(
            world,
            shared_static,
            constant_space,
            host,
            host_queue,
            invocation,
            scope,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Resume one runnable under its retained runnable scope.
    fn execute_retained_runnable(
        &mut self,
        world: &mut WorldState,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        retained: RetainedRunnable,
        stop_reason: Option<program::StopReason>,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let RetainedRunnable {
            id,
            scope,
            fiber_id,
            ..
        } = retained;
        let outcome = self.continue_runnable(
            world,
            shared_static,
            constant_space,
            host,
            host_queue,
            scope,
            fiber_id,
            stop_reason,
        )?;

        self.handle_run_outcome(id, scope, outcome)
    }

    /// Handle one run outcome and retain stopped runnable state.
    fn handle_run_outcome(
        &mut self,
        id: RunnableId,
        scope: RunnableScope,
        outcome: FiberOutcome,
    ) -> RuntimeResult<WorkerRunOutcome> {
        let FiberOutcome {
            fiber_id,
            execution,
            outcome,
        } = outcome;

        match outcome {
            Outcome::Completed { value } => {
                self.event_loop.retire_fiber(fiber_id)?;
                self.event_loop.release(value);

                self.runnable_progress(scope)
            }
            Outcome::Cancelled => {
                self.event_loop.retire_fiber(fiber_id)?;

                self.runnable_progress(scope)
            }
            Outcome::Parked => {
                self.event_loop.park_fiber(fiber_id, execution)?;

                self.runnable_progress(scope)
            }
            Outcome::Stopped { reason } => {
                self.machine.retain_stopped(execution);
                self.retained = Some(RetainedRunnable::new(id, scope, fiber_id, Some(reason)));

                Ok(WorkerRunOutcome::Stopped { fiber_id, reason })
            }
        }
    }

    /// Report progress for one settled runnable scope.
    fn runnable_progress(&self, scope: RunnableScope) -> RuntimeResult<WorkerRunOutcome> {
        let Some(progress) = scope.progress() else {
            return Err(RuntimeError::Internal {
                message: "runnable completed without an active runnable scope".to_string(),
            }
            .boxed());
        };

        Ok(WorkerRunOutcome::Progressed { progress })
    }

    /// Execute one queued program invocation.
    fn execute_invocation(
        &mut self,
        world: &mut WorldState,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        invocation: Invocation,
        scope: RunnableScope,
    ) -> RuntimeResult<FiberOutcome> {
        self.refresh_debugger(world);

        // mount one fiber and its physical execution for the invocation
        let (fiber_id, mut execution) = match &invocation {
            Invocation::Function { .. } => {
                let fiber_id = self.event_loop.insert_fiber();
                let mut execution = match self.machine.reserve_fiber() {
                    Ok(execution) => execution,
                    Err(error) => {
                        self.event_loop.retire_fiber(fiber_id)?;

                        return Err(error);
                    }
                };
                execution.mount(fiber_id);

                (fiber_id, execution)
            }
            Invocation::Wake { fiber_id, .. } => {
                let execution = match self.event_loop.resume_fiber(*fiber_id) {
                    Ok(execution) => execution,
                    Err(error) => {
                        if let Invocation::Wake { value, .. } = invocation {
                            self.event_loop.release(value);
                        }

                        return Err(error);
                    }
                };

                (*fiber_id, execution)
            }
        };
        let mut context = invocation.context().unwrap_or_else(|| execution.context());

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
            Some(fiber_id),
            &mut self.event_loop,
            self.handshake.as_ref(),
        );
        let activation = program::Activation {
            runtime: &mut activation,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: self.allocation_plans.as_ref(),
                local_heap: &mut self.heap,
                shared_heap: self.shared_heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: shared_static,
                constants: constant_space,
                handshake: self.handshake.as_ref(),
            },
        };

        let outcome = match invocation {
            Invocation::Function {
                function,
                environment,
                arguments,
                ..
            } => self.machine.run_function(
                &mut execution,
                activation,
                function,
                environment.as_ref(),
                &arguments,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
            Invocation::Wake { value, .. } => self.machine.resume(
                &mut execution,
                activation,
                &value,
                Some(&self.stop_points),
                Some(&self.watch_points),
                self.profile.as_mut(),
            ),
        };
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                self.event_loop.retire_fiber(fiber_id)?;

                return Err(error);
            }
        };
        Ok(FiberOutcome {
            fiber_id,
            execution,
            outcome,
        })
    }

    /// Continue canonical execution retained by this worker machine.
    #[allow(clippy::too_many_arguments)]
    fn continue_runnable(
        &mut self,
        world: &mut WorldState,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        scope: RunnableScope,
        fiber_id: program::FiberId,
        stop_reason: Option<program::StopReason>,
    ) -> RuntimeResult<FiberOutcome> {
        self.refresh_debugger(world);
        let mut execution = self.machine.take_stopped().ok_or_else(|| {
            RuntimeError::Internal {
                message: "retained execution has no stopped fiber".to_string(),
            }
            .boxed()
        })?;
        let mut context = execution.context();

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
            Some(fiber_id),
            &mut self.event_loop,
            self.handshake.as_ref(),
        );
        let activation = program::Activation {
            runtime: &mut activation,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: self.allocation_plans.as_ref(),
                local_heap: &mut self.heap,
                shared_heap: self.shared_heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: shared_static,
                constants: constant_space,
                handshake: self.handshake.as_ref(),
            },
        };

        let resume_skip = stop_reason.and_then(program::StopReason::resume_skip);
        let outcome = self.machine.continue_execution(
            &mut execution,
            activation,
            Some(&self.stop_points),
            Some(&self.watch_points),
            self.profile.as_mut(),
            resume_skip,
        );
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                self.event_loop.retire_fiber(fiber_id)?;

                return Err(error);
            }
        };
        Ok(FiberOutcome {
            fiber_id,
            execution,
            outcome,
        })
    }

    /// Publish root changes and donate GC work after one bounded run.
    fn finish_run(
        &mut self,
        world: &mut WorldState,
        collection: &Arc<SharedCollectionState>,
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        outcome: WorkerRunOutcome,
    ) -> RuntimeResult<WorkerRunOutcome> {
        // destroy released values only after physical execution has completed
        if matches!(outcome, WorkerRunOutcome::Progressed { .. }) {
            self.destroy_released_values(world, shared_static, constant_space, host, host_queue)?;
        }

        if outcome != WorkerRunOutcome::Idle {
            self.sequence = self.sequence.next()?;
        }

        if outcome != WorkerRunOutcome::Idle && self.shared_heap.gc_phase() == heap::GcPhase::Mark {
            collection.queue_root_scan(self.id);
        }

        if matches!(outcome, WorkerRunOutcome::Progressed { .. }) {
            self.advance_gc(
                world,
                collection,
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<()> {
        while let Some(value) = self.event_loop.pop_drop() {
            self.destroy_value(
                world,
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
        shared_static: &mut program::StaticSpace,
        constant_space: &program::StaticSpace,
        host: &dyn Host,
        host_queue: &HostQueue,
        value: program::Value,
    ) -> RuntimeResult<()> {
        let mut context = program::Context::empty();
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
            None,
            &mut self.event_loop,
            self.handshake.as_ref(),
        );
        let activation = program::Activation {
            runtime: &mut activation,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: self.allocation_plans.as_ref(),
                local_heap: &mut self.heap,
                shared_heap: self.shared_heap.as_ref(),
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: shared_static,
                constants: constant_space,
                handshake: self.handshake.as_ref(),
            },
        };

        self.machine.destroy_value(activation, value)
    }
}
