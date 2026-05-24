use std::collections::VecDeque;

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Constraint, VariableId};

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
    pub(in crate::check) fn new(constraints: Vec<Constraint>) -> Self {
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

/// Variable changes produced by one solver reduction.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Progress {
    /// The reduction did not change any variable.
    Unchanged,
    /// The work item changed these variables.
    Changed(SmallVec<[VariableId; 4]>),
}

impl Progress {
    /// Return one changed variable.
    pub(in crate::check) fn changed(variable: VariableId) -> Self {
        Self::Changed(smallvec::smallvec![variable])
    }

    /// Return progress from a conditional variable change.
    pub(in crate::check) fn from_change(variable: VariableId, is_changed: bool) -> Self {
        if is_changed {
            Self::changed(variable)
        } else {
            Self::Unchanged
        }
    }

    /// Return whether this progress did not change any variable.
    pub(in crate::check) fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }

    /// Merge two progress values.
    pub(in crate::check) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unchanged, progress) | (progress, Self::Unchanged) => progress,
            (Self::Changed(mut left), Self::Changed(right)) => {
                left.extend(right);

                Self::Changed(left)
            }
        }
    }
}
