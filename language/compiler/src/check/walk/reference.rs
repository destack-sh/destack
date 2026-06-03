use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Condition, GenericArgument, GenericParameterId, NameLookup, Obligation, Origin, PathLookup,
    ReceiverTerm, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Bind one identifier value reference and return its type operand.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn bind_identifier_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        let guard = self.active_static_guard();
        let symbol = self.check.symbol_by_name_under(
            tree.module_id,
            id.into_any(),
            name,
            dir::SymbolSpace::Value,
            &guard,
        )?;
        let source = id.into_global_any(tree.module_id);

        self.select_value_reference(source, symbol);
        self.check_value_read_assigned(source, id.into_any(), symbol);

        Some(self.walk_value_reference_operand(tree, id, symbol, &[]))
    }

    /// Bind one qualified value reference and return its type operand.
    ///
    /// Example:
    /// ```ds
    /// namespace.value<T>
    /// ```
    pub(in crate::check) fn bind_qualified_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        let guard = self.active_static_guard();
        let symbol = self.check.symbol_by_path_under(
            tree.module_id,
            id.into_any(),
            path,
            dir::SymbolSpace::Value,
            &guard,
        )?;
        let source = id.into_global_any(tree.module_id);

        self.select_value_reference(source, symbol);
        self.check_value_read_assigned(source, id.into_any(), symbol);

        Some(self.walk_value_reference_operand(tree, id, symbol, generic_arguments))
    }

    /// Bind one namespace path expression when the path root was resolved as a namespace.
    ///
    /// Example:
    /// ```ds
    /// dep.value
    /// ```
    pub(in crate::check) fn bind_namespace_path_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        tree: &dir::Tree,
    ) -> bool {
        // skip ordinary runtime member expressions
        if !self
            .check
            .path_has_namespace_root(tree.module_id, id.into_any())
        {
            return false;
        }

        // resolve the full namespace path
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_path(tree.module_id, id.into_any(), path, dir::SymbolSpace::Value)
            .available_under(&guard);

        match lookup {
            // bind a resolved value symbol
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(tree.module_id, id.into_any(), path);

                    return true;
                };
                let source = id.into_global_any(tree.module_id);
                let operand = self.walk_value_reference_operand(tree, id, symbol, &[]);

                self.select_value_reference(source, symbol);
                self.check_value_read_assigned(source, id.into_any(), symbol);
                self.bind_node_type_operand(id, operand);
            }
            // report missing namespace member
            PathLookup::Missing => {
                self.check
                    .report_unresolved_reference(tree.module_id, id.into_any(), path);
            }
            // report ambiguous namespace member
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(tree.module_id, id.into_any(), path);
            }
        }

        true
    }

    /// Bind one member call receiver and return its type operand.
    ///
    /// Example:
    /// ```ds
    /// T.default()
    /// ```
    pub(in crate::check) fn bind_member_call_receiver_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        match tree.get(id) {
            // value.member(), T.member()
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec::smallvec![*name],
                };

                self.bind_reference_receiver_operand(id, &path, &[], tree)
            }
            // namespace.value.member(), Box<T>.member()
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => self.bind_reference_receiver_operand(id, path, generic_arguments, tree),
            // expression.member()
            _ => {
                self.walk_expression(tree, id, tree.get(id));

                let source = id.into_global_any(self.module);
                let operand = self.check.node_type_operand(source);

                Some(operand)
            }
        }
    }

    /// Bind one value or type receiver reference.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn bind_reference_receiver_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        let guard = self.active_static_guard();
        let source = id.into_global_any(tree.module_id);
        let lookup = self
            .check
            .lookup_path(tree.module_id, id.into_any(), path, dir::SymbolSpace::Value)
            .available_under(&guard);

        match lookup {
            // bind ordinary value receiver
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(tree.module_id, id.into_any(), path);

                    return None;
                };
                let operand =
                    self.walk_value_reference_operand(tree, id, symbol, generic_arguments);

                self.select_value_reference(source, symbol);
                self.check_value_read_assigned(source, id.into_any(), symbol);
                self.bind_node_type_operand(id, operand);

                Some(operand)
            }
            // bind type receiver when no value receiver exists
            PathLookup::Missing => {
                self.bind_type_receiver_path_operand(id, path, generic_arguments, tree)
            }
            // report ambiguous value receiver
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(tree.module_id, id.into_any(), path);

                None
            }
        }
    }

    /// Bind one type receiver path from expression syntax.
    ///
    /// Example:
    /// ```ds
    /// Box<T>.new()
    /// ```
    fn bind_type_receiver_path_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        let source = id.into_global_any(tree.module_id);
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_path(tree.module_id, id.into_any(), path, dir::SymbolSpace::Type)
            .available_under(&guard);

        // no type receiver exists
        let symbol = match lookup {
            PathLookup::Found(candidate) => candidate.symbol()?,
            PathLookup::Missing => return None,
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(tree.module_id, id.into_any(), path);

                return None;
            }
        };

        self.check.select_name(source, symbol);

        // lower bare type parameter receiver
        let term = if generic_arguments.is_empty()
            && let Some(slot) = self.generic_type_parameter(symbol)
        {
            TypeTerm::Parameter(slot)
        }
        // lower nominal type receiver
        else {
            let arguments = self.walk_generic_arguments(Some(symbol), generic_arguments, tree);

            self.lower_type_symbol_reference_term(source, symbol, arguments)
        };

        Some(self.bind_node_type(id, term))
    }

    /// Walk one resolved value reference and return its operand.
    ///
    /// Example:
    /// ```ds
    /// value<T>
    /// ```
    fn walk_value_reference_operand(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> TypeOperand {
        let source = id.into_global_any(tree.module_id);

        // return direct symbol type for bare references
        let operand = if generic_arguments.is_empty() {
            self.value_symbol_operand(tree, id, symbol)
        }
        // instantiate explicit generic references
        else {
            let arguments = self.walk_generic_arguments(Some(symbol), generic_arguments, tree);
            self.check
                .inference
                .push_term(TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments: arguments.into_vec(),
                })
                .into()
        };

        operand
    }

    /// Return the operand for one bare value symbol.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn value_symbol_operand(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> TypeOperand {
        if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
            return narrowed;
        }

        self.allocate_symbol_type_operand(symbol)
    }

    /// Bind one reference type expression and return its type term.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    pub(in crate::check) fn bind_reference_type_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let source = id.into_global_any(tree.module_id);

        // bind bare static parameter
        let term = if generic_arguments.is_empty()
            && let Some(term) = self.bind_bare_static_parameter_term(id, path, tree)
        {
            term
        }
        // bind type symbol reference
        else {
            let guard = self.active_static_guard();
            let symbol = self.check.symbol_by_path_under(
                tree.module_id,
                id.into_any(),
                path,
                dir::SymbolSpace::Type,
                &guard,
            )?;
            self.check.select_name(source, symbol);

            // lower bare type parameter
            if generic_arguments.is_empty()
                && let Some(slot) = self.generic_type_parameter(symbol)
            {
                TypeTerm::Parameter(slot)
            } else {
                let arguments = self.walk_generic_arguments(Some(symbol), generic_arguments, tree);

                self.lower_type_symbol_reference_term(source, symbol, arguments)
            }
        };

        Some(term)
    }

    /// Lower the term for one selected type symbol reference.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn lower_type_symbol_reference_term(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> TypeTerm {
        let item = self.check.environment.language.item(symbol);
        if let Some(item) = item {
            self.constrain_language_item_type_reference(source, item, &arguments);
        }

        // lower language item reference
        let term = if let Some(item) = item
            && let Some(term) = self.lower_language_item_type_term(item, &arguments)
        {
            term
        }
        // return reference term
        else {
            TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments: arguments.into_vec(),
            }
        };

        term
    }

    /// Return an inducible variable for one transparent storage constraint.
    ///
    /// Example:
    /// ```ds
    /// writer: Writer
    /// ```
    pub(in crate::check) fn induce_transparent_type_operand(
        &mut self,
        source: dir::GlobalNodeIdAny,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        if !self.is_transparent_type_operand(operand) {
            return operand;
        }
        let variable = self
            .check
            .create_type_variable(source.module_id, Origin::Node(source));

        self.check.induce_type_generic(
            variable,
            "T",
            Some(operand),
            dir::GenericParameterInduction::Constraint,
        );
        self.check.relate_type(
            Origin::Node(source),
            TypeRelation::Assignable,
            variable,
            operand,
            condition,
        );

        variable.into()
    }

    /// Return whether one operand is a transparent type constraint.
    fn is_transparent_type_operand(&self, operand: TypeOperand) -> bool {
        match operand {
            TypeOperand::Term(term) => {
                self.is_transparent_type_term(self.check.inference.term(term))
            }
            TypeOperand::Variable(_) | TypeOperand::Type(_) => false,
        }
    }

    /// Return whether one type term is a transparent type constraint.
    fn is_transparent_type_term(&self, term: &TypeTerm) -> bool {
        match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments: _,
            } => self.is_transparent_type_symbol(*symbol),
            TypeTerm::Shape(_)
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::Operation(_) => true,
            _ => false,
        }
    }

    /// Return whether one symbol names a transparent type constraint.
    fn is_transparent_type_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        if let Some(item) = self.check.environment.language.item(symbol) {
            return Self::is_memory_type_intrinsic(item);
        }
        let kind = self.check.symbol_kind(symbol);

        matches!(
            kind,
            dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface
                | dir::SymbolKind::TypeAlias,
        )
    }

    /// Lower one language item type term.
    ///
    /// Example:
    /// ```ds
    /// LifetimeOf<T>
    /// ```
    fn lower_language_item_type_term(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[GenericArgument],
    ) -> Option<TypeTerm> {
        // lower memory intrinsic
        if Self::is_memory_type_intrinsic(item) {
            let operation = self
                .check
                .inference
                .push_term(TypeOperationTerm::Intrinsic {
                    item,
                    arguments: arguments.iter().cloned().collect(),
                });

            return Some(TypeTerm::Operation(operation));
        }

        None
    }

    /// Constrain obligations attached to one language item reference.
    ///
    /// Example:
    /// ```ds
    /// Dynamic<T>
    /// ```
    fn constrain_language_item_type_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        item: dir::LanguageItem,
        arguments: &[GenericArgument],
    ) {
        let Some(constraint) = lower_dynamic_safe_constraint(item, arguments) else {
            return;
        };
        let condition = self.active_static_guard();

        self.check.push_obligation(Obligation::DynamicSafe {
            source,
            constraint,
            condition,
        });
    }

    /// Bind one bare static parameter term.
    ///
    /// Example:
    /// ```ds
    /// Size
    /// ```
    fn bind_bare_static_parameter_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let [name] = path.segments.as_slice() else {
            return None;
        };

        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_name_by_name(
                tree.module_id,
                id.into_any(),
                *name,
                dir::SymbolSpace::Value,
            )
            .available_under(&guard);
        let symbol = match lookup {
            NameLookup::Found(candidate) => candidate.symbol()?,
            NameLookup::Missing => return None,
            NameLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(tree.module_id, id.into_any(), path);

                return None;
            }
        };

        if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericValueParameter {
            return None;
        }

        let source = id.into_global_any(tree.module_id);
        self.check.select_name(source, symbol);

        let parameter_id = self
            .check
            .inference
            .generic_parameter_id_for_symbol(symbol)?;

        Some(TypeTerm::Parameter(parameter_id))
    }

    /// Return the generic slot for one type parameter symbol.
    ///
    /// Example:
    /// ```ds
    /// T
    /// ```
    fn generic_type_parameter(&self, symbol: dir::GlobalSymbolId) -> Option<GenericParameterId> {
        if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericTypeParameter {
            return None;
        }

        self.check.inference.generic_parameter_id_for_symbol(symbol)
    }

    /// Lower the contextual receiver type term.
    ///
    /// Example:
    /// ```ds
    /// this
    /// ```
    pub(in crate::check) fn lower_this_receiver_type_term(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<TypeTerm> {
        let receiver = self.resolve_this_receiver(source)?;
        let receiver = self.check.inference.push_term(ReceiverTerm {
            source,
            kind: dir::ReceiverKind::This,
            ty: receiver.ty,
        });

        Some(TypeTerm::Receiver(receiver))
    }

    /// Select one resolved value reference.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn select_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        self.capture_symbol_reference(symbol);
        self.check.select_name(source, symbol);
    }

    /// Check one value read against definite assignment flow.
    pub(in crate::check) fn check_value_read_assigned(
        &mut self,
        source: dir::GlobalNodeIdAny,
        anchor: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        if symbol.module_id != source.module_id {
            return;
        }
        let bindings = self.check.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);
        if binding.binding_mutability.is_none() {
            return;
        }
        if self.flow().is_assigned(symbol) {
            return;
        }

        self.check
            .report_use_before_assigned(source.module_id, anchor);
    }
}

/// Return the dynamic safe constraint for one language item reference.
///
/// Example:
/// ```ds
/// Dynamic<T>
/// ```
fn lower_dynamic_safe_constraint(
    item: dir::LanguageItem,
    arguments: &[GenericArgument],
) -> Option<TypeOperand> {
    if item != dir::LanguageItem::Dynamic {
        return None;
    }

    arguments.first().and_then(GenericArgument::type_operand)
}
