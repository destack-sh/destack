use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CauseId, CheckFailure, ConstraintId, FlowSite, FunctionBody, ObligationFailure, ObligationId,
    PlaceUse, Relation, ValueUse,
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
    /// The judgment cause that produced the constraint.
    pub(in crate::check) cause: CauseId,
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
    /// Check one source node value constraint.
    Check(ConstraintId),
    /// Infer one source use.
    Infer {
        /// The inferred source use.
        site: FlowSite,
        /// The syntactic place use.
        use_: PlaceUse,
    },
    /// Check one function declaration body.
    CheckBody(FunctionBody),
    /// Check one deferred obligation.
    Oblige(ObligationId),
}

impl Task {
    /// Return the scheduler priority for this task.
    fn priority(&self) -> TaskPriority {
        match self {
            Self::Relate(_) => TaskPriority::Relate,
            Self::Check(_) | Self::CheckBody(_) => TaskPriority::Check,
            Self::Infer { .. } => TaskPriority::Infer,
            Self::Oblige(_) => TaskPriority::Oblige,
        }
    }
}

/// Static task scheduling priority, in scheduler order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskPriority {
    /// Relation constraints run first.
    Relate,
    /// Target-directed source checks run before unconstrained inference.
    Check,
    /// Unconstrained source inference runs after contextual checks.
    Infer,
    /// Obligations run after solving.
    Oblige,
}

impl TaskPriority {
    /// Priorities that produce checked types and resolutions.
    const JUDGMENTS: [Self; 3] = [Self::Relate, Self::Check, Self::Infer];

    /// Priorities that settle value and type constraints.
    const CONSTRAINTS: [Self; 2] = [Self::Relate, Self::Check];

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

    /// Pop the next type or value judgment.
    pub(in crate::check) fn pop_judgment(&mut self) -> Option<Task> {
        TaskPriority::JUDGMENTS
            .into_iter()
            .find_map(|priority| self.pop_at(priority))
    }

    /// Pop the next deferred obligation.
    pub(in crate::check) fn pop_obligation(&mut self) -> Option<Task> {
        self.pop_at(TaskPriority::Oblige)
    }

    /// Pop the next constraint task.
    pub(in crate::check) fn pop_constraint(&mut self) -> Option<Task> {
        TaskPriority::CONSTRAINTS
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

    /// Return whether one task completed successfully.
    pub(in crate::check) fn is_finished(&self, task: &Task) -> bool {
        self.finished.contains(task)
    }

    /// Return the total number of queued tasks.
    pub(in crate::check) fn len(&self) -> usize {
        self.active.len()
    }
}
