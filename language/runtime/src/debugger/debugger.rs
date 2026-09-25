use serde::{Deserialize, Serialize};

use tspp_program as program;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

use super::{Breakpoint, Probe, ProbeAction, ProbeId, Watchpoint};

/// Debugger state for one World.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Debugger {
    /// Active breakpoints.
    breakpoints: Vec<Breakpoint>,
    /// Active watchpoints.
    watchpoints: Vec<Watchpoint>,
    /// Active probes.
    probes: Vec<Probe>,

    /// Next breakpoint id to allocate.
    next_breakpoint_id: u64,
    /// Next watchpoint id to allocate.
    next_watchpoint_id: u64,
    /// Next probe id to allocate.
    next_probe_id: u64,

    /// Monotonic debugger configuration generation.
    generation: u64,
}

impl Debugger {
    /// Return whether no debugger entries are active.
    pub fn is_empty(&self) -> bool {
        self.breakpoints.is_empty() && self.watchpoints.is_empty() && self.probes.is_empty()
    }

    /// Return active breakpoints.
    pub fn breakpoints(&self) -> &[Breakpoint] {
        self.breakpoints.as_slice()
    }

    /// Return active watchpoints.
    pub fn watchpoints(&self) -> &[Watchpoint] {
        self.watchpoints.as_slice()
    }

    /// Return active probes.
    pub fn probes(&self) -> &[Probe] {
        self.probes.as_slice()
    }

    /// Return the debugger configuration generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Allocate one user breakpoint id.
    pub fn allocate_breakpoint_id(&mut self) -> program::BreakpointId {
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id = id + 1;

        program::BreakpointId::new(id)
    }

    /// Allocate one watchpoint id.
    pub fn allocate_watchpoint_id(&mut self) -> program::WatchpointId {
        let id = self.next_watchpoint_id;
        self.next_watchpoint_id = id + 1;

        program::WatchpointId::new(id)
    }

    /// Allocate one probe id.
    pub fn allocate_probe_id(&mut self) -> ProbeId {
        let id = self.next_probe_id;
        self.next_probe_id = id + 1;

        ProbeId::new(id)
    }

    /// Add one breakpoint.
    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
        self.advance_generation();
    }

    /// Update one breakpoint.
    pub fn update_breakpoint(&mut self, breakpoint: Breakpoint) -> bool {
        let Some(existing) = self
            .breakpoints
            .iter_mut()
            .find(|existing| existing.id == breakpoint.id)
        else {
            return false;
        };

        *existing = breakpoint;
        self.advance_generation();

        true
    }

    /// Remove one breakpoint.
    pub fn remove_breakpoint(&mut self, id: program::BreakpointId) -> bool {
        let Some(index) = self
            .breakpoints
            .iter()
            .position(|breakpoint| breakpoint.id == id)
        else {
            return false;
        };

        self.breakpoints.remove(index);
        self.advance_generation();

        true
    }

    /// Add one watchpoint.
    pub fn add_watchpoint(&mut self, watchpoint: Watchpoint) {
        self.watchpoints.push(watchpoint);
        self.advance_generation();
    }

    /// Update one watchpoint.
    pub fn update_watchpoint(&mut self, watchpoint: Watchpoint) -> bool {
        let Some(existing) = self
            .watchpoints
            .iter_mut()
            .find(|existing| existing.id == watchpoint.id)
        else {
            return false;
        };

        *existing = watchpoint;
        self.advance_generation();

        true
    }

    /// Remove one watchpoint.
    pub fn remove_watchpoint(&mut self, id: program::WatchpointId) -> bool {
        let Some(index) = self
            .watchpoints
            .iter()
            .position(|watchpoint| watchpoint.id == id)
        else {
            return false;
        };

        self.watchpoints.remove(index);
        self.advance_generation();

        true
    }

    /// Add one probe.
    pub fn add_probe(&mut self, probe: Probe) {
        self.probes.push(probe);
        self.advance_generation();
    }

    /// Update one probe.
    pub fn update_probe(&mut self, probe: Probe) -> bool {
        let Some(existing) = self
            .probes
            .iter_mut()
            .find(|existing| existing.id == probe.id)
        else {
            return false;
        };

        *existing = probe;
        self.advance_generation();

        true
    }

    /// Remove one probe.
    pub fn remove_probe(&mut self, id: ProbeId) -> bool {
        let Some(index) = self.probes.iter().position(|probe| probe.id == id) else {
            return false;
        };

        self.probes.remove(index);
        self.advance_generation();

        true
    }

    /// Build the executable stop set for one worker.
    pub fn stop_set(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> program::StopSet {
        let mut instructions = Vec::new();

        // collect executable instruction stops
        for breakpoint in &self.breakpoints {
            if !breakpoint.is_enabled {
                continue;
            }
            if !breakpoint.filter.selects_worker(runtime_id, worker_id) {
                continue;
            }

            instructions.push(program::StopPoint::new(
                breakpoint.filter.point,
                program::StopReason::Breakpoint {
                    breakpoint_id: breakpoint.id,
                    point: breakpoint.filter.point,
                },
            ));
        }

        program::StopSet::new(instructions)
    }

    /// Build the executable watch set for one worker.
    pub fn watch_set(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> program::WatchSet {
        let mut memory = Vec::new();

        // collect executable memory stops
        for watchpoint in &self.watchpoints {
            if !watchpoint.is_enabled {
                continue;
            }
            if !watchpoint.filter.selects_worker(runtime_id, worker_id) {
                continue;
            }

            memory.push(watchpoint.memory_stop());
        }

        program::WatchSet::new(memory)
    }

    /// Return the Program event categories probed for one Worker.
    pub fn probe_events(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> program::EventSet {
        let mut events = program::EventSet::default();

        // collect enabled Probe categories for this exact Worker
        for probe in &self.probes {
            let probe = *probe;
            if !probe.is_enabled || !probe.filter.selects_worker(runtime_id, worker_id) {
                continue;
            }

            events.insert(probe.filter.event.kind());
        }

        events
    }

    /// Process one instrumentable Program execution event.
    pub fn probe(
        &mut self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        fiber_id: Option<program::FiberId>,
        event: program::Event,
        mut observe: impl FnMut(ProbeId, u64) -> RuntimeResult<()>,
    ) -> RuntimeResult<()> {
        for probe in &mut self.probes {
            if !probe.is_enabled || !probe.filter.selects(runtime_id, worker_id, fiber_id, event) {
                continue;
            }

            // advance the durable hit count for cadence and inspection
            probe.hit_count = probe.hit_count.checked_add(1).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("Probe {} matching-event count exhausted", probe.id.get()),
                }
                .boxed()
            })?;

            // materialize only observations selected by the configured cadence
            if let ProbeAction::Observe { interval } = probe.action
                && probe.hit_count % interval.get() == 0
            {
                observe(probe.id, probe.hit_count)?;
            }
        }

        Ok(())
    }

    /// Advance the debugger generation after one configuration mutation.
    fn advance_generation(&mut self) {
        self.generation += 1;
    }
}
