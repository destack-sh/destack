use std::collections::VecDeque;

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckState, Constraint, Definition, Progress, VariableId};

/// Component solver queue.
pub(in crate::check) struct SolverQueue {
    /// Items in component solve order.
    items: Vec<SolverItem>,
    /// Item indexes keyed by watched variables.
    dependents: IndexMap<VariableId, SmallVec<[usize; 4]>>,
    /// Queued item indexes.
    pending: VecDeque<usize>,
    /// Whether an item index is currently queued.
    queued: Vec<bool>,
}

impl SolverQueue {
    /// Create a queue containing every work item.
    pub(in crate::check) fn from(items: Vec<SolverItem>, state: &CheckState<'_>) -> Self {
        let pending = (0..items.len()).collect::<VecDeque<_>>();
        let queued = vec![true; items.len()];
        let mut dependents = IndexMap::<VariableId, SmallVec<[usize; 4]>>::new();

        // index items by watched variables
        for (index, item) in items.iter().enumerate() {
            for variable in item.watched_variables(state) {
                dependents.entry(variable).or_default().push(index);
            }
        }

        Self {
            items,
            dependents,
            pending,
            queued,
        }
    }

    /// Return the next queued work item.
    pub(in crate::check) fn next(&mut self) -> Option<SolverItem> {
        let index = self.pending.pop_front()?;
        self.queued[index] = false;

        Some(self.items[index].clone())
    }

    /// Wake items affected by one reduction.
    pub(in crate::check) fn wake(&mut self, progress: Progress) {
        match progress {
            Progress::Unchanged => {}
            Progress::Changed(variables) => self.enqueue_dependents(&variables),
        }
    }

    /// Wake every item.
    pub(in crate::check) fn wake_all(&mut self) {
        // bound solving can satisfy variables watched by earlier items
        for index in 0..self.items.len() {
            if !self.queued[index] {
                self.pending.push_back(index);
                self.queued[index] = true;
            }
        }
    }

    /// Enqueue items that depend on changed variables.
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

/// One queued solver item.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum SolverItem {
    /// Variable definition.
    Definition(Definition),
    /// Relation constraint.
    Constraint(Constraint),
}

impl SolverItem {
    /// Return variables watched by this item.
    fn watched_variables(&self, state: &CheckState<'_>) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Definition(definition) => definition.watched_variables(state),
            Self::Constraint(constraint) => constraint.watched_variables(state),
        }
    }
}
