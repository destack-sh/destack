use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckState, GenericArgument, StaticOperand, TypeOperand, TypeTerm};

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
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
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
    /// The generic slot variables in declaration order.
    pub(in crate::check) slots: Vec<VariableId>,
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

/// Generic templates, slots, and applications for one checked component.
#[derive(Debug)]
pub(in crate::check) struct GenericTable {
    /// Generic templates keyed by owning symbol.
    pub(in crate::check) templates_by_owner: IndexMap<dir::GlobalSymbolId, GenericTemplate>,
    /// Generic applications keyed by source node and owner.
    pub(in crate::check) applications: IndexMap<GenericApplicationKey, GenericApplication>,
    /// Generic slot variables keyed by parameter symbol.
    pub(in crate::check) slots_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Generic slots keyed by variable.
    pub(in crate::check) slots_by_variable: IndexMap<VariableId, GenericSlot>,
}

impl GenericTable {
    /// Create an empty generic table.
    pub(in crate::check) fn new() -> Self {
        Self {
            templates_by_owner: IndexMap::new(),
            applications: IndexMap::new(),
            slots_by_symbol: IndexMap::new(),
            slots_by_variable: IndexMap::new(),
        }
    }
}

/// One declaration term checked for hidden owner generics.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Induction {
    /// The declaration that receives induced generic parameters.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The declaration term to traverse.
    pub(in crate::check) term: TypeTerm,
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

/// Generic parameter metadata attached to one check variable.
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
    /// Push one declaration term that can induce owner generics.
    pub(in crate::check) fn push_induction_root(
        &mut self,
        owner: dir::GlobalSymbolId,
        term: TypeTerm,
    ) {
        let root = Induction { owner, term };

        self.inference.push_induction(root);
    }

    /// Return an existing static generic parameter variable for one symbol.
    pub(in crate::check) fn generic_static_variable_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let variable = if self.modules.contains_key(&symbol.module_id) {
            self.generics.slots_by_symbol.get(&symbol).copied()
        } else {
            self.module(module)
                .imported_generic_by_symbol
                .get(&symbol)
                .copied()
        }?;
        let (_, generic) = self.variable_generic_slot(variable)?;

        generic.is_static().then_some(variable)
    }

    /// Return generic parameters in allocation order.
    pub(in crate::check) fn generic_slots(
        &self,
    ) -> impl Iterator<Item = (VariableId, GenericSlot)> {
        let parameters = self
            .generics
            .templates_by_owner
            .values()
            .flat_map(|template| template.slots.iter())
            .filter_map(|variable| self.variable_generic_slot(*variable))
            .collect::<Vec<_>>();

        parameters.into_iter()
    }

    /// Return generic parameters owned by one symbol.
    pub(in crate::check) fn generic_slots_for_owner(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = (VariableId, GenericSlot)> {
        let variables = if self.modules.contains_key(&owner.module_id) {
            self.generics
                .templates_by_owner
                .get(&owner)
                .map(|template| template.slots.as_slice())
        } else {
            self.module(module)
                .imported_generics
                .get(&owner)
                .map(Vec::as_slice)
        };
        let parameters = variables
            .into_iter()
            .flatten()
            .copied()
            .filter_map(|variable| self.variable_generic_slot(variable))
            .collect::<Vec<_>>();

        parameters.into_iter()
    }

    /// Resolve one type generic slot to its parameter variable.
    pub(in crate::check) fn resolve_type_generic_slot(
        &self,
        module: ModuleId,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        if self.modules.contains_key(&slot.owner.module_id) {
            let variables = &self.generics.templates_by_owner.get(&slot.owner)?.slots;

            return variables.iter().find_map(|variable| {
                let (_, generic) = self.variable_generic_slot(*variable)?;

                (generic.slot().id() == slot && generic.is_type()).then_some(*variable)
            });
        }

        let variable = self.module(module).imported_generic_by_slot.get(&slot)?;
        let (_, generic) = self.variable_generic_slot(*variable)?;

        (generic.is_type()).then_some(*variable)
    }

    /// Return one variable's generic parameter metadata.
    pub(in crate::check) fn generic_slot(&self, variable: VariableId) -> Option<&GenericSlot> {
        self.generics.slots_by_variable.get(&variable)
    }

    /// Return one already attached generic application argument.
    pub(in crate::check) fn generic_application_argument(
        &self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        index: dir::GenericSlotIndex,
    ) -> Option<GenericArgument> {
        let key = GenericApplicationKey { source, owner };

        self.generics
            .applications
            .get(&key)
            .and_then(|application| application.arguments.get(index.0 as usize))
            .cloned()
    }

    /// Attach or return one generic application for a source node.
    pub(in crate::check) fn attach_generic_application(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 4]>,
    ) -> GenericApplication {
        let key = GenericApplicationKey { source, owner };

        if let Some(application) = self.generics.applications.get(&key) {
            return application.clone();
        }

        let application = GenericApplication { owner, arguments };

        self.generics.applications.insert(key, application.clone());

        application
    }

    /// Return one variable's generic parameter metadata with its id.
    fn variable_generic_slot(&self, variable: VariableId) -> Option<(VariableId, GenericSlot)> {
        let generic = self.generics.slots_by_variable.get(&variable)?;

        Some((variable, generic.clone()))
    }

    /// Return the generic slot declared by one parameter symbol.
    pub(in crate::check) fn generic_slot_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericSlotId> {
        let variable = if self.modules.contains_key(&symbol.module_id) {
            self.generics.slots_by_symbol.get(&symbol).copied()
        } else {
            self.module(module)
                .imported_generic_by_symbol
                .get(&symbol)
                .copied()
        }?;
        let (_, generic) = self.variable_generic_slot(variable)?;

        Some(generic.slot().id())
    }

    /// Attach one variable to a generic slot.
    pub(in crate::check) fn attach_generic_slot(
        &mut self,
        variable: VariableId,
        generic: GenericSlot,
    ) {
        let owner = generic.slot().owner;
        let key = generic.slot().key;

        self.generics
            .slots_by_variable
            .insert(variable, generic.clone());
        self.generics
            .templates_by_owner
            .entry(owner)
            .or_insert_with(|| GenericTemplate::new(owner))
            .slots
            .push(variable);
        if let dir::GenericSlotKey::Symbol(symbol) = key {
            self.generics.slots_by_symbol.insert(symbol, variable);
        }
    }

    /// Attach one imported variable to a generic slot.
    pub(in crate::check) fn attach_imported_generic_slot(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        generic: GenericSlot,
    ) {
        let owner = generic.slot().owner;
        let key = generic.slot().key;
        let slot_id = generic.slot().id();

        self.generics
            .slots_by_variable
            .insert(variable, generic.clone());
        self.module_mut(module)
            .imported_generics
            .entry(owner)
            .or_default()
            .push(variable);
        self.module_mut(module)
            .imported_generic_by_slot
            .insert(slot_id, variable);
        if let dir::GenericSlotKey::Symbol(symbol) = key {
            self.module_mut(module)
                .imported_generic_by_symbol
                .insert(symbol, variable);
        }
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
        self.allocate_symbol_generic_slot(owner, symbol, dir::GenericSlotOrigin::Induced)
    }

    /// Allocate one induced generic slot for one owner.
    pub(in crate::check) fn allocate_induced_generic_slot(
        &mut self,
        owner: dir::GlobalSymbolId,
        prefix: &str,
    ) -> GenericSlotHeader {
        let index = self.next_generic_slot_index(owner);
        let name = self.generated_generic_name(owner, prefix, index);

        GenericSlotHeader {
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
        self.module_mut(owner.module_id)
            .strings
            .intern(&format!("{prefix}{}", index.get()))
    }

    /// Allocate the next generic slot index for one owner.
    fn next_generic_slot_index(&mut self, owner: dir::GlobalSymbolId) -> dir::GenericSlotIndex {
        let index = self
            .generics
            .templates_by_owner
            .entry(owner)
            .or_insert_with(|| GenericTemplate::new(owner))
            .slots
            .len();

        dir::GenericSlotIndex::new(index as u32)
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
