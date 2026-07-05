use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Constraint, ConstraintId, ConstraintState, ConstraintTable, Dependency, ObligationEntry,
    ObligationId, ObligationTable, Opening, Origin, RelationCache, RelationCacheSnapshot, Task,
    TaskKey, TypeSubstitution, VariableState, VariableTable, Widening, WorkMark, WorkQueue,
};

/// Solver state for one checked component.
#[derive(Debug)]
pub(in crate::check) struct Solver {
    /// The next component-global variable index to allocate.
    next_variable: u32,
    /// The next component-global constraint id.
    next_constraint: u32,
    /// The next component-global obligation id.
    next_obligation: u32,

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
    /// Generic substitutions opened by source sites across solver polls.
    pub(in crate::check) opened_substitutions: IndexMap<Opening, TypeSubstitution>,

    /// Tasks parked on unresolved dependencies.
    waiters: IndexMap<Dependency, SmallVec<[Task; 2]>>,
    /// Variables whose weak solutions wait until the regular queue drains.
    weak_solves: IndexSet<dir::TypeVariableId>,
    /// Completed source typing task keys.
    completed_keys: IndexSet<TaskKey>,
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
    queue: WorkMark,
    /// The undo log length before the probe.
    undo: usize,
    /// Relation cache snapshot before the probe.
    relations: RelationCacheSnapshot,
}

/// One solver storage undo entry.
#[derive(Debug, Clone)]
enum Undo {
    /// Undo one variable entry mutation.
    Variable {
        /// The changed variable.
        id: dir::TypeVariableId,
        /// The previous variable state.
        previous: Option<VariableState>,
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
    /// Undo one opened substitution mutation.
    OpenedSubstitution {
        /// The changed opening.
        opening: Opening,
        /// The previous substitution.
        previous: Option<TypeSubstitution>,
    },
    /// Undo one completed source typing task key.
    CompletedKey {
        /// The completed task key.
        key: TaskKey,
    },
}

impl Solver {
    /// Create an empty solver.
    pub(in crate::check) fn new() -> Self {
        Self {
            next_variable: 0,
            next_constraint: 0,
            next_obligation: 0,
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            queue: WorkQueue::new(),
            relations: RelationCache::new(),
            obligations: ObligationTable::new(),
            opened_substitutions: IndexMap::new(),
            waiters: IndexMap::new(),
            weak_solves: IndexSet::new(),
            completed_keys: IndexSet::new(),
            undo: Vec::new(),
            snapshot_depth: 0,
        }
    }

    /// Defer one variable's weak solution until the regular queue drains.
    pub(in crate::check) fn defer_weak_solve(&mut self, variable: dir::TypeVariableId) {
        self.weak_solves.insert(variable);
    }

    /// Take the next variable whose weak solution can run after the regular queue drains.
    pub(in crate::check) fn pop_weak_solve(&mut self) -> Option<dir::TypeVariableId> {
        self.weak_solves.shift_remove_index(0)
    }

    /// Snapshot the solver before one probe.
    pub(in crate::check) fn snapshot(&mut self) -> SolverSnapshot {
        self.snapshot_depth += 1;

        SolverSnapshot {
            next_constraint: self.next_constraint,
            next_obligation: self.next_obligation,
            queue: self.queue.mark(),
            undo: self.undo.len(),
            relations: self.relations.snapshot(),
        }
    }

    /// Roll back to one solver snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: SolverSnapshot) {
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
        _module: ModuleId,
        origin: Origin,
        widening: Widening,
    ) -> dir::TypeVariableId {
        let variable = dir::TypeVariableId(self.next_variable);
        self.next_variable += 1;
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
    pub(in crate::check) fn allocate_obligation(&mut self, entry: ObligationEntry) -> ObligationId {
        let id = ObligationId::at(self.next_obligation as usize);
        self.next_obligation += 1;
        self.obligations.insert(id, entry);

        id
    }

    /// Return one substitution opened by a source site.
    pub(in crate::check) fn opened_substitution(
        &self,
        opening: Opening,
    ) -> Option<&TypeSubstitution> {
        self.opened_substitutions.get(&opening)
    }

    /// Insert one substitution opened by a source site.
    pub(in crate::check) fn insert_opened_substitution(
        &mut self,
        opening: Opening,
        substitution: TypeSubstitution,
    ) {
        self.record_opened_substitution(opening);
        self.opened_substitutions.insert(opening, substitution);
    }

    /// Return the representative for one variable.
    pub(super) fn representative(
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

    /// Return whether this source task has already completed.
    pub(in crate::check) fn is_task_complete(&self, task: &Task) -> bool {
        task.key()
            .is_some_and(|key| self.completed_keys.contains(&key))
    }

    /// Mark this source task as completed.
    pub(in crate::check) fn complete_task(&mut self, task: &Task) {
        let Some(key) = task.key() else {
            return;
        };

        if self.completed_keys.contains(&key) {
            return;
        }

        if self.snapshot_depth > 0 {
            self.undo.push(Undo::CompletedKey { key: key.clone() });
        }
        self.completed_keys.insert(key);
    }

    /// Drain every parked dependency, leaving the waiter table empty.
    pub(in crate::check) fn drain_waiters(&mut self) -> Vec<(Dependency, SmallVec<[Task; 2]>)> {
        self.waiters.drain(..).collect()
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

    /// Record one variable entry if a snapshot is active.
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

    /// Record one opened substitution if a snapshot is active.
    fn record_opened_substitution(&mut self, opening: Opening) {
        if self.snapshot_depth > 0 {
            self.undo.push(Undo::OpenedSubstitution {
                opening,
                previous: self.opened_substitutions.get(&opening).cloned(),
            });
        }
    }

    /// Apply one undo entry.
    fn rollback_undo(&mut self, undo: Undo) {
        match undo {
            Undo::Variable { id, previous } => match previous {
                Some(previous) => self.variables.insert(id, previous),
                None => {
                    // the allocation counter stays monotonic; undone ids simply go unused
                    self.variables.remove(id);
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
            Undo::OpenedSubstitution { opening, previous } => match previous {
                Some(previous) => {
                    self.opened_substitutions.insert(opening, previous);
                }
                None => {
                    self.opened_substitutions.swap_remove(&opening);
                }
            },
            Undo::CompletedKey { key } => {
                self.completed_keys.swap_remove(&key);
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
