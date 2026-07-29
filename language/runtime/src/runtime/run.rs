use destack_heap as heap;
use destack_program as program;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::poller::PollerEvent;
use crate::host::{Host, HostEvent, HostQueue};
use crate::machine::Entry;
use crate::runtime::Runtime;
use crate::worker::scheduler::{HostWake, Readiness, ResourceWake, ScheduledTimer, Wake};
use crate::worker::{RunnableProgress, WorkerId, WorkerRunOutcome};
use crate::world::time::Instant;
use crate::world::{RuntimeId, WorkerWake, WorldState};

/// Outcome from one bounded runtime run operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeRunOutcome {
    /// One worker made progress.
    Progressed {
        /// Worker that progressed.
        worker_id: WorkerId,
        /// Work that made progress.
        progress: RunnableProgress,
    },
    /// No worker was runnable.
    Idle,
    /// One worker stopped at a runtime stop point.
    Stopped {
        /// Worker that stopped.
        worker_id: WorkerId,
        /// Reason execution stopped.
        reason: program::StopReason,
    },
    /// One worker is paused at a previously reached stop point.
    Paused {
        /// Worker that stopped.
        worker_id: WorkerId,
        /// Reason execution stopped.
        reason: program::StopReason,
    },
}

impl RuntimeRunOutcome {
    /// Convert a worker run outcome when work happened.
    fn from_worker(worker_id: WorkerId, outcome: WorkerRunOutcome) -> Option<Self> {
        match outcome {
            WorkerRunOutcome::Progressed { progress } => Some(Self::Progressed {
                worker_id,
                progress,
            }),
            WorkerRunOutcome::Idle => None,
            WorkerRunOutcome::Stopped { reason } => Some(Self::Stopped { worker_id, reason }),
            WorkerRunOutcome::Paused { reason } => Some(Self::Paused { worker_id, reason }),
        }
    }
}

impl Runtime {
    /// Run one entrypoint through one runtime worker event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
        worker_id: WorkerId,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        let worker = self
            .workers
            .get_mut(&worker_id)
            .ok_or_else(|| RuntimeError::worker_not_found(worker_id.0).boxed())?;
        worker.run_entrypoint(
            world,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            host,
            host_queue,
            entry,
            args,
        )
    }

    /// Run one pending worker microtask in stable order.
    pub(crate) fn run_microtask(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<RuntimeRunOutcome> {
        // workers
        let worker_count = self.workers.len();
        let start_index = if worker_count == 0 {
            0
        } else {
            self.next_worker_cursor % worker_count
        };
        let shared = &self.heap;
        let shared_static = &mut self.shared_static;
        let constant_space = &self.constant_space;
        let workers = &mut self.workers;

        for (worker_index, worker) in workers.values_mut().enumerate().skip(start_index) {
            let outcome = worker.run_microtask(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            if let Some(outcome) = RuntimeRunOutcome::from_worker(worker.id, outcome) {
                self.next_worker_cursor = (worker_index + 1) % worker_count;

                return Ok(outcome);
            }
        }

        for (worker_index, worker) in workers.values_mut().enumerate().take(start_index) {
            let outcome = worker.run_microtask(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            if let Some(outcome) = RuntimeRunOutcome::from_worker(worker.id, outcome) {
                self.next_worker_cursor = worker_index + 1;

                return Ok(outcome);
            }
        }

        Ok(RuntimeRunOutcome::Idle)
    }

    /// Continue one stopped worker in scheduler order.
    pub(crate) fn continue_stop(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<RuntimeRunOutcome> {
        let worker_count = self.workers.len();
        if worker_count == 0 {
            return Ok(RuntimeRunOutcome::Idle);
        }
        let start_index = self.next_worker_cursor % worker_count;
        let shared = &self.heap;
        let shared_static = &mut self.shared_static;
        let constant_space = &self.constant_space;
        let workers = &mut self.workers;

        // scan retained stops after the scheduler cursor
        for (worker_index, (worker_id, worker)) in workers.iter_mut().enumerate().skip(start_index)
        {
            let outcome = worker.continue_stop(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            let Some(outcome) = RuntimeRunOutcome::from_worker(*worker_id, outcome) else {
                continue;
            };
            self.next_worker_cursor = (worker_index + 1) % worker_count;

            return Ok(outcome);
        }

        // wrap retained stops around to workers before the scheduler cursor
        for (worker_index, (worker_id, worker)) in workers.iter_mut().enumerate().take(start_index)
        {
            let outcome = worker.continue_stop(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            let Some(outcome) = RuntimeRunOutcome::from_worker(*worker_id, outcome) else {
                continue;
            };
            self.next_worker_cursor = (worker_index + 1) % worker_count;

            return Ok(outcome);
        }

        Ok(RuntimeRunOutcome::Idle)
    }

    /// Run one pending worker task in stable scheduler order.
    pub(crate) fn run_task(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<RuntimeRunOutcome> {
        // workers
        let worker_count = self.workers.len();
        let start_index = if worker_count == 0 {
            0
        } else {
            self.next_worker_cursor % worker_count
        };
        let shared = &self.heap;
        let shared_static = &mut self.shared_static;
        let constant_space = &self.constant_space;
        let workers = &mut self.workers;

        for (worker_index, (worker_id, worker)) in workers.iter_mut().enumerate().skip(start_index)
        {
            let outcome = worker.run_task(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            if let Some(outcome) = RuntimeRunOutcome::from_worker(*worker_id, outcome) {
                self.next_worker_cursor = (worker_index + 1) % worker_count;

                return Ok(outcome);
            }
        }

        for (worker_index, (worker_id, worker)) in workers.iter_mut().enumerate().take(start_index)
        {
            let outcome = worker.run_task(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )?;
            if let Some(outcome) = RuntimeRunOutcome::from_worker(*worker_id, outcome) {
                self.next_worker_cursor = worker_index + 1;

                return Ok(outcome);
            }
        }

        Ok(RuntimeRunOutcome::Idle)
    }

    /// Run one idle worker safepoint in stable scheduler order.
    pub(crate) fn run_safepoint(
        &mut self,
        world: &mut WorldState,
        host: &dyn Host,
        host_queue: &HostQueue,
    ) -> RuntimeResult<Option<(WorkerId, heap::GcAdvance)>> {
        // empty runtimes have no worker maintenance to donate
        let worker_count = self.workers.len();
        if worker_count == 0 {
            return Ok(None);
        }

        let start_index = self.next_worker_cursor % worker_count;
        let shared = &self.heap;
        let shared_static = &mut self.shared_static;
        let constant_space = &self.constant_space;

        // scan workers after the scheduler cursor
        for (worker_index, (worker_id, worker)) in
            self.workers.iter_mut().enumerate().skip(start_index)
        {
            if let Some(progress) = worker.run_safepoint(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )? {
                self.next_worker_cursor = (worker_index + 1) % worker_count;

                return Ok(Some((*worker_id, progress)));
            }
        }

        // wrap around to workers before the scheduler cursor
        for (worker_index, (worker_id, worker)) in
            self.workers.iter_mut().enumerate().take(start_index)
        {
            if let Some(progress) = worker.run_safepoint(
                world,
                shared,
                shared_static,
                constant_space,
                host,
                host_queue,
            )? {
                self.next_worker_cursor = worker_index + 1;

                return Ok(Some((*worker_id, progress)));
            }
        }

        Ok(None)
    }

    /// Return the next virtual deadline across all workers.
    pub(crate) fn next_deadline(&mut self, world: &mut WorldState) -> Option<Instant> {
        // current virtual timestamps: monotonic deadlines are projected onto wall time
        let wall_now = world.wall();
        let mono_now = world.mono();

        self.workers
            .values_mut()
            .filter_map(|worker| worker.event_loop.next_deadline(wall_now, mono_now))
            .min()
    }

    /// Drain due worker timers after the world advances time.
    pub(crate) fn collect_due_timers(
        &mut self,
        world: &mut WorldState,
    ) -> RuntimeResult<Vec<(RuntimeId, WorkerId, ScheduledTimer)>> {
        let wall_now = world.wall();
        let mono_now = world.mono();
        let mut worker_timers = Vec::new();

        for worker in self.workers.values_mut() {
            while let Some(timer) = worker.event_loop.pop_ready_timer(wall_now, mono_now)? {
                worker_timers.push((self.id, worker.id, timer));
            }
        }

        Ok(worker_timers)
    }

    /// Deliver externally collected events to worker event loops.
    pub(crate) fn deliver_events(
        &mut self,
        host_events: &[HostEvent],
        poller_events: &[PollerEvent],
    ) -> RuntimeResult<bool> {
        let mut handled_any = false;
        let is_marking_shared = self.heap.is_marking();

        // host events
        for event in host_events {
            if self.deliver_host_event(event.clone(), is_marking_shared)? {
                handled_any = true;
            }
        }

        // poller events
        for event in poller_events {
            if self.deliver_poller_event(*event, is_marking_shared)? {
                handled_any = true;
            }
        }

        Ok(handled_any)
    }

    /// Deliver one host event into matching worker event loops.
    pub(crate) fn deliver_host_event(
        &mut self,
        event: HostEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let kind = event.kind();
        let shared = &self.heap;
        let mut handled_any = false;

        for (worker_id, worker) in &mut self.workers {
            if !worker.event_loop.has_host_waiter(kind) {
                continue;
            }

            handled_any = true;
            worker
                .event_loop
                .enqueue_wake(Wake::Host(HostWake::new(event.clone())));

            // shared mark: events can change direct worker roots between worker runs
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(handled_any)
    }

    /// Deliver one poller event into matching worker event loops.
    pub(crate) fn deliver_poller_event(
        &mut self,
        event: PollerEvent,
        is_marking_shared: bool,
    ) -> RuntimeResult<bool> {
        let readiness = Readiness::from_poller_mask(event.mask);
        let shared = &self.heap;
        let mut handled_any = false;

        for (worker_id, worker) in &mut self.workers {
            if !worker
                .event_loop
                .has_resource_waiter(event.resource_id, readiness)
            {
                continue;
            }

            handled_any = true;
            worker
                .event_loop
                .enqueue_wake(Wake::Resource(ResourceWake::poller(event)));

            // shared mark: events can change direct worker roots between worker runs
            if is_marking_shared {
                shared.queue_root_scan(*worker_id);
            }
        }

        Ok(handled_any)
    }

    /// Deliver one batch of due worker-timer wakes.
    pub(crate) fn deliver_wakes(&mut self, wakes: Vec<WorkerWake>) -> RuntimeResult<()> {
        for wake in wakes {
            if wake.runtime_id != self.id {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "runtime {} received wake for runtime {}",
                        self.id.0, wake.runtime_id.0
                    ),
                }
                .boxed());
            }

            let worker = self
                .worker_mut(wake.worker_id)
                .ok_or_else(|| RuntimeError::worker_not_found(wake.worker_id.0).boxed())?;
            worker.event_loop.enqueue_wake(wake.wake);
        }

        Ok(())
    }
}
