use serde::{Deserialize, Serialize};

use crate::runtime::time::Instant;

/// Simulation state for one deterministic world.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Simulation {
    /// Simulation schema version.
    pub version: u32,
    /// Scheduled simulation events in world time.
    scheduled_events: Vec<SimulationEvent>,
    /// Delivered simulation events ready for subsystem handling.
    ready_events: Vec<SimulationEvent>,
    /// Next deterministic event sequence.
    next_sequence: u64,
}

impl Simulation {
    /// Schedule one simulation event at one explicit world instant.
    pub fn schedule_event(&mut self, at: Instant) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);

        self.scheduled_events.push(SimulationEvent { at, sequence });

        sequence
    }

    /// Return the earliest scheduled simulation deadline.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.scheduled_events.iter().map(SimulationEvent::at).min()
    }

    /// Deliver simulation events that became due at one world timestamp.
    pub fn deliver_due(&mut self, now: Instant) -> usize {
        // partition due and pending events
        let mut due_events = Vec::new();
        let mut pending_events = Vec::with_capacity(self.scheduled_events.len());

        for event in self.scheduled_events.drain(..) {
            if event.at() <= now {
                due_events.push(event);
            } else {
                pending_events.push(event);
            }
        }

        self.scheduled_events = pending_events;

        // deterministic order
        due_events.sort_by_key(SimulationEvent::sort_key);
        let delivered_count = due_events.len();
        self.ready_events.extend(due_events);

        delivered_count
    }

    /// Return delivered simulation events waiting for handling.
    pub fn ready_events(&self) -> &[SimulationEvent] {
        &self.ready_events
    }

    /// Drain delivered simulation events waiting for handling.
    pub fn take_ready_events(&mut self) -> Vec<SimulationEvent> {
        std::mem::take(&mut self.ready_events)
    }
}

/// One scheduled simulation event in world time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationEvent {
    /// Due world instant.
    at: Instant,
    /// Stable insertion order.
    sequence: u64,
}

impl SimulationEvent {
    /// Return the due world instant.
    pub const fn at(&self) -> Instant {
        self.at
    }

    /// Return the deterministic event sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Return the stable ordering key for this event.
    pub(crate) const fn sort_key(&self) -> (Instant, u64) {
        (self.at, self.sequence)
    }
}
