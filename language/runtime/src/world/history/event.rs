use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::{RuntimeId, WorkerId};
use crate::world::trace::{
    EntrypointCall, Observation, ObservationCategory, ObservationEntry, ObservationScope, Outcome,
    Trace, TraceRecord, TraceSequence,
};
use crate::world::{Mutation, World};

use super::{BranchId, HistoryQuery, Moment};

/// Query-visible event class projected from trace and custom event state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// One input that entered the world.
    Mutation,
    /// One entrypoint call that entered the world.
    Entrypoint,
    /// One observed outcome that replay could not derive.
    Outcome,
    /// One retained or user-visible history label.
    Label,
    /// One emitted custom event.
    Custom,
}

/// Query-visible event payload projected from trace or custom event state.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EventPayload {
    /// One projected trace input.
    Mutation(Mutation),
    /// One projected entrypoint call.
    Entrypoint(EntrypointCall),
    /// One projected trace outcome.
    Outcome(Outcome),
    /// One projected trace label.
    Label(String),
    /// One emitted custom event payload.
    Custom(Observation),
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
        matches!(self.kind, EventKind::Mutation | EventKind::Entrypoint)
    }

    /// Report whether this event is one projected outcome.
    pub const fn is_outcome(&self) -> bool {
        matches!(self.kind, EventKind::Outcome)
    }

    /// Report whether this event is one projected label.
    pub const fn is_label(&self) -> bool {
        matches!(self.kind, EventKind::Label)
    }

    /// Report whether this event is one emitted custom event.
    pub const fn is_custom(&self) -> bool {
        matches!(self.kind, EventKind::Custom)
    }

    /// Return one projected input payload when present.
    pub const fn input(&self) -> Option<&Mutation> {
        match &self.payload {
            EventPayload::Mutation(input) => Some(input),
            _ => None,
        }
    }

    /// Return one projected entrypoint call when present.
    pub const fn entrypoint(&self) -> Option<&EntrypointCall> {
        match &self.payload {
            EventPayload::Entrypoint(invocation) => Some(invocation),
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

    /// Return one projected label payload when present.
    pub fn label(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Label(label) => Some(label.as_str()),
            _ => None,
        }
    }

    /// Return one custom event payload when present.
    pub const fn custom(&self) -> Option<&Observation> {
        match &self.payload {
            EventPayload::Custom(event) => Some(event),
            _ => None,
        }
    }

    /// Return the stable event name when available.
    pub fn name(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Mutation(input) => Some(input.name()),
            EventPayload::Entrypoint(_) => Some("runtime.instance.entrypoint.run"),
            EventPayload::Outcome(outcome) => Some(outcome.name()),
            EventPayload::Label(_) => Some("label"),
            EventPayload::Custom(event) => Some(event.name.as_str()),
        }
    }

    /// Return the event category when this event wraps one custom event.
    pub const fn category(&self) -> Option<ObservationCategory> {
        match &self.payload {
            EventPayload::Custom(event) => Some(event.category),
            _ => None,
        }
    }

    /// Return the event scope when this event wraps one custom event.
    pub fn scope(&self) -> Option<&ObservationScope> {
        self.custom().map(|event| &event.scope)
    }

    /// Return the event label value when this event wraps one labeled custom event.
    pub fn label_value(&self, key: &str) -> Option<&str> {
        self.custom().and_then(|event| event.label_value(key))
    }

    /// Build one projected trace event at the moment after one record.
    pub(super) fn from_trace(moment: Moment, record: TraceRecord) -> Self {
        match record {
            TraceRecord::Mutation(input) => Self {
                moment,
                kind: EventKind::Mutation,
                payload: EventPayload::Mutation(input),
            },
            TraceRecord::Entrypoint(invocation) => Self {
                moment,
                kind: EventKind::Entrypoint,
                payload: EventPayload::Entrypoint(invocation),
            },
            TraceRecord::Outcome(outcome) => Self {
                moment,
                kind: EventKind::Outcome,
                payload: EventPayload::Outcome(outcome),
            },
            TraceRecord::Label(label) => Self {
                moment,
                kind: EventKind::Label,
                payload: EventPayload::Label(label),
            },
        }
    }

    /// Build one projected custom event.
    pub(super) fn from_observation(record: ObservationEntry) -> Self {
        Self {
            moment: record.moment,
            kind: EventKind::Custom,
            payload: EventPayload::Custom(record.observation),
        }
    }

    /// Return the stable event-order rank at one shared moment.
    pub(super) fn order_rank(&self) -> u8 {
        match self.kind {
            EventKind::Mutation | EventKind::Entrypoint | EventKind::Outcome | EventKind::Label => {
                0
            }
            EventKind::Custom => 1,
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

    /// Keep only events of one class.
    pub fn kind(self, kind: EventKind) -> Self {
        self.filter(|event| event.kind == kind)
    }

    /// Keep only projected input events.
    pub fn inputs(self) -> Self {
        self.filter(Event::is_input)
    }

    /// Keep only projected outcome events.
    pub fn outcomes(self) -> Self {
        self.kind(EventKind::Outcome)
    }

    /// Keep only projected label events.
    pub fn labels(self) -> Self {
        self.kind(EventKind::Label)
    }

    /// Keep only emitted custom events.
    pub fn custom(self) -> Self {
        self.kind(EventKind::Custom)
    }

    /// Keep only events with one exact stable name.
    pub fn name(self, name: &str) -> Self {
        self.filter(|event| event.name() == Some(name))
    }

    /// Keep only custom events in one exact category.
    pub fn category(self, category: ObservationCategory) -> Self {
        self.filter(|event| event.category() == Some(category))
    }

    /// Keep only custom events labeled with one exact key-value pair.
    pub fn label(self, key: &str, value: &str) -> Self {
        self.filter(|event| match event.label_value(key) {
            Some(label) => value == label,
            None => false,
        })
    }

    /// Keep only custom events on one exact scope.
    pub fn on(self, scope: ObservationScope) -> Self {
        self.filter(|event| event.scope() == Some(&scope))
    }

    /// Keep only world-scoped custom events.
    pub fn world(self) -> Self {
        self.on(ObservationScope::world())
    }

    /// Keep only runtime-scoped custom events.
    pub fn runtime(self, runtime_id: RuntimeId) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.runtime_id()) == Some(runtime_id))
    }

    /// Keep only worker-scoped custom events.
    pub fn worker(self, worker_id: WorkerId) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.worker_id()) == Some(worker_id))
    }

    /// Keep only entity-scoped custom events.
    pub fn entity(self, entity_id: &str) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.entity_id()) == Some(entity_id))
    }

    /// Keep only edge-scoped custom events.
    pub fn edge(self, edge_id: &str) -> Self {
        self.filter(|event| event.scope().and_then(|scope| scope.edge_id()) == Some(edge_id))
    }

    /// Keep only resource-scoped custom events.
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
}

/// One history-rooted committed event query.
#[derive(Debug, Clone, Copy)]
pub struct EventQuery<'a> {
    /// The history query root that owns the query.
    history: HistoryQuery<'a>,
}

impl<'a> EventQuery<'a> {
    /// Create one committed event query on one history query root.
    pub(super) const fn new(history: HistoryQuery<'a>) -> Self {
        Self { history }
    }

    /// Return every committed event visible on one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        self.history.events_on(branch_id)
    }

    /// Return every committed event visible on one branch and its descendants.
    pub fn descendants_of(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        self.history.events_descendants_of(branch_id)
    }

    /// Return every committed event up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<EventSet> {
        self.history.events_up_to(moment)
    }

    /// Return every committed event in one exact branch-local range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.history.events_between(start, end)
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
