use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    CheckState, Condition, Definition, FormTerm, GenericParameter, Origin, Solution, StaticOperand,
    StaticTerm, TypeOperationTerm, TypeTerm, VariableId, VariableKind, VariableOutput,
};
use crate::{CompilerError, CompilerResult};

/// Stable key for one induced generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InducedGenericKey {
    /// The concrete owner that receives the generated generic slot.
    owner: dir::GlobalSymbolId,
    /// The transparent leaf rewritten to the generated parameter.
    leaf: VariableId,
}

/// Parameter to induce for one transparent leaf.
#[derive(Debug, Clone, PartialEq)]
enum InducedParameter {
    /// Type parameter constrained by the transparent type leaf.
    Type {
        /// The generated type constraint.
        constraint: TypeTerm,
    },
    /// Static parameter constrained by the transparent static domain.
    Static {
        /// The generated slot name prefix.
        prefix: &'static str,
        /// The generated static value type constraint.
        constraint: TypeTerm,
    },
}

impl CheckState<'_> {
    /// Prepare induced generic slots before solving constraints.
    pub(in crate::check) fn prepare_generics(&mut self) -> CompilerResult<()> {
        let definitions = self.type_definitions_by_result();
        let generics = self.collect_induced_generics(&definitions)?;

        self.insert_induced_generics(generics)
    }

    /// Return first type definitions keyed by result variable.
    fn type_definitions_by_result(&self) -> IndexMap<VariableId, TypeTerm> {
        let mut definitions = IndexMap::new();

        for definition in &self.variables.definitions {
            let Definition::Type {
                result,
                term,
                origin: _,
                condition: _,
            } = definition
            else {
                continue;
            };

            definitions
                .entry(*result)
                .or_insert_with(|| self.terms.get(*term).clone());
        }

        definitions
    }

    /// Collect induced generics from concrete declaration surfaces.
    fn collect_induced_generics(
        &self,
        definitions: &IndexMap<VariableId, TypeTerm>,
    ) -> CompilerResult<IndexMap<InducedGenericKey, InducedParameter>> {
        let mut generics = IndexMap::new();

        for (result, term) in definitions {
            let Some(owner) = self.concrete_generic_owner(*result) else {
                continue;
            };
            let mut visited = IndexSet::new();

            self.collect_induced_generics_for_term(
                owner,
                term,
                definitions,
                &mut visited,
                &mut generics,
            )?;
        }

        Ok(generics)
    }

    /// Return the concrete symbol that can receive induced generics.
    fn concrete_generic_owner(&self, variable: VariableId) -> Option<dir::GlobalSymbolId> {
        let Some(VariableOutput::Symbol(symbol)) = self.variable(variable).output else {
            return None;
        };
        if !self.modules.contains_key(&symbol.module_id) {
            return None;
        }
        let kind = self.symbol_kind(symbol.module_id, symbol)?;

        Self::symbol_kind_accepts_induced_generics(kind).then_some(symbol)
    }

    /// Collect induced generics referenced by one type term.
    fn collect_induced_generics_for_term(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
        generics: &mut IndexMap<InducedGenericKey, InducedParameter>,
    ) -> CompilerResult<()> {
        if let TypeTerm::Variable(variable) = term {
            return self.collect_induced_generics_for_variable(
                owner,
                *variable,
                definitions,
                visited,
                generics,
            );
        }

        if let TypeTerm::Form { form, payload: _ } = term {
            self.collect_induced_generics_for_form(owner, self.terms.get(*form), generics);
        }

        for variable in term.referenced_variables(self) {
            self.collect_induced_generics_for_variable(
                owner,
                variable,
                definitions,
                visited,
                generics,
            )?;
        }

        Ok(())
    }

    /// Collect induced generics referenced by one type variable.
    fn collect_induced_generics_for_variable(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
        generics: &mut IndexMap<InducedGenericKey, InducedParameter>,
    ) -> CompilerResult<()> {
        if !visited.insert(variable) {
            return Ok(());
        }
        let Some(term) = definitions.get(&variable) else {
            return Ok(());
        };
        if let Some(parameter) = self.induced_type_parameter(term, definitions, visited)? {
            let key = InducedGenericKey {
                owner,
                leaf: variable,
            };

            generics.entry(key).or_insert(parameter);

            return Ok(());
        }

        self.collect_induced_generics_for_term(owner, term, definitions, visited, generics)
    }

    /// Collect induced static generics referenced by one form.
    fn collect_induced_generics_for_form(
        &self,
        owner: dir::GlobalSymbolId,
        form: &FormTerm,
        generics: &mut IndexMap<InducedGenericKey, InducedParameter>,
    ) {
        match form {
            FormTerm::Borrowed { lifetime, access } => {
                self.collect_induced_static_generic(
                    owner,
                    *lifetime,
                    "L",
                    dir::LanguageItem::Lifetime,
                    generics,
                );
                self.collect_induced_static_generic(
                    owner,
                    *access,
                    "A",
                    dir::LanguageItem::Access,
                    generics,
                );
            }
            FormTerm::Placed { place } => {
                self.collect_induced_static_generic(
                    owner,
                    *place,
                    "P",
                    dir::LanguageItem::Place,
                    generics,
                );
            }
            FormTerm::Managed | FormTerm::Owned | FormTerm::Raw | FormTerm::Readonly => {}
        }
    }

    /// Collect one induced static generic.
    fn collect_induced_static_generic(
        &self,
        owner: dir::GlobalSymbolId,
        value: StaticOperand,
        prefix: &'static str,
        item: dir::LanguageItem,
        generics: &mut IndexMap<InducedGenericKey, InducedParameter>,
    ) {
        let StaticOperand::Variable(variable) = value else {
            return;
        };
        let state = self.variable(variable);
        if state.kind != VariableKind::Static || state.output.is_some() {
            return;
        }
        let symbol = self.language_symbol(variable.module, item);
        let constraint = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: Vec::new().into(),
        };
        let key = InducedGenericKey {
            owner,
            leaf: variable,
        };
        let parameter = InducedParameter::Static { prefix, constraint };

        generics.entry(key).or_insert(parameter);
    }

    /// Return the induced parameter represented by one transparent type term.
    fn induced_type_parameter(
        &self,
        term: &TypeTerm,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
    ) -> CompilerResult<Option<InducedParameter>> {
        let is_constraint = match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments: _,
            } => self.symbol_induces_generic_constraint(*symbol, definitions, visited)?,
            TypeTerm::Operation(operation) => match self.terms.get(*operation) {
                TypeOperationTerm::Intrinsic { item, arguments: _ } => {
                    Self::language_item_induces_generic_constraint(*item)
                }
                _ => false,
            },
            _ => false,
        };
        if !is_constraint {
            return Ok(None);
        }

        Ok(Some(InducedParameter::Type {
            constraint: term.clone(),
        }))
    }

    /// Return whether one symbol names a transparent constraint.
    fn symbol_induces_generic_constraint(
        &self,
        symbol: dir::GlobalSymbolId,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
    ) -> CompilerResult<bool> {
        if self.symbol_is_memory_query_language_item(symbol) {
            return Ok(true);
        }
        let kind = self.symbol_kind(symbol.module_id, symbol);
        let is_constraint = match kind {
            Some(
                dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface,
            ) => true,
            Some(dir::SymbolKind::TypeAlias) => {
                self.type_alias_induces_generic_constraint(symbol, definitions, visited)?
            }
            Some(_) | None => false,
        };

        Ok(is_constraint)
    }

    /// Return whether one alias expands to a transparent constraint.
    fn type_alias_induces_generic_constraint(
        &self,
        symbol: dir::GlobalSymbolId,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
    ) -> CompilerResult<bool> {
        let Some(variable) = self.variables.type_by_symbol.get(&symbol).copied() else {
            return Ok(false);
        };

        self.type_variable_induces_generic_constraint(variable, definitions, visited)
    }

    /// Return whether one type variable expands to a transparent constraint.
    fn type_variable_induces_generic_constraint(
        &self,
        variable: VariableId,
        definitions: &IndexMap<VariableId, TypeTerm>,
        visited: &mut IndexSet<VariableId>,
    ) -> CompilerResult<bool> {
        if !visited.insert(variable) {
            return Ok(false);
        }
        let Some(term) = definitions.get(&variable) else {
            return Ok(false);
        };
        if self
            .induced_type_parameter(term, definitions, visited)?
            .is_some()
        {
            return Ok(true);
        }

        for variable in term.referenced_variables(self) {
            if self.type_variable_induces_generic_constraint(variable, definitions, visited)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one symbol names a memory query language item.
    fn symbol_is_memory_query_language_item(&self, symbol: dir::GlobalSymbolId) -> bool {
        let Some(item) = self.environment.language.item(symbol) else {
            return false;
        };

        Self::language_item_induces_generic_constraint(item)
    }

    /// Return whether one language item names a transparent memory query.
    fn language_item_induces_generic_constraint(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::AccessOf
                | dir::LanguageItem::AccessOr
                | dir::LanguageItem::BaseOf
                | dir::LanguageItem::IsBorrowed
                | dir::LanguageItem::IsManaged
                | dir::LanguageItem::IsOwned
                | dir::LanguageItem::IsRaw
                | dir::LanguageItem::IsShared
                | dir::LanguageItem::IsSharedIn
                | dir::LanguageItem::LifetimeOf
                | dir::LanguageItem::LifetimeOr
                | dir::LanguageItem::OwnershipOf
                | dir::LanguageItem::OwnershipOr
                | dir::LanguageItem::PayloadOf
                | dir::LanguageItem::PlaceIn
                | dir::LanguageItem::PlaceOf
                | dir::LanguageItem::PlaceOr
                | dir::LanguageItem::SpaceOf
                | dir::LanguageItem::SpaceOr
        )
    }

    /// Insert collected induced generic slots into the type graph.
    fn insert_induced_generics(
        &mut self,
        generics: IndexMap<InducedGenericKey, InducedParameter>,
    ) -> CompilerResult<()> {
        for (key, parameter) in generics {
            match parameter {
                InducedParameter::Type { constraint } => {
                    self.insert_induced_type_generic(key, constraint)?;
                }
                InducedParameter::Static { prefix, constraint } => {
                    self.insert_induced_static_generic(key, prefix, constraint)?;
                }
            }
        }

        Ok(())
    }

    /// Insert one induced type generic slot.
    fn insert_induced_type_generic(
        &mut self,
        key: InducedGenericKey,
        constraint: TypeTerm,
    ) -> CompilerResult<()> {
        self.assert_induced_generic_is_local(key)?;

        let constraint = self.terms.push(constraint);
        let slot = self.allocate_induced_generic_slot(key.owner, "T");
        let slot_id = slot.id();
        let generic = GenericParameter::Type {
            slot,
            variance: None,
            constraint: Some(constraint.into()),
            default: None,
        };
        let variable = self.allocate_output_variable(
            key.leaf.module,
            VariableKind::Type,
            VariableOutput::Generic(generic.clone()),
        );
        self.attach_generic_parameter(variable, generic);
        self.add_type_definition(variable, TypeTerm::Parameter(slot_id), Condition::Always);

        // rewrite the transparent leaf as the induced parameter
        let term = self.terms.push(TypeTerm::Parameter(slot_id));
        self.insert_known_solution(key.leaf, Solution::Type(term));
        self.add_induced_generic_to_owner_function(key.owner, variable);

        Ok(())
    }

    /// Insert one induced static generic slot.
    fn insert_induced_static_generic(
        &mut self,
        key: InducedGenericKey,
        prefix: &'static str,
        constraint: TypeTerm,
    ) -> CompilerResult<()> {
        self.assert_induced_generic_is_local(key)?;

        let constraint = self.terms.push(constraint);
        let slot = self.allocate_induced_generic_slot(key.owner, prefix);
        let slot_id = slot.id();
        let generic = GenericParameter::Static {
            slot,
            constraint: Some(constraint.into()),
            default: None,
        };
        let variable = self.allocate_output_variable(
            key.leaf.module,
            VariableKind::Static,
            VariableOutput::Generic(generic.clone()),
        );
        self.attach_generic_parameter(variable, generic);
        self.add_static_definition(variable, StaticTerm::Parameter(slot_id), Condition::Always);

        // rewrite the transparent leaf as the induced parameter
        let term = self.terms.push(StaticTerm::Parameter(slot_id));
        self.insert_known_solution(key.leaf, Solution::Static(term));
        self.add_induced_generic_to_owner_function(key.owner, variable);

        Ok(())
    }

    /// Assert that one induced generic belongs to its owner module.
    fn assert_induced_generic_is_local(&self, key: InducedGenericKey) -> CompilerResult<()> {
        if key.owner.module_id == key.leaf.module {
            return Ok(());
        }

        Err(CompilerError::Internal {
            message: "induced generic owner and leaf are in different modules".to_string(),
        })
    }

    /// Return whether one symbol kind accepts induced generics.
    fn symbol_kind_accepts_induced_generics(kind: dir::SymbolKind) -> bool {
        matches!(
            kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Function
                | dir::SymbolKind::Newtype
                | dir::SymbolKind::Struct
        )
    }

    /// Add one induced generic parameter to an owner function type.
    fn add_induced_generic_to_owner_function(
        &mut self,
        owner: dir::GlobalSymbolId,
        parameter: VariableId,
    ) {
        let Some(owner_type) = self.variables.type_by_symbol.get(&owner).copied() else {
            return;
        };
        let function = self
            .variables
            .definitions
            .iter()
            .find_map(|definition| match definition {
                Definition::Type { result, term, .. } if *result == owner_type => {
                    let TypeTerm::Function(function) = self.terms.get(*term) else {
                        return None;
                    };

                    Some(*function)
                }
                _ => None,
            });
        let Some(function) = function else {
            return;
        };
        let function = self.terms.get_mut(function);

        // append induced owner generics after explicit generics
        if !function.generic_parameters.contains(&parameter) {
            function.generic_parameters.push(parameter);
        }
    }
}
