use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::check::{CheckState, ConstraintId, Relation, Task};
use crate::{CompilerError, CompilerResult};

/// One undoable inference mutation.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Mutation {
    /// A variable was allocated.
    VariableAllocated { variable: dir::TypeVariableId },
    /// A working type was allocated in one module.
    TypeAllocated { module: ModuleId },
    /// A lower bound was pushed onto one variable.
    LowerBoundPushed { variable: dir::TypeVariableId },
    /// An upper bound was pushed onto one variable.
    UpperBoundPushed { variable: dir::TypeVariableId },
    /// A solution was set on one variable, draining its waiters.
    SolutionSet {
        /// The solved variable.
        variable: dir::TypeVariableId,
        /// The drained waiter tasks.
        waiters: SmallVec<[Task; 2]>,
    },
    /// An alias was set on one variable, draining its bounds and waiters.
    AliasSet {
        /// The aliased variable.
        variable: dir::TypeVariableId,
        /// The drained lower bounds.
        lower: SmallVec<[dir::GlobalTypeId; 2]>,
        /// The drained upper bounds.
        upper: SmallVec<[dir::GlobalTypeId; 2]>,
        /// The drained waiter tasks.
        waiters: SmallVec<[Task; 2]>,
    },
    /// A waiter task was pushed onto one variable.
    WaiterPushed { variable: dir::TypeVariableId },
    /// A constraint was allocated.
    ConstraintAllocated,
    /// An obligation was allocated.
    ObligationAllocated,
    /// A constraint was completed.
    ConstraintCompleted { constraint: ConstraintId },
    /// A relation decision was memoized.
    RelationDecided {
        /// The decided relation kind.
        relation: Relation,
        /// The reduced left operand root.
        left: dir::GlobalTypeId,
        /// The reduced right operand root.
        right: dir::GlobalTypeId,
    },
    /// A decision was recorded for one node, draining its waiters.
    DecisionSet {
        /// The decided node.
        node: dir::GlobalNodeIdAny,
        /// The drained waiter tasks.
        waiters: SmallVec<[Task; 2]>,
    },
    /// An implicit coercion was recorded for one value node.
    CoercionSet {
        /// The coerced value node.
        node: dir::GlobalNodeIdAny,
        /// The replaced coercion entry.
        previous: Option<dir::Coercion>,
    },
    /// A waiter task was pushed onto one undecided node.
    DecisionWaiterPushed { node: dir::GlobalNodeIdAny },
    /// A task was queued.
    TaskQueued { task: Task },
    /// A task was popped off the queue.
    TaskPopped { task: Task },
}

/// Mutation log enabling speculative inference rollback.
#[derive(Debug)]
pub(in crate::check) struct Journal {
    /// The recorded mutations since the oldest active probe.
    mutations: Vec<Mutation>,
    /// The number of active probes.
    depth: usize,
}

/// One live speculative inference probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Probe {
    /// The probe nesting depth at creation.
    depth: usize,
    /// The journal length at creation.
    mark: usize,
}

impl Journal {
    /// Create an empty journal.
    pub(in crate::check) fn new() -> Self {
        Self {
            mutations: Vec::new(),
            depth: 0,
        }
    }

    /// Return whether a probe is active and mutations are recorded.
    pub(in crate::check) fn is_active(&self) -> bool {
        self.depth > 0
    }

    /// Record one mutation when a probe is active.
    /// Rollback is the journal's only reader, so committed work outside
    /// any probe records nothing and costs nothing.
    pub(in crate::check) fn record(&mut self, mutation: Mutation) {
        if self.depth > 0 {
            self.mutations.push(mutation);
        }
    }

    /// Record one lazily built mutation when a probe is active.
    /// The closure defers payload clones to recording probes.
    pub(in crate::check) fn record_with(&mut self, mutation: impl FnOnce() -> Mutation) {
        if self.depth > 0 {
            self.mutations.push(mutation());
        }
    }

    /// Begin one speculative probe.
    fn begin(&mut self) -> Probe {
        let probe = Probe {
            depth: self.depth,
            mark: self.mutations.len(),
        };
        self.depth += 1;

        probe
    }

    /// Keep one probe's mutations and close it.
    fn keep(&mut self, probe: Probe) -> CompilerResult<()> {
        self.expect_innermost(probe)?;
        self.depth -= 1;

        // forget recorded mutations once no probe is active
        if self.depth == 0 {
            self.mutations.clear();
        }

        Ok(())
    }

    /// Close one probe and return its mutations in recorded order.
    fn unwind(&mut self, probe: Probe) -> CompilerResult<Vec<Mutation>> {
        self.expect_innermost(probe)?;
        self.depth -= 1;

        Ok(self.mutations.split_off(probe.mark))
    }

    /// Require one probe to be the innermost active probe.
    fn expect_innermost(&self, probe: Probe) -> CompilerResult<()> {
        if probe.depth + 1 != self.depth {
            return Err(CompilerError::Internal {
                message: "check probes must close in LIFO order".into(),
            });
        }

        Ok(())
    }
}

impl CheckState<'_> {
    /// Begin one speculative probe.
    pub(in crate::check) fn begin_probe(&mut self) -> Probe {
        self.journal.begin()
    }

    /// Keep one probe's mutations and close it.
    pub(in crate::check) fn keep_probe(&mut self, probe: Probe) -> CompilerResult<()> {
        self.journal.keep(probe)
    }

    /// Roll back one probe's mutations and close it.
    pub(in crate::check) fn unwind_probe(&mut self, probe: Probe) -> CompilerResult<()> {
        let mutations = self.journal.unwind(probe)?;

        // undo mutations newest first
        for mutation in mutations.into_iter().rev() {
            self.undo(mutation)?;
        }

        // validate that no rolled back id leaked into live state
        self.validate_variable_bounds();

        Ok(())
    }

    /// Roll back one probe's shared solver mutations and close it.
    pub(in crate::check) fn harvest_probe(&mut self, probe: Probe) -> CompilerResult<()> {
        let mutations = self.journal.unwind(probe)?;

        // collect variables allocated inside the probe
        let mut local = IndexSet::new();
        for mutation in &mutations {
            if let Mutation::VariableAllocated { variable } = mutation {
                local.insert(*variable);
            }
        }

        // split kept allocations and probe-local bindings from shared
        // solver mutations
        let mut kept = Vec::new();
        let mut undone = Vec::new();
        for mutation in mutations {
            match &mutation {
                Mutation::TypeAllocated { .. } | Mutation::VariableAllocated { .. } => {
                    kept.push(mutation)
                }
                Mutation::SolutionSet { variable, .. }
                | Mutation::AliasSet { variable, .. }
                | Mutation::LowerBoundPushed { variable }
                | Mutation::UpperBoundPushed { variable }
                    if local.contains(variable) =>
                {
                    kept.push(mutation)
                }
                _ => undone.push(mutation),
            }
        }

        // undo shared solver mutations newest first
        for mutation in undone.into_iter().rev() {
            self.undo(mutation)?;
        }

        // surviving allocations stay speculative under an enclosing
        // probe, so its rollback still truncates them positionally
        for mutation in kept {
            self.journal.record(mutation);
        }

        // validate that no rolled back id leaked into live state
        self.validate_variable_bounds();

        Ok(())
    }

    /// Panic on any variable bound referencing an unallocated type.
    fn validate_variable_bounds(&self) {
        for (variable, state) in self.variables.iter() {
            for bound in state.lower.iter().chain(state.upper.iter()) {
                let live = self.modules.get(&bound.module_id).is_some_and(|module| {
                    module
                        .working
                        .types
                        .get_type_maybe(bound.local_id)
                        .is_some()
                        || module.type_maybe(bound.local_id).is_some()
                }) || self.external_modules.contains_key(&bound.module_id);

                if !live {
                    panic!(
                        "probe unwind leaked bound {bound:?} on variable {variable:?}                          lower={:?} upper={:?}",
                        state.lower, state.upper
                    );
                }
            }
        }
    }

    /// Undo one recorded mutation.
    fn undo(&mut self, mutation: Mutation) -> CompilerResult<()> {
        match mutation {
            // drop the youngest working type
            Mutation::TypeAllocated { module } => {
                if let Some(module) = self.modules.get_mut(&module) {
                    let types = &mut module.working.types;
                    let count = types.type_count();
                    types.truncate_types(count.saturating_sub(1));
                }
            }
            // drop the youngest allocation
            Mutation::VariableAllocated { variable } => {
                let count = self.variables.count_in(variable.module_id);
                self.variables
                    .truncate(variable.module_id, count.saturating_sub(1));
            }
            // pop pushed bounds and waiters
            Mutation::LowerBoundPushed { variable } => {
                self.variables.get_mut(variable)?.lower.pop();
            }
            Mutation::UpperBoundPushed { variable } => {
                self.variables.get_mut(variable)?.upper.pop();
            }
            Mutation::WaiterPushed { variable } => {
                self.variables.get_mut(variable)?.waiters.pop();
            }
            // restore drained solution state
            Mutation::SolutionSet { variable, waiters } => {
                let state = self.variables.get_mut(variable)?;
                state.solution = None;
                state.waiters = waiters;
            }
            // restore drained alias state
            Mutation::AliasSet {
                variable,
                lower,
                upper,
                waiters,
            } => {
                let state = self.variables.get_mut(variable)?;
                state.alias = None;
                state.lower = lower;
                state.upper = upper;
                state.waiters = waiters;
            }
            // drop the youngest constraint
            Mutation::ConstraintAllocated => {
                let count = self.constraints.count();
                self.constraints.truncate(count.saturating_sub(1));
            }
            // drop the youngest obligation
            Mutation::ObligationAllocated => {
                let count = self.obligations.count();
                self.obligations.truncate(count.saturating_sub(1));
            }
            Mutation::ConstraintCompleted { constraint } => {
                self.constraints.reopen(constraint);
            }
            // forget speculative relation verdicts
            Mutation::RelationDecided {
                relation,
                left,
                right,
            } => self.relations.remove(relation, left, right),
            // forget speculative decisions, restoring parked waiters
            Mutation::DecisionSet { node, waiters } => self.decisions.undecide(node, waiters),
            // forget speculative coercions, restoring replaced entries
            Mutation::CoercionSet { node, previous } => match previous {
                Some(previous) => {
                    self.coercions.insert(node, previous);
                }
                None => {
                    self.coercions.shift_remove(&node);
                }
            },
            Mutation::DecisionWaiterPushed { node } => self.decisions.pop_waiter(node),
            // restore queue order
            Mutation::TaskQueued { task } => self.queue.remove_last(task),
            Mutation::TaskPopped { task } => self.queue.push_front(task),
        }

        Ok(())
    }
}
