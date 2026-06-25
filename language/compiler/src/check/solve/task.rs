use destack_dir as dir;

use crate::check::{ConstraintId, ObligationId, SelectionId};

/// One scheduled solver task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Task {
    /// Solve one type relation constraint.
    Relate(ConstraintId),
    /// Select one source-node operation.
    Select(SelectionId),
    /// Check one deferred obligation.
    Oblige(ObligationId),
    /// Solve one variable from its bounds.
    Solve(dir::TypeVariableId),
}

/// Priority-ordered solver work queues.
#[derive(Debug)]
pub(in crate::check) struct Queue {
    /// Pending relation constraints.
    relate: WorkQueue<ConstraintId>,
    /// Pending source-node selections.
    select: WorkQueue<SelectionId>,
    /// Pending variable solves.
    solve: WorkQueue<dir::TypeVariableId>,
    /// Pending obligations.
    oblige: WorkQueue<ObligationId>,
}

/// Mark of a priority-ordered solver work queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct QueueMark {
    /// Mark for relation constraints.
    relate: WorkMark,
    /// Mark for source-node selections.
    select: WorkMark,
    /// Mark for variable solves.
    solve: WorkMark,
    /// Mark for obligations.
    oblige: WorkMark,
}

/// One FIFO work queue with cheap rollback marks.
#[derive(Debug)]
struct WorkQueue<T> {
    /// Queued entries.
    entries: Vec<T>,
    /// Index of the next unread entry.
    head: usize,
}

/// Mark of one FIFO work queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkMark {
    /// The next unread entry at mark time.
    head: usize,
    /// The queue length at mark time.
    len: usize,
}

impl Queue {
    /// Create an empty queue.
    pub(in crate::check) fn new() -> Self {
        Self {
            relate: WorkQueue::new(),
            select: WorkQueue::new(),
            solve: WorkQueue::new(),
            oblige: WorkQueue::new(),
        }
    }

    /// Mark the queue for rollback.
    pub(in crate::check) fn mark(&self) -> QueueMark {
        QueueMark {
            relate: self.relate.mark(),
            select: self.select.mark(),
            solve: self.solve.mark(),
            oblige: self.oblige.mark(),
        }
    }

    /// Roll back to one queue mark.
    pub(in crate::check) fn rollback(&mut self, mark: QueueMark) {
        self.relate.rollback(mark.relate);
        self.select.rollback(mark.select);
        self.solve.rollback(mark.solve);
        self.oblige.rollback(mark.oblige);
    }

    /// Queue one task at the back of its priority class.
    pub(in crate::check) fn push(&mut self, task: Task) {
        match task {
            Task::Relate(id) => self.relate.push(id),
            Task::Select(id) => self.select.push(id),
            Task::Oblige(id) => self.oblige.push(id),
            Task::Solve(variable) => self.solve.push(variable),
        }
    }

    /// Pop the next task in priority order.
    pub(in crate::check) fn pop(&mut self) -> Option<Task> {
        // drain relations, selections, variables, then obligations
        if let Some(id) = self.relate.pop() {
            Some(Task::Relate(id))
        } else if let Some(id) = self.select.pop() {
            Some(Task::Select(id))
        } else if let Some(variable) = self.solve.pop() {
            Some(Task::Solve(variable))
        } else if let Some(id) = self.oblige.pop() {
            Some(Task::Oblige(id))
        } else {
            None
        }
    }

    /// Return the total number of queued tasks.
    pub(in crate::check) fn len(&self) -> usize {
        self.relate.len() + self.solve.len() + self.select.len() + self.oblige.len()
    }
}

impl<T: Copy> WorkQueue<T> {
    /// Create an empty work queue.
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            head: 0,
        }
    }

    /// Mark the queue for rollback.
    fn mark(&self) -> WorkMark {
        WorkMark {
            head: self.head,
            len: self.entries.len(),
        }
    }

    /// Roll back to one queue mark.
    fn rollback(&mut self, mark: WorkMark) {
        self.entries.truncate(mark.len);
        self.head = mark.head;
    }

    /// Push one entry to the back.
    fn push(&mut self, entry: T) {
        self.entries.push(entry);
    }

    /// Pop one entry from the front.
    fn pop(&mut self) -> Option<T> {
        let entry = self.entries.get(self.head).copied()?;
        self.head += 1;

        Some(entry)
    }

    /// Return the number of pending entries.
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.head)
    }
}
