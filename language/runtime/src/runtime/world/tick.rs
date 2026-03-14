use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::{AgentId, TickOutcome};
use destack_workspace::TimeMode;

use super::{Input, Observation, ObservationSchedulerOutcome, RuntimeId, Wake, World};

impl World {
    /// Advance one virtual world clock to one explicit wall-clock deadline.
    pub(crate) fn advance_virtual_to(&self, deadline: WorldInstant) -> RuntimeResult<WorldInstant> {
        // host time cannot be advanced by the deterministic scheduler
        if self.time_mode != TimeMode::Virtual {
            return Err(RuntimeError::HostTimeAdvance.boxed());
        }

        Ok(self.clock.advance_virtual_to(deadline))
    }

    /// Return the earliest deadline contributed by simulation state.
    pub(crate) fn next_simulation_deadline(&self) -> Option<WorldInstant> {
        let simulation = self.simulation.borrow();
        simulation.next_deadline()
    }

    /// Return the earliest deadline across agent-local and world-local timed work.
    pub(crate) fn next_deadline<I>(&self, agent_deadlines: I) -> Option<WorldInstant>
    where
        I: IntoIterator<Item = Option<WorldInstant>>,
    {
        let mut next_deadline = self.next_simulation_deadline();

        for agent_deadline in agent_deadlines {
            next_deadline = min_deadline(next_deadline, agent_deadline);
        }

        next_deadline
    }

    /// Drain due world wakes at one wall-clock timestamp.
    pub(crate) fn drain_due<I>(&self, agent_timers: I) -> Vec<Wake>
    where
        I: IntoIterator<Item = (RuntimeId, AgentId, Timer)>,
    {
        // due agent timers
        let mut wakes = agent_timers
            .into_iter()
            .map(|(runtime_id, agent_id, timer)| Wake::AgentTimer {
                runtime_id,
                agent_id,
                timer,
            })
            .collect::<Vec<_>>();

        // deterministic order
        wakes.sort_by_key(Wake::sort_key);

        wakes
    }

    /// Execute one world tick across all stored runtimes in stable order.
    pub fn tick(&self) -> RuntimeResult<TickOutcome> {
        let input = self.resolve_input(Input::Tick)?;

        if self.trace.mode() == destack_workspace::ExecutionMode::Record {
            self.ingest(input.clone())?;
        }

        self.tick_inner()
    }

    /// Execute one world tick without tracing the outer invocation.
    pub(crate) fn tick_inner(&self) -> RuntimeResult<TickOutcome> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.borrow_mut();

        // runnable work and ingress
        for runtime in runtimes.values_mut() {
            if runtime.tick(self)?.progressed() {
                self.observe(Observation::scheduler(
                    ObservationSchedulerOutcome::Progressed,
                    None,
                ));

                return Ok(TickOutcome::Progressed);
            }
        }

        // host time cannot advance under world control
        if self.time_mode != TimeMode::Virtual {
            return Ok(TickOutcome::Idle);
        }

        // next global deadline
        let next_deadline =
            self.next_deadline(runtimes.values().map(|runtime| runtime.next_deadline(self)));
        let Some(deadline) = next_deadline else {
            return Ok(TickOutcome::Idle);
        };

        // record the resolved world time jump
        let deadline = self.trace.resolve_time_advance(deadline)?;
        self.advance_virtual_to(deadline)?;
        self.trace.record_time_advance(deadline)?;

        // collect due agent timers across runtimes
        let mut agent_timers = Vec::new();
        for runtime in runtimes.values_mut() {
            agent_timers.extend(runtime.collect_due_timers(self)?);
        }

        // deliver due simulation events into the simulation ready queue
        self.simulation.borrow_mut().deliver_due(deadline);

        // drain world timer wakes into per-runtime batches
        let wakes = self.drain_due(agent_timers);
        let mut wakes_by_runtime = BTreeMap::<RuntimeId, Vec<Wake>>::new();
        for wake in wakes {
            match wake {
                Wake::AgentTimer { runtime_id, .. } => {
                    wakes_by_runtime.entry(runtime_id).or_default().push(wake);
                }
            }
        }

        // deliver due agent wakes
        for (runtime_id, runtime) in runtimes.iter_mut() {
            let wakes = wakes_by_runtime.remove(runtime_id).unwrap_or_default();
            if !wakes.is_empty() {
                runtime.deliver_wakes(self, wakes)?;
            }
        }

        self.observe(Observation::scheduler(
            ObservationSchedulerOutcome::AdvancedTime,
            Some(deadline),
        ));

        Ok(TickOutcome::AdvancedTime)
    }

    /// Execute world ticks across all stored runtimes until idle.
    pub fn tick_until_idle(&self) -> RuntimeResult<()> {
        loop {
            if self.tick()? == TickOutcome::Idle {
                return Ok(());
            }
        }
    }

    /// Execute one world tick through one stored runtime.
    #[cfg(test)]
    pub(crate) fn tick_runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<TickOutcome> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.borrow_mut();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        // local runtime progress wins before any virtual time advance
        if runtime.tick(self)?.progressed() {
            self.observe(Observation::scheduler(
                ObservationSchedulerOutcome::Progressed,
                None,
            ));

            return Ok(TickOutcome::Progressed);
        }

        // host time cannot advance under world control
        if self.time_mode != TimeMode::Virtual {
            return Ok(TickOutcome::Idle);
        }

        // single-runtime next deadline
        let next_deadline = self.next_deadline(std::iter::once(runtime.next_deadline(self)));
        let Some(deadline) = next_deadline else {
            return Ok(TickOutcome::Idle);
        };

        // record the resolved world time jump
        let deadline = self.trace.resolve_time_advance(deadline)?;
        self.advance_virtual_to(deadline)?;
        self.trace.record_time_advance(deadline)?;

        // drain only the target runtime wakes
        let agent_timers = runtime.collect_due_timers(self)?;
        self.simulation.borrow_mut().deliver_due(deadline);
        let wakes = self.drain_due(agent_timers);
        runtime.deliver_wakes(self, wakes)?;

        self.observe(Observation::scheduler(
            ObservationSchedulerOutcome::AdvancedTime,
            Some(deadline),
        ));

        Ok(TickOutcome::AdvancedTime)
    }
}

/// Return the earliest non-empty deadline.
fn min_deadline(
    current: Option<WorldInstant>,
    candidate: Option<WorldInstant>,
) -> Option<WorldInstant> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}
