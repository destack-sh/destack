use destack_dir as dir;

use crate::check::{
    CheckState, GenericArgument, Obligation, Origin, ReceiverTerm, StaticTerm, TypeOperationTerm,
    TypeTerm, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Build one identifier reference term.
    pub(in crate::check) fn build_identifier_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let symbol = self.require_symbol_by_name(
            tree.module_id,
            id.into_any(),
            name,
            dir::SymbolSpace::Value,
        )?;
        let source = id.into_global_any(tree.module_id);

        self.use_value_symbol(source, symbol);
        self.check_value_read_assigned(source, id.into_any(), symbol);

        Some(self.value_reference_term(tree, id, symbol, &[]))
    }

    /// Build one qualified reference term.
    pub(in crate::check) fn build_qualified_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let symbol =
            self.require_path_symbol(tree.module_id, id.into_any(), path, dir::SymbolSpace::Value)?;
        let source = id.into_global_any(tree.module_id);

        self.use_value_symbol(source, symbol);
        self.check_value_read_assigned(source, id.into_any(), symbol);

        Some(self.value_reference_term(tree, id, symbol, generic_arguments))
    }

    /// Return the term for one resolved value reference.
    fn value_reference_term(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> TypeTerm {
        // direct references can use flow narrowed types
        if generic_arguments.is_empty()
            && let Some(narrowed) = self.flow_path_narrowing(tree, id)
        {
            return narrowed.to_type_term(self);
        }

        let arguments = self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
        let source = id.into_global_any(tree.module_id);

        TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments,
        }
    }

    /// Build one reference type expression term.
    pub(in crate::check) fn build_reference_type_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let source = id.into_global_any(tree.module_id);

        // generic value parameters can appear as static type operands
        if generic_arguments.is_empty()
            && let [name] = path.segments.as_slice()
            && let Some(symbol) = self
                .lookup_symbol_by_name(
                    tree.module_id,
                    id.into_any(),
                    *name,
                    dir::SymbolSpace::Value,
                )
                .unique_symbol()
            && self.symbol_kind(tree.module_id, symbol)
                == Some(dir::SymbolKind::GenericValueParameter)
        {
            self.select_name(source, symbol);

            let slot_id = self.generic_slot_for_symbol(tree.module_id, symbol)?;

            return Some(TypeTerm::Parameter(slot_id));
        }

        let symbol =
            self.require_path_symbol(tree.module_id, id.into_any(), path, dir::SymbolSpace::Type)?;

        self.select_name(source, symbol);

        // direct generic parameter references are local type variables
        if generic_arguments.is_empty()
            && self.symbol_kind(tree.module_id, symbol)
                == Some(dir::SymbolKind::GenericTypeParameter)
        {
            let variable = self.intern_local_symbol_type_variable(tree.module_id, symbol);

            return Some(TypeTerm::Variable(variable));
        }

        let arguments = self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
        if let Some(item) = self.environment.language.item(symbol) {
            if item == dir::LanguageItem::Dynamic
                && let Some(constraint) = arguments.first().and_then(GenericArgument::type_operand)
            {
                self.add_obligation(Obligation::DynamicSafe {
                    source,
                    constraint,
                    condition: self.active_static_condition(tree.module_id),
                });
            }

            if Self::memory_type_intrinsic_item(item) {
                let operation = self.terms.push(TypeOperationTerm::Intrinsic {
                    item,
                    arguments: arguments.into(),
                });

                return Some(TypeTerm::Operation(operation));
            }
            if Self::memory_static_intrinsic_item(item) {
                let term = StaticTerm::Intrinsic {
                    item,
                    arguments: arguments.into(),
                };
                let term = self.terms.push(term);

                return Some(TypeTerm::StaticValue { value: term.into() });
            }
        }

        Some(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments,
        })
    }

    /// Return whether one language item is a type returning memory intrinsic.
    pub(in crate::check) fn memory_type_intrinsic_item(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::PayloadOf
                | dir::LanguageItem::BaseOf
                | dir::LanguageItem::WithBase
                | dir::LanguageItem::WithPlace
                | dir::LanguageItem::WithSpace
                | dir::LanguageItem::WithLifetime
                | dir::LanguageItem::WithAccess
                | dir::LanguageItem::WithOwnership
        )
    }

    /// Return whether one language item is a static returning memory intrinsic.
    pub(in crate::check) fn memory_static_intrinsic_item(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::OwnershipOf
                | dir::LanguageItem::OwnershipOr
                | dir::LanguageItem::PlaceOf
                | dir::LanguageItem::PlaceOr
                | dir::LanguageItem::PlaceIn
                | dir::LanguageItem::SpaceOf
                | dir::LanguageItem::SpaceOr
                | dir::LanguageItem::LifetimeOf
                | dir::LanguageItem::LifetimeOr
                | dir::LanguageItem::AccessOf
                | dir::LanguageItem::AccessOr
                | dir::LanguageItem::IsManaged
                | dir::LanguageItem::IsOwned
                | dir::LanguageItem::IsBorrowed
                | dir::LanguageItem::IsRaw
                | dir::LanguageItem::IsShared
                | dir::LanguageItem::IsSharedIn
        )
    }

    /// Define one intermediate variable for the contextual receiver.
    pub(in crate::check) fn define_this_receiver_variable(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<VariableId> {
        let receiver = self.resolve_this_receiver(source)?;
        let origin = Origin::Node(source);
        let module = source.module_id;
        let variable = self.allocate_inference_variable(module, VariableKind::Type, origin);
        let receiver = self.terms.push(ReceiverTerm {
            source,
            kind: dir::ReceiverKind::This,
            ty: receiver.ty,
        });
        let term = TypeTerm::Receiver(receiver);
        let condition = self.active_static_condition(module);

        self.add_type_definition(variable, term, condition);

        Some(variable)
    }

    /// Apply one resolved value reference.
    pub(in crate::check) fn use_value_symbol(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        self.capture_symbol_reference(source.module_id, symbol);
        self.select_name(source, symbol);
    }

    /// Check one value read against definite assignment flow.
    fn check_value_read_assigned(
        &mut self,
        source: dir::GlobalNodeIdAny,
        anchor: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        if symbol.module_id != source.module_id {
            return;
        }
        let bindings = self.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);
        if binding.binding_mutability.is_none() {
            return;
        }
        if self.flow(source.module_id).is_assigned(symbol) {
            return;
        }

        self.report_use_before_assigned(source.module_id, anchor);
    }
}
