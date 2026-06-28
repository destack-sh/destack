use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, ConstraintId, FlowPointId, ObligationId, Origin, Relation, ValueUse,
    Widening, WriteTarget,
};

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
    Solve(dir::TypeVariableId),
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
    /// Bind one symbol type from an initializer expression.
    Bind {
        /// The binding symbol.
        symbol: dir::GlobalSymbolId,
        /// The initializer expression.
        initializer: dir::GlobalNodeId<dir::Expression>,
        /// The binding widening policy.
        widening: Widening,
    },
}

impl Task {
    /// Return the task family name.
    pub(in crate::check) fn name(&self) -> &'static str {
        match self {
            Self::Relate(_) => "relate",
            Self::Propagate(_) => "propagate",
            Self::Oblige(_) => "oblige",
            Self::Solve(_) => "solve",
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
            Self::Relate(_) | Self::Oblige(_) | Self::Solve(_) | Self::Bind { .. } => None,
        }
    }

    /// Return the stable completion key for source-node work.
    pub(in crate::check) fn key(&self) -> Option<TaskKey> {
        match self {
            Self::Infer { site, use_ } => Some(TaskKey::Infer {
                site: *site,
                use_: *use_,
            }),
            Self::Check {
                site,
                relation,
                origin,
                use_,
                ..
            } => Some(TaskKey::Check {
                site: *site,
                relation: *relation,
                origin: *origin,
                use_: *use_,
            }),
            Self::Relate(_)
            | Self::Propagate(_)
            | Self::Oblige(_)
            | Self::Solve(_)
            | Self::Bind { .. } => None,
        }
    }
}

/// One propagation of a fallible try value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct TryPropagation {
    /// The try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected node whose checked type is the tried value.
    pub(in crate::check) value: dir::GlobalNodeIdAny,
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

/// Stable identity of source-node solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum TaskKey {
    /// Inference of one source node.
    Infer {
        /// The inferred source node.
        site: FlowSite,
        /// The syntactic place use when the node is a place expression.
        use_: PlaceUse,
    },
    /// Checking of one source use against one expected context.
    Check {
        /// The checked source use.
        site: FlowSite,
        /// The checked relation.
        relation: Relation,
        /// The source that produced the check.
        origin: Origin,
        /// The checked value use.
        use_: ValueUse,
    },
}

/// Type expected by a deferred check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ExpectedType {
    /// A concrete expected type.
    Type(dir::GlobalTypeId),
    /// The checked type of another source node.
    Node(dir::GlobalNodeIdAny),
    /// The checked type of one selected writable place.
    Place(WriteTarget),
}

impl ExpectedType {
    /// Resolve this expected type.
    pub(in crate::check) fn resolve(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match self {
            Self::Type(ty) => Ok(Answer::Ready(*ty)),
            Self::Node(node) => check.node_type(*node),
            Self::Place(place) => check.place_type(place.clone()),
        }
    }
}

/// One source use under a flow point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct FlowSite {
    /// The source node.
    pub(in crate::check) node: dir::GlobalNodeIdAny,
    /// The flow point where the node is used.
    pub(in crate::check) flow: FlowPointId,
}

/// Priority-ordered solver work queues.
#[derive(Debug)]
pub(in crate::check) struct Queue {
    /// Pending relation constraints.
    relate: WorkQueue<ConstraintId>,
    /// Pending try propagation work.
    propagate: WorkQueue<TryPropagation>,
    /// Pending source checks with expected types.
    check: WorkQueue<Task>,
    /// Pending source inference.
    infer: WorkQueue<Task>,
    /// Pending binding publication.
    bind: WorkQueue<Task>,
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
    /// Mark for try propagation.
    propagate: WorkMark,
    /// Mark for source checks.
    check: WorkMark,
    /// Mark for source inference.
    infer: WorkMark,
    /// Mark for binding publication.
    bind: WorkMark,
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
            propagate: WorkQueue::new(),
            check: WorkQueue::new(),
            infer: WorkQueue::new(),
            bind: WorkQueue::new(),
            solve: WorkQueue::new(),
            oblige: WorkQueue::new(),
        }
    }

    /// Mark the queue for rollback.
    pub(in crate::check) fn mark(&self) -> QueueMark {
        QueueMark {
            relate: self.relate.mark(),
            propagate: self.propagate.mark(),
            check: self.check.mark(),
            infer: self.infer.mark(),
            bind: self.bind.mark(),
            solve: self.solve.mark(),
            oblige: self.oblige.mark(),
        }
    }

    /// Roll back to one queue mark.
    pub(in crate::check) fn rollback(&mut self, mark: QueueMark) {
        self.relate.rollback(mark.relate);
        self.propagate.rollback(mark.propagate);
        self.check.rollback(mark.check);
        self.infer.rollback(mark.infer);
        self.bind.rollback(mark.bind);
        self.solve.rollback(mark.solve);
        self.oblige.rollback(mark.oblige);
    }

    /// Queue one task at the back of its priority class.
    pub(in crate::check) fn push(&mut self, task: Task) {
        match task {
            Task::Relate(id) => self.relate.push(id),
            Task::Propagate(propagation) => self.propagate.push(propagation),
            Task::Oblige(id) => self.oblige.push(id),
            Task::Solve(variable) => self.solve.push(variable),
            task @ Task::Check { .. } => self.check.push(task),
            task @ Task::Infer { .. } => self.infer.push(task),
            task @ Task::Bind { .. } => self.bind.push(task),
        }
    }

    /// Pop the next task in priority order.
    pub(in crate::check) fn pop(&mut self) -> Option<Task> {
        // drain bound-producing work before source inference and validation
        if let Some(id) = self.relate.pop() {
            Some(Task::Relate(id))
        } else if let Some(propagation) = self.propagate.pop() {
            Some(Task::Propagate(propagation))
        } else if let Some(task) = self.check.pop() {
            Some(task)
        } else if let Some(task) = self.infer.pop() {
            Some(task)
        } else if let Some(task) = self.bind.pop() {
            Some(task)
        } else if let Some(variable) = self.solve.pop() {
            Some(Task::Solve(variable))
        } else {
            self.oblige.pop().map(Task::Oblige)
        }
    }

    /// Return the total number of queued tasks.
    pub(in crate::check) fn len(&self) -> usize {
        self.relate.len()
            + self.check.len()
            + self.propagate.len()
            + self.infer.len()
            + self.bind.len()
            + self.solve.len()
            + self.oblige.len()
    }
}

impl<T> WorkQueue<T> {
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
    fn pop(&mut self) -> Option<T>
    where
        T: Clone,
    {
        let entry = self.entries.get(self.head).cloned()?;
        self.head += 1;

        Some(entry)
    }

    /// Return the number of pending entries.
    fn len(&self) -> usize {
        self.entries.len().saturating_sub(self.head)
    }
}
