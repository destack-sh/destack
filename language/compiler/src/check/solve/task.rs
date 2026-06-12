use std::collections::VecDeque;

use destack_dir as dir;

use crate::check::{ConstraintId, ObligationId};

/// One scheduled solver task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Task {
    /// Solve one type relation constraint.
    Relate(ConstraintId),
    /// Select the meaning of one source node.
    Select(dir::GlobalNodeIdAny),
    /// Solve one variable from its bounds.
    Solve(dir::TypeVariableId),
    /// Check one collected obligation.
    Oblige(ObligationId),
}

/// Priority-ordered solver work queues.
#[derive(Debug)]
pub(in crate::check) struct Queue {
    /// Pending relation constraints.
    relate: VecDeque<Task>,
    /// Pending node selections.
    select: VecDeque<Task>,
    /// Pending variable solves.
    solve: VecDeque<Task>,
    /// Pending obligation checks.
    oblige: VecDeque<Task>,
}

impl Queue {
    /// Create an empty queue.
    pub(in crate::check) fn new() -> Self {
        Self {
            relate: VecDeque::new(),
            select: VecDeque::new(),
            solve: VecDeque::new(),
            oblige: VecDeque::new(),
        }
    }

    /// Queue one task at the back of its priority class.
    pub(in crate::check) fn push(&mut self, task: Task) {
        match task {
            Task::Relate(_) => self.relate.push_back(task),
            Task::Select(_) => self.select.push_back(task),
            Task::Solve(_) => self.solve.push_back(task),
            Task::Oblige(_) => self.oblige.push_back(task),
        }
    }

    /// Queue one task at the front of its priority class.
    pub(in crate::check) fn push_front(&mut self, task: Task) {
        match task {
            Task::Relate(_) => self.relate.push_front(task),
            Task::Select(_) => self.select.push_front(task),
            Task::Solve(_) => self.solve.push_front(task),
            Task::Oblige(_) => self.oblige.push_front(task),
        }
    }

    /// Pop the next task in priority order.
    pub(in crate::check) fn pop(&mut self) -> Option<Task> {
        // drain relations before decisions before solves before obligations
        if let Some(task) = self.relate.pop_front() {
            Some(task)
        } else if let Some(task) = self.select.pop_front() {
            Some(task)
        } else if let Some(task) = self.solve.pop_front() {
            Some(task)
        } else {
            self.oblige.pop_front()
        }
    }

    /// Return the number of queued solve tasks.
    pub(in crate::check) fn solve_count(&self) -> usize {
        self.solve.len()
    }

    /// Pop the newest solve task queued above one floor.
    pub(in crate::check) fn pop_solve_above(&mut self, floor: usize) -> Option<Task> {
        if self.solve.len() > floor {
            self.solve.pop_back()
        } else {
            None
        }
    }

    /// Remove the most recently queued occurrence of one task.
    pub(in crate::check) fn remove_last(&mut self, task: Task) {
        let queue = match task {
            Task::Relate(_) => &mut self.relate,
            Task::Select(_) => &mut self.select,
            Task::Solve(_) => &mut self.solve,
            Task::Oblige(_) => &mut self.oblige,
        };

        // drop the latest matching entry
        if let Some(position) = queue.iter().rposition(|queued| *queued == task) {
            queue.remove(position);
        }
    }

    /// Return the total number of queued tasks.
    pub(in crate::check) fn len(&self) -> usize {
        self.relate.len() + self.select.len() + self.solve.len() + self.oblige.len()
    }
}
