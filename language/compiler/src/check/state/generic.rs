use destack_dir as dir;

use crate::check::{ArgumentTerm, CheckState, ConstraintOrigin, TermId};

use super::{VariableId, VariableOutput};

/// Stable key for one inferred generic argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericArgumentKey {
    /// The syntax node that owns the generic use.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic declaration being instantiated.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The generic slot being instantiated.
    pub(in crate::check) index: dir::GenericSlotIndex,
}

/// Stable id for one declaration-side generic slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericSlotId {
    /// The generic owner symbol.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The referenced slot key.
    pub(in crate::check) key: dir::GenericSlotKey,
    /// The declaration order index.
    pub(in crate::check) index: dir::GenericSlotIndex,
}

/// One solved generic instance.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInstance {
    /// The instantiated symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The solved arguments.
    pub(in crate::check) arguments: Vec<TermId<ArgumentTerm>>,
}

/// Generic slot identity shared by explicit and induced generic parameters.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSlot {
    /// The generic owner symbol.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The slot key.
    pub(in crate::check) key: dir::GenericSlotKey,
    /// The declaration order index.
    pub(in crate::check) index: dir::GenericSlotIndex,
    /// The slot origin.
    pub(in crate::check) origin: dir::GenericSlotOrigin,
}

impl GenericSlot {
    /// Return a stable slot id.
    pub(in crate::check) fn id(&self) -> GenericSlotId {
        GenericSlotId {
            owner: self.owner,
            key: self.key,
            index: self.index,
        }
    }
}

impl From<GenericSlotId> for dir::ParameterType {
    fn from(reference: GenericSlotId) -> Self {
        Self {
            owner: reference.owner,
            key: reference.key,
            index: reference.index,
        }
    }
}

impl From<dir::ParameterType> for GenericSlotId {
    fn from(parameter: dir::ParameterType) -> Self {
        Self {
            owner: parameter.owner,
            key: parameter.key,
            index: parameter.index,
        }
    }
}

/// Generic parameter metadata attached to one check variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum GenericParameter {
    /// Type generic slot.
    Type {
        /// The slot identity.
        slot: GenericSlot,
        /// The slot variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<VariableId>,
        /// The optional type default.
        default: Option<VariableId>,
    },
    /// Variadic type generic slot.
    VariadicType {
        /// The slot identity.
        slot: GenericSlot,
        /// The slot variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<VariableId>,
        /// The optional type default.
        default: Option<VariableId>,
    },
    /// Static generic slot.
    Static {
        /// The slot identity.
        slot: GenericSlot,
        /// The optional static value type constraint.
        constraint: Option<VariableId>,
        /// The optional static default.
        default: Option<VariableId>,
    },
    /// Variadic static generic slot.
    VariadicStatic {
        /// The slot identity.
        slot: GenericSlot,
        /// The optional static value type constraint.
        constraint: Option<VariableId>,
        /// The optional static default.
        default: Option<VariableId>,
    },
}

impl GenericParameter {
    /// Return this generic parameter's slot identity.
    pub(in crate::check) fn slot(&self) -> &GenericSlot {
        match self {
            Self::Type { slot, .. }
            | Self::VariadicType { slot, .. }
            | Self::Static { slot, .. }
            | Self::VariadicStatic { slot, .. } => slot,
        }
    }

    /// Return whether this is a type-level generic slot.
    pub(in crate::check) fn is_type(&self) -> bool {
        matches!(self, Self::Type { .. } | Self::VariadicType { .. })
    }

    /// Return whether this is a static generic slot.
    pub(in crate::check) fn is_static(&self) -> bool {
        matches!(self, Self::Static { .. } | Self::VariadicStatic { .. })
    }

    /// Return whether this is a variadic generic slot.
    pub(in crate::check) fn is_variadic(&self) -> bool {
        matches!(
            self,
            Self::VariadicType { .. } | Self::VariadicStatic { .. }
        )
    }

    /// Return this generic parameter's type constraint.
    pub(in crate::check) fn type_constraint(&self) -> Option<VariableId> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }
}

impl CheckState<'_> {
    /// Return an existing static generic parameter variable for one symbol.
    pub(in crate::check) fn generic_static_variable_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let variable = self.variables.static_by_symbol.get(&symbol).copied()?;
        let Some(VariableOutput::Generic(generic)) =
            &self.variables.variables[variable.index as usize].output
        else {
            return None;
        };

        generic.is_static().then_some(variable)
    }

    /// Return generic parameters in allocation order.
    pub(in crate::check) fn generic_parameters(
        &self,
    ) -> impl Iterator<Item = (VariableId, GenericParameter)> {
        let parameters = self
            .variables
            .variables
            .iter()
            .filter_map(|variable| {
                if let Some(VariableOutput::Generic(generic)) = &variable.output {
                    Some((variable.id, generic.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        parameters.into_iter()
    }

    /// Return generic parameters owned by one symbol.
    pub(in crate::check) fn generic_parameters_for_owner(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = (VariableId, GenericParameter)> {
        let parameters = self
            .variables
            .generic_parameter_by_owner
            .get(&owner)
            .into_iter()
            .flatten()
            .filter_map(|variable| {
                let Some(VariableOutput::Generic(generic)) =
                    &self.variables.variables[variable.index as usize].output
                else {
                    return None;
                };

                Some((*variable, generic.clone()))
            })
            .collect::<Vec<_>>();

        parameters.into_iter()
    }

    /// Return the generic parameter variable for one slot.
    pub(in crate::check) fn generic_slot_variable(
        &self,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        self.generic_parameters_for_owner(slot.owner)
            .find_map(|(variable, generic)| (generic.slot().id() == slot).then_some(variable))
    }

    /// Return the generic slot declared by one parameter symbol.
    pub(in crate::check) fn generic_slot_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericSlotId> {
        self.generic_parameters().find_map(|(_, generic)| {
            if generic.slot().key == dir::GenericSlotKey::Symbol(symbol) {
                Some(generic.slot().id())
            } else {
                None
            }
        })
    }

    /// Record one variable as a generic slot.
    pub(in crate::check) fn record_generic_parameter(
        &mut self,
        variable: VariableId,
        generic: GenericParameter,
    ) {
        let owner = generic.slot().owner;
        let source = ConstraintOrigin::Symbol(owner);

        let check_variable = &mut self.variables.variables[variable.index as usize];
        check_variable.source = source;
        check_variable.output = Some(VariableOutput::Generic(generic));
        self.variables
            .generic_parameter_by_owner
            .entry(owner)
            .or_default()
            .push(variable);
    }

    /// Allocate one explicit generic slot for one owner.
    pub(in crate::check) fn allocate_explicit_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericSlot {
        GenericSlot {
            owner,
            key: dir::GenericSlotKey::Symbol(symbol),
            index: self.next_generic_slot_index(owner),
            origin: dir::GenericSlotOrigin::Explicit,
        }
    }

    /// Allocate one induced generic slot for one owner.
    pub(in crate::check) fn allocate_induced_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
    ) -> GenericSlot {
        let index = self.next_generic_slot_index(owner);
        let name = self.generated_generic_name(owner, prefix, index);

        GenericSlot {
            owner,
            key: dir::GenericSlotKey::Generated(name),
            index,
            origin: dir::GenericSlotOrigin::Induced,
        }
    }

    /// Return the generated local name for one generic slot.
    fn generated_generic_name(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
        index: dir::GenericSlotIndex,
    ) -> dir::StringId {
        self.input_mut(owner.module_id)
            .strings
            .intern(&format!("{prefix}{}", index.get()))
    }

    /// Allocate the next generic slot index for one owner.
    fn next_generic_slot_index(&mut self, owner: dir::GlobalSymbolId) -> dir::GenericSlotIndex {
        let index = self
            .variables
            .generic_slot_index
            .entry(owner)
            .or_insert(dir::GenericSlotIndex::new(0));
        let next = *index;
        *index = next.next();

        next
    }
}
