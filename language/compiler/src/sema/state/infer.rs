use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::sema::{
    BoundSide, Cause, CauseArena, CauseId, CheckId, CheckOutcome, CheckTable, Fulfillment,
    GenericParameterId, InferenceScope, Origin, OriginArena, OriginId, RelationStack, TypeBound,
    Variable, VariableRole, VariableState, VariableTable, Widening,
};
use crate::{CompilerError, CompilerResult};

/// One module's transient inference: variables, their trail, and pending work.
pub(in crate::sema) struct InferContext {
    // open inference
    /// Inference variables and their bounds.
    pub(in crate::sema) variables: VariableTable,
    /// In-flight relation decisions with their cycle stack.
    pub(in crate::sema) relations: RelationStack,
    /// Variables opened for generic parameters, keyed by application.
    pub(in crate::sema) instantiations:
        FxIndexMap<(OriginId, GenericParameterId), dir::TypeVariableId>,

    // solving rounds
    /// The open inference scope depth.
    pub(in crate::sema) scope_depth: usize,
    /// Open variables standing for uninferred symbol types.
    pub(in crate::sema) symbol_variables: FxIndexMap<dir::GlobalSymbolId, dir::TypeVariableId>,

    // provenance
    /// Interned check origins.
    pub(in crate::sema) origins: OriginArena,
    /// Interned constraint causes.
    pub(in crate::sema) causes: CauseArena,

    // speculation
    /// Inference mutations recorded while speculation is active.
    pub(in crate::sema) trail: Vec<InferUndo>,
    /// The number of nested trail marks.
    pub(in crate::sema) marks: usize,
}

/// One inference trail entry.
#[derive(Debug, Clone)]
pub(in crate::sema) enum InferUndo {
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
    /// Undo one check entry mutation.
    Check {
        /// The changed check.
        id: CheckId,
        /// The previous completed outcome.
        previous: Option<CheckOutcome>,
    },
    /// Undo one recorded instantiation.
    Instantiation {
        /// The typing position that opened the parameter.
        key: (OriginId, GenericParameterId),
    },
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
            scope_depth: 0,
            symbol_variables: FxIndexMap::default(),
            trail: Vec::new(),
            marks: 0,
        }
    }
}

/// Mark of the inference trail before one speculative attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct TrailMark {
    /// The variable count at the mark.
    variables: usize,
    /// The check count at the mark.
    checks: usize,
    /// The trail length at the mark.
    trail: usize,
}

impl TrailMark {
    /// Return the inference scope opened by this mark.
    pub(in crate::sema) fn inference_scope(&self) -> InferenceScope {
        InferenceScope::open(self.variables)
    }

    /// Return the check count at this mark.
    pub(in crate::sema) fn check_count(&self) -> usize {
        self.checks
    }
}

impl InferContext {
    /// Mark the trail before one speculative attempt.
    pub(in crate::sema) fn mark(&mut self, fulfill: &Fulfillment) -> TrailMark {
        self.marks += 1;

        TrailMark {
            variables: self.variables.count(),
            checks: fulfill.checks.count(),
            trail: self.trail.len(),
        }
    }

    /// Roll inference state back to one trail mark.
    ///
    /// Allocation is permanent, binding is speculative: interned entries may still reference a
    /// rolled-back variable, so its slot stays allocated and poisons to the error type.
    pub(in crate::sema) fn rollback(
        &mut self,
        mark: TrailMark,
        poison: dir::GlobalTypeId,
        fulfill: &mut Fulfillment,
    ) -> CompilerResult<()> {
        // undo every mutation recorded past the mark
        while self.trail.len() > mark.trail {
            let undo = self.trail.pop().ok_or_else(|| CompilerError::Internal {
                message: "solver trail ended before its mark".into(),
            })?;

            self.rollback_undo(undo, poison, fulfill)?;
        }

        // drop the speculative checks with their scheduling, then close the mark
        fulfill.truncate(mark.checks);
        self.marks -= 1;

        Ok(())
    }

    /// Close one trail mark, keeping the mutations it recorded.
    pub(in crate::sema) fn commit(&mut self, _mark: TrailMark) {
        self.marks -= 1;
        if self.marks == 0 {
            self.trail.clear();
        }
    }

    /// Allocate one variable.
    pub(in crate::sema) fn allocate_variable(
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
    pub(in crate::sema) fn variable_role(
        &self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<VariableRole> {
        self.variables.role(id)
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
    pub(in crate::sema) fn set_variable_default(
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
    pub(in crate::sema) fn variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&Variable> {
        self.variables.get(variable)
    }

    /// Return one variable mutably.
    pub(in crate::sema) fn variable_mut(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<&mut Variable> {
        self.record_variable(variable)?;

        self.variables.get_mut(variable)
    }

    /// Set one check's outcome, recording its prior state on the trail.
    pub(in crate::sema) fn set_check_result(
        &mut self,
        checks: &mut CheckTable,
        id: CheckId,
        outcome: Option<CheckOutcome>,
    ) -> CompilerResult<()> {
        self.record_check(checks, id)?;
        checks.set_result(id, outcome)?;

        Ok(())
    }

    /// Return the root that one variable forwards to.
    pub(in crate::sema) fn alias_root(
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
    pub(in crate::sema) fn solution(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let root = self.alias_root(variable)?;

        Ok(self.variable(root)?.state.ty())
    }

    /// Return all variable entries.
    pub(in crate::sema) fn variables(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> {
        self.variables.iter()
    }

    /// Return whether trail entries past one point solved a variable
    /// allocated before it.
    pub(in crate::sema) fn solves_variable_before(
        &self,
        trail_from: usize,
        variables: usize,
    ) -> bool {
        self.trail[trail_from..].iter().any(|undo| match undo {
            InferUndo::Variable {
                id,
                previous: Some((variable, _)),
            } => (id.0 as usize) < variables && variable.state.is_open(),
            _ => false,
        })
    }

    /// Return the number of allocated variables.
    pub(in crate::sema) fn variable_count(&self) -> usize {
        self.variables.count()
    }

    /// Record one trail entry if speculation is active.
    fn record_undo(&mut self, undo: InferUndo) {
        if self.marks > 0 {
            self.trail.push(undo);
        }
    }

    /// Return the variable already opened for one parameter at one typing position.
    pub(in crate::sema) fn instantiation(
        &self,
        origin: OriginId,
        parameter: GenericParameterId,
    ) -> Option<dir::TypeVariableId> {
        self.instantiations.get(&(origin, parameter)).copied()
    }

    /// Record the variable opened for one parameter at one typing position.
    pub(in crate::sema) fn record_instantiation(
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

    /// Record one check entry if speculation is active.
    fn record_check(&mut self, checks: &CheckTable, id: CheckId) -> CompilerResult<()> {
        if self.marks > 0 {
            self.trail.push(InferUndo::Check {
                id,
                previous: checks.result(id)?.cloned(),
            });
        }

        Ok(())
    }

    /// Undo one recorded trail entry.
    fn rollback_undo(
        &mut self,
        undo: InferUndo,
        poison: dir::GlobalTypeId,
        fulfill: &mut Fulfillment,
    ) -> CompilerResult<()> {
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
            InferUndo::Check { id, previous } => fulfill.checks.set_result(id, previous)?,
            InferUndo::Instantiation { key } => {
                self.instantiations.swap_remove(&key);
            }
        }

        Ok(())
    }
}
