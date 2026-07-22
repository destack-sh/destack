use std::mem::size_of;
use std::ops::Range;

use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::check::{
    BoundEntry, BoundIter, BoundList, BoundSide, CauseId, EMPTY, OriginId, TypeBound,
};
use crate::{CompilerError, CompilerResult};

/// Special behavior attached to one inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableRole {
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
}

impl VariableRole {
    /// Return whether this role solves as ordinary inference.
    pub(in crate::check) fn is_inference(self) -> bool {
        matches!(
            self,
            Self::Regular | Self::Parameter | Self::Return | Self::Instantiation { .. }
        )
    }

    /// Return the declared parameter this variable instantiates.
    pub(in crate::check) fn parameter(self) -> Option<dir::GlobalGenericParameterId> {
        match self {
            Self::Instantiation { parameter } => Some(parameter),
            Self::Regular | Self::Parameter | Self::Return | Self::Memory { .. } => None,
        }
    }
}

/// Literal widening policy for one solved variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Widening {
    /// Keep literal solutions exact.
    Never,
    /// Widen every fresh literal candidate to its base type.
    Always,
    /// Widen written literal candidates, keeping read candidates exact.
    WhenWritten,
}

/// Variables opened by one task, owned as a dense arena interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InferenceScope {
    /// The first owned variable.
    first: u32,
}

impl InferenceScope {
    /// The scope owning every component variable.
    pub(in crate::check) const ROOT: Self = Self { first: 0 };

    /// Open a scope owning every variable allocated from one count on.
    pub(in crate::check) fn open(count: usize) -> Self {
        Self {
            first: count as u32,
        }
    }

    /// Return whether this scope owns one variable.
    pub(in crate::check) fn owns(self, variable: dir::TypeVariableId) -> bool {
        variable.0 >= self.first
    }

    /// Return the owned variable indices below one arena length.
    pub(in crate::check) fn indices(self, count: usize) -> Range<usize> {
        self.first as usize..count
    }
}

/// One inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Variable {
    /// The interned origin that produced the variable.
    pub(in crate::check) origin: OriginId,
    /// Bounds that must relate to the variable.
    pub(in crate::check) lower: BoundList,
    /// Bounds the variable must relate to.
    pub(in crate::check) upper: BoundList,
    /// The solved type, when solving finished.
    pub(in crate::check) solution: Option<dir::GlobalTypeId>,
    /// The literal widening policy applied when solving.
    pub(in crate::check) widening: Widening,
}

/// Variables and their shared bound storage, owned by one solver.
#[derive(Debug, Default)]
pub(in crate::check) struct VariableTable {
    /// Variables indexed by variable id.
    variables: Vec<Variable>,
    /// Rarely read variable roles, parallel to `variables`.
    roles: Vec<VariableRole>,
    /// The shared bounds linked per variable side.
    bounds: Vec<BoundEntry>,
    /// Declared defaults completing dry variables, present on few variables.
    defaults: FxIndexMap<dir::TypeVariableId, dir::GlobalTypeId>,
    /// Declared parameter bounds discharged on solutions, present on few variables.
    parameter_bounds: FxIndexMap<dir::TypeVariableId, (dir::GlobalTypeId, CauseId)>,
}

impl VariableTable {
    /// Create an empty variable table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Allocate one variable.
    pub(in crate::check) fn allocate(
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
            solution: None,
            widening,
        });
        self.roles.push(role);
    }

    /// Return one variable's role.
    pub(in crate::check) fn role(&self, id: dir::TypeVariableId) -> CompilerResult<VariableRole> {
        self.roles
            .get(id.0 as usize)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Replace one variable's role.
    pub(in crate::check) fn set_role(
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
    pub(in crate::check) fn get(&self, id: dir::TypeVariableId) -> CompilerResult<&Variable> {
        self.variables
            .get(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return one variable mutably.
    pub(in crate::check) fn get_mut(
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
    pub(in crate::check) fn push_bound(
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
        let row = self.bounds.len() as u32;
        self.bounds.push(BoundEntry { bound, next: EMPTY });
        let list = *self.side(id, side)?;
        if list.tail != EMPTY {
            self.bounds[list.tail as usize].next = row;
        }
        *self.side_mut(id, side)? = BoundList {
            head: if list.head == EMPTY { row } else { list.head },
            tail: row,
            count: list.count + 1,
        };

        Ok(true)
    }

    /// Drop the newest bound, which must belong to the given side.
    pub(in crate::check) fn pop_bound(
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
    pub(in crate::check) fn side_bounds(
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
    pub(in crate::check) fn set_default(
        &mut self,
        id: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.defaults.insert(id, default);
    }

    /// Return the declared default of one variable.
    pub(in crate::check) fn variable_default(
        &self,
        id: dir::TypeVariableId,
    ) -> Option<dir::GlobalTypeId> {
        self.defaults.get(&id).copied()
    }

    /// Remove the declared default of one variable, for probe rollback.
    pub(in crate::check) fn remove_default(&mut self, id: dir::TypeVariableId) {
        self.defaults.swap_remove(&id);
    }

    /// Record the declared parameter bound one variable discharges when solved.
    pub(in crate::check) fn set_parameter_bound(
        &mut self,
        id: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
        cause: CauseId,
    ) {
        self.parameter_bounds.insert(id, (bound, cause));
    }

    /// Return the declared parameter bound of one variable.
    pub(in crate::check) fn parameter_bound(
        &self,
        id: dir::TypeVariableId,
    ) -> Option<(dir::GlobalTypeId, CauseId)> {
        self.parameter_bounds.get(&id).copied()
    }

    /// Remove the declared parameter bound of one variable, for probe rollback.
    pub(in crate::check) fn remove_parameter_bound(&mut self, id: dir::TypeVariableId) {
        self.parameter_bounds.swap_remove(&id);
    }

    /// Return the total number of allocated variables.
    pub(in crate::check) fn count(&self) -> usize {
        self.variables.len()
    }

    /// Return the total number of collected bounds.
    pub(in crate::check) fn bound_count(&self) -> usize {
        self.bounds.len()
    }

    /// Truncate variables undone by one probe rollback.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.variables.truncate(count);
        self.roles.truncate(count);
    }

    /// Iterate all variables with their ids.
    pub(in crate::check) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> + '_ {
        self.variables
            .iter()
            .enumerate()
            .map(|(index, variable)| (dir::TypeVariableId(index as u32), variable))
    }
}

// lock the hot solver row shape
#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<Variable>() == 64);
