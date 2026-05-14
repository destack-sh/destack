use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::host::time::TimerClock;
use crate::runtime::scheduler::{ScheduledTimer, TimerWake, Wake};
use crate::runtime::{TickResult, WorkerId};
use crate::world::trace::Observation;
use destack_workspace::ClockSource;

use super::{RuntimeId, WorkerWake, World};

impl World {
    /// Drain due world wakes at one wall-clock timestamp.
    pub(crate) fn drain_due<I>(worker_timers: I) -> Vec<WorkerWake>
    where
        I: IntoIterator<Item = (RuntimeId, WorkerId, ScheduledTimer)>,
    {
        // due worker timers
        let mut timers = worker_timers.into_iter().collect::<Vec<_>>();

        // deterministic order
        timers.sort_by_key(|(runtime_id, worker_id, timer)| {
            due_timer_sort_key(*runtime_id, *worker_id, *timer)
        });

        timers
            .into_iter()
            .map(|(runtime_id, worker_id, timer)| WorkerWake {
                runtime_id,
                worker_id,
                wake: Wake::Timer(TimerWake::new(timer.resource_id)),
            })
            .collect()
    }

    /// Execute one world tick across all stored runtimes in stable order.
    pub fn tick(&mut self) -> RuntimeResult<TickResult> {
        self.do_tick()
    }

    /// Execute one world tick without tracing the outer invocation.
    pub(crate) fn do_tick(&mut self) -> RuntimeResult<TickResult> {
        let world = &mut self.state;

        // runnable work and ingress
        for runtime in self.runtimes.values_mut() {
            if runtime.tick(world)?.is_progress() {
                runtime.tick_shared_gc()?;

                world.observe(Observation::scheduler_progressed());

                return Ok(TickResult::Progress);
            }
        }

        // shared heap work also counts as scheduler progress
        for runtime in self.runtimes.values_mut() {
            if runtime.tick_shared_gc()? {
                world.observe(Observation::scheduler_progressed());

                return Ok(TickResult::Progress);
            }
        }

        // background shared GC still counts as live world work,
        // but it did not advance on this caller thread
        if self
            .runtimes
            .values()
            .any(|runtime| runtime.shared_gc_in_flight())
        {
            return Ok(TickResult::Background);
        }

        // host time cannot advance under world control
        if world.clock.source() != ClockSource::Virtual {
            return Ok(TickResult::Idle);
        }

        // next global deadline
        let worker_deadlines = self
            .runtimes
            .values_mut()
            .map(|runtime| runtime.next_deadline(world))
            .collect::<Vec<_>>();
        let next_deadline = world.next_deadline(worker_deadlines);
        let Some(deadline) = next_deadline else {
            return Ok(TickResult::Idle);
        };

        // record the resolved world time jump
        let deadline = world.trace.resolve_time_advance(deadline)?;
        world.clock.advance_virtual_to(deadline);
        world.trace.record_time_advance(deadline)?;

        // collect due worker timers across runtimes
        let mut worker_timers = Vec::new();
        for runtime in self.runtimes.values_mut() {
            worker_timers.extend(runtime.collect_due_timers(world)?);
        }

        // deliver due simulation events into the simulation ready queue
        world.simulation.deliver_due(deadline);

        // drain world timer wakes into per-runtime batches
        let wakes = Self::drain_due(worker_timers);
        let mut wakes_by_runtime = BTreeMap::<RuntimeId, Vec<WorkerWake>>::new();
        for wake in wakes {
            wakes_by_runtime
                .entry(wake.runtime_id)
                .or_default()
                .push(wake);
        }

        // deliver due worker wakes
        for (runtime_id, runtime) in self.runtimes.iter_mut() {
            let wakes = wakes_by_runtime.remove(runtime_id).unwrap_or_default();
            if !wakes.is_empty() {
                runtime.deliver_wakes(world, wakes)?;
            }
        }

        world.observe(Observation::scheduler_advanced_time(deadline));

        Ok(TickResult::TimeAdvanced)
    }

    /// Execute world ticks across all stored runtimes until idle.
    pub fn tick_until_idle(&mut self) -> RuntimeResult<()> {
        loop {
            let outcome = self.tick()?;

            // keep yielding while background work is still draining
            if outcome == TickResult::Background {
                std::thread::yield_now();
                continue;
            }

            // stop when the scheduler cannot make further progress
            if outcome == TickResult::Idle {
                break;
            }
        }

        Ok(())
    }
}

/// Return the deterministic ordering key for one timer wake.
fn due_timer_sort_key(
    runtime_id: RuntimeId,
    worker_id: WorkerId,
    timer: ScheduledTimer,
) -> (u8, u64, u64, u64, (u64, u64)) {
    (
        timer_clock_rank(timer.deadline.clock),
        timer.deadline.at.get(),
        runtime_id.0,
        worker_id.0,
        timer.sort_key(),
    )
}

/// Return deterministic ordering for one timer clock.
fn timer_clock_rank(clock: TimerClock) -> u8 {
    match clock {
        TimerClock::Monotonic => 0,
        TimerClock::Wall => 1,
    }
}
