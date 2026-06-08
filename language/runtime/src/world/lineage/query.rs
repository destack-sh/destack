use destack_repository::ExecutionMode;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::trace::{Trace, TraceRecord, TraceSequence};

use super::{
    Branch, BranchId, Divergence, Event, EventQuery, EventSet, Moment, Transition, TransitionQuery,
    TransitionSet, WorldEventQuery, WorldTransitionQuery, WorldView,
};

/// One eager branch-query result.
#[derive(Debug, Clone)]
pub struct BranchSet {
    /// The branches produced by the query.
    branches: Vec<Branch>,
}

impl BranchSet {
    /// Create one branch set from one eager branch vector.
    fn new(mut branches: Vec<Branch>) -> Self {
        branches.sort_by_key(|branch| branch.id);
        Self { branches }
    }

    /// Return the number of branches in this set.
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Report whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// Return the branches in stable query order.
    pub fn as_slice(&self) -> &[Branch] {
        &self.branches
    }

    /// Consume this set and return its branches.
    pub fn into_vec(self) -> Vec<Branch> {
        self.branches
    }

    /// Return the branch identifiers in stable query order.
    pub fn ids(&self) -> Vec<BranchId> {
        self.branches.iter().map(|branch| branch.id).collect()
    }
}

/// One eager moment-query result.
#[derive(Debug, Clone)]
pub struct MomentSet {
    /// The moments produced by the query.
    moments: Vec<Moment>,
}

impl MomentSet {
    /// Create one moment set from one eager moment vector.
    fn new(mut moments: Vec<Moment>) -> Self {
        moments.sort_by_key(|moment| (moment.branch_id, moment.sequence));
        Self { moments }
    }

    /// Return the number of moments in this set.
    pub fn len(&self) -> usize {
        self.moments.len()
    }

    /// Report whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.moments.is_empty()
    }

    /// Return the moments in stable query order.
    pub fn as_slice(&self) -> &[Moment] {
        &self.moments
    }

    /// Consume this set and return its moments.
    pub fn into_vec(self) -> Vec<Moment> {
        self.moments
    }

    /// Return the first moment in this set by value.
    pub fn start(&self) -> Option<Moment> {
        self.moments.first().copied()
    }

    /// Return the last moment in this set by value.
    pub fn end(&self) -> Option<Moment> {
        self.moments.last().copied()
    }
}

/// One lineage-rooted committed moment query.
#[derive(Debug, Clone, Copy)]
pub struct MomentQuery<'a> {
    /// The lineage query root that owns the query.
    lineage: LineageQuery<'a>,
}

impl<'a> MomentQuery<'a> {
    /// Return every committed moment visible on one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<MomentSet> {
        self.lineage.moments_on(branch_id)
    }

    /// Return every committed moment up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<MomentSet> {
        self.lineage.moments_up_to(moment)
    }

    /// Return every committed moment in one exact branch-local range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<MomentSet> {
        self.lineage.moments_between(start, end)
    }
}

/// One committed-lineage query root over shared lineage.
#[derive(Debug, Clone, Copy)]
pub struct LineageQuery<'a> {
    /// The world that owns the shared lineage.
    world: &'a World,
}

impl<'a> LineageQuery<'a> {
    /// Return metadata for one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<Branch> {
        let lineage = self.world.lineage.read();
        lineage.branch(branch_id)
    }

    /// Return every known branch in stable lineage order.
    pub fn branches(self) -> BranchSet {
        let lineage = self.world.lineage.read();
        let branches = lineage.branches.values().cloned().collect();

        BranchSet::new(branches)
    }

    /// Return one committed moment query root.
    pub fn moments(self) -> MomentQuery<'a> {
        MomentQuery { lineage: self }
    }

    /// Return one committed event query root.
    pub fn events(self) -> EventQuery<'a> {
        EventQuery::new(self)
    }

    /// Return one committed transition query root.
    pub fn transitions(self) -> TransitionQuery<'a> {
        TransitionQuery::new(self)
    }

    /// Return every branch that descends from one ancestor branch.
    pub fn descendants_of(self, branch_id: BranchId) -> RuntimeResult<BranchSet> {
        let lineage = self.world.lineage.read();
        let branches = lineage.descendant_branches(branch_id)?;

        Ok(BranchSet::new(branches))
    }

    /// Return the branch-origin moment for one branch.
    pub fn branch_origin_moment(self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let lineage = self.world.lineage.read();
        lineage.branch_origin_moment(branch_id)
    }

    /// Return the committed head moment for one branch.
    pub fn branch_head_moment(self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let lineage = self.world.lineage.read();
        lineage.branch_head_moment(branch_id)
    }

    /// Return one exact committed world view at one moment.
    pub fn view(self, moment: Moment) -> RuntimeResult<WorldView> {
        self.require_committed_query_range(
            Moment::new(moment.branch_id, TraceSequence::new(0)),
            moment,
        )?;

        let image = self.world.image_at_moment(moment)?;

        Ok(WorldView::new(moment, image))
    }

    /// Return the committed divergence moment shared by two branches.
    pub fn divergence_moment(
        self,
        left_branch_id: BranchId,
        right_branch_id: BranchId,
    ) -> RuntimeResult<Moment> {
        Ok(self.divergence(left_branch_id, right_branch_id)?.base)
    }

    /// Return one committed branch divergence summary.
    pub fn divergence(
        self,
        left_branch_id: BranchId,
        right_branch_id: BranchId,
    ) -> RuntimeResult<Divergence> {
        let lineage = self.world.lineage.read();
        let revision = lineage.common_ancestor_revision(left_branch_id, right_branch_id)?;
        let left = lineage.branch_head_moment(left_branch_id)?;
        let right = lineage.branch_head_moment(right_branch_id)?;

        Ok(Divergence {
            base: Moment::new(revision.branch_id, revision.sequence),
            left,
            right,
        })
    }

    /// Return every committed moment visible on one branch.
    pub fn moments_on(self, branch_id: BranchId) -> RuntimeResult<MomentSet> {
        let end = self.branch_head_moment(branch_id)?;
        self.moments_between(Moment::new(branch_id, TraceSequence::new(0)), end)
    }

    /// Return every committed moment up to one target moment.
    pub fn moments_up_to(self, moment: Moment) -> RuntimeResult<MomentSet> {
        let start = Moment::new(moment.branch_id, TraceSequence::new(0));
        self.moments_between(start, moment)
    }

    /// Return every committed moment in one exact branch-local range.
    pub fn moments_between(self, start: Moment, end: Moment) -> RuntimeResult<MomentSet> {
        self.require_committed_query_range(start, end)?;

        let moments = (start.sequence.get()..=end.sequence.get())
            .map(|sequence| Moment::new(start.branch_id, TraceSequence::new(sequence)))
            .collect();

        Ok(MomentSet::new(moments))
    }

    /// Return every committed query event visible on one branch.
    pub fn events_on(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        let end = self.branch_head_moment(branch_id)?;
        self.events_between(Moment::new(branch_id, TraceSequence::new(0)), end)
    }

    /// Return every committed query event visible on one branch and its descendants.
    pub fn events_descendants_of(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        let descendants = self.descendants_of(branch_id)?;
        let mut events = Vec::new();

        for branch in descendants.into_vec() {
            let branch_events = self.events_on(branch.id)?;
            events.extend(branch_events.into_vec());
        }

        Ok(EventSet::new(events))
    }

    /// Return every committed query event up to one target moment.
    pub fn events_up_to(self, moment: Moment) -> RuntimeResult<EventSet> {
        let start = Moment::new(moment.branch_id, TraceSequence::new(0));
        self.events_between(start, moment)
    }

    /// Return every committed query event in one exact branch-local range.
    pub fn events_between(self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.require_committed_query_range(start, end)?;

        let mut events = self.trace_events_between(start, end)?;
        events.extend(self.observation_events_between(start, end)?);

        Ok(EventSet::new(events))
    }

    /// Return every committed transition visible on one branch.
    pub fn transitions_on(self, branch_id: BranchId) -> RuntimeResult<TransitionSet> {
        let end = self.branch_head_moment(branch_id)?;
        self.transitions_between(Moment::new(branch_id, TraceSequence::new(0)), end)
    }

    /// Return every committed transition visible on one branch and its descendants.
    pub fn transitions_descendants_of(self, branch_id: BranchId) -> RuntimeResult<TransitionSet> {
        let descendants = self.descendants_of(branch_id)?;
        let mut transitions = Vec::new();

        for branch in descendants.into_vec() {
            let branch_transitions = self.transitions_on(branch.id)?;
            transitions.extend(branch_transitions.into_vec());
        }

        Ok(TransitionSet::new(transitions))
    }

    /// Return every committed transition up to one target moment.
    pub fn transitions_up_to(self, moment: Moment) -> RuntimeResult<TransitionSet> {
        let start = Moment::new(moment.branch_id, TraceSequence::new(0));
        self.transitions_between(start, moment)
    }

    /// Return every committed transition in one exact branch-local range.
    pub fn transitions_between(self, start: Moment, end: Moment) -> RuntimeResult<TransitionSet> {
        self.require_committed_query_range(start, end)?;

        let transitions = self.trace_transitions_between(start, end)?;
        Ok(TransitionSet::new(transitions))
    }

    /// Validate one committed-lineage query range.
    fn require_committed_query_range(self, start: Moment, end: Moment) -> RuntimeResult<()> {
        if start.branch_id != end.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                start.branch_id.get(),
                end.branch_id.get(),
            )
            .boxed());
        }

        if start.sequence.get() > end.sequence.get() {
            return Err(RuntimeError::Internal {
                message: "moment query start sequence must be at or before the end sequence"
                    .to_string(),
            }
            .boxed());
        }

        let committed_head = self.branch_head_moment(end.branch_id)?;
        if end.sequence.get() > committed_head.sequence.get() {
            return Err(
                RuntimeError::moment_not_found(end.branch_id.get(), end.sequence.get()).boxed(),
            );
        }

        Ok(())
    }

    /// Project committed trace records into query events for one range.
    fn trace_events_between(self, start: Moment, end: Moment) -> RuntimeResult<Vec<Event>> {
        let trace = self.replay_trace_for_branch(end.branch_id)?;
        trace.events_between_on_branch(end.branch_id, start.sequence, end.sequence)
    }

    /// Project committed observations into query events for one range.
    fn observation_events_between(self, start: Moment, end: Moment) -> RuntimeResult<Vec<Event>> {
        let lineage = self.world.lineage.read();
        let records = lineage.observation_records_between(start, end)?;

        Ok(records.into_iter().map(Event::from_observation).collect())
    }

    /// Derive committed transitions from trace records for one range.
    fn trace_transitions_between(
        self,
        start: Moment,
        end: Moment,
    ) -> RuntimeResult<Vec<Transition>> {
        let trace = self.replay_trace_for_branch(end.branch_id)?;
        trace.transitions_between_on_branch(end.branch_id, start.sequence, end.sequence)
    }

    /// Build one replay trace for the committed head of one branch.
    fn replay_trace_for_branch(self, branch_id: BranchId) -> RuntimeResult<Trace> {
        let trace_image = {
            let lineage = self.world.lineage.read();
            let branch = lineage.branch(branch_id)?;
            self.world.trace_image(branch.head_revision_id)?
        };

        Trace::replay_from_image(&trace_image)
    }

    /// Resolve one authoritative trace record for one committed transition.
    pub fn transition_record(self, transition: &Transition) -> RuntimeResult<TraceRecord> {
        let trace = self.replay_trace_for_branch(transition.after.branch_id)?;
        trace.record_at(transition.cause_sequence)
    }
}

impl World {
    /// Return one live branch-local event query root.
    pub fn events(&self) -> WorldEventQuery<'_> {
        WorldEventQuery::new(self)
    }

    /// Return one live branch-local transition query root.
    pub fn transitions(&self) -> WorldTransitionQuery<'_> {
        WorldTransitionQuery::new(self)
    }

    /// Return one committed-lineage query root over shared lineage.
    pub fn lineage(&self) -> LineageQuery<'_> {
        LineageQuery { world: self }
    }

    /// Return every query event after one start moment and up to one end moment.
    pub fn events_between(&self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.require_query_range(start, end)?;

        let mut events = self.trace_events_between(start.sequence, end.sequence)?;
        events.extend(self.observation_events_between(start, end)?);

        Ok(EventSet::new(events))
    }

    /// Return every query event up to one target moment.
    pub fn events_up_to(&self, moment: Moment) -> RuntimeResult<EventSet> {
        let start = Moment::new(moment.branch_id, TraceSequence::new(0));
        self.events_between(start, moment)
    }

    /// Return every derived transition after one start moment and up to one end moment.
    pub fn transitions_between(&self, start: Moment, end: Moment) -> RuntimeResult<TransitionSet> {
        self.require_query_range(start, end)?;

        let transitions = self.trace_transitions_between(start.sequence, end.sequence)?;
        Ok(TransitionSet::new(transitions))
    }

    /// Return every derived transition up to one target moment.
    pub fn transitions_up_to(&self, moment: Moment) -> RuntimeResult<TransitionSet> {
        let start = Moment::new(moment.branch_id, TraceSequence::new(0));
        self.transitions_between(start, moment)
    }

    /// Resolve one authoritative trace record for one branch-local transition.
    pub fn transition_record(&self, transition: &Transition) -> RuntimeResult<TraceRecord> {
        if transition.after.branch_id != self.state.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                transition.after.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        let trace = Trace::from_log(ExecutionMode::Replay, self.state.trace.log().clone());
        trace.record_at(transition.cause_sequence)
    }

    /// Validate one query range against the active world branch.
    fn require_query_range(&self, start: Moment, end: Moment) -> RuntimeResult<()> {
        if start.branch_id != self.state.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                start.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        if end.branch_id != self.state.branch_id {
            return Err(RuntimeError::moment_branch_mismatch(
                end.branch_id.get(),
                self.state.branch_id.get(),
            )
            .boxed());
        }

        if start.sequence.get() > end.sequence.get() {
            return Err(RuntimeError::Internal {
                message: "moment query start sequence must be at or before the end sequence"
                    .to_string(),
            }
            .boxed());
        }

        let current_moment = self.moment();
        if end.sequence.get() > current_moment.sequence.get() {
            return Err(
                RuntimeError::moment_not_found(end.branch_id.get(), end.sequence.get()).boxed(),
            );
        }

        Ok(())
    }

    /// Project trace records into query events for one sequence range.
    fn trace_events_between(
        &self,
        start: TraceSequence,
        end: TraceSequence,
    ) -> RuntimeResult<Vec<Event>> {
        let trace = Trace::from_log(ExecutionMode::Replay, self.state.trace.log().clone());
        trace.events_between_on_branch(self.state.branch_id, start, end)
    }

    /// Project committed and live observation entries into query events for one range.
    fn observation_events_between(&self, start: Moment, end: Moment) -> RuntimeResult<Vec<Event>> {
        let committed_head = {
            let lineage = self.lineage.read();
            lineage.branch_head_moment(self.state.branch_id)?
        };

        let mut events = Vec::new();

        if start.sequence.get() < committed_head.sequence.get() {
            let committed_end = Moment::new(
                self.state.branch_id,
                TraceSequence::new(end.sequence.get().min(committed_head.sequence.get())),
            );
            let lineage = self.lineage.read();
            let records = lineage.observation_records_between(start, committed_end)?;
            events.extend(records.into_iter().map(Event::from_observation));
        }

        if end.sequence.get() > committed_head.sequence.get() {
            let local_start = Moment::new(
                self.state.branch_id,
                TraceSequence::new(start.sequence.get().max(committed_head.sequence.get())),
            );
            let records = self.state.observations.records_between(local_start, end);
            events.extend(records.into_iter().map(Event::from_observation));
        }

        Ok(events)
    }

    /// Derive transitions from trace records for one sequence range.
    fn trace_transitions_between(
        &self,
        start: TraceSequence,
        end: TraceSequence,
    ) -> RuntimeResult<Vec<Transition>> {
        let trace = Trace::from_log(ExecutionMode::Replay, self.state.trace.log().clone());
        trace.transitions_between_on_branch(self.state.branch_id, start, end)
    }
}
