use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;
use crate::world::observation::{
    Observation, ObservationEntry, ObservationScope, ObservationSequence,
};
use crate::world::World;

use super::{BranchId, LineageQuery, Moment};

/// One query event at one precise execution coordinate.
#[derive(Debug, Clone)]
pub struct Event {
    /// The observation sequence that orders events at one moment.
    pub sequence: ObservationSequence,
    /// The moment where this event is visible.
    pub moment: Moment,
    /// The emitted observation.
    pub observation: Observation,
}

impl Event {
    /// Return the stable event name.
    pub fn name(&self) -> &str {
        self.observation.name()
    }

    /// Return the event scope.
    pub fn scope(&self) -> ObservationScope {
        self.observation.scope()
    }

    /// Build one event from one observation entry.
    pub(super) fn from_observation(record: ObservationEntry) -> Self {
        Self {
            sequence: record.sequence,
            moment: record.moment,
            observation: record.observation,
        }
    }
}

/// One eager event-query result.
#[derive(Debug, Clone)]
pub struct EventSet {
    /// The events produced by the query.
    events: Vec<Event>,
}

impl EventSet {
    /// Create one event set from one eager event vector.
    pub(super) fn new(mut events: Vec<Event>) -> Self {
        events.sort_by_key(|event| {
            (
                event.moment.branch_id,
                event.moment.sequence,
                event.sequence,
            )
        });

        Self { events }
    }

    /// Return the number of events in this set.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Report whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Return the events in stable query order.
    pub fn as_slice(&self) -> &[Event] {
        &self.events
    }

    /// Consume this set and return its events.
    pub fn into_vec(self) -> Vec<Event> {
        self.events
    }

    /// Keep only events with one exact stable name.
    pub fn name(self, name: &str) -> Self {
        self.filter(|event| event.name() == name)
    }

    /// Keep only events on one exact scope.
    pub fn on(self, scope: ObservationScope) -> Self {
        self.filter(|event| event.observation.is_on(&scope))
    }

    /// Keep only world-scoped events.
    pub fn world(self) -> Self {
        self.on(ObservationScope::world())
    }

    /// Keep only runtime-scoped events.
    pub fn runtime(self, runtime_id: RuntimeId) -> Self {
        self.filter(|event| event.observation.runtime_id() == Some(runtime_id))
    }

    /// Keep only worker-scoped events.
    pub fn worker(self, worker_id: WorkerId) -> Self {
        self.filter(|event| event.observation.worker_id() == Some(worker_id))
    }

    /// Keep only resource-scoped events.
    pub fn resource(self, resource_id: ResourceId) -> Self {
        self.filter(|event| event.observation.resource_id() == Some(resource_id))
    }

    /// Keep only events that satisfy one predicate.
    pub fn filter(mut self, mut predicate: impl FnMut(&Event) -> bool) -> Self {
        self.events.retain(|event| predicate(event));
        self
    }
}

/// One lineage-rooted committed event query.
#[derive(Debug, Clone, Copy)]
pub struct EventQuery<'a> {
    /// The lineage query root that owns the query.
    lineage: LineageQuery<'a>,
}

impl<'a> EventQuery<'a> {
    /// Create one committed event query on one lineage query root.
    pub(super) const fn new(lineage: LineageQuery<'a>) -> Self {
        Self { lineage }
    }

    /// Return every committed event visible on one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        self.lineage.events_on(branch_id)
    }

    /// Return every committed event visible on one branch and its descendants.
    pub fn descendants_of(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        self.lineage.events_descendants_of(branch_id)
    }

    /// Return every committed event up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<EventSet> {
        self.lineage.events_up_to(moment)
    }

    /// Return every committed event in one exact branch-local range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.lineage.events_between(start, end)
    }
}

/// One live branch-local event query rooted in one world.
#[derive(Debug, Clone, Copy)]
pub struct WorldEventQuery<'a> {
    /// The world that owns the query.
    world: &'a World,
}

impl<'a> WorldEventQuery<'a> {
    /// Create one live event query on one world.
    pub(super) const fn new(world: &'a World) -> Self {
        Self { world }
    }

    /// Return every live-branch event up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<EventSet> {
        self.world.events_up_to(moment)
    }

    /// Return every live-branch event in one exact range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.world.events_between(start, end)
    }
}
