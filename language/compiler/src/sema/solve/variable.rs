use std::mem::size_of;
use std::ops::Range;

use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::sema::{BoundEntry, BoundIter, BoundList, BoundSide, EMPTY, OriginId, TypeBound};
use crate::{CompilerError, CompilerResult};

/// Special behavior attached to one inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum VariableRole {
    /// Ordinary inference variable.
    Regular,
    /// Contextually typed parameter slot, received by assignment.
    Parameter,
    /// Variable inferred from a callable body return.
    Return,
    /// Variable instantiating one declared generic parameter.
    Instantiation {
        /// The instantiated parameter.
        parameter: dir::GlobalGenericParameterId,
    },
    /// Variable that ranges over one well-known memory parameter kind.
    Memory {
        /// The memory parameter kind.
        kind: dir::MemoryParameter,
        /// The kind's constraint.
        constraint: Option<dir::GlobalTypeId>,
    },
    /// Cell standing for one local symbol's type until it commits.
    Symbol {
        /// The awaited symbol.
        symbol: dir::GlobalSymbolId,
    },
}

impl VariableRole {
    /// Return whether this variable participates in inference.
    pub(in crate::sema) fn is_inference(self) -> bool {
        matches!(
            self,
            Self::Regular | Self::Parameter | Self::Return | Self::Instantiation { .. }
        )
    }

    /// Return the local symbol this variable stands for.
    pub(in crate::sema) fn symbol(self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Symbol { symbol } => Some(symbol),
            _ => None,
        }
    }

    /// Return the declared parameter this variable instantiates.
    pub(in crate::sema) fn parameter(self) -> Option<dir::GlobalGenericParameterId> {
        match self {
            Self::Instantiation { parameter } => Some(parameter),
            Self::Regular
            | Self::Parameter
            | Self::Return
            | Self::Memory { .. }
            | Self::Symbol { .. } => None,
        }
    }
}

/// Literal widening policy for one solved variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Widening {
    /// Keep literal solutions exact.
    Never,
    /// Keep the solution exact while widening mutable aggregate descendants.
    Aggregate,
    /// Widen when several exact literal candidates compete.
    Multiple,
    /// Widen every exact literal candidate to its base type.
    Always,
    /// Widen literal candidates into their const family, yielding to a typed candidate.
    Const,
}

/// Variables opened by one task, owned as a dense arena interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct InferenceScope {
    /// The first owned variable.
    first_variable: u32,
}

impl InferenceScope {
    /// Open a scope at one variable count.
    pub(in crate::sema) fn open(variable_count: usize) -> Self {
        Self {
            first_variable: variable_count as u32,
        }
    }

    /// Return the owned variable indices below one arena length.
    pub(in crate::sema) fn indices(self, count: usize) -> Range<usize> {
        self.first_variable as usize..count
    }

    /// Return the first owned variable index.
    pub(in crate::sema) fn first_variable(self) -> usize {
        self.first_variable as usize
    }
}

/// One inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct Variable {
    /// The interned origin that produced the variable.
    pub(in crate::sema) origin: OriginId,
    /// Bounds that must relate to the variable.
    pub(in crate::sema) lower: BoundList,
    /// Bounds the variable must relate to.
    pub(in crate::sema) upper: BoundList,
    /// The variable's inference state.
    pub(in crate::sema) state: VariableState,
    /// The literal widening policy applied when solving.
    pub(in crate::sema) widening: Widening,
}

/// Inference state of one variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum VariableState {
    /// The variable is awaiting bounds.
    Open,
    /// The variable forwards to an equal earlier variable.
    Alias(dir::TypeVariableId),
    /// The variable inferred one type.
    Resolved(dir::GlobalTypeId),
    /// Inference failed and produced the compiler error type.
    Error(dir::GlobalTypeId),
}

impl VariableState {
    /// Return the completed type, when inference finished.
    pub(in crate::sema) fn ty(self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Open | Self::Alias(_) => None,
            Self::Resolved(ty) | Self::Error(ty) => Some(ty),
        }
    }

    /// Return whether the variable still awaits bounds.
    pub(in crate::sema) fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }
}

/// Variables and their shared bound storage, owned by one solver.
#[derive(Debug, Default)]
pub(in crate::sema) struct VariableTable {
    /// Variables indexed by variable id.
    variables: Vec<Variable>,
    /// Rarely read variable roles, parallel to `variables`.
    roles: Vec<VariableRole>,
    /// The shared bounds linked per variable side.
    bounds: Vec<BoundEntry>,
    /// Declared defaults completing dry variables, present on few variables.
    defaults: FxIndexMap<dir::TypeVariableId, dir::GlobalTypeId>,
}

impl VariableTable {
    /// Create an empty variable table.
    pub(in crate::sema) fn new() -> Self {
        Self::default()
    }

    /// Allocate one variable.
    pub(in crate::sema) fn allocate(
        &mut self,
        variable: dir::TypeVariableId,
        origin: OriginId,
        widening: Widening,
        role: VariableRole,
    ) {
        debug_assert_eq!(self.variables.len() as u32, variable.0);
        self.variables.push(Variable {
            origin,
            lower: BoundList::new(),
            upper: BoundList::new(),
            state: VariableState::Open,
            widening,
        });
        self.roles.push(role);
    }

    /// Return one variable's role.
    pub(in crate::sema) fn role(&self, id: dir::TypeVariableId) -> CompilerResult<VariableRole> {
        self.roles
            .get(id.0 as usize)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Replace one variable's role.
    pub(in crate::sema) fn set_role(
        &mut self,
        id: dir::TypeVariableId,
        role: VariableRole,
    ) -> CompilerResult<()> {
        let slot = self
            .roles
            .get_mut(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })?;
        *slot = role;

        Ok(())
    }

    /// Return one variable.
    pub(in crate::sema) fn get(&self, id: dir::TypeVariableId) -> CompilerResult<&Variable> {
        self.variables
            .get(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return one variable mutably.
    pub(in crate::sema) fn get_mut(
        &mut self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<&mut Variable> {
        self.variables
            .get_mut(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Append one bound to one variable side and return whether it is new.
    pub(in crate::sema) fn push_bound(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
        bound: TypeBound,
    ) -> CompilerResult<bool> {
        // reject bounds already collected on this side; the first
        //  cause wins, since provenance never changes the solution
        if self
            .side_bounds(id, side)?
            .any(|existing| existing.ty == bound.ty && existing.relation == bound.relation)
        {
            return Ok(false);
        }

        // append the bound and link it as the new tail
        let entry = self.bounds.len() as u32;
        self.bounds.push(BoundEntry { bound, next: EMPTY });
        let list = *self.side(id, side)?;
        if list.tail != EMPTY {
            self.bounds[list.tail as usize].next = entry;
        }
        *self.side_mut(id, side)? = BoundList {
            head: if list.head == EMPTY { entry } else { list.head },
            tail: entry,
            count: list.count + 1,
        };

        Ok(true)
    }

    /// Drop the newest bound, which must belong to the given side.
    pub(in crate::sema) fn pop_bound(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
    ) -> CompilerResult<()> {
        // the popped bound is the newest append for this side
        let list = *self.side(id, side)?;
        debug_assert_eq!(list.tail + 1, self.bounds.len() as u32);
        self.bounds.pop();

        // relink the previous bound as the new tail
        let mut previous = EMPTY;
        if list.head != list.tail {
            previous = list.head;
            while self.bounds[previous as usize].next != list.tail {
                previous = self.bounds[previous as usize].next;
            }
            self.bounds[previous as usize].next = EMPTY;
        }

        *self.side_mut(id, side)? = BoundList {
            head: if list.head == list.tail {
                EMPTY
            } else {
                list.head
            },
            tail: previous,
            count: list.count - 1,
        };

        Ok(())
    }

    /// Iterate one variable side in insertion order.
    pub(in crate::sema) fn side_bounds(
        &self,
        id: dir::TypeVariableId,
        side: BoundSide,
    ) -> CompilerResult<BoundIter<'_>> {
        let list = self.side(id, side)?;

        Ok(BoundIter {
            bounds: &self.bounds,
            current: list.head,
        })
    }

    /// Return one side list.
    fn side(&self, id: dir::TypeVariableId, side: BoundSide) -> CompilerResult<&BoundList> {
        let variable = self.get(id)?;

        Ok(match side {
            BoundSide::Lower => &variable.lower,
            BoundSide::Upper => &variable.upper,
        })
    }

    /// Return one mutable side list.
    fn side_mut(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
    ) -> CompilerResult<&mut BoundList> {
        let variable = self.get_mut(id)?;

        Ok(match side {
            BoundSide::Lower => &mut variable.lower,
            BoundSide::Upper => &mut variable.upper,
        })
    }

    /// Record the declared default completing one variable when inference stays dry.
    pub(in crate::sema) fn set_default(
        &mut self,
        id: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.defaults.insert(id, default);
    }

    /// Return the declared default of one variable.
    pub(in crate::sema) fn variable_default(
        &self,
        id: dir::TypeVariableId,
    ) -> Option<dir::GlobalTypeId> {
        self.defaults.get(&id).copied()
    }

    /// Remove the declared default of one variable, for probe rollback.
    pub(in crate::sema) fn remove_default(&mut self, id: dir::TypeVariableId) {
        self.defaults.swap_remove(&id);
    }

    /// Return the total number of allocated variables.
    pub(in crate::sema) fn count(&self) -> usize {
        self.variables.len()
    }

    /// Return the total number of collected bounds.
    pub(in crate::sema) fn bound_count(&self) -> usize {
        self.bounds.len()
    }

    /// Iterate all variables with their ids.
    pub(in crate::sema) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> + '_ {
        self.variables
            .iter()
            .enumerate()
            .map(|(index, variable)| (dir::TypeVariableId(index as u32), variable))
    }
}

// lock the hot solver entry shape
#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<Variable>() <= 96);
