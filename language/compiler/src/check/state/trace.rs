use std::collections::VecDeque;

use destack_dir as dir;

use crate::check::{Progress, VariableId};

/// Maximum check trace events kept per component.
const CHECK_TRACE_LIMIT: usize = 4096;

/// Bounded trace for one check component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct CheckTrace {
    /// The retained check events.
    pub(in crate::check) events: VecDeque<CheckEvent>,
    /// The total number of events emitted.
    pub(in crate::check) total: usize,
    /// The number of events dropped from the front.
    pub(in crate::check) dropped: usize,
    /// Type variables currently being emitted by commit.
    pub(in crate::check) active_type_commits: Vec<VariableId>,
}

impl CheckTrace {
    /// Create an empty check trace.
    pub(in crate::check) fn new() -> Self {
        Self {
            events: VecDeque::new(),
            total: 0,
            dropped: 0,
            active_type_commits: Vec::new(),
        }
    }

    /// Record one check event.
    pub(in crate::check) fn record(&mut self, event: CheckEvent) {
        self.total += 1;

        if self.events.len() == CHECK_TRACE_LIMIT {
            self.events.pop_front();
            self.dropped += 1;
        }

        self.events.push_back(event);
    }

    /// Enter commit for one type variable.
    pub(in crate::check) fn enter_type_variable_commit(&mut self, event: CommitEvent) -> bool {
        let CommitEvent::TypeVariableStarted { variable, .. } = event else {
            panic!("type variable commit entry requires a start event");
        };

        self.record(CheckEvent::Commit(event));
        if self.active_type_commits.contains(&variable) {
            self.record(CheckEvent::Commit(CommitEvent::CircularTypeVariable {
                variable,
            }));

            return false;
        }

        self.active_type_commits.push(variable);

        true
    }

    /// Leave commit for one type variable.
    pub(in crate::check) fn finish_type_variable_commit(&mut self, event: CommitEvent) {
        let CommitEvent::TypeVariableFinished { variable, .. } = event else {
            panic!("type variable commit exit requires a finish event");
        };
        let active = self
            .active_type_commits
            .pop()
            .unwrap_or_else(|| panic!("type variable commit stack is empty"));
        assert_eq!(
            active, variable,
            "type variable commit finished out of stack order"
        );

        self.record(CheckEvent::Commit(event));
    }
}

/// One event emitted by check collection, solving, or commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckEvent {
    /// Solver event.
    Solve(SolveEvent),
    /// Commit event.
    Commit(CommitEvent),
}

/// One event emitted by the solver scheduler or reduction loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveEvent {
    /// The solver started.
    Started {
        /// The number of queued tasks.
        tasks: usize,
        /// The number of variables present before solving.
        variables: usize,
    },
    /// One solver pass ran once.
    PassStepped {
        /// The pass index.
        pass: usize,
        /// The progress produced by the task.
        progress: SolveProgress,
    },
    /// The solver reached an empty queue.
    Finished {
        /// The number of passes run.
        passes: usize,
        /// The final number of variables.
        variables: usize,
    },
}

/// One event emitted while committing checked output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CommitEvent {
    /// A type variable started committing.
    TypeVariableStarted {
        /// The variable being committed.
        variable: VariableId,
        /// The source node receiving the committed type.
        source: dir::LocalNodeIdAny,
        /// The symbol output when the variable has one.
        symbol: Option<dir::GlobalSymbolId>,
        /// The symbol kind when the variable has one.
        kind: Option<dir::SymbolKind>,
    },
    /// A type variable committed.
    TypeVariableFinished {
        /// The variable that was committed.
        variable: VariableId,
        /// The resulting type id when one was emitted.
        ty: Option<dir::GlobalTypeId>,
    },
    /// Commit reached a type variable already active on the stack.
    CircularTypeVariable {
        /// The variable that closed the cycle.
        variable: VariableId,
    },
}

/// Compact progress summary for one solver task step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveProgress {
    /// The step changed no variables.
    Unchanged,
    /// The step changed variables.
    Changed {
        /// The number of changed variables.
        variables: usize,
    },
}

impl From<&Progress> for SolveProgress {
    /// Return a compact solver progress summary.
    fn from(progress: &Progress) -> Self {
        match progress {
            Progress::Unchanged => Self::Unchanged,
            Progress::Changed(variables) => Self::Changed {
                variables: variables.len(),
            },
        }
    }
}
