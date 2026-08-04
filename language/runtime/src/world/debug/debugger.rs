use serde::{Deserialize, Serialize};

use destack_program as program;

use crate::worker::WorkerId;
use crate::world::{ProbeId, RuntimeId};

use super::{Breakpoint, Probe, Watchpoint};

/// Debugger configuration for one world.
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

        self.breakpoints.swap_remove(index);
        self.advance_generation();

        true
    }

    /// Set one breakpoint enabled state.
    pub fn set_breakpoint_enabled(&mut self, id: program::BreakpointId, is_enabled: bool) -> bool {
        let Some(breakpoint) = self
            .breakpoints
            .iter_mut()
            .find(|breakpoint| breakpoint.id == id)
        else {
            return false;
        };

        breakpoint.is_enabled = is_enabled;
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

        self.watchpoints.swap_remove(index);
        self.advance_generation();

        true
    }

    /// Set one watchpoint enabled state.
    pub fn set_watchpoint_enabled(&mut self, id: program::WatchpointId, is_enabled: bool) -> bool {
        let Some(watchpoint) = self
            .watchpoints
            .iter_mut()
            .find(|watchpoint| watchpoint.id == id)
        else {
            return false;
        };

        watchpoint.is_enabled = is_enabled;
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

        self.probes.swap_remove(index);
        self.advance_generation();

        true
    }

    /// Set one probe enabled state.
    pub fn set_probe_enabled(&mut self, id: ProbeId, is_enabled: bool) -> bool {
        let Some(probe) = self.probes.iter_mut().find(|probe| probe.id == id) else {
            return false;
        };

        probe.is_enabled = is_enabled;
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
            if !breakpoint.target.selects(runtime_id, worker_id) {
                continue;
            }

            instructions.push(program::StopPoint::new(
                breakpoint.target.point,
                program::StopReason::Breakpoint {
                    breakpoint_id: breakpoint.id,
                    point: breakpoint.target.point,
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
            if !watchpoint.selects(runtime_id, worker_id) {
                continue;
            }

            memory.push(watchpoint.memory_stop());
        }

        program::WatchSet::new(memory)
    }

    /// Advance the debugger generation after one configuration mutation.
    fn advance_generation(&mut self) {
        self.generation += 1;
    }
}
