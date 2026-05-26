use destack_dir as dir;

use crate::check::{
    CheckModuleState, ConstraintOrigin, StaticTerm, TypeOperationTerm, TypeTerm, VariableKind,
};

impl CheckModuleState {
    /// Resolve one identifier reference term.
    pub(in crate::check) fn resolve_identifier_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let symbol = self.require_name(id.into_any(), name, dir::SymbolSpace::Value)?;
        let source = id.into_global_any(self.input.module_id);

        // record the resolved value symbol
        self.capture_symbol_reference(symbol);
        self.record_name_resolution(source, symbol);

        // prefer active flow narrowing for direct value paths
        if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
            return Some(TypeTerm::Variable(narrowed));
        }

        Some(TypeTerm::Variable(self.intern_symbol_type_variable(symbol)))
    }

    /// Resolve one `this` receiver expression.
    pub(in crate::check) fn resolve_this_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<TypeTerm> {
        let source = id.into_global_any(self.input.module_id);
        let receiver = self.resolve_this_receiver(source)?;

        Some(TypeTerm::Variable(receiver.ty))
    }

    /// Resolve one qualified reference term.
    pub(in crate::check) fn resolve_reference_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let [name] = path.segments.as_slice() else {
            self.report_unresolved_reference(id.into_any(), path);

            return None;
        };
        let symbol = self.require_name(id.into_any(), *name, dir::SymbolSpace::Value)?;
        let source = id.into_global_any(self.input.module_id);

        // record the resolved value symbol
        self.capture_symbol_reference(symbol);
        self.record_name_resolution(source, symbol);

        // direct references can use flow narrowed types
        if generic_arguments.is_empty() {
            if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                return Some(TypeTerm::Variable(narrowed));
            }

            return Some(TypeTerm::Variable(self.intern_symbol_type_variable(symbol)));
        }

        // explicit generic references instantiate the value symbol
        let arguments = self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);

        Some(TypeTerm::Reference {
            source: Some(source),
            symbol,
            arguments,
        })
    }

    /// Resolve the symbol candidate named by one direct callee.
    pub(in crate::check) fn resolve_call_candidate_symbol(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<dir::GlobalSymbolId> {
        let dir::Expression::Identifier { name } = tree.get(left) else {
            return None;
        };
        let symbol = self
            .lookup_name(left.into_any(), *name, dir::SymbolSpace::Value)
            .unique_symbol()?;
        let source = left.into_global_any(self.input.module_id);

        self.record_name_resolution(source, symbol);

        Some(symbol)
    }

    /// Resolve one reference type expression term.
    pub(in crate::check) fn resolve_reference_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let source = id.into_global_any(self.input.module_id);

        // generic value parameters may appear as type expressions
        if generic_arguments.is_empty()
            && let [name] = path.segments.as_slice()
            && let Some(symbol) = self
                .lookup_name(id.into_any(), *name, dir::SymbolSpace::Value)
                .unique_symbol()
            && self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericValueParameter)
        {
            let slot_id = self.generic_slot_for_symbol(symbol)?;

            return Some(TypeTerm::Parameter(slot_id));
        }

        // normal type references must resolve in type space
        let [name] = path.segments.as_slice() else {
            self.report_unresolved_reference(id.into_any(), path);

            return None;
        };
        let symbol = self.require_name(id.into_any(), *name, dir::SymbolSpace::Type)?;

        // direct generic parameter references are local type variables
        if generic_arguments.is_empty()
            && self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericTypeParameter)
        {
            let variable = self.intern_symbol_type_variable(symbol);

            return Some(TypeTerm::Variable(variable));
        }

        self.record_name_resolution(source, symbol);

        let arguments = self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
        if let Some(item) = self.input.environment.language.item(symbol) {
            if Self::memory_type_intrinsic_item(item) {
                return Some(TypeTerm::Operation(TypeOperationTerm::Intrinsic {
                    item,
                    arguments: arguments.into(),
                }));
            }
            if Self::memory_static_intrinsic_item(item) {
                let origin = ConstraintOrigin::Node(source);
                let variable = self.allocate_anonymous_variable(VariableKind::Static, origin);
                let term = StaticTerm::Intrinsic {
                    item,
                    arguments: arguments.into(),
                };

                self.define_static(variable, term);

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
}
