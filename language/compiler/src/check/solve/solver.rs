use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    Constraint, ConstraintId, ConstraintState, ConstraintTable, Decision, DecisionSlot,
    DecisionTable, Obligation, ObligationId, ObligationTable, Origin, Queue, QueueMark,
    RelationCache, RelationCacheSnapshot, Selection, SelectionId, SelectionState, SelectionTable,
    Task, VariableState, VariableTable, Widening,
};
use crate::{CompilerError, CompilerResult};

/// Solver state for one checked component.
#[derive(Debug)]
pub(in crate::check) struct Solver {
    /// The next variable index to allocate per module.
    next_variable: IndexMap<ModuleId, u32>,
    /// The next component-global constraint id.
    next_constraint: u32,
    /// The next component-global selection id.
    next_selection: u32,
    /// The next component-global obligation id.
    next_obligation: u32,
    /// Variables allocated for this component.
    pub(in crate::check) variables: VariableTable,
    /// Constraints collected for this component.
    pub(in crate::check) constraints: ConstraintTable,
    /// Selections collected for this component.
    pub(in crate::check) selections: SelectionTable,
    /// Tasks queued for this component.
    pub(in crate::check) queue: Queue,
    /// Relation decisions memoized for this component.
    pub(in crate::check) relations: RelationCache,
    /// Node types created while walking this component.
    pub(in crate::check) node_types: IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Node decisions selected for this component.
    pub(in crate::check) decisions: DecisionTable,
    /// Obligations collected for this component.
    pub(in crate::check) obligations: ObligationTable,
    /// Undo entries recorded while snapshots are active.
    undo: Vec<Undo>,
    /// The number of active snapshots.
    active_snapshots: usize,
}

/// Snapshot of solver state before one speculative probe.
#[derive(Debug)]
pub(in crate::check) struct SolverSnapshot {
    /// The next constraint id before the probe.
    next_constraint: u32,
    /// The next selection id before the probe.
    next_selection: u32,
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
    /// Undo one selection row mutation.
    Selection {
        /// The changed selection.
        id: SelectionId,
        /// The previous selection state row.
        previous: SelectionState,
    },
    /// Undo one node decision slot mutation.
    Decision {
        /// The changed source node.
        node: dir::GlobalNodeIdAny,
        /// The previous decision slot.
        previous: Option<DecisionSlot>,
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
            next_selection: 0,
            next_obligation: 0,
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            selections: SelectionTable::new(),
            queue: Queue::new(),
            relations: RelationCache::new(),
            node_types: IndexMap::new(),
            decisions: DecisionTable::new(),
            obligations: ObligationTable::new(),
            undo: Vec::new(),
            active_snapshots: 0,
        }
    }

    /// Snapshot the solver before one speculative probe.
    pub(in crate::check) fn snapshot(&mut self, types: TypeMark) -> SolverSnapshot {
        self.active_snapshots += 1;

        SolverSnapshot {
            next_constraint: self.next_constraint,
            next_selection: self.next_selection,
            next_obligation: self.next_obligation,
            queue: self.queue.mark(),
            undo: self.undo.len(),
            relations: self.relations.snapshot(),
            types,
        }
    }

    /// Roll back to a previous speculative snapshot.
    pub(in crate::check) fn rollback(&mut self, snapshot: SolverSnapshot) -> TypeMark {
        while self.undo.len() > snapshot.undo {
            if let Some(undo) = self.undo.pop() {
                self.rollback_undo(undo);
            }
        }

        self.relations.rollback(snapshot.relations);
        self.queue.rollback(snapshot.queue);
        self.remove_constraints_from(snapshot.next_constraint);
        self.remove_selections_from(snapshot.next_selection);
        self.remove_obligations_from(snapshot.next_obligation);
        self.next_constraint = snapshot.next_constraint;
        self.next_selection = snapshot.next_selection;
        self.next_obligation = snapshot.next_obligation;
        self.active_snapshots -= 1;

        snapshot.types
    }

    /// Commit a previous speculative snapshot.
    pub(in crate::check) fn commit(&mut self, snapshot: SolverSnapshot) {
        self.relations.commit(snapshot.relations);
        self.active_snapshots -= 1;

        if self.active_snapshots == 0 {
            self.undo.clear();
        }
    }

    /// Return whether a speculative snapshot is active.
    pub(in crate::check) fn is_probing(&self) -> bool {
        self.active_snapshots > 0
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

    /// Allocate one selection.
    pub(in crate::check) fn allocate_selection(&mut self, selection: Selection) -> SelectionId {
        let id = SelectionId::at(self.next_selection as usize);
        self.next_selection += 1;
        self.selections.insert(id, selection);

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

    /// Set one selection state.
    pub(in crate::check) fn set_selection_state(
        &mut self,
        id: SelectionId,
        state: SelectionState,
    ) -> CompilerResult<()> {
        self.record_selection(id)?;
        self.selections.set_state(id, state);

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

    /// Return the number of decided nodes.
    pub(in crate::check) fn decision_count(&self) -> usize {
        self.decisions.count()
    }

    /// Return one checked node type.
    pub(in crate::check) fn node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.node_types.get(&node).copied()
    }

    /// Set one checked node type.
    pub(in crate::check) fn set_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.node_type(node) {
            if previous != ty {
                return Err(CompilerError::Internal {
                    message: format!("check node {node:?} received two types"),
                });
            }
        }

        self.node_types.insert(node, ty);

        Ok(())
    }

    /// Iterate checked node types.
    pub(in crate::check) fn node_types(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, dir::GlobalTypeId)> + '_ {
        self.node_types.iter().map(|(node, ty)| (*node, *ty))
    }

    /// Return one selected node decision.
    pub(in crate::check) fn decision(&self, node: dir::GlobalNodeIdAny) -> Option<&Decision> {
        self.decisions.get(node)
    }

    /// Record one node decision and return the woken waiters.
    pub(in crate::check) fn decide(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<smallvec::SmallVec<[Task; 2]>> {
        self.record_decision(node);

        self.decisions.decide(node, decision)
    }

    /// Park one task until the node decides.
    pub(in crate::check) fn wait_for_decision(&mut self, node: dir::GlobalNodeIdAny, task: Task) {
        self.record_decision(node);
        self.decisions.wait(node, task);
    }

    /// Queue one solver task.
    pub(in crate::check) fn push_task(&mut self, task: Task) {
        self.queue.push(task);
    }

    /// Pop one solver task.
    pub(in crate::check) fn pop_task(&mut self) -> Option<Task> {
        self.queue.pop()
    }

    /// Record one undo entry if a snapshot is active.
    fn record_undo(&mut self, undo: Undo) {
        if self.active_snapshots > 0 {
            self.undo.push(undo);
        }
    }

    /// Record one variable row if a snapshot is active.
    fn record_variable(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        if self.active_snapshots > 0 {
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
        if self.active_snapshots > 0 {
            self.undo.push(Undo::Constraint {
                id,
                previous: self.constraints.state(id)?,
            });
        }

        Ok(())
    }

    /// Record one selection row if a snapshot is active.
    fn record_selection(&mut self, id: SelectionId) -> CompilerResult<()> {
        if self.active_snapshots > 0 {
            self.undo.push(Undo::Selection {
                id,
                previous: self.selections.state(id)?,
            });
        }

        Ok(())
    }

    /// Record one decision slot if a snapshot is active.
    fn record_decision(&mut self, node: dir::GlobalNodeIdAny) {
        if self.active_snapshots > 0 {
            self.undo.push(Undo::Decision {
                node,
                previous: self.decisions.slot(node).cloned(),
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
            Undo::Selection { id, previous } => self.selections.set_state(id, previous),
            Undo::Decision { node, previous } => match previous {
                Some(previous) => self.decisions.insert_slot(node, previous),
                None => {
                    self.decisions.remove(node);
                }
            },
        }
    }

    /// Remove constraints allocated after a snapshot mark.
    fn remove_constraints_from(&mut self, next: u32) {
        for index in next..self.next_constraint {
            self.constraints.remove(ConstraintId::at(index as usize));
        }
    }

    /// Remove selections allocated after a snapshot mark.
    fn remove_selections_from(&mut self, next: u32) {
        for index in next..self.next_selection {
            self.selections.remove(SelectionId::at(index as usize));
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
