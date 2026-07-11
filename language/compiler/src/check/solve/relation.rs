use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::CheckState;

/// One type relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Relation {
    /// Both operands solve to the same type.
    Equal,
    /// The source operand is assignable to the target operand.
    Assignable,
    /// The source operand is assignable to the target operand through
    /// identity-witnessed widenings only: no coercion may be required.
    Widens,
    /// The source method signature serves every use of the target one.
    MethodAssignable,
    /// The source operand is assignable to the target operand without influencing it.
    Writable,
    /// The source operand is castable to the target operand.
    Castable,
    /// The source operand satisfies the target operand without influencing it.
    Satisfies,
    /// The source operand extends the target operand.
    Extends,
    /// The source operand implements the target operand.
    Implements,
}

impl Relation {
    /// Return the relation for slots inside one related value.
    ///
    /// Value interiors have no store site to witness a conversion, so
    /// assignability restricts to identity-witnessed widening inside.
    /// Constraint judgments never move a value and read interior slots
    /// per use, so their slots relate by full assignability.
    pub(in crate::check) fn interior(self) -> Relation {
        match self {
            Self::Assignable | Self::Widens => Self::Widens,
            Self::Equal => Self::Equal,
            _ => Self::Assignable,
        }
    }

    /// Return the edge one handle-context payload relates by.
    ///
    /// Value relations restrict to identity-witnessed widening.
    /// Unlike `interior`, constraint judgments pass through unchanged, so
    /// payloads nested under further forms keep the constraint flavor.
    pub(in crate::check) fn payload_edge(self) -> Relation {
        match self {
            Self::Assignable | Self::Widens => Self::Widens,
            relation => relation,
        }
    }

    /// Return whether a union target accepts any successful element relation.
    pub(in crate::check) fn distributes_over_union_target(self) -> bool {
        matches!(
            self,
            Self::Assignable
                | Self::Writable
                | Self::Castable
                | Self::Satisfies
                | Self::Extends
                | Self::Implements
        )
    }
}

/// One relation pair identity over two reduced roots.
pub(in crate::check) type RelationKey = (
    Relation,
    dir::GlobalTypeId,
    dir::GlobalTypeId,
    Option<dir::GlobalGenericTemplateId>,
);

/// One memoized relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationDecision {
    /// The pair is being decided at one active frame index.
    InProgress(usize),
    /// The pair held under the still-active cycle frame at one index.
    Provisional(usize),
    /// The pair holds unconditionally.
    Holds,
    /// The pair fails unconditionally.
    Fails,
}

/// One active relation decision frame.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct RelationFrame {
    /// The decided pair.
    key: RelationKey,
    /// The frame's position on the decision stack.
    index: usize,
}

/// One stack entry tracking cycle use during a frame.
#[derive(Debug, Clone)]
struct RelationStackEntry {
    /// The decided pair.
    key: RelationKey,
    /// The outermost cycle frame this frame's result depends on.
    dependency: usize,
}

/// Memoized relation decisions.
#[derive(Debug)]
pub(in crate::check) struct RelationCache {
    /// The decisions keyed by relation pair.
    decisions: FxIndexMap<RelationKey, RelationDecision>,
    /// The active decision frames, outermost first.
    stack: Vec<RelationStackEntry>,
    /// Provisional holds with the cycle frame they depend on.
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
            let undo = self
                .undo
                .pop()
                .expect("relation undo length checked before pop");

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
            // cycle hits make the consuming frame provisional
            RelationDecision::InProgress(index) | RelationDecision::Provisional(index) => {
                if let Some(top) = self.stack.last_mut() {
                    top.dependency = top.dependency.min(index);
                }

                Some(true)
            }
        }
    }

    /// Open one decision frame for an undecided pair.
    pub(in crate::check) fn enter(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        scope: Option<dir::GlobalGenericTemplateId>,
    ) -> RelationFrame {
        let key = (relation, source, target, scope);
        let index = self.stack.len();

        self.set_decision(key, RelationDecision::InProgress(index));
        self.stack.push(RelationStackEntry {
            key,
            dependency: index,
        });

        RelationFrame { key, index }
    }

    /// Close one frame with its decided answer.
    pub(in crate::check) fn finish(
        &mut self,
        frame: RelationFrame,
        holds: bool,
    ) -> SmallVec<[RelationKey; 2]> {
        let entry = self.pop(frame);
        let mut settled = SmallVec::new();

        // failure is robust: cycle hypotheses only widen relations,
        //  so a failure reached under one holds without it
        if !holds {
            self.resolve_dependents(frame.index, None, &mut settled);
            self.set_decision(frame.key, RelationDecision::Fails);
            settled.push(frame.key);
        }
        // hold through an outer cycle: stay provisional and pass
        //  the dependency on to both dependents and the parent frame
        else if entry.dependency < frame.index {
            self.resolve_dependents(frame.index, Some(entry.dependency), &mut settled);
            self.set_decision(frame.key, RelationDecision::Provisional(entry.dependency));
            self.provisional.push((frame.key, entry.dependency));
            if let Some(top) = self.stack.last_mut() {
                top.dependency = top.dependency.min(entry.dependency);
            }
        }
        // holds on its own: the hypothesis this frame provided is
        //  justified, settling every dependent along with it
        else {
            self.resolve_dependents(frame.index, Some(frame.index), &mut settled);
            self.set_decision(frame.key, RelationDecision::Holds);
            settled.push(frame.key);
        }

        settled
    }

    /// Close one frame without an answer, forgetting its dependents.
    pub(in crate::check) fn cancel(&mut self, frame: RelationFrame) {
        self.pop(frame);

        let mut settled = SmallVec::new();
        self.resolve_dependents(frame.index, None, &mut settled);
        debug_assert!(settled.is_empty());
        self.remove_decision(frame.key);
    }

    /// Pop one frame off the stack, requiring LIFO closing.
    fn pop(&mut self, frame: RelationFrame) -> RelationStackEntry {
        let entry = self.stack.pop();
        match entry {
            Some(entry) if entry.key == frame.key && self.stack.len() == frame.index => entry,
            _ => unreachable!("check relation frames must close in LIFO order"),
        }
    }

    /// Resolve every provisional decision depending on one closing frame.
    fn resolve_dependents(
        &mut self,
        index: usize,
        outcome: Option<usize>,
        settled: &mut SmallVec<[RelationKey; 2]>,
    ) {
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
                    settled.push(key);
                    self.provisional.swap_remove(position);
                }
                // the cycle itself depends on an outer frame
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

impl CheckState<'_> {
    /// Return the active relation cache.
    pub(in crate::check) fn relations(&mut self) -> &mut RelationCache {
        &mut self.solver.relations
    }
}
