use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Constraint, ConstraintId, ConstraintState, ConstraintTable, Dependency, Obligation,
    ObligationId, ObligationTable, Origin, Queue, QueueMark, RelationCache, RelationCacheSnapshot,
    Task, TaskKey, VariableState, VariableTable, Widening,
};

/// Solver state for one checked component.
#[derive(Debug)]
pub(in crate::check) struct Solver {
    /// The next variable index to allocate per module.
    next_variable: IndexMap<ModuleId, u32>,
    /// The next component-global constraint id.
    next_constraint: u32,
    /// The next component-global obligation id.
    next_obligation: u32,
    /// Variables allocated for this component.
    pub(in crate::check) variables: VariableTable,
    /// Constraints collected for this component.
    pub(in crate::check) constraints: ConstraintTable,
    /// Tasks queued for this component.
    pub(in crate::check) queue: Queue,
    /// Relation decisions memoized for this component.
    pub(in crate::check) relations: RelationCache,
    /// Obligations collected for this component.
    pub(in crate::check) obligations: ObligationTable,
    /// Tasks parked on unresolved dependencies.
    waiters: IndexMap<Dependency, SmallVec<[Task; 2]>>,
    /// Completed source-node tasks.
    completed: IndexSet<TaskKey>,
    /// Undo entries recorded by active snapshots.
    undo: Vec<Undo>,
    /// The number of nested snapshots.
    snapshot_depth: usize,
}

/// Snapshot of solver state before one probe.
#[derive(Debug)]
pub(in crate::check) struct SolverSnapshot {
    /// The next constraint id before the probe.
    next_constraint: u32,
    /// The next obligation id before the probe.
    next_obligation: u32,
    /// The queued work before the probe.
    queue: QueueMark,
    /// The undo log length before the probe.
    undo: usize,
    /// Relation cache snapshot before the probe.
    relations: RelationCacheSnapshot,
    /// Type arena mark at probe entry.
    types: TypeMark,
}

/// One solver storage undo entry.
#[derive(Debug, Clone)]
enum Undo {
    /// Undo one variable row mutation.
    Variable {
        /// The changed variable.
        id: dir::TypeVariableId,
        /// The previous variable state.
        previous: Option<VariableState>,
    },
    /// Undo one constraint row mutation.
    Constraint {
        /// The changed constraint.
        id: ConstraintId,
        /// The previous constraint state row.
        previous: ConstraintState,
    },
    /// Undo one waiter row mutation.
    Waiters {
        /// The changed dependency.
        dependency: Dependency,
        /// The previous waiter row.
        previous: Option<SmallVec<[Task; 2]>>,
    },
    /// Undo one completed source-node task.
    Completed {
        /// The completed task.
        task: TaskKey,
    },
}

/// Type arena mark for all loaded module type segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct TypeMark {
    /// The type count for each loaded component module.
    modules: IndexMap<ModuleId, u32>,
}

impl Solver {
    /// Create an empty solver.
    pub(in crate::check) fn new() -> Self {
        Self {
            next_variable: IndexMap::new(),
            next_constraint: 0,
            next_obligation: 0,
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            queue: Queue::new(),
            relations: RelationCache::new(),
            obligations: ObligationTable::new(),
            waiters: IndexMap::new(),
            completed: IndexSet::new(),
            undo: Vec::new(),
            snapshot_depth: 0,
        }
    }

    /// Snapshot the solver before one probe.
    pub(in crate::check) fn snapshot(&mut self, types: TypeMark) -> SolverSnapshot {
        self.snapshot_depth += 1;

        SolverSnapshot {
            next_constraint: self.next_constraint,
            next_obligation: self.next_obligation,
            queue: self.queue.mark(),
            undo: self.undo.len(),
            relations: self.relations.snapshot(),
            types,
        }
    }

    /// Roll back to one solver snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: SolverSnapshot) -> TypeMark {
        while self.undo.len() > snapshot.undo {
            if let Some(undo) = self.undo.pop() {
                self.rollback_undo(undo);
            }
        }

        self.relations.rollback(snapshot.relations);
        self.queue.rollback(snapshot.queue);
        self.remove_constraints_from(snapshot.next_constraint);
        self.remove_obligations_from(snapshot.next_obligation);
        self.next_constraint = snapshot.next_constraint;
        self.next_obligation = snapshot.next_obligation;
        self.snapshot_depth -= 1;

        snapshot.types
    }

    /// Commit one solver snapshot.
    pub(in crate::check) fn commit(&mut self, snapshot: SolverSnapshot) {
        self.relations.commit(snapshot.relations);
        self.snapshot_depth -= 1;

        if self.snapshot_depth == 0 {
            self.undo.truncate(snapshot.undo);
        }
    }

    /// Return whether a snapshot is active.
    pub(in crate::check) fn is_probing(&self) -> bool {
        self.snapshot_depth > 0
    }

    /// Allocate one variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        module: ModuleId,
        origin: Origin,
        widening: Widening,
    ) -> dir::TypeVariableId {
        let next = self.next_variable.entry(module).or_insert(0);
        let variable = dir::TypeVariableId::new(module, *next);
        *next += 1;
        self.record_undo(Undo::Variable {
            id: variable,
            previous: None,
        });
        self.variables.allocate(variable, origin, widening);

        variable
    }

    /// Return one variable state.
    pub(in crate::check) fn variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&VariableState> {
        self.variables.get(variable)
    }

    /// Return one variable state mutably.
    pub(in crate::check) fn variable_mut(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&mut VariableState> {
        self.record_variable(variable)?;

        self.variables.get_mut(variable)
    }

    /// Allocate one constraint.
    pub(in crate::check) fn allocate_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = ConstraintId::at(self.next_constraint as usize);
        self.next_constraint += 1;
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
    pub(in crate::check) fn allocate_obligation(&mut self, obligation: Obligation) -> ObligationId {
        let id = ObligationId::at(self.next_obligation as usize);
        self.next_obligation += 1;
        self.obligations.insert(id, obligation);

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
    ) -> impl Iterator<Item = (dir::TypeVariableId, &VariableState)> {
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

    /// Park one task until a dependency changes.
    pub(in crate::check) fn wait_for(&mut self, dependency: Dependency, task: Task) {
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

    /// Return whether one source-node task already completed.
    pub(in crate::check) fn is_task_complete(&self, task: &Task) -> bool {
        task.key()
            .is_some_and(|task| self.completed.contains(&task))
    }

    /// Mark one source-node task complete.
    pub(in crate::check) fn complete_task(&mut self, task: Task) {
        let Some(task) = task.key() else {
            return;
        };

        if self.completed.contains(&task) {
            return;
        }

        self.record_undo(Undo::Completed { task });
        self.completed.insert(task);
    }

    /// Pop one solver task.
    pub(in crate::check) fn pop_task(&mut self) -> Option<Task> {
        self.queue.pop()
    }

    /// Record one undo entry if a snapshot is active.
    fn record_undo(&mut self, undo: Undo) {
        if self.snapshot_depth > 0 {
            self.undo.push(undo);
        }
    }

    /// Record one variable row if a snapshot is active.
    fn record_variable(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        if self.snapshot_depth > 0 {
            let previous = self.variables.get(id)?.clone();
            self.undo.push(Undo::Variable {
                id,
                previous: Some(previous),
            });
        }

        Ok(())
    }

    /// Record one constraint row if a snapshot is active.
    fn record_constraint(&mut self, id: ConstraintId) -> CompilerResult<()> {
        if self.snapshot_depth > 0 {
            self.undo.push(Undo::Constraint {
                id,
                previous: self.constraints.state(id)?,
            });
        }

        Ok(())
    }

    /// Record one waiter row if a snapshot is active.
    fn record_waiters(&mut self, dependency: Dependency) {
        if self.snapshot_depth > 0 {
            self.undo.push(Undo::Waiters {
                dependency,
                previous: self.waiters.get(&dependency).cloned(),
            });
        }
    }

    /// Apply one undo entry.
    fn rollback_undo(&mut self, undo: Undo) {
        match undo {
            Undo::Variable { id, previous } => match previous {
                Some(previous) => self.variables.insert(id, previous),
                None => {
                    self.variables.remove(id);
                    self.next_variable.insert(id.module_id, id.index);
                }
            },
            Undo::Constraint { id, previous } => self.constraints.set_state(id, previous),
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
            Undo::Completed { task } => {
                self.completed.swap_remove(&task);
            }
        }
    }

    /// Remove constraints allocated after a snapshot mark.
    fn remove_constraints_from(&mut self, next: u32) {
        for index in next..self.next_constraint {
            self.constraints.remove(ConstraintId::at(index as usize));
        }
    }

    /// Remove obligations allocated after a snapshot mark.
    fn remove_obligations_from(&mut self, next: u32) {
        for index in next..self.next_obligation {
            self.obligations.remove(ObligationId::at(index as usize));
        }
    }
}

impl TypeMark {
    /// Create a type mark from loaded module type counts.
    pub(in crate::check) fn new(modules: IndexMap<ModuleId, u32>) -> Self {
        Self { modules }
    }

    /// Iterate per-module type counts.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ModuleId, u32)> + '_ {
        self.modules.iter().map(|(module, count)| (*module, *count))
    }
}
