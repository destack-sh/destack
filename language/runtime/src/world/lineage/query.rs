use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::{RestoreContext, World, WorldImage};

use super::{
    Branch, BranchId, Checkpoint, CheckpointId, Event, EventQuery, EventSet, Moment,
    MomentSequence, WorldEventQuery,
};

/// One committed-lineage query root over shared lineage.
#[derive(Debug, Clone, Copy)]
pub struct LineageQuery<'a> {
    /// The world that owns the shared lineage.
    world: &'a World,
}

/// One branch divergence between two committed branch heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    /// The shared base Moment before the branches diverged.
    pub base: Moment,
    /// The committed head Moment on the left branch.
    pub left: Moment,
    /// The committed head Moment on the right branch.
    pub right: Moment,
}

/// One lineage-rooted committed moment query.
#[derive(Debug, Clone, Copy)]
pub struct MomentQuery<'a> {
    /// The lineage query root that owns the query.
    lineage: LineageQuery<'a>,
}

impl<'a> LineageQuery<'a> {
    /// Return metadata for one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<Branch> {
        let lineage = self.world.lineage.read();
        lineage.branch(branch_id)
    }

    /// Return every known branch in stable lineage order.
    pub fn branches(self) -> Vec<Branch> {
        let lineage = self.world.lineage.read();
        let mut branches = lineage.branches.values().cloned().collect::<Vec<_>>();
        branches.sort_by_key(|branch| branch.id);

        branches
    }

    /// Return metadata for one checkpoint.
    pub fn checkpoint(self, checkpoint_id: CheckpointId) -> RuntimeResult<Checkpoint> {
        let lineage = self.world.lineage.read();
        let checkpoint = lineage
            .checkpoints
            .get(&checkpoint_id)
            .ok_or_else(|| RuntimeError::checkpoint_not_found(checkpoint_id.get()).boxed())?;

        Ok(checkpoint.clone())
    }

    /// Return every known checkpoint in stable lineage order.
    pub fn checkpoints(self) -> Vec<Checkpoint> {
        let lineage = self.world.lineage.read();
        let mut checkpoints = lineage.checkpoints.values().cloned().collect::<Vec<_>>();
        checkpoints.sort_by_key(|checkpoint| checkpoint.id);

        checkpoints
    }

    /// Return one committed moment query root.
    pub fn moments(self) -> MomentQuery<'a> {
        MomentQuery { lineage: self }
    }

    /// Return one committed event query root.
    pub fn events(self) -> EventQuery<'a> {
        EventQuery::new(self)
    }

    /// Return every branch that descends from one ancestor branch.
    pub fn descendants_of(self, branch_id: BranchId) -> RuntimeResult<Vec<Branch>> {
        let lineage = self.world.lineage.read();
        let mut branches = lineage.descendant_branches(branch_id)?;
        branches.sort_by_key(|branch| branch.id);

        Ok(branches)
    }

    /// Return one Branch origin Moment.
    pub fn origin(self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let lineage = self.world.lineage.read();
        lineage.branch_origin_moment(branch_id)
    }

    /// Return one Branch head Moment.
    pub fn head(self, branch_id: BranchId) -> RuntimeResult<Moment> {
        let lineage = self.world.lineage.read();
        lineage.branch_head_moment(branch_id)
    }

    /// Materialize one exact committed Moment as a World image.
    pub fn image(self, moment: Moment, restore: RestoreContext<'_>) -> RuntimeResult<WorldImage> {
        self.require_committed_query_range(
            Moment::new(moment.branch_id, MomentSequence::new(0)),
            moment,
        )?;

        self.world.image_at_moment(moment, restore)
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
    pub(super) fn moments_on(self, branch_id: BranchId) -> RuntimeResult<Vec<Moment>> {
        let end = self.head(branch_id)?;
        self.moments_between(Moment::new(branch_id, MomentSequence::new(0)), end)
    }

    /// Return every committed moment up to one target moment.
    pub(super) fn moments_up_to(self, moment: Moment) -> RuntimeResult<Vec<Moment>> {
        let start = Moment::new(moment.branch_id, MomentSequence::new(0));
        self.moments_between(start, moment)
    }

    /// Return every committed moment in one exact branch-local range.
    pub(super) fn moments_between(self, start: Moment, end: Moment) -> RuntimeResult<Vec<Moment>> {
        self.require_committed_query_range(start, end)?;

        let moments = (start.sequence.get()..=end.sequence.get())
            .map(|sequence| Moment::new(start.branch_id, MomentSequence::new(sequence)))
            .collect();

        Ok(moments)
    }

    /// Return every committed query event visible on one branch.
    pub(super) fn events_on(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        let end = self.head(branch_id)?;
        self.events_between(Moment::new(branch_id, MomentSequence::new(0)), end)
    }

    /// Return every committed query event visible on one branch and its descendants.
    pub(super) fn events_descendants_of(self, branch_id: BranchId) -> RuntimeResult<EventSet> {
        let descendants = self.descendants_of(branch_id)?;
        let mut events = Vec::new();

        for branch in descendants {
            let branch_events = self.events_on(branch.id)?;
            events.extend(branch_events.into_vec());
        }

        Ok(EventSet::new(events))
    }

    /// Return every committed query event up to one target moment.
    pub(super) fn events_up_to(self, moment: Moment) -> RuntimeResult<EventSet> {
        let start = Moment::new(moment.branch_id, MomentSequence::new(0));
        self.events_between(start, moment)
    }

    /// Return every committed query event in one exact branch-local range.
    pub(super) fn events_between(self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.require_committed_query_range(start, end)?;

        let events = self.observation_events_between(start, end)?;

        Ok(EventSet::new(events))
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

        let committed_head = self.head(end.branch_id)?;
        if end.sequence.get() > committed_head.sequence.get() {
            return Err(
                RuntimeError::moment_not_found(end.branch_id.get(), end.sequence.get()).boxed(),
            );
        }

        Ok(())
    }

    /// Project committed observations into query events for one range.
    fn observation_events_between(self, start: Moment, end: Moment) -> RuntimeResult<Vec<Event>> {
        let lineage = self.world.lineage.read();
        let records = lineage.observation_records_between(start, end)?;

        Ok(records.into_iter().map(Event::from_observation).collect())
    }
}

impl<'a> MomentQuery<'a> {
    /// Return every committed moment visible on one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<Vec<Moment>> {
        self.lineage.moments_on(branch_id)
    }

    /// Return every committed moment up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<Vec<Moment>> {
        self.lineage.moments_up_to(moment)
    }

    /// Return every committed moment in one exact branch-local range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<Vec<Moment>> {
        self.lineage.moments_between(start, end)
    }
}

impl World {
    /// Return one live branch-local event query root.
    pub fn events(&self) -> WorldEventQuery<'_> {
        WorldEventQuery::new(self)
    }

    /// Return one committed-lineage query root over shared lineage.
    pub fn lineage(&self) -> LineageQuery<'_> {
        LineageQuery { world: self }
    }

    /// Return every query event after one start moment and up to one end moment.
    pub(super) fn events_between(&self, start: Moment, end: Moment) -> RuntimeResult<EventSet> {
        self.require_query_range(start, end)?;

        let events = self.observation_events_between(start, end)?;

        Ok(EventSet::new(events))
    }

    /// Return every query event up to one target moment.
    pub(super) fn events_up_to(&self, moment: Moment) -> RuntimeResult<EventSet> {
        let start = Moment::new(moment.branch_id, MomentSequence::new(0));
        self.events_between(start, moment)
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
                MomentSequence::new(end.sequence.get().min(committed_head.sequence.get())),
            );
            let lineage = self.lineage.read();
            let records = lineage.observation_records_between(start, committed_end)?;
            events.extend(records.into_iter().map(Event::from_observation));
        }

        if end.sequence.get() > committed_head.sequence.get() {
            let local_start = Moment::new(
                self.state.branch_id,
                MomentSequence::new(start.sequence.get().max(committed_head.sequence.get())),
            );
            let records = self.state.observations.records_between(local_start, end);
            events.extend(records.into_iter().map(Event::from_observation));
        }

        Ok(events)
    }
}
