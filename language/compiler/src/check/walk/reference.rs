use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Condition, GenericArgument, GenericInduction, GenericInductionParameter, NameLookup,
    Obligation, Origin, PathLookup, ReceiverTerm, TypeOperand, TypeOperationTerm, TypeRelation,
    TypeTerm, WalkState,
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
    ) -> CompilerResult<Option<TypeOperand>> {
        let guard = self.active_static_guard();
        let Some(symbol) = self.check.symbol_by_name_under(
            self.module,
            id.into_any(),
            name,
            dir::SymbolSpace::Value,
            &guard,
        ) else {
            return Ok(None);
        };
        let source = id.into_global_any(self.module);

        self.bind_value_read(source, id.into_any(), symbol)?;

        Ok(Some(self.walk_value_reference_operand(id, symbol, &[])?))
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
    ) -> CompilerResult<Option<TypeOperand>> {
        let guard = self.active_static_guard();
        let Some(symbol) = self.check.symbol_by_path_under(
            self.module,
            id.into_any(),
            path,
            dir::SymbolSpace::Value,
            &guard,
        ) else {
            return Ok(None);
        };
        let source = id.into_global_any(self.module);

        self.bind_value_read(source, id.into_any(), symbol)?;

        Ok(Some(self.walk_value_reference_operand(
            id,
            symbol,
            generic_arguments,
        )?))
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
    ) -> CompilerResult<bool> {
        // skip ordinary runtime member expressions
        if !self
            .check
            .path_has_namespace_root(self.module, id.into_any())
        {
            return Ok(false);
        }

        // resolve the full namespace path
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_path(self.module, id.into_any(), path, dir::SymbolSpace::Value)
            .available_under(&guard);

        match lookup {
            // bind a resolved value symbol
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);

                    return Ok(true);
                };
                let source = id.into_global_any(self.module);
                let operand = self.walk_value_reference_operand(id, symbol, &[])?;

                self.bind_value_read(source, id.into_any(), symbol)?;
                self.bind_node_type_operand(id, operand)?;
            }
            // report missing namespace member
            PathLookup::Missing => {
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), path);
            }
            // report ambiguous namespace member
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);
            }
        }

        Ok(true)
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
    ) -> CompilerResult<Option<TypeOperand>> {
        match self.tree.get(id) {
            // value.member(), T.member()
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec::smallvec![*name],
                };

                self.bind_reference_receiver_operand(id, &path, &[])
            }
            // namespace.value.member(), Box<T>.member()
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => self.bind_reference_receiver_operand(id, path, generic_arguments),
            // expression.member()
            _ => {
                self.walk_expression(id, self.tree.get(id))?;

                let source = id.into_global_any(self.module);
                let operand = self.check.node_type_operand(source)?;

                Ok(Some(operand))
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
    ) -> CompilerResult<Option<TypeOperand>> {
        let guard = self.active_static_guard();
        let source = id.into_global_any(self.module);
        let lookup = self
            .check
            .lookup_path(self.module, id.into_any(), path, dir::SymbolSpace::Value)
            .available_under(&guard);

        match lookup {
            // bind ordinary value receiver
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);

                    return Ok(None);
                };
                if self.check.symbol_kind(symbol).is_nominal() {
                    return self.bind_type_receiver_path_operand(id, path, generic_arguments);
                }
                let operand = self.walk_value_reference_operand(id, symbol, generic_arguments)?;

                self.bind_value_read(source, id.into_any(), symbol)?;
                self.bind_node_type_operand(id, operand)?;

                Ok(Some(operand))
            }
            // bind type receiver when no value receiver exists
            PathLookup::Missing => {
                self.bind_type_receiver_path_operand(id, path, generic_arguments)
            }
            // report ambiguous value receiver
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                Ok(None)
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
    ) -> CompilerResult<Option<TypeOperand>> {
        let source = id.into_global_any(self.module);
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_path(self.module, id.into_any(), path, dir::SymbolSpace::Type)
            .available_under(&guard);

        // no type receiver exists
        let symbol = match lookup {
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            PathLookup::Missing => return Ok(None),
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                return Ok(None);
            }
        };

        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))?;

        let parameter = self.check.inference.symbol_generic_parameter(symbol);
        let is_type_parameter =
            self.check.symbol_kind(symbol) == dir::SymbolKind::GenericTypeParameter;

        // return bare type parameter receiver
        let term = if generic_arguments.is_empty() && is_type_parameter {
            let Some(parameter) = parameter else {
                return Ok(None);
            };

            TypeTerm::Parameter(parameter)
        } else {
            let arguments = self.walk_generic_arguments(generic_arguments)?;

            self.type_symbol_reference_term(source, symbol, arguments)
        };

        Ok(Some(self.bind_node_type(id, term)?))
    }

    /// Walk one resolved value reference and return its operand.
    ///
    /// Example:
    /// ```ds
    /// value<T>
    /// ```
    fn walk_value_reference_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<TypeOperand> {
        let source = id.into_global_any(self.module);

        // return direct symbol type for bare references
        let operand = if generic_arguments.is_empty() {
            self.value_symbol_operand(id, symbol)?
        }
        // instantiate explicit generic references
        else {
            let arguments = self.walk_generic_arguments(generic_arguments)?;

            self.check
                .inference
                .push_term(TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments: arguments.into_vec(),
                })
                .into()
        };

        Ok(operand)
    }

    /// Return the operand for one bare value symbol.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn value_symbol_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(narrowed) = self.flow_path_narrowing(id) {
            return Ok(narrowed);
        }

        self.symbol_type_operand(symbol)
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
    ) -> CompilerResult<Option<TypeTerm>> {
        let source = id.into_global_any(self.module);

        // bind bare static parameter
        let bare_static_parameter = if generic_arguments.is_empty() {
            self.bind_bare_static_parameter_term(id, path)?
        } else {
            None
        };
        let term = if let Some(term) = bare_static_parameter {
            term
        }
        // bind type symbol reference
        else {
            let guard = self.active_static_guard();
            let Some(symbol) = self.check.symbol_by_path_under(
                self.module,
                id.into_any(),
                path,
                dir::SymbolSpace::Type,
                &guard,
            ) else {
                return Ok(None);
            };
            self.check
                .inference
                .select_name(source, dir::NameResolution::new(symbol))?;

            let parameter = self.check.inference.symbol_generic_parameter(symbol);
            let is_type_parameter =
                self.check.symbol_kind(symbol) == dir::SymbolKind::GenericTypeParameter;

            // return bare type parameter
            if generic_arguments.is_empty() && is_type_parameter {
                let Some(parameter) = parameter else {
                    return Ok(None);
                };

                TypeTerm::Parameter(parameter)
            } else {
                let arguments = self.walk_generic_arguments(generic_arguments)?;

                self.type_symbol_reference_term(source, symbol, arguments)
            }
        };

        Ok(Some(term))
    }

    /// Return the term for one selected type symbol reference.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn type_symbol_reference_term(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> TypeTerm {
        let item = self.check.environment.language.item(symbol);
        if let Some(item) = item {
            self.constrain_language_item_type_reference(source, item, &arguments);
        }

        // return language item reference
        if let Some(item) = item
            && let Some(term) = self.language_item_type_term(item, &arguments)
        {
            return term;
        }

        // return nominal or structural reference term
        TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: arguments.into_vec(),
        }
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

        let induction = GenericInduction::new(
            variable,
            GenericInductionParameter::r#type(
                "T",
                Some(operand),
                dir::GenericParameterInduction::Constraint,
            ),
        );

        self.check.inference.insert_generic_induction(induction);
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

    /// Return one language item type term.
    ///
    /// Example:
    /// ```ds
    /// LifetimeOf<T>
    /// ```
    fn language_item_type_term(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[GenericArgument],
    ) -> Option<TypeTerm> {
        // build memory intrinsic
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
        let Some(constraint) = dynamic_safe_constraint(item, arguments) else {
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
    ) -> CompilerResult<Option<TypeTerm>> {
        let [name] = path.segments.as_slice() else {
            return Ok(None);
        };

        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_name_by_name(self.module, id.into_any(), *name, dir::SymbolSpace::Value)
            .available_under(&guard);
        let symbol = match lookup {
            NameLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            NameLookup::Missing => return Ok(None),
            NameLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                return Ok(None);
            }
        };

        if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericValueParameter {
            return Ok(None);
        }

        let source = id.into_global_any(self.module);
        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))?;

        let Some(parameter_id) = self.check.inference.symbol_generic_parameter(symbol) else {
            return Ok(None);
        };

        Ok(Some(TypeTerm::Parameter(parameter_id)))
    }

    /// Return the contextual receiver type term.
    ///
    /// Example:
    /// ```ds
    /// this
    /// ```
    pub(in crate::check) fn this_receiver_type_term(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(receiver) = self.resolve_this_receiver(source)? else {
            return Ok(None);
        };
        let receiver = self.check.inference.push_term(ReceiverTerm {
            source,
            kind: dir::ReceiverKind::This,
            ty: receiver.ty,
        });

        Ok(Some(TypeTerm::Receiver(receiver)))
    }

    /// Bind one resolved value name.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn bind_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))
    }

    /// Bind one resolved value read.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn bind_value_read(
        &mut self,
        source: dir::GlobalNodeIdAny,
        anchor: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        self.capture_symbol_reference(symbol);
        self.bind_value_reference(source, symbol)?;

        // validate active module bindings
        if symbol.module_id == source.module_id {
            self.check_local_binding_read_assigned(anchor, symbol);
        }

        Ok(())
    }

    /// Report one local binding read that is not definitely assigned.
    fn check_local_binding_read_assigned(
        &mut self,
        anchor: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        // read local binding metadata
        let bindings = self.check.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);

        // report local bindings that have not been definitely assigned
        let is_local_binding = binding.binding_mutability.is_some();
        if is_local_binding && !self.flow().is_assigned(symbol) {
            self.check
                .report_use_before_assigned(symbol.module_id, anchor);
        }
    }
}

/// Return the dynamic safe constraint for one language item reference.
///
/// Example:
/// ```ds
/// Dynamic<T>
/// ```
fn dynamic_safe_constraint(
    item: dir::LanguageItem,
    arguments: &[GenericArgument],
) -> Option<TypeOperand> {
    if item != dir::LanguageItem::Dynamic {
        return None;
    }

    arguments.first().and_then(GenericArgument::type_operand)
}
