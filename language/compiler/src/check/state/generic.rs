use destack_dir as dir;

use super::{CheckModuleState, VariableId, VariableOrigin};

/// Generic slot identity shared by explicit and induced generic variables.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericVariableSlot {
    /// The generic owner symbol.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The slot key.
    pub(in crate::check) key: dir::GenericSlotKey,
    /// The declaration order index.
    pub(in crate::check) index: dir::GenericSlotIndex,
    /// The slot origin.
    pub(in crate::check) origin: dir::GenericSlotOrigin,
}

/// Generic variable recorded by check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum GenericVariable {
    /// Type generic slot.
    Type {
        /// The slot identity.
        slot: GenericVariableSlot,
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
        slot: GenericVariableSlot,
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
        slot: GenericVariableSlot,
        /// The optional static value type constraint.
        constraint: Option<VariableId>,
        /// The optional static default.
        default: Option<VariableId>,
    },
    /// Variadic static generic slot.
    VariadicStatic {
        /// The slot identity.
        slot: GenericVariableSlot,
        /// The optional static value type constraint.
        constraint: Option<VariableId>,
        /// The optional static default.
        default: Option<VariableId>,
    },
}

impl GenericVariable {
    /// Return this generic variable's slot identity.
    pub(in crate::check) fn slot(&self) -> &GenericVariableSlot {
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

    /// Return this generic variable's type constraint.
    pub(in crate::check) fn type_constraint(&self) -> Option<VariableId> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }

    /// Set this generic variable's type constraint.
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
    /// Return an existing static generic variable for one symbol.
    pub(in crate::check) fn generic_static_variable(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let variable = self.symbol_static_variables.get(&symbol).copied()?;
        let VariableOrigin::Generic(generic) = &self.variable(variable).origin else {
            return None;
        };

        generic.is_static().then_some(variable)
    }

    /// Return generic variables in allocation order.
    pub(in crate::check) fn generic_variables(
        &self,
    ) -> impl Iterator<Item = (VariableId, &GenericVariable)> + '_ {
        self.variables.iter().filter_map(|variable| {
            if let VariableOrigin::Generic(generic) = &variable.origin {
                Some((variable.id, generic))
            } else {
                None
            }
        })
    }

    /// Allocate one explicit generic slot for one owner.
    pub(in crate::check) fn explicit_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericVariableSlot {
        GenericVariableSlot {
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
    ) -> (GenericVariableSlot, dir::StringId) {
        let name = self.next_induced_generic_name(owner, prefix);

        let slot = GenericVariableSlot {
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
        let index = self.induced_generic_indexes.entry(owner).or_insert(0);
        let next = *index;
        *index += 1;

        self.strings
            .intern(&format!("{}.{prefix}{next}", self.symbol_label(owner)))
    }

    /// Allocate the next generic slot index for one owner.
    fn next_generic_slot_index(&mut self, owner: dir::GlobalSymbolId) -> dir::GenericSlotIndex {
        let index = self
            .generic_slot_indexes
            .entry(owner)
            .or_insert(dir::GenericSlotIndex::new(0));
        let next = *index;
        *index = next.next();

        next
    }
}
