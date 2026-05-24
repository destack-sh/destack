use destack_dir as dir;

use crate::check::ArgumentTerm;

use super::{CheckModuleState, VariableId, VariableOrigin};

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

/// One solved generic instance.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInstance {
    /// The instantiated symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The solved arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
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

    /// Return this generic parameter's type constraint.
    pub(in crate::check) fn type_constraint(&self) -> Option<VariableId> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }

    /// Set this generic parameter's type constraint.
    pub(in crate::check) fn set_type_constraint(&mut self, constraint: VariableId) -> bool {
        match self {
            Self::Type {
                constraint: slot, ..
            }
            | Self::VariadicType {
                constraint: slot, ..
            } => {
                *slot = Some(constraint);

                true
            }
            Self::Static { .. } | Self::VariadicStatic { .. } => false,
        }
    }
}

impl CheckModuleState {
    /// Return an existing static generic parameter variable for one symbol.
    pub(in crate::check) fn generic_static_variable(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let variable = self.work.variables.static_by_symbol.get(&symbol).copied()?;
        let VariableOrigin::Generic(generic) = &self.variable(variable).origin else {
            return None;
        };

        generic.is_static().then_some(variable)
    }

    /// Return generic parameters in allocation order.
    pub(in crate::check) fn generic_parameters(
        &self,
    ) -> impl Iterator<Item = (VariableId, &GenericParameter)> + '_ {
        self.work.variables.all.iter().filter_map(|variable| {
            if let VariableOrigin::Generic(generic) = &variable.origin {
                Some((variable.id, generic))
            } else {
                None
            }
        })
    }

    /// Return generic parameters owned by one symbol.
    pub(in crate::check) fn generic_parameters_for_owner(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = (VariableId, &GenericParameter)> + '_ {
        self.work
            .variables
            .generic_parameter_by_owner
            .get(&owner)
            .into_iter()
            .flatten()
            .filter_map(|variable| {
                let VariableOrigin::Generic(generic) = &self.variable(*variable).origin else {
                    return None;
                };

                Some((*variable, generic))
            })
    }

    /// Record one variable as a generic slot.
    pub(in crate::check) fn record_generic_parameter(
        &mut self,
        variable: VariableId,
        generic: GenericParameter,
    ) {
        let owner = generic.slot().owner;

        self.variable_mut(variable).origin = VariableOrigin::Generic(generic);
        self.work
            .variables
            .generic_parameter_by_owner
            .entry(owner)
            .or_default()
            .push(variable);
    }

    /// Allocate one explicit generic slot for one owner.
    pub(in crate::check) fn explicit_generic_slot(
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
    pub(in crate::check) fn induced_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
    ) -> (GenericSlot, dir::StringId) {
        let name = self.next_induced_generic_name(owner, prefix);

        let slot = GenericSlot {
            owner,
            key: dir::GenericSlotKey::Generated(name),
            index: self.next_generic_slot_index(owner),
            origin: dir::GenericSlotOrigin::Induced,
        };

        (slot, name)
    }

    /// Allocate the next induced generic name for one owner.
    fn next_induced_generic_name(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
    ) -> dir::StringId {
        let index = self
            .work
            .variables
            .induced_generic_index
            .entry(owner)
            .or_insert(0);
        let next = *index;
        *index += 1;

        self.input
            .strings
            .intern(&format!("{}.{prefix}{next}", self.symbol_label(owner)))
    }

    /// Allocate the next generic slot index for one owner.
    fn next_generic_slot_index(&mut self, owner: dir::GlobalSymbolId) -> dir::GenericSlotIndex {
        let index = self
            .work
            .variables
            .generic_slot_index
            .entry(owner)
            .or_insert(dir::GenericSlotIndex::new(0));
        let next = *index;
        *index = next.next();

        next
    }
}
