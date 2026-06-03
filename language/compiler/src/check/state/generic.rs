use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CheckState, GenericArgument, Origin, StaticOperand, TypeOperand, TypeTerm, VariableKind,
};

use super::VariableId;

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

/// One generic template application.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericApplication {
    /// The applied generic owner.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The generic arguments in declaration order.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

/// Stable key for one source-level generic application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GenericApplicationKey {
    /// The syntax node that applies the generic owner.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic owner being applied.
    pub(in crate::check) owner: dir::GlobalSymbolId,
}

/// One owner-level declaration of generic slots.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericTemplate {
    /// The symbol that owns this generic template.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The generic slots in declaration order.
    pub(in crate::check) slots: Vec<GenericSlotId>,
}

impl GenericTemplate {
    /// Create an empty generic template.
    pub(in crate::check) fn new(owner: dir::GlobalSymbolId) -> Self {
        Self {
            owner,
            slots: Vec::new(),
        }
    }
}

/// One declaration operand that can induce owner generics.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionRoot {
    /// The declaration that receives induced generic parameters.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The declaration operand to traverse.
    pub(in crate::check) operand: TypeOperand,
}

/// One escaping inference variable that may become an induced owner generic.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInduction {
    /// The variable rewritten to the generated parameter.
    pub(in crate::check) variable: VariableId,
    /// The generated slot recipe.
    pub(in crate::check) slot: GenericInductionSlot,
}

/// One generated generic slot recipe.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionSlot {
    /// The induced variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The generated slot name prefix.
    pub(in crate::check) prefix: &'static str,
    /// The optional generated slot constraint.
    pub(in crate::check) constraint: Option<TypeOperand>,
    /// The reason this slot was induced.
    pub(in crate::check) induction: dir::GenericSlotInduction,
}

impl GenericInductionSlot {
    /// Create one induced type slot.
    pub(in crate::check) fn r#type(
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericSlotInduction,
    ) -> Self {
        Self {
            kind: VariableKind::Type,
            prefix,
            constraint,
            induction,
        }
    }

    /// Create one induced static slot.
    pub(in crate::check) fn r#static(
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericSlotInduction,
    ) -> Self {
        Self {
            kind: VariableKind::Static,
            prefix,
            constraint,
            induction,
        }
    }
}

/// Generic slot identity shared by explicit and induced generic slots.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSlotHeader {
    /// The generic owner symbol.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The slot key.
    pub(in crate::check) key: dir::GenericSlotKey,
    /// The declaration order index.
    pub(in crate::check) index: dir::GenericSlotIndex,
    /// The slot origin.
    pub(in crate::check) origin: dir::GenericSlotOrigin,
}

impl GenericSlotHeader {
    /// Return a stable slot id.
    pub(in crate::check) fn id(&self) -> GenericSlotId {
        GenericSlotId {
            owner: self.owner,
            key: self.key,
            index: self.index,
        }
    }
}

impl From<GenericSlotId> for dir::GenericParameterRef {
    fn from(reference: GenericSlotId) -> Self {
        Self {
            owner: reference.owner,
            key: reference.key,
            index: reference.index,
        }
    }
}

impl From<dir::GenericParameterRef> for GenericSlotId {
    fn from(parameter: dir::GenericParameterRef) -> Self {
        Self {
            owner: parameter.owner,
            key: parameter.key,
            index: parameter.index,
        }
    }
}

/// Generic slot metadata shared by explicit and induced parameters.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum GenericSlot {
    /// Type generic slot.
    Type {
        /// The slot identity.
        slot: GenericSlotHeader,
        /// The slot variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Variadic type generic slot.
    VariadicType {
        /// The slot identity.
        slot: GenericSlotHeader,
        /// The slot variance.
        variance: Option<dir::VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<TypeOperand>,
        /// The optional type default.
        default: Option<TypeOperand>,
    },
    /// Static generic slot.
    Static {
        /// The slot identity.
        slot: GenericSlotHeader,
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
    /// Variadic static generic slot.
    VariadicStatic {
        /// The slot identity.
        slot: GenericSlotHeader,
        /// The optional static value type constraint.
        constraint: Option<TypeOperand>,
        /// The optional static default.
        default: Option<StaticOperand>,
    },
}

impl GenericSlot {
    /// Return this generic parameter's slot identity.
    pub(in crate::check) fn slot(&self) -> &GenericSlotHeader {
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
    pub(in crate::check) fn type_constraint(&self) -> Option<TypeOperand> {
        match self {
            Self::Type { constraint, .. } | Self::VariadicType { constraint, .. } => *constraint,
            Self::Static { .. } | Self::VariadicStatic { .. } => None,
        }
    }
}

impl CheckState<'_> {
    /// Push one declaration operand that can induce owner generics.
    pub(in crate::check) fn push_generic_induction_root(
        &mut self,
        owner: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) {
        let root = GenericInductionRoot { owner, operand };

        self.inference.push_generic_induction_root(root);
    }

    /// Induce one static generic from an escaping variable.
    pub(in crate::check) fn induce_static_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericSlotInduction,
    ) {
        let generic_induction = GenericInduction {
            variable,
            slot: GenericInductionSlot::r#static(prefix, constraint, induction),
        };

        self.inference.insert_generic_induction(generic_induction);
    }

    /// Induce one static generic constrained by a language item type.
    pub(in crate::check) fn induce_language_static_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        item: dir::LanguageItem,
    ) {
        let symbol = self.language_symbol(variable.module, item);
        let constraint = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: Vec::new().into(),
        };
        let constraint = self.inference.push_term(constraint).into();

        self.induce_static_generic(
            variable,
            prefix,
            Some(constraint),
            dir::GenericSlotInduction::Form,
        );
    }

    /// Induce one type generic from an escaping variable.
    pub(in crate::check) fn induce_type_generic(
        &mut self,
        variable: VariableId,
        prefix: &'static str,
        constraint: Option<TypeOperand>,
        induction: dir::GenericSlotInduction,
    ) {
        let generic_induction = GenericInduction {
            variable,
            slot: GenericInductionSlot::r#type(prefix, constraint, induction),
        };

        self.inference.insert_generic_induction(generic_induction);
    }

    /// Insert one generic application for a source node.
    pub(in crate::check) fn insert_generic_application(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> GenericApplication {
        let key = GenericApplicationKey { source, owner };

        self.inference.insert_generic_application(key, arguments)
    }

    /// Add one type constraint to an existing generic type slot.
    pub(in crate::check) fn constrain_generic_type_slot(
        &mut self,
        slot_id: GenericSlotId,
        constraint: TypeOperand,
    ) {
        let current = self.inference.generic_slot(slot_id).type_constraint();
        let constraint = match current {
            Some(current) => TypeOperand::Term(self.inference.push_term(TypeTerm::Intersection {
                elements: vec![current, constraint],
            })),
            None => constraint,
        };
        let generic = self.inference.generic_slot_by_id_mut(slot_id);

        match generic {
            GenericSlot::Type {
                constraint: current,
                ..
            }
            | GenericSlot::VariadicType {
                constraint: current,
                ..
            } => *current = Some(constraint),
            GenericSlot::Static { .. } | GenericSlot::VariadicStatic { .. } => {
                panic!("generic slot {slot_id:?} is not a type slot")
            }
        }
    }

    /// Insert one generic slot.
    pub(in crate::check) fn insert_generic_slot(&mut self, generic: GenericSlot) {
        self.inference.insert_generic_slot(generic);
    }

    /// Allocate one explicit generic slot for one owner.
    pub(in crate::check) fn allocate_explicit_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericSlotHeader {
        self.allocate_symbol_generic_slot(owner, symbol, dir::GenericSlotOrigin::Explicit)
    }

    /// Allocate one induced generic slot for one source symbol.
    pub(in crate::check) fn allocate_induced_symbol_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> GenericSlotHeader {
        self.allocate_symbol_generic_slot(
            owner,
            symbol,
            dir::GenericSlotOrigin::Induced(dir::GenericSlotInduction::Comptime),
        )
    }

    /// Allocate one induced generic slot for one owner.
    pub(in crate::check) fn allocate_generic_induction_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
        induction: dir::GenericSlotInduction,
    ) -> GenericSlotHeader {
        let index = self.next_generic_slot_index(owner);
        let name = self.generated_generic_name(owner, prefix, index);

        GenericSlotHeader {
            owner,
            key: dir::GenericSlotKey::Generated(name),
            index,
            origin: dir::GenericSlotOrigin::Induced(induction),
        }
    }

    /// Return the generated local name for one generic slot.
    fn generated_generic_name(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
        index: dir::GenericSlotIndex,
    ) -> dir::StringId {
        self.module_mut(owner.module_id)
            .strings
            .intern(&format!("{prefix}{}", index.get()))
    }

    /// Allocate the next generic slot index for one owner.
    fn next_generic_slot_index(&mut self, owner: dir::GlobalSymbolId) -> dir::GenericSlotIndex {
        self.inference.next_generic_slot_index(owner)
    }

    /// Allocate one source-symbol generic slot for one owner.
    fn allocate_symbol_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        origin: dir::GenericSlotOrigin,
    ) -> GenericSlotHeader {
        GenericSlotHeader {
            owner,
            key: dir::GenericSlotKey::Symbol(symbol),
            index: self.next_generic_slot_index(owner),
            origin,
        }
    }
}
