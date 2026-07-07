use destack_dir as dir;

use crate::check::{
    BoundMode, ConstraintId, FlowSite, ObligationId, Origin, Relation, ValueUse, Widening,
};

const TASK_PRIORITY_COUNT: usize = 8;

/// One syntactic use of a place expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum PlaceUse {
    /// Place value is read.
    Read,
    /// Place value is replaced.
    Write,
    /// Place value is read, transformed, and replaced.
    Update,
}

/// Result shape produced by one construct expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ConstructResult {
    /// Construct expression produces the constructed value directly.
    Direct,
    /// Construct expression produces the fallible construction carrier.
    Fallible,
}

/// One scheduled solver task.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Task {
    /// Solve one type relation constraint.
    Relate(ConstraintId),
    /// Propagate one try result.
    Propagate(TryPropagation),
    /// Check one deferred obligation.
    Oblige(ObligationId),
    /// Solve one variable from its bounds.
    Solve {
        /// The variable to solve.
        variable: dir::TypeVariableId,
        /// The weakest bounds allowed to choose a solution.
        mode: BoundMode,
    },
    /// Infer one expression occurrence.
    Infer {
        /// The inferred source use.
        site: FlowSite,
        /// The syntactic place use when the node is a place expression.
        use_: PlaceUse,
    },
    /// Check one expression occurrence against an expected type.
    Check {
        /// The checked source use.
        site: FlowSite,
        /// The type expected by the check.
        expected: ExpectedType,
        /// The relation the checked value must satisfy.
        relation: Relation,
        /// The source that produced the check.
        origin: Origin,
        /// The checked value use.
        use_: ValueUse,
    },
    /// Bind one symbol type from its source type.
    Bind {
        /// The binding symbol.
        symbol: dir::GlobalSymbolId,
        /// The source that produces the checked type.
        source: BindSource,
    },
}

/// Source used to bind one symbol type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum BindSource {
    /// An authored annotation type graph.
    Type(dir::GlobalTypeId),
    /// An initializer expression occurrence.
    Initializer {
        /// The initializer expression use.
        site: FlowSite,
        /// The binding widening policy.
        widening: Widening,
    },
}

impl Task {
    /// Return the scheduler priority for this task.
    fn priority(&self) -> TaskPriority {
        match self {
            Self::Relate(_) => TaskPriority::Relate,
            Self::Propagate(_) => TaskPriority::Propagate,
            Self::Check { .. } => TaskPriority::Check,
            Self::Infer { .. } => TaskPriority::Infer,
            Self::Bind { .. } => TaskPriority::Bind,
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

    /// Return the task family name.
    pub(in crate::check) fn name(&self) -> &'static str {
        match self {
            Self::Relate(_) => "relate",
            Self::Propagate(_) => "propagate",
            Self::Oblige(_) => "oblige",
            Self::Solve { .. } => "solve",
            Self::Infer { .. } => "infer",
            Self::Check { .. } => "check",
            Self::Bind { .. } => "bind",
        }
    }

    /// Return the source node owned by this task, if it has one.
    pub(in crate::check) fn node(&self) -> Option<dir::GlobalNodeIdAny> {
        match self {
            Self::Infer { site, .. } => Some(site.node),
            Self::Check { site, .. } => Some(site.node),
            Self::Propagate(propagation) => Some(propagation.source),
            Self::Relate(_) | Self::Oblige(_) | Self::Solve { .. } | Self::Bind { .. } => None,
        }
    }

    /// Return the stable dedupe key for source node work.
    pub(in crate::check) fn key(&self) -> Option<TaskKey> {
        match self {
            Self::Infer { site, use_ } => Some(TaskKey::Infer {
                site: *site,
                use_: *use_,
            }),
            Self::Check {
                site,
                expected,
                relation,
                origin,
                use_,
            } => Some(TaskKey::Check {
                site: *site,
                expected: *expected,
                relation: *relation,
                origin: *origin,
                use_: *use_,
            }),
            Self::Relate(_)
            | Self::Propagate(_)
            | Self::Oblige(_)
            | Self::Solve { .. }
            | Self::Bind { .. } => None,
        }
    }
}

/// Static task scheduling priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskPriority {
    /// Relation constraints run first.
    Relate,
    /// Try propagation runs after relation constraints.
    Propagate,
    /// Expected checks run before inference.
    Check,
    /// Source inference runs after expected checks.
    Infer,
    /// Binding publication runs after source typing.
    Bind,
    /// Variable solving runs after source tasks.
    Solve,
    /// Obligations run after inference and solving.
    Oblige,
    /// Weak variable solving runs after every regular task drains.
    WeakSolve,
}

impl TaskPriority {
    /// Return every priority in scheduler order.
    fn all() -> [Self; TASK_PRIORITY_COUNT] {
        [
            Self::Relate,
            Self::Propagate,
            Self::Check,
            Self::Infer,
            Self::Bind,
            Self::Solve,
            Self::Oblige,
            Self::WeakSolve,
        ]
    }

    /// Return the dense array index for this priority.
    fn index(self) -> usize {
        match self {
            Self::Relate => 0,
            Self::Propagate => 1,
            Self::Check => 2,
            Self::Infer => 3,
            Self::Bind => 4,
            Self::Solve => 5,
            Self::Oblige => 6,
            Self::WeakSolve => 7,
        }
    }
}

/// One propagation of a fallible try value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct TryPropagation {
    /// The try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked source use that produced the tried value.
    pub(in crate::check) value: FlowSite,
    /// The receiver of the propagated failure.
    pub(in crate::check) target: TryPropagationTarget,
}

/// Receiver of one propagated try failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum TryPropagationTarget {
    /// A local try target failure type.
    Failure {
        /// The local failure accumulator type.
        ty: dir::GlobalTypeId,
    },
    /// The enclosing function return type.
    Return {
        /// The function return type, if propagation is inside a function.
        ty: Option<dir::GlobalTypeId>,
    },
}

/// Stable dedupe key for one source node task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) enum TaskKey {
    /// Inference of one source node.
    Infer {
        /// The inferred source use.
        site: FlowSite,
        /// The syntactic place use when the node is a place expression.
        use_: PlaceUse,
    },
    /// Checking of one source use against one expected context.
    Check {
        /// The checked source use.
        site: FlowSite,
        /// The expected type resolved by this check.
        expected: ExpectedType,
        /// The checked relation.
        relation: Relation,
        /// The source that produced the check.
        origin: Origin,
        /// The checked value use.
        use_: ValueUse,
    },
}

/// Type expected by a deferred check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ExpectedType {
    /// A concrete expected type.
    Type(dir::GlobalTypeId),
    /// The checked type of another source use.
    Node(FlowSite),
}

/// Priority ordered solver task queue.
#[derive(Debug)]
pub(in crate::check) struct WorkQueue {
    /// Queued tasks by static priority.
    entries: [Vec<Task>; TASK_PRIORITY_COUNT],
    /// Next unread entry by static priority.
    heads: [usize; TASK_PRIORITY_COUNT],
}

/// Mark of a priority ordered solver task queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct WorkMark {
    /// Next unread entry by static priority.
    heads: [usize; TASK_PRIORITY_COUNT],
    /// Queue length by static priority.
    lengths: [usize; TASK_PRIORITY_COUNT],
}

impl WorkQueue {
    /// Create an empty queue.
    pub(in crate::check) fn new() -> Self {
        Self {
            entries: std::array::from_fn(|_| Vec::new()),
            heads: [0; TASK_PRIORITY_COUNT],
        }
    }

    /// Mark the queue for rollback.
    pub(in crate::check) fn mark(&self) -> WorkMark {
        WorkMark {
            heads: self.heads,
            lengths: std::array::from_fn(|index| self.entries[index].len()),
        }
    }

    /// Roll back to one queue mark.
    pub(in crate::check) fn rollback(&mut self, mark: WorkMark) {
        for (index, entries) in self.entries.iter_mut().enumerate() {
            entries.truncate(mark.lengths[index]);
        }
        self.heads = mark.heads;
    }

    /// Queue one task at the back of its priority class.
    pub(in crate::check) fn push(&mut self, task: Task) {
        self.entries[task.priority().index()].push(task);
    }

    /// Pop the next task in priority order.
    pub(in crate::check) fn pop(&mut self) -> Option<Task> {
        for priority in TaskPriority::all() {
            let index = priority.index();
            let head = self.heads[index];
            if let Some(task) = self.entries[index].get(head).cloned() {
                self.heads[index] += 1;

                return Some(task);
            }
        }

        None
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
