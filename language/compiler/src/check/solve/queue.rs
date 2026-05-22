use std::collections::VecDeque;

use indexmap::IndexMap;

use crate::check::{Constraint, VariableId};

/// Component solver work queue.
pub(in crate::check) struct WorkQueue {
    /// Constraints in component solve order.
    constraints: Vec<Constraint>,
    /// Queued constraint indexes.
    pending: VecDeque<usize>,
    /// Whether a constraint index is currently queued.
    is_pending: Vec<bool>,
    /// Constraint indexes that read each variable.
    dependents: IndexMap<VariableId, Vec<usize>>,
}

impl WorkQueue {
    /// Create a queue containing every work item.
    pub(in crate::check) fn new(constraints: Vec<Constraint>) -> Self {
        let pending = (0..constraints.len()).collect::<VecDeque<_>>();
        let is_pending = vec![true; constraints.len()];
        let dependents = Self::dependents(&constraints);

        Self {
            constraints,
            pending,
            is_pending,
            dependents,
        }
    }

    /// Pop the next queued work item.
    pub(in crate::check) fn pop(&mut self) -> Option<Constraint> {
        let index = self.pending.pop_front()?;
        self.is_pending[index] = false;

        Some(self.constraints[index].clone())
    }

    /// Enqueue constraints that depend on one changed variable.
    pub(in crate::check) fn enqueue_dependents(&mut self, variable: VariableId) {
        let Some(dependents) = self.dependents.get(&variable) else {
            return;
        };

        // enqueue each dependent once
        for dependent in dependents {
            if !self.is_pending[*dependent] {
                self.pending.push_back(*dependent);
                self.is_pending[*dependent] = true;
            }
        }
    }

    /// Build dependency edges for the queue.
    fn dependents(constraints: &[Constraint]) -> IndexMap<VariableId, Vec<usize>> {
        let mut dependents = IndexMap::new();

        // index every item by the variables that can unblock it
        for (index, constraint) in constraints.iter().enumerate() {
            for variable in constraint.variables() {
                dependents
                    .entry(variable)
                    .or_insert_with(Vec::new)
                    .push(index);
            }
        }

        dependents
    }
}
