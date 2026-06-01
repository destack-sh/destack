use destack_dir as dir;
use destack_source::ModuleId;
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

/// One declaration operand that can induce owner generics.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericInductionRoot {
    /// The declaration that receives induced generic parameters.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The declaration operand to traverse.
    pub(in crate::check) operand: TypeOperand,
}

/// One variable that may become an induced owner generic.
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
    pub(in crate::check) fn ty(
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
    pub(in crate::check) fn static_value(
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

    /// Return this static parameter's type constraint.
    pub(in crate::check) fn static_constraint(&self) -> Option<TypeOperand> {
        match self {
            Self::Static { constraint, .. } | Self::VariadicStatic { constraint, .. } => {
                *constraint
            }
            Self::Type { .. } | Self::VariadicType { .. } => None,
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
            slot: GenericInductionSlot::static_value(prefix, constraint, induction),
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
        let constraint = self.push_term(constraint).into();

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
            slot: GenericInductionSlot::ty(prefix, constraint, induction),
        };

        self.inference.insert_generic_induction(generic_induction);
    }

    /// Return the induced generic slot attached to one variable.
    pub(in crate::check) fn generic_induction_slot(
        &self,
        variable: VariableId,
    ) -> Option<GenericInductionSlot> {
        self.inference.generic_induction_slot(variable)
    }

    /// Return an existing static generic parameter variable for one symbol.
    pub(in crate::check) fn generic_static_variable_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let variable = self.generic_variable_for_symbol(module, symbol)?;
        let (_, generic) = self.variable_generic_slot(variable)?;

        generic.is_static().then_some(variable)
    }

    /// Return generic parameters in allocation order.
    pub(in crate::check) fn generic_slots(
        &self,
    ) -> impl Iterator<Item = (VariableId, GenericSlot)> {
        let parameters = self
            .inference
            .segments
            .iter()
            .flat_map(|segment| segment.generic_templates.values())
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
            self.generic_slot_variables_for_owner(owner)
        } else {
            self.dependency_generic_variables_for_owner(module, owner)
        };
        let parameters = variables
            .into_iter()
            .filter_map(|variable| self.variable_generic_slot(variable))
            .collect::<Vec<_>>();

        parameters.into_iter()
    }

    /// Return one variable's generic parameter metadata.
    pub(in crate::check) fn generic_slot(&self, variable: VariableId) -> Option<&GenericSlot> {
        self.inference
            .segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_slots_by_variable.get(&variable))
    }

    /// Return one already attached generic application argument.
    pub(in crate::check) fn generic_application_argument(
        &self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        index: dir::GenericSlotIndex,
    ) -> Option<GenericArgument> {
        let key = GenericApplicationKey { source, owner };

        self.generic_application(key)
            .and_then(|application| application.arguments.get(index.0 as usize))
            .cloned()
    }

    /// Attach or return one generic application for a source node.
    pub(in crate::check) fn attach_generic_application(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> GenericApplication {
        let key = GenericApplicationKey { source, owner };

        if let Some(application) = self.generic_application(key) {
            return application.clone();
        }

        let application = GenericApplication { owner, arguments };

        self.inference
            .current_mut()
            .generic_applications
            .insert(key, application.clone());

        application
    }

    /// Return one variable's generic parameter metadata with its id.
    fn variable_generic_slot(&self, variable: VariableId) -> Option<(VariableId, GenericSlot)> {
        let generic = self.generic_slot(variable)?;

        Some((variable, generic.clone()))
    }

    /// Return the generic slot declared by one parameter symbol.
    pub(in crate::check) fn generic_slot_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericSlotId> {
        let variable = self.generic_variable_for_symbol(module, symbol)?;
        let (_, generic) = self.variable_generic_slot(variable)?;

        Some(generic.slot().id())
    }

    /// Add one type constraint to an existing generic type slot.
    pub(in crate::check) fn constrain_generic_type_slot(
        &mut self,
        slot_id: GenericSlotId,
        constraint: TypeOperand,
    ) {
        let variable = self.generic_type_slot_variable(slot_id);
        let current = self
            .generic_slot(variable)
            .and_then(GenericSlot::type_constraint);
        let constraint = match current {
            Some(current) => TypeOperand::Term(self.push_term(TypeTerm::Intersection {
                elements: vec![current, constraint],
            })),
            None => constraint,
        };
        let generic = self.generic_slot_mut(variable);

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

    /// Attach one variable to a generic slot.
    pub(in crate::check) fn attach_generic_slot(
        &mut self,
        variable: VariableId,
        generic: GenericSlot,
    ) {
        let owner = generic.slot().owner;
        let key = generic.slot().key;

        let segment = self.inference.current_mut();

        segment
            .generic_slots_by_variable
            .insert(variable, generic.clone());
        segment
            .generic_templates
            .entry(owner)
            .or_insert_with(|| GenericTemplate::new(owner))
            .slots
            .push(variable);
        if let dir::GenericSlotKey::Symbol(symbol) = key {
            segment.generic_slots_by_symbol.insert(symbol, variable);
        }
    }

    /// Attach one dependency variable to a generic slot.
    pub(in crate::check::state) fn attach_dependency_generic_slot(
        &mut self,
        variable: VariableId,
        generic: GenericSlot,
    ) {
        self.inference
            .current_mut()
            .generic_slots_by_variable
            .insert(variable, generic);
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
        let index = self.generic_slot_variables_for_owner(owner).len();

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

    /// Return the visible generic application for one source and owner.
    fn generic_application(&self, key: GenericApplicationKey) -> Option<&GenericApplication> {
        self.inference
            .segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_applications.get(&key))
    }

    /// Return the visible generic variable declared by one parameter symbol.
    pub(in crate::check) fn generic_variable_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        if self.modules.contains_key(&symbol.module_id) {
            return self.generic_slot_variable_for_symbol(symbol);
        }

        self.dependency_generic_variable_for_symbol(module, symbol)
    }

    /// Return the visible generic variable declared by one local parameter symbol.
    pub(in crate::check) fn generic_slot_variable_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        self.inference
            .segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_slots_by_symbol.get(&symbol).copied())
    }

    /// Return the visible generic variable declared by one generic slot.
    pub(in crate::check) fn generic_slot_variable(
        &self,
        slot_id: GenericSlotId,
    ) -> Option<VariableId> {
        self.inference.segments.iter().rev().find_map(|segment| {
            segment
                .generic_slots_by_variable
                .iter()
                .find_map(|(variable, generic)| {
                    (generic.slot().id() == slot_id).then_some(*variable)
                })
        })
    }

    /// Return the visible dependency generic variable declared by one parameter symbol.
    fn dependency_generic_variable_for_symbol(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        self.dependency_generic_slot_variable_by(|variable, generic| {
            variable.module == module && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
        })
    }

    /// Return the visible dependency generic variable for one source slot.
    pub(in crate::check::state) fn dependency_generic_slot_variable(
        &self,
        module: ModuleId,
        slot_id: GenericSlotId,
    ) -> Option<VariableId> {
        self.dependency_generic_slot_variable_by(|variable, generic| {
            variable.module == module && generic.slot().id() == slot_id
        })
    }

    /// Return visible generic variables owned by one symbol.
    fn generic_slot_variables_for_owner(&self, owner: dir::GlobalSymbolId) -> Vec<VariableId> {
        self.inference
            .segments
            .iter()
            .flat_map(|segment| {
                segment
                    .generic_templates
                    .get(&owner)
                    .into_iter()
                    .flat_map(|template| template.slots.iter().copied())
            })
            .collect()
    }

    /// Return visible dependency generic variables owned by one symbol.
    fn dependency_generic_variables_for_owner(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
    ) -> Vec<VariableId> {
        self.dependency_generic_slot_variables_by(|variable, generic| {
            variable.module == module && generic.slot().owner == owner
        })
    }

    /// Return the variable for one visible generic type slot.
    fn generic_type_slot_variable(&self, slot_id: GenericSlotId) -> VariableId {
        self.inference
            .segments
            .iter()
            .rev()
            .find_map(|segment| {
                segment
                    .generic_slots_by_variable
                    .iter()
                    .find_map(|(variable, generic)| {
                        (generic.slot().id() == slot_id && generic.is_type()).then_some(*variable)
                    })
            })
            .unwrap_or_else(|| panic!("generic type slot {slot_id:?} does not exist"))
    }

    /// Return one visible generic slot mutably.
    fn generic_slot_mut(&mut self, variable: VariableId) -> &mut GenericSlot {
        self.inference
            .segments
            .iter_mut()
            .rev()
            .find_map(|segment| segment.generic_slots_by_variable.get_mut(&variable))
            .unwrap_or_else(|| panic!("generic variable {variable:?} has no slot"))
    }

    /// Return one visible dependency generic variable matching a predicate.
    fn dependency_generic_slot_variable_by(
        &self,
        matches: impl Fn(VariableId, &GenericSlot) -> bool,
    ) -> Option<VariableId> {
        self.dependency_generic_slot_variables_by(matches)
            .into_iter()
            .next()
    }

    /// Return visible dependency generic variables matching a predicate.
    fn dependency_generic_slot_variables_by(
        &self,
        matches: impl Fn(VariableId, &GenericSlot) -> bool,
    ) -> Vec<VariableId> {
        self.inference
            .segments
            .iter()
            .flat_map(|segment| {
                segment
                    .generic_slots_by_variable
                    .iter()
                    .filter_map(|(variable, generic)| {
                        matches(*variable, generic).then_some(*variable)
                    })
            })
            .collect()
    }
}
