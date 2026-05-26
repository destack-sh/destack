use std::collections::VecDeque;

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Constraint, Progress, VariableId};

/// Component constraint solver queue.
pub(in crate::check) struct ConstraintQueue {
    /// Constraints in component solve order.
    constraints: Vec<Constraint>,
    /// Constraint indexes keyed by variables that wake them.
    dependents: IndexMap<VariableId, SmallVec<[usize; 4]>>,
    /// Queued constraint indexes.
    pending: VecDeque<usize>,
    /// Whether a constraint index is currently queued.
    queued: Vec<bool>,
}

impl ConstraintQueue {
    /// Create a queue containing every work item.
    pub(in crate::check) fn from(constraints: Vec<Constraint>) -> Self {
        let pending = (0..constraints.len()).collect::<VecDeque<_>>();
        let queued = vec![true; constraints.len()];
        let mut dependents = IndexMap::<VariableId, SmallVec<[usize; 4]>>::new();

        // index constraints by the variables that wake them
        for (index, constraint) in constraints.iter().enumerate() {
            for variable in constraint.wake_variables() {
                dependents.entry(variable).or_default().push(index);
            }
        }

        Self {
            constraints,
            dependents,
            pending,
            queued,
        }
    }

    /// Return the next queued work item.
    pub(in crate::check) fn next(&mut self) -> Option<Constraint> {
        let index = self.pending.pop_front()?;
        self.queued[index] = false;

        Some(self.constraints[index].clone())
    }

    /// Wake constraints affected by one reduction.
    pub(in crate::check) fn wake(&mut self, progress: Progress) {
        match progress {
            Progress::Unchanged => {}
            Progress::Changed(variables) => self.enqueue_dependents(&variables),
        }
    }

    /// Wake every constraint.
    pub(in crate::check) fn wake_all(&mut self) {
        // global bound solving can solve variables created after the queue index
        for index in 0..self.constraints.len() {
            if !self.queued[index] {
                self.pending.push_back(index);
                self.queued[index] = true;
            }
        }
    }

    /// Enqueue constraints that depend on changed variables.
    fn enqueue_dependents(&mut self, variables: &[VariableId]) {
        for variable in variables {
            let Some(dependents) = self.dependents.get(variable) else {
                continue;
            };

            // keep each constraint queued at most once
            for index in dependents {
                if !self.queued[*index] {
                    self.pending.push_back(*index);
                    self.queued[*index] = true;
                }
            }
        }
    }
}
