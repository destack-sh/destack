use tspp_core::FxIndexMap;
use tspp_dir as dir;

/// One type relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Relation {
    /// Both operands solve to the same type.
    Equal,
    /// Every inhabitant of the source inhabits the target.
    Subtype,
    /// The source stores in a target slot without changing representation.
    Storable,
}

impl From<dir::WhereRelation> for Relation {
    fn from(relation: dir::WhereRelation) -> Self {
        match relation {
            dir::WhereRelation::Satisfies => Self::Subtype,
            dir::WhereRelation::Equal => Self::Equal,
        }
    }
}

impl Relation {
    /// Return the relation two bounds compose to through one variable, the weaker one.
    pub(in crate::sema) fn join(self, other: Relation) -> Relation {
        match (self, other) {
            (Self::Equal, relation) | (relation, Self::Equal) => relation,
            (Self::Subtype, _) | (_, Self::Subtype) => Self::Subtype,
            (Self::Storable, Self::Storable) => Self::Storable,
        }
    }

    /// Return whether this relation flows the source operand into the target operand.
    pub(in crate::sema) fn is_directed(self) -> bool {
        self != Self::Equal
    }
}

/// One relation pair identity.
pub(in crate::sema) type RelationKey = (Relation, dir::GlobalTypeId, dir::GlobalTypeId);

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
pub(in crate::sema) struct RelationAttempt {
    /// The decided pair.
    key: RelationKey,
    /// The attempt's position on the decision stack.
    index: usize,
}

/// The cycle discipline one relation pair decides under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Cycle {
    /// A repeated pair holds as a recursive hypothesis.
    Coinductive,
    /// A repeated pair is refused, refusing self-supported proof.
    Inductive,
}

/// One stack entry tracking cycle use during an attempt.
#[derive(Debug, Clone)]
struct RelationStackEntry {
    /// The decided pair.
    key: RelationKey,
    /// The outermost stack index this attempt's result depends on.
    dependency: usize,
    /// Whether this attempt consumed an inductive refusal.
    refused: bool,
}

/// In-flight relation decisions with their cycle stack.
///
/// This state unwinds with the deciding recursion itself: every entered
/// attempt is finished or cancelled before its caller returns.
#[derive(Debug)]
pub(in crate::sema) struct RelationStack {
    /// The in-flight decisions keyed by relation pair.
    decisions: FxIndexMap<RelationKey, RelationDecision>,
    /// The active decision attempts, outermost first.
    stack: Vec<RelationStackEntry>,
    /// Provisional holds with the cycle attempt they depend on.
    provisional: Vec<(RelationKey, usize)>,
}

impl RelationStack {
    /// Create an empty relation stack.
    pub(in crate::sema) fn new() -> Self {
        Self {
            decisions: FxIndexMap::default(),
            stack: Vec::new(),
            provisional: Vec::new(),
        }
    }

    /// Return the in-flight answer for one pair, noting cycle use.
    pub(in crate::sema) fn lookup(&mut self, key: &RelationKey, cycle: Cycle) -> Option<bool> {
        let verdict = *self.decisions.get(key)?;
        match verdict {
            RelationDecision::InProgress(index) | RelationDecision::Provisional(index) => {
                match cycle {
                    // hold the repeated pair as a recursive hypothesis
                    Cycle::Coinductive => {
                        if let Some(top) = self.stack.last_mut() {
                            top.dependency = top.dependency.min(index);
                        }

                        Some(true)
                    }
                    // refuse self-supported proof, tainting the asking attempt
                    Cycle::Inductive => {
                        if let Some(top) = self.stack.last_mut() {
                            top.refused = true;
                        }

                        Some(false)
                    }
                }
            }
        }
    }

    /// Begin one decision attempt for an undecided pair.
    pub(in crate::sema) fn enter(&mut self, key: RelationKey) -> RelationAttempt {
        let index = self.stack.len();

        // record the pair as in flight
        self.decisions
            .insert(key, RelationDecision::InProgress(index));
        self.stack.push(RelationStackEntry {
            key,
            dependency: index,
            refused: false,
        });

        RelationAttempt { key, index }
    }

    /// Finish one attempt, returning the decision it reached.
    pub(in crate::sema) fn finish(
        &mut self,
        attempt: RelationAttempt,
        holds: bool,
    ) -> Option<bool> {
        let entry = self.pop(attempt);

        // drop a failed pair and wake what waited on it
        if !holds {
            self.resolve_dependents(attempt.index, None);
            self.decisions.swap_remove(&attempt.key);
            // keep refusal-fed failures open, since the pair may hold alone
            if entry.refused {
                if let Some(top) = self.stack.last_mut() {
                    top.refused = true;
                }

                return None;
            }

            return Some(false);
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

        // close the holds this attempt justifies
        self.resolve_dependents(attempt.index, Some(attempt.index));
        self.decisions.swap_remove(&attempt.key);

        Some(true)
    }

    /// Cancel one attempt without an answer, forgetting its dependents.
    pub(in crate::sema) fn cancel(&mut self, attempt: RelationAttempt) {
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

    /// Settle every provisional decision depending on one closing attempt.
    fn resolve_dependents(&mut self, index: usize, outcome: Option<usize>) {
        let mut position = 0;
        while position < self.provisional.len() {
            let (key, dependency) = self.provisional[position];
            if dependency != index {
                position += 1;

                continue;
            }

            match outcome {
                // the cycle decided true on its own
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
