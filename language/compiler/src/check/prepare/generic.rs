use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    CheckState, Definition, GenericParameter, Solution, TypeOperationTerm, TypeTerm, VariableId,
    VariableKind, VariableOutput,
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

impl CheckState<'_> {
    /// Prepare induced generic slots before solving constraints.
    pub(in crate::check) fn prepare_generics(&mut self) -> CompilerResult<()> {
        let definitions = self.collect_type_definitions();
        let mut owned = IndexMap::new();

        // collect transparent leaves directly from type expression definitions
        for (variable, term) in definitions {
            if !self.type_term_induces_generic(variable, &term)? {
                continue;
            }
            let Some(owner) = self.induced_generic_owner(variable)? else {
                continue;
            };
            let key = InducedGenericKey {
                owner,
                leaf: variable,
            };

            owned.entry(key).or_insert(term);
        }

        self.insert_induced_generics(owned)
    }

    /// Collect type definition terms keyed by result variable.
    fn collect_type_definitions(&self) -> IndexMap<VariableId, TypeTerm> {
        let mut definitions = IndexMap::new();

        for definition in self.variables.definitions.iter() {
            let Definition::Type {
                result,
                term,
                origin: _,
                condition: _,
            } = definition
            else {
                continue;
            };

            definitions.insert(*result, self.terms.get(*term).clone());
        }

        definitions
    }

    /// Return the concrete owner that receives an induced generic leaf.
    fn induced_generic_owner(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(source) = self.type_expression_node(variable)? else {
            return Ok(None);
        };
        let Some(owner) = self.scope_owner_symbol(source.module_id, source.local_id) else {
            return Ok(None);
        };

        self.concrete_induced_generic_owner(owner)
    }

    /// Return the concrete declaration that can own induced generic slots.
    fn concrete_induced_generic_owner(
        &self,
        mut symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        loop {
            if !self.inputs.contains_key(&symbol.module_id) {
                return Ok(None);
            };
            let Some(kind) = self.symbol_kind(symbol.module_id, symbol) else {
                return Ok(None);
            };

            // transparent owners keep their own constraints transparent
            if Self::symbol_kind_is_transparent_constraint_owner(kind) {
                return Ok(None);
            }

            // concrete declarations receive generated slots
            if Self::symbol_kind_accepts_induced_generics(kind) {
                return Ok(Some(symbol));
            }

            let Some(source) = self.symbol_source_node(symbol.module_id, symbol) else {
                return Ok(None);
            };
            let Some(owner) = self.scope_owner_symbol(symbol.module_id, source) else {
                return Ok(None);
            };
            if owner == symbol {
                return Ok(None);
            }

            symbol = owner;
        }
    }

    /// Return whether one type term induces a generic.
    fn type_term_induces_generic(
        &self,
        variable: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<bool> {
        if self.type_expression_node(variable)?.is_none() {
            return Ok(false);
        }

        match term {
            // Writer
            TypeTerm::Reference {
                symbol,
                arguments: _,
                source: _,
            } => self.symbol_induces_generic_constraint(variable.module, *symbol),
            // PlaceOf<T>
            TypeTerm::Operation(operation) => match self.terms.get(*operation) {
                TypeOperationTerm::Intrinsic { item, arguments: _ } => {
                    Ok(Self::language_item_induces_generic_constraint(*item))
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    /// Return the type expression node backing one variable.
    fn type_expression_node(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<dir::GlobalNodeIdAny>> {
        let Some(VariableOutput::Node(node)) = self.variable(variable).output else {
            return Ok(None);
        };
        if node.local_id.ty != dir::NodeType::TypeExpression {
            return Ok(None);
        }

        Ok(Some(node))
    }

    /// Return whether one symbol names a constraint that induces a generic.
    fn symbol_induces_generic_constraint(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let is_transparent = self.is_transparent_constraint_symbol(module, symbol)
            || self.symbol_is_memory_query_language_item(symbol);

        Ok(is_transparent)
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
        generics: IndexMap<InducedGenericKey, TypeTerm>,
    ) -> CompilerResult<()> {
        for (key, term) in generics {
            self.insert_induced_generic(key, term)?;
        }

        Ok(())
    }

    /// Insert one induced generic slot for one transparent leaf.
    fn insert_induced_generic(
        &mut self,
        key: InducedGenericKey,
        term: TypeTerm,
    ) -> CompilerResult<()> {
        if key.owner.module_id != key.leaf.module {
            return Err(CompilerError::Internal {
                message: "induced generic owner and leaf are in different modules".to_string(),
            });
        }
        let module = key.leaf.module;
        let origin = self.variable_origin(key.leaf)?;
        let bound = self.allocate_intermediate_variable(module, VariableKind::Type, origin);

        // keep the original transparent constraint as the generated slot bound
        self.define_type(module, bound, term);

        // create the generated generic parameter
        let slot = self.allocate_induced_generic_slot(key.owner, "T");
        let slot_id = slot.id();
        let generic = GenericParameter::Type {
            slot,
            variance: None,
            constraint: Some(bound),
            default: None,
        };
        let variable = self.allocate_variable(
            module,
            VariableKind::Type,
            VariableOutput::Generic(generic.clone()),
        );
        self.record_generic_parameter(variable, generic);

        self.define_type(module, variable, TypeTerm::Parameter(slot_id));

        // make the transparent leaf solve as the generated parameter
        let term = self.terms.push(TypeTerm::Parameter(slot_id));
        self.solve_variable(key.leaf, Solution::Type(term));
        self.add_induced_generic_to_owner_function(key.owner, variable);

        Ok(())
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

    /// Return whether one symbol kind is a transparent constraint owner.
    fn symbol_kind_is_transparent_constraint_owner(kind: dir::SymbolKind) -> bool {
        matches!(
            kind,
            dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface
        )
    }
}

impl CheckState<'_> {
    /// Add one induced generic parameter to an owner function type.
    fn add_induced_generic_to_owner_function(
        &mut self,
        owner: dir::GlobalSymbolId,
        parameter: VariableId,
    ) {
        let Some(owner_type) = self.variables.type_by_symbol.get(&owner).copied() else {
            return;
        };

        let definitions = self.variables.definitions.clone();
        for definition in definitions {
            let Definition::Type { result, term, .. } = definition else {
                continue;
            };
            if result != owner_type {
                continue;
            }
            let TypeTerm::Function(function) = self.terms.get(term) else {
                return;
            };
            let function = *function;
            let function = self.terms.get_mut(function);

            // append induced owner generics after explicit generics
            if !function.generic_parameters.contains(&parameter) {
                function.generic_parameters.push(parameter);
            }

            return;
        }
    }
}
