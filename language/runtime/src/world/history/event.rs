use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::{RuntimeId, WorkerId};
use crate::world::trace::{
    Observation, ObservationCategory, ObservationRecord, ObservationScope, Outcome, Trace,
    TraceRecord, TraceSequence,
};
use crate::world::{Command, World};

use super::{BranchId, LineageView, Moment};

/// Query-visible event class projected from trace and observation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// One input that entered the world.
    Command,
    /// One observed outcome that replay could not derive.
    Outcome,
    /// One retained or user-visible history anchor.
    Anchor,
    /// One explicit emitted observation.
    Observation,
}

/// Query-visible event payload projected from trace or observation state.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EventPayload {
    /// One projected trace input.
    Command(Command),
    /// One projected trace outcome.
    Outcome(Outcome),
    /// One projected trace anchor.
    Anchor(String),
    /// One explicit observation payload.
    Observation(Observation),
}

/// One normalized query event at one precise execution coordinate.
#[derive(Debug, Clone)]
pub struct Event {
    /// The moment where this event is visible.
    pub moment: Moment,
    /// The event class.
    pub kind: EventKind,
    /// The event payload.
    pub payload: EventPayload,
}

impl Event {
    /// Report whether this event is one projected input.
    pub const fn is_input(&self) -> bool {
        matches!(self.kind, EventKind::Command)
    }

    /// Report whether this event is one projected outcome.
    pub const fn is_outcome(&self) -> bool {
        matches!(self.kind, EventKind::Outcome)
    }

    /// Report whether this event is one projected anchor.
    pub const fn is_anchor(&self) -> bool {
        matches!(self.kind, EventKind::Anchor)
    }

    /// Report whether this event is one emitted observation.
    pub const fn is_observation(&self) -> bool {
        matches!(self.kind, EventKind::Observation)
    }

    /// Return one projected input payload when present.
    pub const fn input(&self) -> Option<&Command> {
        match &self.payload {
            EventPayload::Command(input) => Some(input),
            _ => None,
        }
    }

    /// Return one projected outcome payload when present.
    pub const fn outcome(&self) -> Option<&Outcome> {
        match &self.payload {
            EventPayload::Outcome(outcome) => Some(outcome),
            _ => None,
        }
    }

    /// Return one projected anchor payload when present.
    pub fn anchor(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Anchor(anchor) => Some(anchor.as_str()),
            _ => None,
        }
    }

    /// Return one observation payload when present.
    pub const fn observation(&self) -> Option<&Observation> {
        match &self.payload {
            EventPayload::Observation(observation) => Some(observation),
            _ => None,
        }
    }

    /// Return the stable event name when available.
    pub fn name(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Command(input) => Some(input.name()),
            EventPayload::Outcome(outcome) => Some(outcome.name()),
            EventPayload::Anchor(_) => Some("label"),
            EventPayload::Observation(observation) => Some(observation.name.as_str()),
        }
    }

    /// Return the event category when this event wraps one observation.
    pub const fn category(&self) -> Option<ObservationCategory> {
        match &self.payload {
            EventPayload::Observation(observation) => Some(observation.category),
            _ => None,
        }
    }

    /// Return the event scope when this event wraps one observation.
    pub fn scope(&self) -> Option<&ObservationScope> {
        self.observation().map(|observation| &observation.scope)
    }

    /// Return the event label value when this event wraps one labeled observation.
    pub fn label_value(&self, key: &str) -> Option<&str> {
        self.observation()
            .and_then(|observation| observation.label_value(key))
    }

    /// Build one projected trace event at the moment after one record.
    pub(super) fn from_trace(moment: Moment, record: TraceRecord) -> Self {
        match record {
            TraceRecord::Command(input) => Self {
                moment,
                kind: EventKind::Command,
                payload: EventPayload::Command(input),
            },
            TraceRecord::Outcome(outcome) => Self {
                moment,
                kind: EventKind::Outcome,
                payload: EventPayload::Outcome(outcome),
            },
            TraceRecord::Anchor(anchor) => Self {
                moment,
                kind: EventKind::Anchor,
                payload: EventPayload::Anchor(anchor),
            },
        }
    }

    /// Build one projected observation event.
    pub(super) fn from_observation(record: ObservationRecord) -> Self {
        Self {
            moment: record.moment,
            kind: EventKind::Observation,
            payload: EventPayload::Observation(record.observation),
        }
    }

    /// Return the stable event-order rank at one shared moment.
    pub(super) fn order_rank(&self) -> u8 {
        match self.kind {
            EventKind::Command | EventKind::Outcome | EventKind::Anchor => 0,
            EventKind::Observation => 1,
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
                event.order_rank(),
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

    /// Return the first event in this set, if any.
    pub fn first(&self) -> Option<&Event> {
        self.events.first()
    }

    /// Return the last event in this set, if any.
    pub fn last(&self) -> Option<&Event> {
        self.events.last()
    }

    /// Report whether any event in this set matches one predicate.
    pub fn any(&self, predicate: impl FnMut(&Event) -> bool) -> bool {
        self.events.iter().any(predicate)
    }

    /// Report whether every event in this set matches one predicate.
    pub fn all(&self, predicate: impl FnMut(&Event) -> bool) -> bool {
        self.events.iter().all(predicate)
    }

    /// Keep only events of one class.
    pub fn kind(self, kind: EventKind) -> Self {
        self.filter(|event| event.kind == kind)
    }

    /// Keep only projected input events.
    pub fn inputs(self) -> Self {
        self.kind(EventKind::Command)
    }

    /// Keep only projected outcome events.
    pub fn outcomes(self) -> Self {
        self.kind(EventKind::Outcome)
    }

    /// Keep only projected anchor events.
    pub fn anchors(self) -> Self {
        self.kind(EventKind::Anchor)
    }

    /// Keep only emitted observation events.
    pub fn observations(self) -> Self {
        self.kind(EventKind::Observation)
    }

    /// Keep only events with one exact stable name.
    pub fn name(self, name: &str) -> Self {
        self.filter(|event| event.name() == Some(name))
    }

    /// Keep only observation events in one exact category.
    pub fn category(self, category: ObservationCategory) -> Self {
        self.filter(|event| event.category() == Some(category))
    }

    /// Keep only observation events labeled with one exact key-value pair.
    pub fn label(self, key: &str, value: &str) -> Self {
        self.filter(|event| match event.label_value(key) {
            Some(label) => value == label,
            None => false,
        })
    }

    /// Keep only observation events on one exact scope.
    pub fn on(self, scope: ObservationScope) -> Self {
        self.filter(|event| event.scope() == Some(&scope))
    }

    /// Keep only world-scoped observation events.
    pub fn world(self) -> Self {
        self.on(ObservationScope::world())
    }

    /// Keep only runtime-scoped observation events.
    pub fn runtime(self, runtime_id: RuntimeId) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.runtime_id()) == Some(runtime_id))
    }

    /// Keep only worker-scoped observation events.
    pub fn worker(self, worker_id: WorkerId) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.worker_id()) == Some(worker_id))
    }

    /// Keep only entity-scoped observation events.
    pub fn entity(self, entity_id: &str) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.entity_id()) == Some(entity_id))
    }

    /// Keep only edge-scoped observation events.
    pub fn edge(self, edge_id: &str) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.edge_id()) == Some(edge_id))
    }

    /// Keep only resource-scoped observation events.
    pub fn resource(self, resource_id: ResourceId) -> Self {
        self.filter(|event| {
            event.scope().and_then(|scope| scope.resource_id()) == Some(resource_id)
        })
    }

    /// Keep only events that satisfy one predicate.
    pub fn filter(mut self, mut predicate: impl FnMut(&Event) -> bool) -> Self {
        self.events.retain(|event| predicate(event));
        self
    }

    /// Return one union of this set with one second event set.
    pub fn union(mut self, other: Self) -> Self {
        self.events.extend(other.events);
        Self::new(self.events)
    }
}

/// One lineage-rooted committed event query.
#[derive(Debug, Clone, Copy)]
pub struct EventQuery<'a> {
    /// The lineage view that owns the query.
    lineage: LineageView<'a>,
}

impl<'a> EventQuery<'a> {
    /// Create one committed event query on one lineage view.
    pub(super) const fn new(lineage: LineageView<'a>) -> Self {
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

impl Trace {
    /// Return one authoritative record at one exact sequence.
    pub(super) fn record_at(&self, sequence: TraceSequence) -> RuntimeResult<TraceRecord> {
        self.seek_sequence(sequence)?;
        self.next_event()?.ok_or_else(|| {
            RuntimeError::TraceExhausted {
                sequence: sequence.get(),
            }
            .boxed()
        })
    }

    /// Project query events for one branch-local trace range.
    pub(super) fn events_between_on_branch(
        &self,
        branch_id: BranchId,
        start: TraceSequence,
        end: TraceSequence,
    ) -> RuntimeResult<Vec<Event>> {
        let mut events = Vec::new();
        self.seek_sequence(start)?;

        while self.sequence()? != end {
            let record = self.next_event()?.ok_or_else(|| {
                RuntimeError::TraceExhausted {
                    sequence: end.get(),
                }
                .boxed()
            })?;
            let moment = Moment::new(branch_id, self.sequence()?);
            events.push(Event::from_trace(moment, record));
        }

        Ok(events)
    }
}
