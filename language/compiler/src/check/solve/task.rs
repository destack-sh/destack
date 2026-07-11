use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    BoundMode, CheckFailure, ConstraintId, ObligationFailure, ObligationId, Origin, Relation,
    ValueUse,
};

/// Failed judgments returned by one solver task run.
pub(in crate::check) type TaskFailures = SmallVec<[TaskFailure; 1]>;

/// One failed judgment returned by a solver task.
#[derive(Debug, Clone)]
pub(in crate::check) enum TaskFailure {
    /// One value or type constraint does not hold.
    Constraint(ConstraintFailure),
    /// One deferred obligation does not hold.
    Obligation(ObligationFailure),
}

/// One failed constraint judgment.
#[derive(Debug, Clone)]
pub(in crate::check) struct ConstraintFailure {
    /// The source that produced the constraint.
    pub(in crate::check) origin: Origin,
    /// The relation that failed.
    pub(in crate::check) relation: Relation,
    /// The checked value use, if the constraint checked a value.
    pub(in crate::check) use_: Option<ValueUse>,
    /// The constrained source type.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The constraint target type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The failure reason.
    pub(in crate::check) failure: CheckFailure,
}

const TASK_PRIORITY_COUNT: usize = 4;

/// One scheduled solver task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) enum Task {
    /// Solve one type relation constraint.
    Relate(ConstraintId),
    /// Check one deferred obligation.
    Oblige(ObligationId),
    /// Solve one variable from its bounds.
    Solve {
        /// The variable to solve.
        variable: dir::TypeVariableId,
        /// The weakest bounds allowed to choose a solution.
        mode: BoundMode,
    },
}

impl Task {
    /// Return the scheduler priority for this task.
    fn priority(&self) -> TaskPriority {
        match self {
            Self::Relate(_) => TaskPriority::Relate,
            Self::Solve {
                mode: BoundMode::Strong,
                ..
            } => TaskPriority::Solve,
            Self::Solve {
                mode: BoundMode::Weak,
                ..
            } => TaskPriority::WeakSolve,
            Self::Oblige(_) => TaskPriority::Oblige,
        }
    }
}

/// Static task scheduling priority, in scheduler order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskPriority {
    /// Relation constraints run first.
    Relate,
    /// Variable solving runs after relations.
    Solve,
    /// Obligations run after solving.
    Oblige,
    /// Weak variable solving runs after every regular task drains.
    WeakSolve,
}

impl TaskPriority {
    /// Every priority in scheduler order.
    const ALL: [Self; TASK_PRIORITY_COUNT] =
        [Self::Relate, Self::Solve, Self::Oblige, Self::WeakSolve];

    /// Return whether tasks at this priority judge completed bodies.
    fn is_settlement(self) -> bool {
        matches!(self, Self::Oblige)
    }

    /// Return the dense array index for this priority.
    fn index(self) -> usize {
        self as usize
    }
}

/// Priority ordered solver task queue.
#[derive(Debug)]
pub(in crate::check) struct WorkQueue {
    /// Queued tasks by static priority.
    entries: [Vec<Task>; TASK_PRIORITY_COUNT],
    /// Next unread entry by static priority.
    heads: [usize; TASK_PRIORITY_COUNT],
    /// Tasks currently queued or running.
    active: FxIndexSet<Task>,
    /// Tasks that finished successfully.
    finished: FxIndexSet<Task>,
}

impl WorkQueue {
    /// Create an empty queue.
    pub(in crate::check) fn new() -> Self {
        Self {
            entries: std::array::from_fn(|_| Vec::new()),
            heads: [0; TASK_PRIORITY_COUNT],
            active: FxIndexSet::default(),
            finished: FxIndexSet::default(),
        }
    }

    /// Queue one task at the back of its priority class.
    pub(in crate::check) fn push(&mut self, task: Task) {
        if !self.finished.contains(&task) && self.active.insert(task.clone()) {
            self.entries[task.priority().index()].push(task);
        }
    }

    /// Pop the next inference task, leaving settlement judgments queued.
    pub(in crate::check) fn pop_inference(&mut self) -> Option<Task> {
        TaskPriority::ALL
            .into_iter()
            .filter(|priority| !priority.is_settlement())
            .find_map(|priority| self.pop_at(priority))
    }

    /// Pop the next task in priority order.
    pub(in crate::check) fn pop(&mut self) -> Option<Task> {
        TaskPriority::ALL
            .into_iter()
            .find_map(|priority| self.pop_at(priority))
    }

    /// Pop the next live task of one priority class.
    fn pop_at(&mut self, priority: TaskPriority) -> Option<Task> {
        let index = priority.index();
        while let Some(task) = self.entries[index].get(self.heads[index]) {
            self.heads[index] += 1;

            // skip entries whose task completed or parked meanwhile
            if self.active.contains(task) {
                return Some(task.clone());
            }
        }

        None
    }

    /// Complete one active task.
    pub(in crate::check) fn complete(&mut self, task: &Task) {
        self.active.swap_remove(task);
        self.finished.insert(task.clone());
    }

    /// Remove one active task without marking it finished.
    pub(in crate::check) fn park(&mut self, task: &Task) {
        self.active.swap_remove(task);
    }

    /// Return the total number of queued tasks.
    pub(in crate::check) fn len(&self) -> usize {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entries)| entries.len().saturating_sub(self.heads[index]))
            .sum()
    }
}
