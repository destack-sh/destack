use destack_dir as dir;

use crate::check::{
    CheckState, ConstraintOrigin, ExportLookup, GenericArgument, Obligation, ReceiverTerm,
    StaticTerm, TypeOperationTerm, TypeTerm, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Resolve one identifier reference term.
    pub(in crate::check) fn resolve_identifier_expression(
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

        self.record_value_reference(source, symbol);

        Some(self.value_reference_term(tree, id, symbol, &[]))
    }

    /// Resolve one path reference term.
    pub(in crate::check) fn resolve_reference_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let symbol =
            self.require_path_symbol(id.into_any(), path, tree, dir::SymbolSpace::Value)?;
        let source = id.into_global_any(tree.module_id);

        self.record_value_reference(source, symbol);

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
            return TypeTerm::Variable(narrowed);
        }

        // explicit generic references instantiate the value symbol
        if generic_arguments.is_empty() {
            return TypeTerm::Variable(self.intern_symbol_type_variable(tree.module_id, symbol));
        }

        let arguments = self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
        let source = id.into_global_any(tree.module_id);

        TypeTerm::Reference {
            source: Some(source),
            symbol,
            arguments,
        }
    }

    /// Require a symbol named by one source path.
    pub(in crate::check) fn require_path_symbol(
        &mut self,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        tree: &dir::Tree,
        space: dir::SymbolSpace,
    ) -> Option<dir::GlobalSymbolId> {
        match self.resolve_path_symbol(tree.module_id, source, path, space) {
            Ok(ExportLookup::Found(symbol)) => Some(symbol),
            Ok(ExportLookup::Missing) => {
                self.report_unresolved_reference(tree.module_id, source, path);

                None
            }
            Ok(ExportLookup::Ambiguous(_)) => {
                self.report_ambiguous_reference(tree.module_id, source, path);

                None
            }
            Err(error) => {
                self.report_internal(tree.module_id, source, format!("{error:?}"));

                None
            }
        }
    }

    /// Return the directly named callee symbol, when one is visible.
    pub(in crate::check) fn direct_callee_symbol(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = match tree.get(left) {
            // f()
            dir::Expression::Identifier { name } => self
                .lookup_symbol_by_name(
                    tree.module_id,
                    left.into_any(),
                    *name,
                    dir::SymbolSpace::Value,
                )
                .unique_symbol(),
            // ns.f()
            dir::Expression::QualifiedReference { path, .. } => {
                match self.resolve_path_symbol(
                    tree.module_id,
                    left.into_any(),
                    path,
                    dir::SymbolSpace::Value,
                ) {
                    Ok(ExportLookup::Found(symbol)) => Some(symbol),
                    Ok(ExportLookup::Missing | ExportLookup::Ambiguous(_)) | Err(_) => None,
                }
            }
            // dynamic callee
            _ => None,
        };

        symbol
    }

    /// Resolve one reference type expression term.
    pub(in crate::check) fn resolve_reference_type_expression(
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
            self.record_name_resolution(source, symbol);

            let slot_id = self.generic_slot_for_symbol(symbol)?;

            return Some(TypeTerm::Parameter(slot_id));
        }

        let symbol = self.require_path_symbol(id.into_any(), path, tree, dir::SymbolSpace::Type)?;

        self.record_name_resolution(source, symbol);

        // direct generic parameter references are local type variables
        if generic_arguments.is_empty()
            && self.symbol_kind(tree.module_id, symbol)
                == Some(dir::SymbolKind::GenericTypeParameter)
        {
            let variable = self.intern_symbol_type_variable(tree.module_id, symbol);

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
                let origin = ConstraintOrigin::Node(source);
                let variable = self.allocate_intermediate_variable(
                    tree.module_id,
                    VariableKind::Static,
                    origin,
                );
                let term = StaticTerm::Intrinsic {
                    item,
                    arguments: arguments.into(),
                };

                self.define_static(tree.module_id, variable, term);

                return Some(TypeTerm::StaticValue { value: variable });
            }
        }

        Some(TypeTerm::Reference {
            source: Some(source),
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
        let origin = ConstraintOrigin::Node(source);
        let module = source.module_id;
        let variable = self.allocate_intermediate_variable(module, VariableKind::Type, origin);
        let receiver = self.terms.push(ReceiverTerm {
            source,
            kind: dir::ReceiverKind::This,
            ty: receiver.ty,
        });
        let term = TypeTerm::Receiver(receiver);

        self.define_type(module, variable, term);

        Some(variable)
    }

    /// Record one resolved value reference side effect.
    pub(in crate::check) fn record_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        self.capture_symbol_reference(source.module_id, symbol);
        self.record_name_resolution(source, symbol);
    }
}
