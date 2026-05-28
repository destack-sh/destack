use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::World;
use crate::world::trace::{Trace, TraceRecord, TraceSequence};

use super::{BranchId, HistoryQuery, Moment};

/// One transition class derived from one authoritative replay record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    /// One input step.
    Mutation,
    /// One entrypoint call step.
    Entrypoint,
    /// One outcome step.
    Outcome,
    /// One label step.
    Label,
}

/// One derived state transition between two adjacent moments.
#[derive(Debug, Clone)]
pub struct Transition {
    /// The moment before the transition.
    pub before: Moment,
    /// The moment after the transition.
    pub after: Moment,
    /// The class of the authoritative replay record that caused the transition.
    pub kind: TransitionKind,
    /// The exact sequence of the authoritative replay record that caused the transition.
    pub cause_sequence: TraceSequence,
}

impl Transition {
    /// Report whether this transition was caused by one input.
    pub const fn is_input(&self) -> bool {
        matches!(
            self.kind,
            TransitionKind::Mutation | TransitionKind::Entrypoint
        )
    }

    /// Report whether this transition was caused by one outcome.
    pub const fn is_outcome(&self) -> bool {
        matches!(self.kind, TransitionKind::Outcome)
    }

    /// Report whether this transition was caused by one label.
    pub const fn is_label(&self) -> bool {
        matches!(self.kind, TransitionKind::Label)
    }
}

/// One eager transition-query result.
#[derive(Debug, Clone)]
pub struct TransitionSet {
    /// The transitions produced by the query.
    transitions: Vec<Transition>,
}

impl TransitionSet {
    /// Create one transition set from one eager transition vector.
    pub(super) fn new(mut transitions: Vec<Transition>) -> Self {
        transitions
            .sort_by_key(|transition| (transition.after.branch_id, transition.after.sequence));

        Self { transitions }
    }

    /// Return the number of transitions in this set.
    pub fn len(&self) -> usize {
        self.transitions.len()
    }

    /// Report whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }

    /// Return the transitions in stable query order.
    pub fn as_slice(&self) -> &[Transition] {
        &self.transitions
    }

    /// Consume this set and return its transitions.
    pub fn into_vec(self) -> Vec<Transition> {
        self.transitions
    }

    /// Keep only transitions of one class.
    pub fn kind(self, kind: TransitionKind) -> Self {
        self.filter(|transition| transition.kind == kind)
    }

    /// Keep only transitions caused by projected inputs.
    pub fn inputs(self) -> Self {
        self.filter(Transition::is_input)
    }

    /// Keep only transitions caused by projected outcomes.
    pub fn outcomes(self) -> Self {
        self.kind(TransitionKind::Outcome)
    }

    /// Keep only transitions caused by projected labels.
    pub fn labels(self) -> Self {
        self.kind(TransitionKind::Label)
    }

    /// Keep only transitions that satisfy one predicate.
    pub fn filter(mut self, mut predicate: impl FnMut(&Transition) -> bool) -> Self {
        self.transitions.retain(|transition| predicate(transition));
        self
    }
}

/// One history-rooted committed transition query.
#[derive(Debug, Clone, Copy)]
pub struct TransitionQuery<'a> {
    /// The history query root that owns the query.
    history: HistoryQuery<'a>,
}

impl<'a> TransitionQuery<'a> {
    /// Create one committed transition query on one history query root.
    pub(super) const fn new(history: HistoryQuery<'a>) -> Self {
        Self { history }
    }

    /// Return every committed transition visible on one branch.
    pub fn branch(self, branch_id: BranchId) -> RuntimeResult<TransitionSet> {
        self.history.transitions_on(branch_id)
    }

    /// Return every committed transition visible on one branch and its descendants.
    pub fn descendants_of(self, branch_id: BranchId) -> RuntimeResult<TransitionSet> {
        self.history.transitions_descendants_of(branch_id)
    }

    /// Return every committed transition up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<TransitionSet> {
        self.history.transitions_up_to(moment)
    }

    /// Return every committed transition in one exact branch-local range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<TransitionSet> {
        self.history.transitions_between(start, end)
    }
}

/// One live branch-local transition query rooted in one world.
#[derive(Debug, Clone, Copy)]
pub struct WorldTransitionQuery<'a> {
    /// The world that owns the query.
    world: &'a World,
}

impl<'a> WorldTransitionQuery<'a> {
    /// Create one live transition query on one world.
    pub(super) const fn new(world: &'a World) -> Self {
        Self { world }
    }

    /// Return every live-branch transition up to one target moment.
    pub fn up_to(self, moment: Moment) -> RuntimeResult<TransitionSet> {
        self.world.transitions_up_to(moment)
    }

    /// Return every live-branch transition in one exact range.
    pub fn between(self, start: Moment, end: Moment) -> RuntimeResult<TransitionSet> {
        self.world.transitions_between(start, end)
    }
}

impl Trace {
    /// Project derived transitions for one branch-local trace range.
    pub(super) fn transitions_between_on_branch(
        &self,
        branch_id: BranchId,
        start: TraceSequence,
        end: TraceSequence,
    ) -> RuntimeResult<Vec<Transition>> {
        let mut transitions = Vec::new();
        self.seek_sequence(start)?;

        while self.sequence()? != end {
            let before = Moment::new(branch_id, self.sequence()?);
            let cause = self
                .next_event()?
                .ok_or_else(|| RuntimeError::trace_exhausted(end.get()).boxed())?;
            let after = Moment::new(branch_id, self.sequence()?);
            let kind = match cause {
                TraceRecord::Mutation(_) => TransitionKind::Mutation,
                TraceRecord::Entrypoint(_) => TransitionKind::Entrypoint,
                TraceRecord::Outcome(_) => TransitionKind::Outcome,
                TraceRecord::Label(_) => TransitionKind::Label,
            };

            transitions.push(Transition {
                before,
                after,
                kind,
                cause_sequence: before.sequence,
            });
        }

        Ok(transitions)
    }
}
