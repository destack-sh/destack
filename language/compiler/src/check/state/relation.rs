use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::Relation;

/// One relation pair identity: the judgment kind over two reduced roots.
pub(in crate::check) type RelationKey = (Relation, dir::GlobalTypeId, dir::GlobalTypeId);

/// One memoized relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationDecision {
    /// The pair is being decided at one active frame index.
    InProgress(usize),
    /// The pair held under the still-active assumption frame at one index.
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

/// One stack entry tracking assumption use during a frame.
#[derive(Debug)]
struct RelationStackEntry {
    /// The decided pair.
    key: RelationKey,
    /// The outermost assumption frame this frame's result depends on.
    /// Equal to the frame's own index when no assumption was used.
    dependency: usize,
}

/// Memoized relation decisions.
///
/// A pair already on the decision stack answers true, which is what
/// terminates recursive types: a cycle re-enters through the pair that
/// opened it. Decisions made under such an assumption stay provisional
/// until the assumed pair settles: committed when it holds, forgotten
/// when it fails or stays pending. Keys are reduced roots. Decisions
/// settled under a probe are journaled by the caller and removed on
/// unwind.
#[derive(Debug)]
pub(in crate::check) struct RelationCache {
    /// The decisions keyed by relation pair.
    decisions: IndexMap<RelationKey, RelationDecision>,
    /// The active decision frames, outermost first.
    stack: Vec<RelationStackEntry>,
    /// Provisional holds with the assumption frame they depend on.
    provisional: Vec<(RelationKey, usize)>,
}

impl RelationCache {
    /// Create an empty relation cache.
    pub(in crate::check) fn new() -> Self {
        Self {
            decisions: IndexMap::new(),
            stack: Vec::new(),
            provisional: Vec::new(),
        }
    }

    /// Return the memoized answer for one pair, recording assumption use.
    pub(in crate::check) fn lookup(
        &mut self,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> Option<bool> {
        let verdict = *self.decisions.get(&(relation, left, right))?;

        match verdict {
            RelationDecision::Holds => Some(true),
            RelationDecision::Fails => Some(false),
            // assumption hits make the consuming frame provisional
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
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> RelationFrame {
        let key = (relation, left, right);
        let index = self.stack.len();

        self.decisions
            .insert(key, RelationDecision::InProgress(index));
        self.stack.push(RelationStackEntry {
            key,
            dependency: index,
        });

        RelationFrame { key, index }
    }

    /// Close one frame with its decided answer.
    /// Returns the pairs whose decisions became unconditional.
    pub(in crate::check) fn finish(
        &mut self,
        frame: RelationFrame,
        holds: bool,
    ) -> SmallVec<[RelationKey; 2]> {
        let entry = self.pop(frame);
        let mut settled = SmallVec::new();

        // failure is robust: assuming pairs true only widens relations,
        // so a failure reached under assumptions holds without them
        if !holds {
            self.resolve_dependents(frame.index, None, &mut settled);
            self.decisions.insert(frame.key, RelationDecision::Fails);
            settled.push(frame.key);
        }
        // holds under an outer assumption: stay provisional and pass
        // the dependency on to both dependents and the parent frame
        else if entry.dependency < frame.index {
            self.resolve_dependents(frame.index, Some(entry.dependency), &mut settled);
            self.decisions
                .insert(frame.key, RelationDecision::Provisional(entry.dependency));
            self.provisional.push((frame.key, entry.dependency));
            if let Some(top) = self.stack.last_mut() {
                top.dependency = top.dependency.min(entry.dependency);
            }
        }
        // holds on its own: the assumption this frame provided is
        // justified, settling every dependent along with it
        else {
            self.resolve_dependents(frame.index, Some(frame.index), &mut settled);
            self.decisions.insert(frame.key, RelationDecision::Holds);
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
        self.decisions.swap_remove(&frame.key);
    }

    /// Forget one settled pair during probe rollback.
    pub(in crate::check) fn remove(
        &mut self,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        self.decisions.swap_remove(&(relation, left, right));
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
    /// They settle when the frame held on its own, re-target when it held
    /// provisionally, and unwind when it failed or stayed pending.
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
                // the assumption settled true on its own
                Some(target) if target == index => {
                    self.decisions.insert(key, RelationDecision::Holds);
                    settled.push(key);
                    self.provisional.swap_remove(position);
                }
                // the assumption itself depends on an outer frame
                Some(target) => {
                    self.decisions
                        .insert(key, RelationDecision::Provisional(target));
                    self.provisional[position] = (key, target);
                    position += 1;
                }
                // the assumption failed or stayed undecided
                None => {
                    self.decisions.swap_remove(&key);
                    self.provisional.swap_remove(position);
                }
            }
        }
    }
}
