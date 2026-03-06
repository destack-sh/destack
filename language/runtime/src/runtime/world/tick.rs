use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::{AgentId, TickOutcome};
use destack_workspace::TimeMode;

use super::{RuntimeId, Wake, World};

impl World {
    /// Advance one virtual world clock to one explicit wall-clock deadline.
    pub(crate) fn advance_virtual_to(&self, deadline: WorldInstant) -> RuntimeResult<WorldInstant> {
        // host time cannot be advanced by the deterministic scheduler
        if self.time_mode != TimeMode::Virtual {
            return Err(RuntimeError::Internal {
                message: "cannot advance virtual time while world uses host time".to_string(),
            }
            .boxed());
        }

        Ok(self.clock.advance_virtual_to(deadline))
    }

    /// Return the earliest deadline contributed by simulation state.
    pub(crate) fn next_simulation_deadline(&self) -> Option<WorldInstant> {
        let simulation = self.simulation.read();
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
        let mut runtimes = self.runtimes.write();

        // runnable work and ingress
        for runtime in runtimes.values_mut() {
            if runtime.tick(self)?.progressed() {
                return Ok(TickOutcome::Progressed);
            }
        }

        // host time cannot advance under world control
        if self.time_mode != TimeMode::Virtual {
            return Ok(TickOutcome::Idle);
        }

        // next global deadline
        let next_deadline = runtimes
            .values()
            .filter_map(|runtime| runtime.next_deadline(self))
            .min();
        let Some(deadline) = next_deadline else {
            return Ok(TickOutcome::Idle);
        };

        // record the resolved world time jump
        let deadline = self.replay.resolve_tick(deadline)?;
        let _ = self.advance_virtual_to(deadline)?;
        self.replay.record_tick(deadline)?;

        // collect due agent timers across runtimes
        let mut agent_timers = Vec::new();
        for runtime in runtimes.values_mut() {
            agent_timers.extend(runtime.collect_due_timers(self)?);
        }

        // deliver due simulation events into the simulation ready queue
        self.simulation.write().deliver_due(deadline);

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
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime {} does not exist", runtime_id.0),
            }
            .boxed()
        })?;

        // local runtime progress wins before any virtual time advance
        if runtime.tick(self)?.progressed() {
            return Ok(TickOutcome::Progressed);
        }

        // host time cannot advance under world control
        if self.time_mode != TimeMode::Virtual {
            return Ok(TickOutcome::Idle);
        }

        // single-runtime next deadline
        let Some(deadline) = runtime.next_deadline(self) else {
            return Ok(TickOutcome::Idle);
        };

        // record the resolved world time jump
        let deadline = self.replay.resolve_tick(deadline)?;
        let _ = self.advance_virtual_to(deadline)?;
        self.replay.record_tick(deadline)?;

        // drain only the target runtime wakes
        let agent_timers = runtime.collect_due_timers(self)?;
        self.simulation.write().deliver_due(deadline);
        let wakes = self.drain_due(agent_timers);
        runtime.deliver_wakes(self, wakes)?;

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
