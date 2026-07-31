use std::mem::replace;

use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    BoundSide, Cause, CauseArena, CauseId, Constraint, ConstraintId, ConstraintResult,
    ConstraintTable, Dependency, FailedCheck, GenericParameterId, InferenceScope, ObligationEntry,
    ObligationId, ObligationTable, Origin, OriginArena, OriginId, RelationCache,
    RelationCacheSnapshot, Task, TypeBound, Variable, VariableRole, VariableTable, Widening,
    WorkQueue,
};
use crate::{CompilerError, CompilerResult};

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
    /// Interned check origins.
    pub(in crate::check) origins: OriginArena,
    /// Failed checks retained until their cause trees are complete.
    pub(in crate::check) failures: Vec<FailedCheck>,
    /// Interned constraint causes.
    pub(in crate::check) causes: CauseArena,
    /// Variables opened for generic parameters, keyed by application.
    instantiations:
        FxIndexMap<(OriginId, GenericParameterId, Option<dir::GlobalTypeId>), dir::TypeVariableId>,
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
    /// The parked work before the probe.
    waiters: FxIndexMap<Dependency, SmallVec<[Task; 2]>>,
    /// The undo log length before the probe.
    undo: usize,
    /// Relation cache snapshot before the probe.
    relations: RelationCacheSnapshot,
}

impl SolverSnapshot {
    /// Return whether one variable existed before this snapshot.
    pub(in crate::check) fn contains_variable(&self, variable: dir::TypeVariableId) -> bool {
        variable.0 < self.variables as u32
    }

    /// Return the inference scope opened by this snapshot.
    pub(in crate::check) fn inference_scope(&self) -> InferenceScope {
        InferenceScope::open(self.variables, self.undo)
    }

    /// Return the constraint count before this snapshot.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.constraints
    }
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
    /// Undo one constraint entry mutation.
    Constraint {
        /// The changed constraint.
        id: ConstraintId,
        /// The previous completed result.
        previous: Option<ConstraintResult>,
    },
    /// Undo one recorded instantiation.
    Instantiation {
        /// The application that opened the parameter.
        key: (OriginId, GenericParameterId, Option<dir::GlobalTypeId>),
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
            failures: Vec::new(),
            causes: CauseArena::default(),
            instantiations: FxIndexMap::default(),
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
            waiters: std::mem::take(&mut self.waiters),
            undo: self.undo.len(),
            relations: self.relations.snapshot(),
        }
    }

    /// Roll back to one solver snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: SolverSnapshot) -> CompilerResult<()> {
        while self.undo.len() > snapshot.undo {
            let undo = self.undo.pop().ok_or_else(|| CompilerError::Internal {
                message: "solver undo log ended before its snapshot mark".into(),
            })?;

            self.rollback_undo(undo)?;
        }

        self.relations.rollback(snapshot.relations);
        self.queue = snapshot.queue;
        self.waiters = snapshot.waiters;
        self.constraints.truncate(snapshot.constraints);
        self.obligations.truncate(snapshot.obligations);
        debug_assert_eq!(self.variables.count(), snapshot.variables);
        self.snapshot_depth -= 1;

        Ok(())
    }

    /// Commit solver state created after one snapshot.
    pub(in crate::check) fn commit(&mut self, snapshot: SolverSnapshot) {
        let SolverSnapshot {
            queue,
            waiters,
            relations,
            ..
        } = snapshot;

        // restore suspended work before probe-local ready tasks
        let probe_queue = replace(&mut self.queue, queue);
        self.queue.append(probe_queue);

        // retain parked probe work alongside suspended outer waiters
        let probe_waiters = std::mem::replace(&mut self.waiters, waiters);
        for (dependency, tasks) in probe_waiters {
            let waiting = self.waiters.entry(dependency).or_default();
            for task in tasks {
                if !waiting.contains(&task) {
                    waiting.push(task);
                }
            }
        }

        self.relations.commit(relations);
        self.snapshot_depth -= 1;
        if self.snapshot_depth == 0 {
            self.undo.clear();
        }
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

    /// Intern one check origin.
    pub(in crate::check) fn intern_origin(&mut self, origin: Origin) -> OriginId {
        self.origins.intern(origin)
    }

    /// Return preexisting variables bounded by one inference scope.
    pub(in crate::check) fn adopted_variables(
        &self,
        scope: InferenceScope,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 4]>> {
        let mutations =
            self.undo
                .get(scope.first_mutation()..)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "inference scope mutation mark {} exceeds length {}",
                        scope.first_mutation(),
                        self.undo.len(),
                    ),
                })?;
        let mut adopted = SmallVec::new();
        for mutation in mutations {
            let Undo::Bound { id, .. } = mutation else {
                continue;
            };
            if !scope.owns(*id) && !adopted.contains(id) {
                adopted.push(*id);
            }
        }

        Ok(adopted)
    }

    /// Intern one constraint cause.
    pub(in crate::check) fn intern_cause(&mut self, cause: Cause) -> CauseId {
        self.causes.intern(cause)
    }

    /// Return one interned constraint cause.
    pub(in crate::check) fn cause(&self, id: CauseId) -> Cause {
        self.causes.get(id)
    }

    /// Return one interned check origin.
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
        // one task collects one constraint, however often walks repeat it
        if let Some(id) = self.constraints.lookup(&constraint) {
            return id;
        }

        let id = ConstraintId::at(self.constraints.count());
        self.constraints.insert(id, constraint);

        id
    }

    /// Set one constraint result.
    pub(in crate::check) fn set_constraint_result(
        &mut self,
        id: ConstraintId,
        result: ConstraintResult,
    ) -> CompilerResult<()> {
        self.record_constraint(id)?;
        self.constraints.set_result(id, Some(result))?;

        Ok(())
    }

    /// Allocate one obligation.
    pub(in crate::check) fn allocate_obligation(&mut self, entry: ObligationEntry) -> ObligationId {
        let id = ObligationId::at(self.obligations.count());
        self.obligations.insert(id, entry);

        id
    }

    /// Return one variable solution.
    pub(in crate::check) fn solution(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(self.variable(variable)?.state.ty())
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
        let waiters = self.waiters.entry(dependency).or_default();
        if !waiters.contains(&task) {
            waiters.push(task);
        }
    }

    /// Queue one task again after its observed dependencies completed.
    pub(in crate::check) fn retry_task(&mut self, task: Task) {
        self.queue.park(&task);
        self.queue.push(task);
    }

    /// Wake tasks parked on one dependency.
    pub(in crate::check) fn wake(&mut self, dependency: Dependency) -> SmallVec<[Task; 2]> {
        self.waiters.swap_remove(&dependency).unwrap_or_default()
    }

    /// Drain every parked dependency, leaving the waiter table empty.
    pub(in crate::check) fn drain_waiters(&mut self) -> Vec<(Dependency, SmallVec<[Task; 2]>)> {
        self.waiters
            .drain(..)
            .filter_map(|(dependency, mut tasks)| {
                tasks.retain(|task| !self.queue.is_finished(task));

                (!tasks.is_empty()).then_some((dependency, tasks))
            })
            .collect()
    }

    /// Return the dependencies parked tasks currently wait on.
    pub(in crate::check) fn waiting_dependencies(&self) -> Vec<Dependency> {
        self.waiters
            .iter()
            .filter(|(_, tasks)| tasks.iter().any(|task| !self.queue.is_finished(task)))
            .map(|(dependency, _)| *dependency)
            .collect()
    }

    /// Pop the next queued check task.
    pub(in crate::check) fn pop_check(&mut self) -> Option<Task> {
        self.queue.pop_check()
    }

    /// Pop the next queued obligation.
    pub(in crate::check) fn pop_obligation(&mut self) -> Option<Task> {
        self.queue.pop_obligation()
    }

    /// Record one undo entry if a snapshot is active.
    fn record_undo(&mut self, undo: Undo) {
        if self.snapshot_depth > 0 {
            self.undo.push(undo);
        }
    }

    /// Return the variable already opened for one parameter at one typing position.
    pub(in crate::check) fn instantiation(
        &self,
        origin: OriginId,
        parameter: GenericParameterId,
        receiver: Option<dir::GlobalTypeId>,
    ) -> Option<dir::TypeVariableId> {
        self.instantiations
            .get(&(origin, parameter, receiver))
            .copied()
    }

    /// Record the variable opened for one parameter at one typing position.
    pub(in crate::check) fn record_instantiation(
        &mut self,
        origin: OriginId,
        parameter: GenericParameterId,
        receiver: Option<dir::GlobalTypeId>,
        variable: dir::TypeVariableId,
    ) {
        self.record_undo(Undo::Instantiation {
            key: (origin, parameter, receiver),
        });
        self.instantiations
            .insert((origin, parameter, receiver), variable);
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
                previous: self.constraints.result(id)?.cloned(),
            });
        }

        Ok(())
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
            Undo::Constraint { id, previous } => self.constraints.set_result(id, previous)?,
            Undo::Instantiation { key } => {
                self.instantiations.swap_remove(&key);
            }
        }

        Ok(())
    }
}
