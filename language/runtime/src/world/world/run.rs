use std::collections::BTreeMap;

use destack_program as program;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::poll_host_events;
use crate::host::poller::HostPoller;
use crate::host::time::TimerClock;
use crate::runtime::scheduler::{ScheduledTimer, TimerWake, Wake};
use crate::runtime::time::{ClockSource, Instant};
use crate::runtime::{RuntimeRunOutcome, WorkerId};
use crate::world::observation::Observation;

use super::{Moment, RuntimeId, WorkerWake, World, WorldState};

/// Event-loop work to run in one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    /// Run one pending microtask.
    Microtask,
    /// Drain pending microtasks.
    Microtasks,
    /// Resume the first retained runtime stop.
    Continue,
    /// Run one task and its microtask checkpoint.
    Task,
    /// Run event-loop work until the world becomes idle.
    UntilIdle,
}

/// Outcome from one world run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// One task, microtask, wake, or GC step made progress.
    Progressed,
    /// Runtime-controlled time advanced.
    AdvancedTime,
    /// Background work is still active outside this caller.
    Background,
    /// No work was runnable or scheduled.
    Idle,
    /// Execution stopped at one runtime stop point.
    Stopped {
        /// Runtime stop metadata.
        stop: Stop,
    },
}

/// Runtime stop visible at the world boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    /// Lineage coordinate where execution stopped.
    pub moment: Moment,
    /// Runtime that owns the stopped worker.
    pub runtime_id: RuntimeId,
    /// Worker that stopped.
    pub worker_id: WorkerId,
    /// Reason execution stopped.
    pub reason: program::StopReason,
}

impl RunOutcome {
    /// Return whether this outcome advanced deterministic world execution.
    pub const fn is_progress(self) -> bool {
        matches!(
            self,
            Self::Progressed | Self::AdvancedTime | Self::Stopped { .. }
        )
    }
}

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

    /// Run event-loop work in this world.
    pub fn run(&mut self, run: Run) -> RuntimeResult<RunOutcome> {
        match run {
            Run::Microtask => self.run_microtask(),
            Run::Microtasks => self.run_microtasks(),
            Run::Continue => self.run_continue(),
            Run::Task => self.run_task(),
            Run::UntilIdle => self.run_until_idle(),
        }
    }

    /// Run one pending microtask across the stored runtimes.
    fn run_microtask(&mut self) -> RuntimeResult<RunOutcome> {
        let world = &mut self.state;

        // run the first available worker microtask in stable scheduler order
        for (runtime_id, runtime) in &mut self.runtimes {
            let outcome = runtime.run_microtask(world, self.host.as_ref(), &self.host_queue)?;
            let Some(outcome) = world.run_runtime_outcome(*runtime_id, outcome)? else {
                continue;
            };

            return Ok(outcome);
        }

        Ok(RunOutcome::Idle)
    }

    /// Drain pending microtasks across the stored runtimes.
    fn run_microtasks(&mut self) -> RuntimeResult<RunOutcome> {
        let mut progressed = false;

        // drain worker microtasks until no runtime can make progress
        loop {
            let outcome = self.run_microtask()?;
            match outcome {
                RunOutcome::Progressed => progressed = true,
                RunOutcome::Stopped { .. } => return Ok(outcome),
                RunOutcome::AdvancedTime | RunOutcome::Background | RunOutcome::Idle => break,
            }
        }

        if progressed {
            Ok(RunOutcome::Progressed)
        } else {
            Ok(RunOutcome::Idle)
        }
    }

    /// Continue the first retained runtime stop.
    fn run_continue(&mut self) -> RuntimeResult<RunOutcome> {
        let world = &mut self.state;

        // resume the first stopped runtime in deterministic order
        for (runtime_id, runtime) in &mut self.runtimes {
            let outcome = runtime.continue_stop(world, self.host.as_ref(), &self.host_queue)?;
            let Some(outcome) = world.run_runtime_outcome(*runtime_id, outcome)? else {
                continue;
            };

            return Ok(outcome);
        }

        Ok(RunOutcome::Idle)
    }

    /// Run one task or scheduler event across the stored runtimes.
    fn run_task(&mut self) -> RuntimeResult<RunOutcome> {
        let host_events = poll_host_events(self.host.as_ref(), &self.host_queue, Some(0))?.events;
        let poller_events = self.poller.poll(Some(0))?;
        let world = &mut self.state;
        let mut ingress_progressed = false;

        // external events
        for runtime in self.runtimes.values_mut() {
            if runtime.deliver_events(&host_events, &poller_events)? {
                ingress_progressed = true;
            }
        }

        // runnable work and ingress
        for (runtime_id, runtime) in &mut self.runtimes {
            let outcome = runtime.run_task(world, self.host.as_ref(), &self.host_queue)?;
            let Some(outcome) = world.run_runtime_outcome(*runtime_id, outcome)? else {
                continue;
            };
            runtime.tick_shared_gc()?;

            return Ok(outcome);
        }

        // shared heap work also counts as scheduler progress
        for runtime in self.runtimes.values_mut() {
            if runtime.tick_shared_gc()? {
                world.advance_moment()?;
                world.observe(Observation::scheduler_progressed())?;

                return Ok(RunOutcome::Progressed);
            }
        }

        // ingress without immediate worker execution still advanced scheduler state
        if ingress_progressed {
            world.advance_moment()?;
            world.observe(Observation::scheduler_progressed())?;

            return Ok(RunOutcome::Progressed);
        }

        // background shared GC still counts as live world work,
        // but it did not advance on this caller thread
        if self
            .runtimes
            .values()
            .any(|runtime| runtime.shared_gc_in_flight())
        {
            return Ok(RunOutcome::Background);
        }

        // host time cannot advance under world control
        if world.clock.source() != ClockSource::Runtime {
            return Ok(RunOutcome::Idle);
        }

        // next global deadline
        let next_deadline = self
            .runtimes
            .values_mut()
            .filter_map(|runtime| runtime.next_deadline(world))
            .min();
        let Some(deadline) = next_deadline else {
            return Ok(RunOutcome::Idle);
        };

        // advance runtime-controlled time
        self.advance_time(deadline)?;
        let world = &mut self.state;

        // collect due worker timers across runtimes
        let mut worker_timers = Vec::new();
        for runtime in self.runtimes.values_mut() {
            worker_timers.extend(runtime.collect_due_timers(world)?);
        }

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
                runtime.deliver_wakes(wakes)?;
            }
        }

        Ok(RunOutcome::AdvancedTime)
    }

    /// Advance runtime-controlled world time to one wall-clock deadline.
    pub fn advance_time(&mut self, deadline: Instant) -> RuntimeResult<Instant> {
        if self.state.clock.source() != ClockSource::Runtime {
            return Err(RuntimeError::host_time_advance().boxed());
        }

        // record the resolved world time jump
        let deadline = self.state.trace.resolve_time_advance(deadline)?;
        let deadline = self.state.clock.advance_runtime_to(deadline)?;
        self.state.trace.record_time_advance(deadline)?;
        self.state.advance_moment()?;
        self.state
            .observe(Observation::scheduler_advanced_time(deadline))?;

        Ok(deadline)
    }

    /// Run event-loop work until no runtime can make progress.
    fn run_until_idle(&mut self) -> RuntimeResult<RunOutcome> {
        loop {
            let outcome = self.run(Run::Task)?;

            // keep yielding while background work is still draining
            if outcome == RunOutcome::Background {
                std::thread::yield_now();
                continue;
            }

            // stop when execution reaches an explicit stop point
            if let RunOutcome::Stopped { .. } = outcome {
                return Ok(outcome);
            }

            // stop when the scheduler cannot make further progress
            if outcome == RunOutcome::Idle {
                break;
            }
        }

        Ok(RunOutcome::Idle)
    }
}

impl WorldState {
    /// Convert one runtime run outcome into one world run outcome.
    fn run_runtime_outcome(
        &mut self,
        runtime_id: RuntimeId,
        outcome: RuntimeRunOutcome,
    ) -> RuntimeResult<Option<RunOutcome>> {
        match outcome {
            RuntimeRunOutcome::Progressed => {
                self.advance_moment()?;
                self.observe(Observation::scheduler_progressed())?;

                Ok(Some(RunOutcome::Progressed))
            }
            RuntimeRunOutcome::Idle => Ok(None),
            RuntimeRunOutcome::Stopped { worker_id, reason } => {
                let moment = self.advance_moment()?;
                let stop = Stop {
                    moment,
                    runtime_id,
                    worker_id,
                    reason,
                };
                self.observe(Observation::scheduler_progressed())?;

                Ok(Some(RunOutcome::Stopped { stop }))
            }
            RuntimeRunOutcome::Paused { worker_id, reason } => {
                let stop = Stop {
                    moment: self.moment(),
                    runtime_id,
                    worker_id,
                    reason,
                };

                Ok(Some(RunOutcome::Stopped { stop }))
            }
        }
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
