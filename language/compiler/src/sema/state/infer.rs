use tspp_core::FxIndexMap;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Bound, BoundList, BoundSide, Cause, CauseArena, CauseId, GenericParameterId, Origin,
    OriginArena, OriginId, RelationStack, Variable, VariableKind, VariableState, VariableTable,
};
use crate::{CompilerError, CompilerResult};

/// One module's transient inference: variables, decisions, and pending work.
pub(in crate::sema) struct InferContext {
    // open inference
    /// Inference variables and their bounds.
    pub(in crate::sema) variables: VariableTable,
    /// In-flight relation decisions with their cycle stack.
    pub(in crate::sema) relations: RelationStack,
    /// Variables opened for generic parameters, keyed by application.
    pub(in crate::sema) instantiations:
        FxIndexMap<(OriginId, GenericParameterId), dir::TypeVariableId>,
    /// Whether typing positions are sealed, set once solutions are final.
    pub(in crate::sema) sealed: bool,

    // solving rounds
    /// The open inference scope depth.
    pub(in crate::sema) scope_depth: usize,
    /// Open variables standing for uninferred symbol types.
    pub(in crate::sema) symbol_variables: FxIndexMap<dir::GlobalSymbolId, dir::TypeVariableId>,
    /// The completed form of each application given fewer arguments than parameters.
    pub(in crate::sema) filled_applications:
        FxIndexMap<(ModuleId, dir::GenericApplication), dir::GlobalTypeId>,

    // regions
    /// Interned check origins.
    pub(in crate::sema) origins: OriginArena,
    /// Interned constraint causes.
    pub(in crate::sema) causes: CauseArena,

    // decisions
    /// Open decision snapshots, outermost first.
    pub(in crate::sema) snapshots: Vec<Snapshot>,
    /// The writes the open decisions made to state older than themselves, in order.
    pub(in crate::sema) trail: Vec<Undo>,
}

/// One open decision's marks, restoring the state older than the decision on rollback.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct Snapshot {
    /// The trail length when the decision opened.
    pub(in crate::sema) trail: usize,
    /// The variable count when the decision opened.
    pub(in crate::sema) variables: usize,
    /// The bound count when the decision opened.
    pub(in crate::sema) bounds: usize,
    /// The check count when the decision opened.
    pub(in crate::sema) checks: usize,
}

/// One write a decision made to state older than itself.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) enum Undo {
    /// One variable's entry before the write.
    Variable(dir::TypeVariableId, Variable),
    /// One node type the decision committed.
    NodeType(dir::GlobalNodeIdAny),
}

impl InferContext {
    /// Create an empty inference context.
    pub(in crate::sema) fn new() -> Self {
        Self {
            variables: VariableTable::new(),
            origins: OriginArena::default(),
            causes: CauseArena::default(),
            relations: RelationStack::new(),
            instantiations: FxIndexMap::default(),
            sealed: false,
            scope_depth: 0,
            symbol_variables: FxIndexMap::default(),
            filled_applications: FxIndexMap::default(),
            snapshots: Vec::new(),
            trail: Vec::new(),
        }
    }

    /// Return whether a decision is open.
    pub(in crate::sema) fn is_deciding(&self) -> bool {
        !self.snapshots.is_empty()
    }

    /// Open one decision snapshot.
    pub(in crate::sema) fn snapshot(&mut self, checks: usize) -> Snapshot {
        let snapshot = Snapshot {
            trail: self.trail.len(),
            variables: self.variables.count(),
            bounds: self.variables.bound_count(),
            checks,
        };
        self.snapshots.push(snapshot);

        snapshot
    }

    /// Close the innermost decision, returning the node types it committed.
    pub(in crate::sema) fn rollback(&mut self) -> CompilerResult<Vec<dir::GlobalNodeIdAny>> {
        let Some(snapshot) = self.snapshots.pop() else {
            return Err(CompilerError::Internal {
                message: "rollback without an open decision".to_string(),
            });
        };

        // replay the writes in reverse
        let mut nodes = Vec::new();
        let mut restored = Vec::new();
        while self.trail.len() > snapshot.trail {
            let Some(undo) = self.trail.pop() else {
                break;
            };
            match undo {
                Undo::Variable(id, previous) => {
                    *self.variables.get_mut(id)? = previous;
                    restored.push(id);
                }
                Undo::NodeType(node) => nodes.push(node),
            }
        }

        // drop the bounds the decision appended, unlinking them from the restored tails
        self.variables.truncate_bounds(snapshot.bounds);
        for id in restored {
            self.variables.seal_tails(id)?;
        }

        // drop the bounds the scratch variables collected, which the truncation dropped
        for index in snapshot.variables..self.variables.count() {
            let scratch = self.variables.get_mut(dir::TypeVariableId(index as u32))?;
            scratch.lower = BoundList::new();
            scratch.upper = BoundList::new();
        }

        Ok(nodes)
    }

    /// Log one write to a variable older than the open decision.
    fn log_variable(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        if let Some(snapshot) = self.snapshots.last()
            && (id.0 as usize) < snapshot.variables
        {
            let previous = *self.variables.get(id)?;
            self.trail.push(Undo::Variable(id, previous));
        }

        Ok(())
    }

    /// Log one node type the open decision commits.
    pub(in crate::sema) fn log_node_type(&mut self, node: dir::GlobalNodeIdAny) {
        if self.is_deciding() {
            self.trail.push(Undo::NodeType(node));
        }
    }

    /// Return the number of checks the innermost open decision leaves alone.
    pub(in crate::sema) fn decided_checks(&self) -> usize {
        self.snapshots.last().map_or(0, |last| last.checks)
    }

    /// Open one inference variable.
    pub(in crate::sema) fn open_variable(
        &mut self,
        origin: Origin,
        kind: VariableKind,
    ) -> dir::TypeVariableId {
        let variable = dir::TypeVariableId(self.variables.count() as u32);
        let origin = self.origins.intern(origin);
        self.variables.allocate(variable, origin, kind);

        variable
    }

    /// Intern one check origin.
    pub(in crate::sema) fn intern_origin(&mut self, origin: Origin) -> OriginId {
        self.origins.intern(origin)
    }

    /// Intern one constraint cause.
    pub(in crate::sema) fn intern_cause(&mut self, cause: Cause) -> CauseId {
        self.causes.intern(cause)
    }

    /// Return one interned constraint cause.
    pub(in crate::sema) fn cause(&self, id: CauseId) -> Cause {
        self.causes.get(id)
    }

    /// Return one interned check origin.
    pub(in crate::sema) fn origin(&self, id: OriginId) -> Origin {
        self.origins.get(id)
    }

    /// Append one bound to one variable side, returning whether it is new.
    pub(in crate::sema) fn push_bound(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
        bound: Bound,
    ) -> CompilerResult<bool> {
        let id = self.alias_root(id)?;
        self.log_variable(id)?;

        self.variables.push_bound(id, side, bound)
    }

    /// Set the declared default completing one variable.
    pub(in crate::sema) fn set_variable_default(
        &mut self,
        id: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.variable_mut(id)?.default = Some(default);

        Ok(())
    }

    /// Return one variable.
    pub(in crate::sema) fn variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&Variable> {
        self.variables.get(variable)
    }

    /// Return one variable mutably, logging the write for the open decision.
    pub(in crate::sema) fn variable_mut(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&mut Variable> {
        self.log_variable(variable)?;

        self.variables.get_mut(variable)
    }

    /// Return the root that one variable forwards to.
    pub(in crate::sema) fn alias_root(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<dir::TypeVariableId> {
        let mut current = variable;
        while let VariableState::Alias(next) = self.variable(current)?.state {
            current = next;
        }

        Ok(current)
    }

    /// Return one variable's solution, or none while it stays open.
    pub(in crate::sema) fn solution(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(self.variable(self.alias_root(variable)?)?.state.ty())
    }

    /// Return all variable entries.
    pub(in crate::sema) fn variables(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> {
        self.variables.iter()
    }

    /// Return the number of allocated variables.
    pub(in crate::sema) fn variable_count(&self) -> usize {
        self.variables.count()
    }

    /// Seal typing positions once solutions are final, so later projections instantiate fresh.
    pub(in crate::sema) fn seal(&mut self) {
        self.sealed = true;
    }

    /// Return the variable already opened for one parameter at one typing position.
    pub(in crate::sema) fn instantiation(
        &self,
        origin: OriginId,
        parameter: GenericParameterId,
    ) -> Option<dir::TypeVariableId> {
        // sealed positions instantiate fresh
        if self.sealed {
            return None;
        }

        self.instantiations.get(&(origin, parameter)).copied()
    }

    /// Store the variable opened for one parameter at one typing position.
    pub(in crate::sema) fn insert_instantiation(
        &mut self,
        origin: OriginId,
        parameter: GenericParameterId,
        variable: dir::TypeVariableId,
    ) {
        // sealed positions stay unclaimed, decisions own their instantiations
        if self.sealed || self.is_deciding() {
            return;
        }

        self.instantiations.insert((origin, parameter), variable);
    }
}
