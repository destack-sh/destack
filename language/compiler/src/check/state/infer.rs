use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    BoundSide, Cause, CauseArena, CauseId, Constraint, ConstraintId, ConstraintResult,
    ConstraintTable, FailedCheck, GenericParameterId, InferenceScope, ObligationEntry,
    ObligationId, ObligationTable, Origin, OriginArena, OriginId, PendingWork, RelationStack,
    TypeBound, Variable, VariableRole, VariableState, VariableTable, Widening,
};
use crate::{CompilerError, CompilerResult};

/// One module's transient inference: variables, their trail, and pending work.
pub(in crate::check) struct InferContext {
    // open inference
    /// Inference variables and their bounds.
    pub(in crate::check) variables: VariableTable,
    /// Collected constraints and their completed results.
    pub(in crate::check) constraints: ConstraintTable,
    /// Collected obligations awaiting the settle points.
    pub(in crate::check) obligations: ObligationTable,
    /// In-flight relation decisions with their cycle stack.
    pub(in crate::check) relations: RelationStack,
    /// Variables opened for generic parameters, keyed by application.
    pub(in crate::check) instantiations:
        FxIndexMap<(OriginId, GenericParameterId), dir::TypeVariableId>,

    // pending work
    /// Work registered with fulfillment, awaiting inference progress.
    pub(in crate::check) pending: Vec<PendingWork>,

    // solving rounds
    /// Whether the outermost close is judging every remainder.
    pub(in crate::check) forcing: bool,
    /// Whether the current round left ambiguous work behind.
    pub(in crate::check) ambiguity: bool,
    /// The open inference scope depth.
    pub(in crate::check) scope_depth: usize,
    /// Open variables standing for uninferred symbol types.
    pub(in crate::check) symbol_variables: FxIndexMap<dir::GlobalSymbolId, dir::TypeVariableId>,

    // provenance
    /// Interned check origins.
    pub(in crate::check) origins: OriginArena,
    /// Interned constraint causes.
    pub(in crate::check) causes: CauseArena,
    /// Failed checks retained until their cause trees are complete.
    pub(in crate::check) failures: Vec<FailedCheck>,

    // speculation
    /// Inference mutations recorded while speculation is active.
    pub(in crate::check) trail: Vec<InferUndo>,
    /// The number of nested trail marks.
    pub(in crate::check) marks: usize,
}

/// One inference trail entry.
#[derive(Debug, Clone)]
pub(in crate::check) enum InferUndo {
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
        /// The typing position that opened the parameter.
        key: (OriginId, GenericParameterId),
    },
}

impl InferContext {
    /// Create an empty inference context.
    pub(in crate::check) fn new() -> Self {
        Self {
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            obligations: ObligationTable::new(),
            origins: OriginArena::default(),
            causes: CauseArena::default(),
            failures: Vec::new(),
            relations: RelationStack::new(),
            instantiations: FxIndexMap::default(),
            pending: Vec::new(),
            forcing: false,
            ambiguity: false,
            scope_depth: 0,
            symbol_variables: FxIndexMap::default(),
            trail: Vec::new(),
            marks: 0,
        }
    }
}

/// Mark of the inference trail before one speculative attempt.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct TrailMark {
    /// The variable count at the mark.
    variables: usize,
    /// The constraint count at the mark.
    constraints: usize,
    /// The obligation count at the mark.
    obligations: usize,
    /// The trail length at the mark.
    trail: usize,
}

impl TrailMark {
    /// Return the inference scope opened by this mark.
    pub(in crate::check) fn inference_scope(&self) -> InferenceScope {
        InferenceScope::open(self.variables, self.trail)
    }

    /// Return the constraint count at this mark.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.constraints
    }
}

impl InferContext {
    /// Mark the trail before one speculative attempt.
    pub(in crate::check) fn mark(&mut self) -> TrailMark {
        self.marks += 1;

        TrailMark {
            variables: self.variables.count(),
            constraints: self.constraints.count(),
            obligations: self.obligations.count(),
            trail: self.trail.len(),
        }
    }

    /// Roll inference state back to one trail mark.
    ///
    /// Allocation is permanent, binding is speculative: interned rows
    /// may still reference a rolled-back variable, so its slot stays
    /// allocated and poisons to the error type instead of unwinding.
    pub(in crate::check) fn rollback(
        &mut self,
        mark: TrailMark,
        poison: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        while self.trail.len() > mark.trail {
            let undo = self.trail.pop().ok_or_else(|| CompilerError::Internal {
                message: "solver trail ended before its mark".into(),
            })?;

            self.rollback_undo(undo, poison)?;
        }

        // drop the speculative constraints and obligations, then close the mark
        self.constraints.truncate(mark.constraints);
        self.obligations.truncate(mark.obligations);
        self.marks -= 1;

        Ok(())
    }

    /// Close one trail mark, keeping the mutations it recorded.
    pub(in crate::check) fn commit(&mut self, _mark: TrailMark) {
        self.marks -= 1;
        if self.marks == 0 {
            self.trail.clear();
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
        self.record_undo(InferUndo::Variable {
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
            self.trail
                .get(scope.first_mutation()..)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "inference scope mutation mark {} exceeds length {}",
                        scope.first_mutation(),
                        self.trail.len(),
                    ),
                })?;
        let mut adopted = SmallVec::new();
        for mutation in mutations {
            let InferUndo::Bound { id, .. } = mutation else {
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
        let id = self.alias_root(id)?;
        let pushed = self.variables.push_bound(id, side, bound)?;
        if pushed {
            self.record_undo(InferUndo::Bound { id, side });
        }

        Ok(pushed)
    }

    /// Record the declared default completing one variable.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        id: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.record_undo(InferUndo::Default {
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
        // one check collects one constraint, however often walks repeat it
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

    /// Return the root that one variable forwards to.
    pub(in crate::check) fn alias_root(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<dir::TypeVariableId> {
        let mut current = variable;
        loop {
            let VariableState::Alias(next) = self.variable(current)?.state else {
                break;
            };
            current = next;
        }

        Ok(current)
    }

    /// Return one variable's solution, or none while it stays open.
    pub(in crate::check) fn solution(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let root = self.alias_root(variable)?;

        Ok(self.variable(root)?.state.ty())
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

    /// Record one trail entry if speculation is active.
    fn record_undo(&mut self, undo: InferUndo) {
        if self.marks > 0 {
            self.trail.push(undo);
        }
    }

    /// Return the variable already opened for one parameter at one typing position.
    pub(in crate::check) fn instantiation(
        &self,
        origin: OriginId,
        parameter: GenericParameterId,
    ) -> Option<dir::TypeVariableId> {
        self.instantiations.get(&(origin, parameter)).copied()
    }

    /// Record the variable opened for one parameter at one typing position.
    pub(in crate::check) fn record_instantiation(
        &mut self,
        origin: OriginId,
        parameter: GenericParameterId,
        variable: dir::TypeVariableId,
    ) {
        self.record_undo(InferUndo::Instantiation {
            key: (origin, parameter),
        });
        self.instantiations.insert((origin, parameter), variable);
    }

    /// Record one variable's prior state if speculation is active.
    fn record_variable(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        if self.marks > 0 {
            let previous = *self.variables.get(id)?;
            let role = self.variables.role(id)?;
            self.trail.push(InferUndo::Variable {
                id,
                previous: Some((previous, role)),
            });
        }

        Ok(())
    }

    /// Record one constraint entry if speculation is active.
    fn record_constraint(&mut self, id: ConstraintId) -> CompilerResult<()> {
        if self.marks > 0 {
            self.trail.push(InferUndo::Constraint {
                id,
                previous: self.constraints.result(id)?.cloned(),
            });
        }

        Ok(())
    }

    /// Undo one recorded trail entry.
    fn rollback_undo(&mut self, undo: InferUndo, poison: dir::GlobalTypeId) -> CompilerResult<()> {
        match undo {
            InferUndo::Variable { id, previous } => match previous {
                Some((previous, role)) => {
                    *self.variables.get_mut(id)? = previous;
                    self.variables.set_role(id, role)?;
                }
                // poison undone allocations, keeping their slot
                None => self.variables.get_mut(id)?.state = VariableState::Error(poison),
            },
            InferUndo::Bound { id, side } => {
                self.variables.pop_bound(id, side)?;
            }
            InferUndo::Default { id, previous } => match previous {
                Some(previous) => self.variables.set_default(id, previous),
                None => self.variables.remove_default(id),
            },
            InferUndo::Constraint { id, previous } => self.constraints.set_result(id, previous)?,
            InferUndo::Instantiation { key } => {
                self.instantiations.swap_remove(&key);
            }
        }

        Ok(())
    }
}
