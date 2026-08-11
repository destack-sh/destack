use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::check::Scope;

/// One type relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Relation {
    /// Both operands solve to the same type.
    Equal,
    /// Every inhabitant of the source type inhabits the target type.
    Subtype,
    /// The source operand is assignable to the target operand.
    Assignable,
    /// The source operand is assignable to the target operand without
    /// requiring any coercion.
    Widens,
    /// The source operand is castable to the target operand.
    Castable,
    /// The source operand satisfies the target operand without influencing it.
    Satisfies,
    /// The source operand extends the target operand.
    Extends,
}

impl From<dir::WhereRelation> for Relation {
    fn from(relation: dir::WhereRelation) -> Self {
        match relation {
            dir::WhereRelation::Satisfies => Self::Satisfies,
            dir::WhereRelation::Equal => Self::Equal,
        }
    }
}

impl Relation {
    /// Return the transitive relation through one inference variable.
    pub(in crate::check) fn transitive_with(self, next: Relation) -> Option<Relation> {
        match (self, next) {
            (Self::Equal, relation) | (relation, Self::Equal) => Some(relation),
            (Self::Widens, Self::Widens) => Some(Self::Widens),
            (Self::Assignable | Self::Widens, Self::Assignable | Self::Widens) => {
                Some(Self::Assignable)
            }
            // predicates judge the variable's solution, never inflowing bounds
            _ => None,
        }
    }

    /// Return the relation for slots inside one related value.
    pub(in crate::check) fn interior(self) -> Relation {
        match self {
            Self::Assignable | Self::Widens => Self::Widens,
            Self::Equal => Self::Equal,
            Self::Subtype => Self::Subtype,
            _ => Self::Assignable,
        }
    }

    /// Return whether a union target accepts any successful element relation.
    pub(in crate::check) fn distributes_over_union_target(self) -> bool {
        matches!(
            self,
            Self::Subtype | Self::Assignable | Self::Castable | Self::Satisfies | Self::Extends
        )
    }
}

/// One relation pair identity closed under its assuming scope.
pub(in crate::check) type RelationKey = (Relation, dir::GlobalTypeId, dir::GlobalTypeId, Scope);

/// One in-flight relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationDecision {
    /// The pair is being decided at one active stack index.
    InProgress(usize),
    /// The pair held provisionally through the active cycle at one stack index.
    Provisional(usize),
}

/// One active relation decision attempt.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct RelationAttempt {
    /// The decided pair.
    key: RelationKey,
    /// The attempt's position on the decision stack.
    index: usize,
}

/// One stack entry tracking cycle use during an attempt.
#[derive(Debug, Clone)]
struct RelationStackEntry {
    /// The decided pair.
    key: RelationKey,
    /// The outermost stack index this attempt's result depends on.
    dependency: usize,
}

/// In-flight relation decisions with their cycle stack.
///
/// This state unwinds with the deciding recursion itself: every entered
/// attempt is finished or cancelled before its caller returns.
#[derive(Debug)]
pub(in crate::check) struct RelationStack {
    /// The in-flight decisions keyed by relation pair.
    decisions: FxIndexMap<RelationKey, RelationDecision>,
    /// The active decision attempts, outermost first.
    stack: Vec<RelationStackEntry>,
    /// Provisional holds with the cycle attempt they depend on.
    provisional: Vec<(RelationKey, usize)>,
}

impl RelationStack {
    /// Create an empty relation stack.
    pub(in crate::check) fn new() -> Self {
        Self {
            decisions: FxIndexMap::default(),
            stack: Vec::new(),
            provisional: Vec::new(),
        }
    }

    /// Return the in-flight answer for one pair, recording cycle use.
    pub(in crate::check) fn lookup(&mut self, key: &RelationKey) -> Option<bool> {
        let verdict = *self.decisions.get(key)?;

        // make the consuming attempt depend on the encountered cycle
        match verdict {
            RelationDecision::InProgress(index) | RelationDecision::Provisional(index) => {
                if let Some(top) = self.stack.last_mut() {
                    top.dependency = top.dependency.min(index);
                }

                Some(true)
            }
        }
    }

    /// Begin one decision attempt for an undecided pair.
    pub(in crate::check) fn enter(&mut self, key: RelationKey) -> RelationAttempt {
        let index = self.stack.len();

        self.decisions
            .insert(key, RelationDecision::InProgress(index));
        self.stack.push(RelationStackEntry {
            key,
            dependency: index,
        });

        RelationAttempt { key, index }
    }

    /// Finish one attempt, returning the decision when it became durable.
    ///
    /// Durable attempts decide closed pairs: the caller records the returned
    /// decision as a relation fact. Cycle-provisional holds stay in flight
    /// and open answers are not retained at all.
    pub(in crate::check) fn finish(
        &mut self,
        attempt: RelationAttempt,
        holds: bool,
        durable: bool,
    ) -> Option<(RelationKey, bool)> {
        let entry = self.pop(attempt);

        // failure is robust: cycle hypotheses only widen relations,
        //  so a failure reached under one holds without it
        if !holds {
            self.resolve_dependents(attempt.index, None);
            self.decisions.swap_remove(&attempt.key);

            return durable.then_some((attempt.key, false));
        }

        // pass provisional holds through the outer cycle
        if entry.dependency < attempt.index {
            self.resolve_dependents(attempt.index, Some(entry.dependency));
            self.decisions
                .insert(attempt.key, RelationDecision::Provisional(entry.dependency));
            self.provisional.push((attempt.key, entry.dependency));
            if let Some(top) = self.stack.last_mut() {
                top.dependency = top.dependency.min(entry.dependency);
            }

            return None;
        }

        // settle holds justified by this attempt
        self.resolve_dependents(attempt.index, Some(attempt.index));
        self.decisions.swap_remove(&attempt.key);

        durable.then_some((attempt.key, true))
    }

    /// Cancel one attempt without an answer, forgetting its dependents.
    pub(in crate::check) fn cancel(&mut self, attempt: RelationAttempt) {
        self.pop(attempt);

        self.resolve_dependents(attempt.index, None);
        self.decisions.swap_remove(&attempt.key);
    }

    /// Pop one attempt off the stack, requiring LIFO closing.
    fn pop(&mut self, attempt: RelationAttempt) -> RelationStackEntry {
        let entry = self.stack.pop();
        match entry {
            Some(entry) if entry.key == attempt.key && self.stack.len() == attempt.index => entry,
            _ => unreachable!("check relation attempts must close in LIFO order"),
        }
    }

    /// Resolve every provisional decision depending on one closing attempt.
    fn resolve_dependents(&mut self, index: usize, outcome: Option<usize>) {
        let mut position = 0;
        while position < self.provisional.len() {
            let (key, dependency) = self.provisional[position];
            if dependency != index {
                position += 1;

                continue;
            }

            match outcome {
                // the cycle settled true on its own
                Some(target) if target == index => {
                    self.decisions.swap_remove(&key);
                    self.provisional.swap_remove(position);
                }
                // pass the dependency to the outer attempt
                Some(target) => {
                    self.decisions
                        .insert(key, RelationDecision::Provisional(target));
                    self.provisional[position] = (key, target);
                    position += 1;
                }
                // the cycle failed or stayed undecided
                None => {
                    self.decisions.swap_remove(&key);
                    self.provisional.swap_remove(position);
                }
            }
        }
    }
}
