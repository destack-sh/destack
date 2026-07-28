use destack_core::FxIndexMap;
use destack_dir as dir;

/// One type relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Relation {
    /// Both operands solve to the same type.
    Equal,
    /// Every inhabitant of the source type inhabits the target type.
    Subtype,
    /// The source operand is assignable to the target operand.
    Assignable,
    /// The source operand is assignable to the target operand through
    /// identity-witnessed widenings only: no coercion may be required.
    Widens,
    /// The source operand is castable to the target operand.
    Castable,
    /// The source operand satisfies the target operand without influencing it.
    Satisfies,
    /// The source operand extends the target operand.
    Extends,
    /// The source operand implements the target operand.
    Implements,
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
            (Self::Assignable | Self::Widens, Self::Satisfies) => Some(Self::Satisfies),
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
            Self::Subtype
                | Self::Assignable
                | Self::Castable
                | Self::Satisfies
                | Self::Extends
                | Self::Implements
        )
    }

    /// Return whether every union source arm must satisfy this relation.
    pub(in crate::check) fn distributes_over_union_source(self) -> bool {
        matches!(
            self,
            Self::Subtype
                | Self::Assignable
                | Self::Widens
                | Self::Satisfies
                | Self::Extends
                | Self::Implements
        )
    }
}

/// One relation pair identity over two reduced roots.
type RelationKey = (
    Relation,
    dir::GlobalTypeId,
    dir::GlobalTypeId,
    Option<dir::GlobalGenericTemplateId>,
);

/// One memoized relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationDecision {
    /// The pair is being decided at one active stack index.
    InProgress(usize),
    /// The pair held provisionally through the active cycle at one stack index.
    Provisional(usize),
    /// The pair holds unconditionally.
    Holds,
    /// The pair fails unconditionally.
    Fails,
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

/// Memoized relation decisions.
#[derive(Debug)]
pub(in crate::check) struct RelationCache {
    /// The decisions keyed by relation pair.
    decisions: FxIndexMap<RelationKey, RelationDecision>,
    /// The active decision attempts, outermost first.
    stack: Vec<RelationStackEntry>,
    /// Provisional holds with the cycle attempt they depend on.
    provisional: Vec<(RelationKey, usize)>,
    /// Decision map entries to undo when a snapshot rolls back.
    undo: Vec<DecisionUndo>,
    /// The number of nested snapshots.
    snapshot_depth: usize,
}

/// Relation cache snapshot mark.
#[derive(Debug)]
pub(in crate::check) struct RelationCacheSnapshot {
    /// The undo log length before the snapshot.
    undo: usize,
    /// The active decision stack before the snapshot.
    stack: Vec<RelationStackEntry>,
    /// The provisional decision list before the snapshot.
    provisional: Vec<(RelationKey, usize)>,
}

/// One relation decision-map undo entry.
#[derive(Debug, Clone, Copy)]
struct DecisionUndo {
    /// The changed relation key.
    key: RelationKey,
    /// The previous decision for the key.
    previous: Option<RelationDecision>,
}

impl RelationCache {
    /// Create an empty relation cache.
    pub(in crate::check) fn new() -> Self {
        Self {
            decisions: FxIndexMap::default(),
            stack: Vec::new(),
            provisional: Vec::new(),
            undo: Vec::new(),
            snapshot_depth: 0,
        }
    }

    /// Snapshot the cache before one probe.
    pub(in crate::check) fn snapshot(&mut self) -> RelationCacheSnapshot {
        self.snapshot_depth += 1;

        RelationCacheSnapshot {
            undo: self.undo.len(),
            stack: self.stack.clone(),
            provisional: self.provisional.clone(),
        }
    }

    /// Roll back to one relation snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: RelationCacheSnapshot) {
        while self.undo.len() > snapshot.undo {
            let Some(undo) = self.undo.pop() else {
                unreachable!("relation undo length checked before pop");
            };

            match undo.previous {
                Some(previous) => {
                    self.decisions.insert(undo.key, previous);
                }
                None => {
                    self.decisions.swap_remove(&undo.key);
                }
            }
        }

        self.stack = snapshot.stack;
        self.provisional = snapshot.provisional;
        self.snapshot_depth -= 1;
    }

    /// Commit decisions made after one relation snapshot.
    pub(in crate::check) fn commit(&mut self, _snapshot: RelationCacheSnapshot) {
        self.snapshot_depth -= 1;
        if self.snapshot_depth == 0 {
            self.undo.clear();
        }
    }

    /// Return the memoized answer for one pair, recording cycle use.
    pub(in crate::check) fn lookup(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        scope: Option<dir::GlobalGenericTemplateId>,
    ) -> Option<bool> {
        let verdict = *self.decisions.get(&(relation, source, target, scope))?;

        match verdict {
            RelationDecision::Holds => Some(true),
            RelationDecision::Fails => Some(false),
            // make the consuming attempt depend on the encountered cycle
            RelationDecision::InProgress(index) | RelationDecision::Provisional(index) => {
                if let Some(top) = self.stack.last_mut() {
                    top.dependency = top.dependency.min(index);
                }

                Some(true)
            }
        }
    }

    /// Begin one decision attempt for an undecided pair.
    pub(in crate::check) fn enter(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        scope: Option<dir::GlobalGenericTemplateId>,
    ) -> RelationAttempt {
        let key = (relation, source, target, scope);
        let index = self.stack.len();

        self.set_decision(key, RelationDecision::InProgress(index));
        self.stack.push(RelationStackEntry {
            key,
            dependency: index,
        });

        RelationAttempt { key, index }
    }

    /// Finish one attempt with its decided answer.
    pub(in crate::check) fn finish(&mut self, attempt: RelationAttempt, holds: bool) {
        let entry = self.pop(attempt);

        // failure is robust: cycle hypotheses only widen relations,
        //  so a failure reached under one holds without it
        if !holds {
            self.resolve_dependents(attempt.index, None);
            self.set_decision(attempt.key, RelationDecision::Fails);
        }
        // pass provisional holds through the outer cycle
        else if entry.dependency < attempt.index {
            self.resolve_dependents(attempt.index, Some(entry.dependency));
            self.set_decision(attempt.key, RelationDecision::Provisional(entry.dependency));
            self.provisional.push((attempt.key, entry.dependency));
            if let Some(top) = self.stack.last_mut() {
                top.dependency = top.dependency.min(entry.dependency);
            }
        }
        // settle holds justified by this attempt
        else {
            self.resolve_dependents(attempt.index, Some(attempt.index));
            self.set_decision(attempt.key, RelationDecision::Holds);
        }
    }

    /// Cancel one attempt without an answer, forgetting its dependents.
    pub(in crate::check) fn cancel(&mut self, attempt: RelationAttempt) {
        self.pop(attempt);

        self.resolve_dependents(attempt.index, None);
        self.remove_decision(attempt.key);
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
                    self.set_decision(key, RelationDecision::Holds);
                    self.provisional.swap_remove(position);
                }
                // pass the dependency to the outer attempt
                Some(target) => {
                    self.set_decision(key, RelationDecision::Provisional(target));
                    self.provisional[position] = (key, target);
                    position += 1;
                }
                // the cycle failed or stayed undecided
                None => {
                    self.remove_decision(key);
                    self.provisional.swap_remove(position);
                }
            }
        }
    }

    /// Set one decision-map entry.
    fn set_decision(&mut self, key: RelationKey, decision: RelationDecision) {
        self.record_decision(key);
        self.decisions.insert(key, decision);
    }

    /// Remove one decision-map entry.
    fn remove_decision(&mut self, key: RelationKey) {
        self.record_decision(key);
        self.decisions.swap_remove(&key);
    }

    /// Record one decision-map entry before mutating it.
    fn record_decision(&mut self, key: RelationKey) {
        if self.snapshot_depth == 0 {
            return;
        }

        self.undo.push(DecisionUndo {
            key,
            previous: self.decisions.get(&key).copied(),
        });
    }
}
