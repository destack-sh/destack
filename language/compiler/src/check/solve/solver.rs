use std::mem::replace;

use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    BoundSide, Cause, CauseArena, CauseId, Constraint, ConstraintId, ConstraintState,
    ConstraintTable, Dependency, ObligationEntry, ObligationId, ObligationTable, Origin,
    OriginArena, OriginId, RelationCache, RelationCacheSnapshot, Task, TypeBound, Variable,
    VariableRole, VariableTable, Widening, WorkQueue,
};

/// Solver state for one checked component.
#[derive(Debug)]
pub(in crate::check) struct Solver {
    /// Variables allocated for this component.
    pub(in crate::check) variables: VariableTable,
    /// Constraints collected for this component.
    pub(in crate::check) constraints: ConstraintTable,
    /// Tasks queued for this component.
    pub(in crate::check) queue: WorkQueue,
    /// Relation decisions memoized for this component.
    pub(in crate::check) relations: RelationCache,
    /// Obligations collected for this component.
    pub(in crate::check) obligations: ObligationTable,
    /// Interned work origins.
    pub(in crate::check) origins: OriginArena,
    /// Interned judgment causes.
    pub(in crate::check) causes: CauseArena,
    /// Tasks parked on unresolved dependencies.
    waiters: FxIndexMap<Dependency, SmallVec<[Task; 2]>>,
    /// Undo entries recorded by active snapshots.
    undo: Vec<Undo>,
    /// The number of nested snapshots.
    snapshot_depth: usize,
}

/// Snapshot of solver state before one probe.
#[derive(Debug)]
pub(in crate::check) struct SolverSnapshot {
    /// The variable count before the probe.
    variables: usize,
    /// The constraint count before the probe.
    constraints: usize,
    /// The obligation count before the probe.
    obligations: usize,
    /// The queued work before the probe.
    queue: WorkQueue,
    /// The undo log length before the probe.
    undo: usize,
    /// Relation cache snapshot before the probe.
    relations: RelationCacheSnapshot,
}

/// One solver storage undo entry.
#[derive(Debug, Clone)]
enum Undo {
    /// Undo one variable mutation.
    Variable {
        /// The changed variable.
        id: dir::TypeVariableId,
        /// The previous variable and role, absent for undone allocations.
        previous: Option<(Variable, VariableRole)>,
    },
    /// Undo one bound append.
    Bound {
        /// The bounded variable.
        id: dir::TypeVariableId,
        /// The appended side.
        side: BoundSide,
    },
    /// Undo one declared default.
    Default {
        /// The defaulted variable.
        id: dir::TypeVariableId,
        /// The previous default.
        previous: Option<dir::GlobalTypeId>,
    },
    /// Undo one declared parameter bound.
    ParameterBound {
        /// The bounded variable.
        id: dir::TypeVariableId,
        /// The previous parameter bound.
        previous: Option<(dir::GlobalTypeId, CauseId)>,
    },
    /// Undo one constraint entry mutation.
    Constraint {
        /// The changed constraint.
        id: ConstraintId,
        /// The previous constraint state.
        previous: ConstraintState,
    },
    /// Undo one waiter entry mutation.
    Waiters {
        /// The changed dependency.
        dependency: Dependency,
        /// The previous waiter list.
        previous: Option<SmallVec<[Task; 2]>>,
    },
}

impl Solver {
    /// Create an empty solver.
    pub(in crate::check) fn new() -> Self {
        Self {
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            queue: WorkQueue::new(),
            relations: RelationCache::new(),
            obligations: ObligationTable::new(),
            origins: OriginArena::default(),
            causes: CauseArena::default(),
            waiters: FxIndexMap::default(),
            undo: Vec::new(),
            snapshot_depth: 0,
        }
    }

    /// Snapshot the solver before one probe.
    pub(in crate::check) fn snapshot(&mut self) -> SolverSnapshot {
        self.snapshot_depth += 1;

        SolverSnapshot {
            variables: self.variables.count(),
            constraints: self.constraints.count(),
            obligations: self.obligations.count(),
            queue: replace(&mut self.queue, WorkQueue::new()),
            undo: self.undo.len(),
            relations: self.relations.snapshot(),
        }
    }

    /// Roll back to one solver snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: SolverSnapshot) -> CompilerResult<()> {
        while self.undo.len() > snapshot.undo {
            if let Some(undo) = self.undo.pop() {
                self.rollback_undo(undo)?;
            }
        }

        self.relations.rollback(snapshot.relations);
        self.queue = snapshot.queue;
        self.constraints.truncate(snapshot.constraints);
        self.obligations.truncate(snapshot.obligations);
        debug_assert_eq!(self.variables.count(), snapshot.variables);
        self.snapshot_depth -= 1;

        Ok(())
    }

    /// Return whether a snapshot is active.
    pub(in crate::check) fn is_probing(&self) -> bool {
        self.snapshot_depth > 0
    }

    /// Allocate one variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        origin: Origin,
        widening: Widening,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        let variable = dir::TypeVariableId(self.variables.count() as u32);
        self.record_undo(Undo::Variable {
            id: variable,
            previous: None,
        });
        let origin = self.origins.intern(origin);
        self.variables.allocate(variable, origin, widening, role);

        variable
    }

    /// Return one variable's role.
    pub(in crate::check) fn variable_role(
        &self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<VariableRole> {
        self.variables.role(id)
    }

    /// Replace one variable's role, recording undo inside probes.
    pub(in crate::check) fn set_variable_role(
        &mut self,
        id: dir::TypeVariableId,
        role: VariableRole,
    ) -> CompilerResult<()> {
        self.record_variable(id)?;

        self.variables.set_role(id, role)
    }

    /// Intern one work origin.
    pub(in crate::check) fn intern_origin(&mut self, origin: Origin) -> OriginId {
        self.origins.intern(origin)
    }

    /// Intern one judgment cause.
    pub(in crate::check) fn intern_cause(&mut self, cause: Cause) -> CauseId {
        self.causes.intern(cause)
    }

    /// Return one interned judgment cause.
    pub(in crate::check) fn cause(&self, id: CauseId) -> Cause {
        self.causes.get(id)
    }

    /// Return one interned work origin.
    pub(in crate::check) fn origin(&self, id: OriginId) -> Origin {
        self.origins.get(id)
    }

    /// Append one bound to one variable side, returning whether it is new.
    pub(in crate::check) fn push_bound(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
        bound: TypeBound,
    ) -> CompilerResult<bool> {
        let pushed = self.variables.push_bound(id, side, bound)?;
        if pushed {
            self.record_undo(Undo::Bound { id, side });
        }

        Ok(pushed)
    }

    /// Record the declared default completing one variable.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        id: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.record_undo(Undo::Default {
            id,
            previous: self.variables.variable_default(id),
        });
        self.variables.set_default(id, default);
    }

    /// Record the declared parameter bound one variable discharges when solved.
    pub(in crate::check) fn set_variable_parameter_bound(
        &mut self,
        id: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
        cause: CauseId,
    ) {
        self.record_undo(Undo::ParameterBound {
            id,
            previous: self.variables.parameter_bound(id),
        });
        self.variables.set_parameter_bound(id, bound, cause);
    }

    /// Return one variable.
    pub(in crate::check) fn variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&Variable> {
        self.variables.get(variable)
    }

    /// Return one variable mutably.
    pub(in crate::check) fn variable_mut(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&mut Variable> {
        self.record_variable(variable)?;

        self.variables.get_mut(variable)
    }

    /// Allocate one constraint.
    pub(in crate::check) fn allocate_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = ConstraintId::at(self.constraints.count());
        self.constraints.insert(id, constraint);

        id
    }

    /// Set one constraint state.
    pub(in crate::check) fn set_constraint_state(
        &mut self,
        id: ConstraintId,
        state: ConstraintState,
    ) -> CompilerResult<()> {
        self.record_constraint(id)?;
        self.constraints.set_state(id, state);

        Ok(())
    }

    /// Allocate one obligation.
    pub(in crate::check) fn allocate_obligation(&mut self, entry: ObligationEntry) -> ObligationId {
        let id = ObligationId::at(self.obligations.count());
        self.obligations.insert(id, entry);

        id
    }

    /// Return the representative for one variable.
    pub(in crate::check) fn representative(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<dir::TypeVariableId> {
        let mut current = variable;
        while let Some(alias) = self.variable(current)?.alias {
            current = alias;
        }

        Ok(current)
    }

    /// Return one variable solution.
    pub(in crate::check) fn solution(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let representative = self.representative(variable)?;

        Ok(self.variable(representative)?.solution)
    }

    /// Return all variable entries.
    pub(in crate::check) fn variables(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> {
        self.variables.iter()
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.variables.count()
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.constraints.count()
    }

    /// Return the number of collected obligations.
    pub(in crate::check) fn obligation_count(&self) -> usize {
        self.obligations.count()
    }

    /// Queue one solver task.
    pub(in crate::check) fn push_task(&mut self, task: Task) {
        self.queue.push(task);
    }

    /// Complete one active solver task.
    ///
    /// Parked copies on other dependencies stay collected: the queue's
    /// finished set drops their wakes, and the parked sweep skips them.
    pub(in crate::check) fn complete_task(&mut self, task: &Task) {
        self.queue.complete(task);
    }

    /// Park one task until a dependency changes.
    pub(in crate::check) fn wait_for(&mut self, dependency: Dependency, task: Task) {
        self.queue.park(&task);
        self.record_waiters(dependency);
        let waiters = self.waiters.entry(dependency).or_default();
        if !waiters.contains(&task) {
            waiters.push(task);
        }
    }

    /// Wake tasks parked on one dependency.
    pub(in crate::check) fn wake(&mut self, dependency: Dependency) -> SmallVec<[Task; 2]> {
        self.record_waiters(dependency);

        self.waiters.swap_remove(&dependency).unwrap_or_default()
    }

    /// Drain every parked dependency, leaving the waiter table empty.
    pub(in crate::check) fn drain_waiters(&mut self) -> Vec<(Dependency, SmallVec<[Task; 2]>)> {
        self.waiters.drain(..).collect()
    }

    /// Return the dependencies parked tasks currently wait on.
    pub(in crate::check) fn waiting_dependencies(&self) -> Vec<Dependency> {
        self.waiters.keys().copied().collect()
    }

    /// Pop one solver task.
    pub(in crate::check) fn pop_task(&mut self) -> Option<Task> {
        self.queue.pop()
    }

    /// Pop one inference task, leaving obligations for settlement.
    pub(in crate::check) fn pop_inference_task(&mut self) -> Option<Task> {
        self.queue.pop_inference()
    }

    /// Record one undo entry if a snapshot is active.
    fn record_undo(&mut self, undo: Undo) {
        if self.snapshot_depth > 0 {
            self.undo.push(undo);
        }
    }

    /// Record one variable if a snapshot is active.
    fn record_variable(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        if self.snapshot_depth > 0 {
            let previous = *self.variables.get(id)?;
            let role = self.variables.role(id)?;
            self.undo.push(Undo::Variable {
                id,
                previous: Some((previous, role)),
            });
        }

        Ok(())
    }

    /// Record one constraint entry if a snapshot is active.
    fn record_constraint(&mut self, id: ConstraintId) -> CompilerResult<()> {
        if self.snapshot_depth > 0 {
            self.undo.push(Undo::Constraint {
                id,
                previous: self.constraints.state(id)?,
            });
        }

        Ok(())
    }

    /// Record one waiter entry if a snapshot is active.
    fn record_waiters(&mut self, dependency: Dependency) {
        if self.snapshot_depth > 0 {
            self.undo.push(Undo::Waiters {
                dependency,
                previous: self.waiters.get(&dependency).cloned(),
            });
        }
    }

    /// Apply one undo entry.
    fn rollback_undo(&mut self, undo: Undo) -> CompilerResult<()> {
        match undo {
            Undo::Variable { id, previous } => match previous {
                Some((previous, role)) => {
                    *self.variables.get_mut(id)? = previous;
                    self.variables.set_role(id, role)?;
                }
                // undone allocations unwind newest first, so variables truncate
                None => self.variables.truncate(id.0 as usize),
            },
            Undo::Bound { id, side } => {
                self.variables.pop_bound(id, side)?;
            }
            Undo::Default { id, previous } => match previous {
                Some(previous) => self.variables.set_default(id, previous),
                None => self.variables.remove_default(id),
            },
            Undo::ParameterBound { id, previous } => match previous {
                Some((bound, cause)) => self.variables.set_parameter_bound(id, bound, cause),
                None => self.variables.remove_parameter_bound(id),
            },
            Undo::Constraint { id, previous } => {
                self.constraints.set_state(id, previous);
            }
            Undo::Waiters {
                dependency,
                previous,
            } => match previous {
                Some(previous) => {
                    self.waiters.insert(dependency, previous);
                }
                None => {
                    self.waiters.swap_remove(&dependency);
                }
            },
        }

        Ok(())
    }
}
