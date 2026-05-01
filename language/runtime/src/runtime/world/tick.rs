use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::runtime::observe::Observation;
use crate::runtime::scheduler::Timer;
use crate::runtime::{TickOutcome, WorkerId};
use destack_workspace::{ExecutionMode, TimeMode};

use super::{Command, RuntimeId, Wake, World};

impl World {
    /// Drain due world wakes at one wall-clock timestamp.
    pub(crate) fn drain_due<I>(worker_timers: I) -> Vec<Wake>
    where
        I: IntoIterator<Item = (RuntimeId, WorkerId, Timer)>,
    {
        // due worker timers
        let mut wakes = worker_timers
            .into_iter()
            .map(|(runtime_id, worker_id, timer)| Wake::WorkerTimer {
                runtime_id,
                worker_id,
                timer,
            })
            .collect::<Vec<_>>();

        // deterministic order
        wakes.sort_by_key(Wake::sort_key);

        wakes
    }

    /// Execute one world tick across all stored runtimes in stable order.
    pub fn tick(&mut self) -> RuntimeResult<TickOutcome> {
        self.tick_drive()
    }

    /// Execute one world tick directly on the current lane.
    pub(crate) fn tick_drive(&mut self) -> RuntimeResult<TickOutcome> {
        let command = self.resolve_command(Command::Tick)?;
        if self.trace.mode() == ExecutionMode::Record {
            self.ingest(command.clone())?;
        }

        self.tick_inner()
    }

    /// Execute one world tick without tracing the outer invocation.
    pub(crate) fn tick_inner(&mut self) -> RuntimeResult<TickOutcome> {
        let world = self.world_scope();

        // runnable work and ingress
        for runtime in self.runtimes.values_mut() {
            if runtime.tick(&world)?.progressed() {
                runtime.tick_shared_gc()?;

                world.observe(Observation::scheduler_progressed());

                return Ok(TickOutcome::Progressed);
            }
        }

        // shared heap work also counts as scheduler progress
        for runtime in self.runtimes.values_mut() {
            if runtime.tick_shared_gc()? {
                world.observe(Observation::scheduler_progressed());

                return Ok(TickOutcome::Progressed);
            }
        }

        // background shared GC still counts as live world work,
        // but it did not advance on this caller thread
        if self
            .runtimes
            .values()
            .any(|runtime| runtime.shared_gc_in_flight())
        {
            return Ok(TickOutcome::Concurrent);
        }

        // host time cannot advance under world control
        if world.time_mode != TimeMode::Virtual {
            return Ok(TickOutcome::Idle);
        }

        // next global deadline
        let next_deadline = world.next_deadline(
            self.runtimes
                .values()
                .map(|runtime| runtime.next_deadline(&world)),
        );
        let Some(deadline) = next_deadline else {
            return Ok(TickOutcome::Idle);
        };

        // record the resolved world time jump
        let deadline = self.trace.resolve_time_advance(deadline)?;
        self.clock.advance_virtual_to(deadline);
        self.trace.record_time_advance(deadline)?;

        // collect due worker timers across runtimes
        let mut worker_timers = Vec::new();
        for runtime in self.runtimes.values_mut() {
            worker_timers.extend(runtime.collect_due_timers(&world)?);
        }

        // deliver due simulation events into the simulation ready queue
        self.simulation.deliver_due(deadline);

        // drain world timer wakes into per-runtime batches
        let wakes = Self::drain_due(worker_timers);
        let mut wakes_by_runtime = BTreeMap::<RuntimeId, Vec<Wake>>::new();
        for wake in wakes {
            match wake {
                Wake::WorkerTimer { runtime_id, .. } => {
                    wakes_by_runtime.entry(runtime_id).or_default().push(wake);
                }
            }
        }

        // deliver due worker wakes
        for (runtime_id, runtime) in self.runtimes.iter_mut() {
            let wakes = wakes_by_runtime.remove(runtime_id).unwrap_or_default();
            if !wakes.is_empty() {
                runtime.deliver_wakes(&world, wakes)?;
            }
        }

        world.observe(Observation::scheduler_advanced_time(deadline));

        Ok(TickOutcome::AdvancedTime)
    }

    /// Execute world ticks across all stored runtimes until idle.
    pub fn tick_until_idle(&mut self) -> RuntimeResult<()> {
        loop {
            let outcome = self.tick()?;

            // keep yielding while one background side plane is still draining
            if outcome == TickOutcome::Concurrent {
                std::thread::yield_now();
                continue;
            }

            // stop when the scheduler cannot make further progress
            if outcome == TickOutcome::Idle {
                break;
            }
        }

        Ok(())
    }
}
