use std::collections::BTreeMap;
use std::thread;

use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_program as program;
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::poller::Poller;
use crate::host::time::TimerClock;
use crate::runtime::{RuntimeId, RuntimeRunOutcome};
use crate::scheduler::{ScheduledTimer, TimerWake, Wake};
use crate::worker::{RunnableProgress, WorkerId};
use crate::world::observation::Observation;
use crate::world::time::{ClockSource, Instant};

use super::{Moment, WorkerWake, World, WorldState};

/// One bounded World execution operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Run {
    /// Run one pending microtask.
    Microtask,
    /// Drain pending microtasks.
    MicrotaskCheckpoint,
    /// Run one task.
    Task,
}

/// Outcome from one bounded World execution operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
        /// Runtime stop.
        stop: Stop,
    },
}

/// Runtime stop visible at the World boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Stop {
    /// Lineage coordinate where execution stopped.
    pub moment: Moment,
    /// Runtime that owns the stopped worker.
    pub runtime_id: RuntimeId,
    /// Worker that stopped.
    pub worker_id: WorkerId,
    /// Fiber that stopped.
    pub fiber_id: program::FiberId,
    /// Reason execution stopped.
    pub reason: program::StopReason,
}

/// Runtime work kind visible in the World timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunStep {
    /// One microtask ran.
    Microtask,
    /// One retained stop resumed.
    Resume,
    /// One task ran.
    Task,
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

impl RunStep {
    /// Return the scheduler observation for one progressed runnable.
    const fn observation(
        self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        progress: RunnableProgress,
    ) -> Observation {
        match (self, progress) {
            (Self::Resume, RunnableProgress::Task { task_id }) => Observation::TaskResumed {
                runtime_id,
                worker_id,
                task_id,
            },
            (Self::Resume, RunnableProgress::Microtask { microtask_id }) => {
                Observation::MicrotaskResumed {
                    runtime_id,
                    worker_id,
                    microtask_id,
                }
            }
            (_, RunnableProgress::Task { task_id }) => Observation::TaskRan {
                runtime_id,
                worker_id,
                task_id,
            },
            (_, RunnableProgress::Microtask { microtask_id }) => Observation::MicrotaskRan {
                runtime_id,
                worker_id,
                microtask_id,
            },
        }
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
            Self::timer_order(*runtime_id, *worker_id, *timer)
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

    /// Run the selected bounded World execution operation.
    pub fn run(&mut self, run: Run) -> RuntimeResult<RunOutcome> {
        match run {
            Run::Microtask => self.run_microtask(),
            Run::MicrotaskCheckpoint => self.run_microtasks(),
            Run::Task => self.run_task(),
        }
    }

    /// Resume one exact debugger-stopped Worker.
    pub fn resume(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
    ) -> RuntimeResult<RunOutcome> {
        let world = &mut self.state;
        let runtime = self
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeError::runtime_not_found(runtime_id.0).boxed())?;
        let outcome = runtime.resume(world, self.host.as_ref(), &self.host_queue, worker_id)?;

        // project the exact Worker outcome into World time
        world.run_runtime_outcome(runtime_id, outcome, RunStep::Resume)
    }

    /// Drain event-loop work until this World becomes idle or stops.
    pub fn drain(&mut self) -> RuntimeResult<RunOutcome> {
        loop {
            // drain the microtask checkpoint before selecting another task
            let outcome = self.run(Run::MicrotaskCheckpoint)?;
            let outcome = if outcome == RunOutcome::Idle {
                self.run(Run::Task)?
            } else {
                outcome
            };

            // keep progressing until the World stops or becomes idle
            match outcome {
                RunOutcome::Background => thread::yield_now(),
                RunOutcome::Idle | RunOutcome::Stopped { .. } => return Ok(outcome),
                RunOutcome::Progressed | RunOutcome::AdvancedTime => {}
            }
        }
    }

    /// Run one pending microtask across the stored runtimes.
    fn run_microtask(&mut self) -> RuntimeResult<RunOutcome> {
        let runtime_count = self.runtimes.len();
        if runtime_count == 0 {
            return Ok(RunOutcome::Idle);
        }
        let start_index = self.next_runtime_cursor % runtime_count;
        let world = &mut self.state;
        let runtimes = &mut self.runtimes;

        // scan runtimes after the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            runtimes.iter_mut().enumerate().skip(start_index)
        {
            let outcome = runtime.run_microtask(world, self.host.as_ref(), &self.host_queue)?;
            let outcome = world.run_runtime_outcome(*runtime_id, outcome, RunStep::Microtask)?;
            if outcome == RunOutcome::Idle {
                continue;
            }

            self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

            return Ok(outcome);
        }

        // wrap around to runtimes before the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            runtimes.iter_mut().enumerate().take(start_index)
        {
            let outcome = runtime.run_microtask(world, self.host.as_ref(), &self.host_queue)?;
            let outcome = world.run_runtime_outcome(*runtime_id, outcome, RunStep::Microtask)?;
            if outcome == RunOutcome::Idle {
                continue;
            }

            self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

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

    /// Run one task or scheduler event across the stored runtimes.
    fn run_task(&mut self) -> RuntimeResult<RunOutcome> {
        let host_events = self.host_queue.poll(self.host.as_ref(), Some(0))?;
        let poller_events = self.poller.poll(Some(0))?;
        let world = &mut self.state;
        let mut ingress_progressed = false;

        // external events
        for runtime in self.runtimes.values_mut() {
            if runtime.deliver_events(&host_events, &poller_events)? {
                ingress_progressed = true;
            }
        }

        let runtime_count = self.runtimes.len();
        let start_index = if runtime_count == 0 {
            0
        } else {
            self.next_runtime_cursor % runtime_count
        };

        // scan runnable work after the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().skip(start_index)
        {
            let outcome = runtime.run_task(world, self.host.as_ref(), &self.host_queue)?;
            let outcome = world.run_runtime_outcome(*runtime_id, outcome, RunStep::Task)?;
            if outcome == RunOutcome::Idle {
                continue;
            }

            self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

            return Ok(outcome);
        }

        // wrap runnable work around to runtimes before the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().take(start_index)
        {
            let outcome = runtime.run_task(world, self.host.as_ref(), &self.host_queue)?;
            let outcome = world.run_runtime_outcome(*runtime_id, outcome, RunStep::Task)?;
            if outcome == RunOutcome::Idle {
                continue;
            }

            self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

            return Ok(outcome);
        }

        // scan shared heap work after the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().skip(start_index)
        {
            if let Some(advance) = runtime.advance_shared_gc()? {
                if !world.observe_gc_advance(*runtime_id, None, advance)? {
                    continue;
                }
                self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

                return Ok(RunOutcome::Progressed);
            }
        }

        // wrap shared heap work around to runtimes before the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().take(start_index)
        {
            if let Some(advance) = runtime.advance_shared_gc()? {
                if !world.observe_gc_advance(*runtime_id, None, advance)? {
                    continue;
                }
                self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

                return Ok(RunOutcome::Progressed);
            }
        }

        // scan idle worker GC work after the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().skip(start_index)
        {
            if let Some((worker_id, advance)) =
                runtime.advance_gc(world, self.host.as_ref(), &self.host_queue)?
            {
                if !world.observe_gc_advance(*runtime_id, Some(worker_id), advance)? {
                    continue;
                }
                self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

                return Ok(RunOutcome::Progressed);
            }
        }

        // wrap idle worker GC work around to runtimes before the scheduler cursor
        for (runtime_index, (runtime_id, runtime)) in
            self.runtimes.iter_mut().enumerate().take(start_index)
        {
            if let Some((worker_id, advance)) =
                runtime.advance_gc(world, self.host.as_ref(), &self.host_queue)?
            {
                if !world.observe_gc_advance(*runtime_id, Some(worker_id), advance)? {
                    continue;
                }
                self.next_runtime_cursor = (runtime_index + 1) % runtime_count;

                return Ok(RunOutcome::Progressed);
            }
        }

        // ingress without immediate worker execution still advanced scheduler state
        if ingress_progressed {
            world.advance_moment()?;
            world.observe(Observation::IngressDelivered {
                host_events: host_events.len(),
                poller_events: poller_events.len(),
            })?;

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
        self.advance(deadline)?;
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
            if let Some(wakes) = wakes_by_runtime.remove(runtime_id) {
                runtime.deliver_wakes(wakes)?;
            }
        }

        Ok(RunOutcome::AdvancedTime)
    }

    /// Advance runtime-controlled World time to one wall-clock deadline.
    pub fn advance(&mut self, deadline: Instant) -> RuntimeResult<Instant> {
        if self.state.clock.source() != ClockSource::Runtime {
            return Err(RuntimeError::host_time_advance().boxed());
        }

        // record the resolved world time jump
        let deadline = self.state.trace.resolve_time_advance(deadline)?;
        let deadline = self.state.clock.advance_runtime_to(deadline)?;
        self.state.trace.record_time_advance(deadline)?;
        self.state.advance_moment()?;
        self.state.observe(Observation::TimeAdvanced { deadline })?;

        Ok(deadline)
    }

    /// Return the deterministic ordering key for one timer wake.
    fn timer_order(
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        timer: ScheduledTimer,
    ) -> (u8, u64, u64, u64, (u64, u64)) {
        (
            Self::clock_order(timer.deadline.clock),
            timer.deadline.at.get(),
            runtime_id.0,
            worker_id.0,
            timer.sort_key(),
        )
    }

    /// Return deterministic ordering for one timer clock.
    const fn clock_order(clock: TimerClock) -> u8 {
        match clock {
            TimerClock::Monotonic => 0,
            TimerClock::Wall => 1,
        }
    }
}

impl WorldState {
    /// Observe one GC advancement when it contains completed collector work.
    fn observe_gc_advance(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: Option<WorkerId>,
        advance: heap::GcAdvance,
    ) -> RuntimeResult<bool> {
        match advance {
            heap::GcAdvance::Idle => Ok(false),
            heap::GcAdvance::Started(start) => {
                self.observe_gc_start(runtime_id, worker_id, start.collector)?;

                Ok(true)
            }
            heap::GcAdvance::Stepped(step) => {
                if step.is_start {
                    self.observe_gc_start(runtime_id, worker_id, step.collector)?;
                }

                let observation = Self::gc_step(runtime_id, worker_id, step)?;

                self.advance_moment()?;
                self.observe(observation)?;

                Ok(true)
            }
            heap::GcAdvance::Drop(drop) => {
                if drop.is_start {
                    self.observe_gc_start(runtime_id, worker_id, drop.collector)?;
                }

                let step = heap::GcStep {
                    collector: drop.collector,
                    is_start: drop.is_start,
                    phase: heap::GcPhase::Drop,
                    budget_bytes: drop.budget_bytes,
                    work_bytes: drop.work_bytes,
                };
                let observation = Self::gc_step(runtime_id, worker_id, step)?;

                self.advance_moment()?;
                self.observe(observation)?;

                Ok(true)
            }
            heap::GcAdvance::Completed(cycle) => {
                if cycle.is_start {
                    self.observe_gc_start(runtime_id, worker_id, cycle.collector)?;
                }

                let observation = Self::gc_cycle(runtime_id, worker_id, cycle)?;

                self.advance_moment()?;
                self.observe(observation)?;

                Ok(true)
            }
        }
    }

    /// Observe one GC start.
    fn observe_gc_start(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: Option<WorkerId>,
        collector: heap::GcCollector,
    ) -> RuntimeResult<()> {
        let observation = match (collector, worker_id) {
            (heap::GcCollector::Local, Some(worker_id)) => Observation::LocalGcStarted {
                runtime_id,
                worker_id,
                collector,
            },
            (heap::GcCollector::Shared, _) => Observation::SharedGcStarted {
                runtime_id,
                collector,
            },
            (heap::GcCollector::Local, None) => {
                return Err(RuntimeError::Internal {
                    message: "local gc start missing worker".to_string(),
                }
                .boxed());
            }
        };

        self.advance_moment()?;
        self.observe(observation)?;

        Ok(())
    }

    /// Return the observation for one GC step.
    fn gc_step(
        runtime_id: RuntimeId,
        worker_id: Option<WorkerId>,
        step: heap::GcStep,
    ) -> RuntimeResult<Observation> {
        match (step.collector, worker_id) {
            (heap::GcCollector::Local, Some(worker_id)) => Ok(Observation::LocalGcStepped {
                runtime_id,
                worker_id,
                step,
            }),
            (heap::GcCollector::Shared, worker_id) => Ok(Observation::SharedGcStepped {
                runtime_id,
                worker_id,
                step,
            }),
            (heap::GcCollector::Local, None) => Err(RuntimeError::Internal {
                message: "local gc step missing worker".to_string(),
            }
            .boxed()),
        }
    }

    /// Return the observation for one completed GC cycle.
    fn gc_cycle(
        runtime_id: RuntimeId,
        worker_id: Option<WorkerId>,
        cycle: heap::GcCycle,
    ) -> RuntimeResult<Observation> {
        match (cycle.collector, worker_id) {
            (heap::GcCollector::Local, Some(worker_id)) => Ok(Observation::LocalGcCompleted {
                runtime_id,
                worker_id,
                cycle,
            }),
            (heap::GcCollector::Shared, worker_id) => Ok(Observation::SharedGcCompleted {
                runtime_id,
                worker_id,
                cycle,
            }),
            (heap::GcCollector::Local, None) => Err(RuntimeError::Internal {
                message: "local gc cycle missing worker".to_string(),
            }
            .boxed()),
        }
    }

    /// Convert one runtime run outcome into one world run outcome.
    fn run_runtime_outcome(
        &mut self,
        runtime_id: RuntimeId,
        outcome: RuntimeRunOutcome,
        step: RunStep,
    ) -> RuntimeResult<RunOutcome> {
        match outcome {
            RuntimeRunOutcome::Progressed {
                worker_id,
                progress,
            } => {
                self.advance_moment()?;
                self.observe(step.observation(runtime_id, worker_id, progress))?;

                Ok(RunOutcome::Progressed)
            }
            RuntimeRunOutcome::Idle => Ok(RunOutcome::Idle),
            RuntimeRunOutcome::Stopped {
                worker_id,
                fiber_id,
                reason,
            } => {
                let moment = self.advance_moment()?;
                let stop = Stop {
                    moment,
                    runtime_id,
                    worker_id,
                    fiber_id,
                    reason,
                };
                self.observe(Observation::StopReached {
                    runtime_id,
                    worker_id,
                    fiber_id,
                    reason,
                })?;

                Ok(RunOutcome::Stopped { stop })
            }
            RuntimeRunOutcome::Paused {
                worker_id,
                fiber_id,
                reason,
            } => {
                let stop = Stop {
                    moment: self.moment(),
                    runtime_id,
                    worker_id,
                    fiber_id,
                    reason,
                };

                Ok(RunOutcome::Stopped { stop })
            }
        }
    }
}
