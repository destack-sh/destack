use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Condition, DumpContext, GenericArgument, GenericInductionParameter, GenericInductionPosition,
    MemberReceiver, NameLookup, Origin, PathLookup, ReceiverTerm, TypeOperand, TypeOperationTerm,
    TypeRelation, TypeTerm, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one identifier value reference and return its type operand.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn walk_identifier_reference_term(
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
        self.read_value_reference(source, id.into_any(), symbol)?;

        Ok(Some(self.walk_value_reference_operand(id, symbol, &[])?))
    }

    /// Walk one qualified value reference and return its type operand.
    ///
    /// Example:
    /// ```ds
    /// namespace.value<T>
    /// ```
    pub(in crate::check) fn walk_qualified_reference_term(
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
        self.read_value_reference(source, id.into_any(), symbol)?;

        Ok(Some(self.walk_value_reference_operand(
            id,
            symbol,
            generic_arguments,
        )?))
    }

    /// Walk one namespace path expression when the path root was resolved as a namespace.
    ///
    /// Example:
    /// ```ds
    /// dep.value
    /// ```
    pub(in crate::check) fn walk_namespace_path_reference_term(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<bool> {
        // skip runtime member expressions
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
            // use a resolved value symbol
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);

                    return Ok(true);
                };
                let source = id.into_global_any(self.module);
                let operand = self.walk_value_reference_operand(id, symbol, &[])?;
                self.read_value_reference(source, id.into_any(), symbol)?;
                self.constrain_node_type(id, operand)?;
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

    /// Walk one member call receiver.
    ///
    /// Example:
    /// ```ds
    /// T.default()
    /// ```
    pub(in crate::check) fn walk_member_call_receiver(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<MemberReceiver>> {
        match self.tree.get(id) {
            // value.member(), T.member()
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec::smallvec![*name],
                };
                self.walk_reference_receiver(id, &path, &[])
            }
            // namespace.value.member(), Box<T>.member()
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => self.walk_reference_receiver(id, path, generic_arguments),
            // expression.member()
            _ => {
                self.walk_expression(id, self.tree.get(id))?;

                let source = id.into_global_any(self.module);
                let operand = self.check.node_type_operand(source)?;

                Ok(Some(MemberReceiver::Value(operand)))
            }
        }
    }

    /// Walk one value or type receiver reference.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn walk_reference_receiver(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<MemberReceiver>> {
        let guard = self.active_static_guard();
        let source = id.into_global_any(self.module);
        let lookup = self
            .check
            .lookup_path(self.module, id.into_any(), path, dir::SymbolSpace::Value)
            .available_under(&guard);

        match lookup {
            // use value receiver
            PathLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);

                    return Ok(None);
                };
                if self.check.symbol_kind(symbol).is_nominal() {
                    return self.walk_type_receiver_path(id, path, generic_arguments);
                }
                let operand = self.walk_value_reference_operand(id, symbol, generic_arguments)?;
                self.read_value_reference(source, id.into_any(), symbol)?;
                self.constrain_node_type(id, operand)?;

                Ok(Some(MemberReceiver::Value(operand)))
            }
            // use type receiver when no value receiver exists
            PathLookup::Missing => self.walk_type_receiver_path(id, path, generic_arguments),
            // report ambiguous value receiver
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                Ok(None)
            }
        }
    }

    /// Walk one declaration receiver path.
    ///
    /// Example:
    /// ```ds
    /// Box<T>.new()
    /// ```
    fn walk_type_receiver_path(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<MemberReceiver>> {
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
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);

                    return Ok(None);
                };

                symbol
            }
            PathLookup::Missing => {
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), path);

                return Ok(None);
            }
            PathLookup::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                return Ok(None);
            }
        };
        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))?;

        let parameter = self.check.inference.generic_parameter_by_symbol(symbol);
        let is_type_parameter =
            self.check.symbol_kind(symbol) == dir::SymbolKind::GenericTypeParameter;

        // use single-name type parameter receiver
        if generic_arguments.is_empty() && is_type_parameter {
            let Some(parameter) = parameter else {
                return Ok(None);
            };
            let term = TypeTerm::Parameter(parameter);
            self.constrain_node_type_term(id, term)?;

            return Ok(Some(MemberReceiver::GenericParameter(parameter)));
        }

        // use declaration receiver
        let arguments = self.walk_selected_generic_arguments(symbol, generic_arguments)?;
        let term = TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: arguments.iter().copied().collect(),
        };
        self.constrain_node_type_term(id, term)?;

        Ok(Some(MemberReceiver::Declaration {
            origin: Origin::Node(source),
            symbol,
            arguments,
        }))
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

        // return direct symbol type for single-name references
        let operand = if generic_arguments.is_empty() {
            self.value_symbol_operand(id, symbol)?
        }
        // instantiate explicit generic references
        else {
            let arguments = self.walk_selected_generic_arguments(symbol, generic_arguments)?;
            self.check
                .inference
                .push_term(TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments,
                })
                .into()
        };

        // static generic value references carry a static operand too
        if generic_arguments.is_empty()
            && self.check.symbol_kind(symbol) == dir::SymbolKind::GenericValueParameter
        {
            let operand = self.check.symbol_static_operand(self.module, symbol)?;
            self.set_node_static(id, operand)?;
        }

        Ok(operand)
    }

    /// Return the operand for one single-name value symbol.
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

    /// Walk one reference type expression and return its type term.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    pub(in crate::check) fn walk_reference_type_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<TypeTerm>> {
        let source = id.into_global_any(self.module);

        // resolve value generic parameter in type position
        let value_parameter = if generic_arguments.is_empty() {
            self.resolve_value_parameter_type_term(id, path)?
        } else {
            None
        };
        let term = if let Some(term) = value_parameter {
            term
        }
        // resolve type symbol reference
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

            let parameter = self.check.inference.generic_parameter_by_symbol(symbol);
            let is_type_parameter =
                self.check.symbol_kind(symbol) == dir::SymbolKind::GenericTypeParameter;

            // return single-name type parameter
            if generic_arguments.is_empty() && is_type_parameter {
                let Some(parameter) = parameter else {
                    return Ok(None);
                };

                TypeTerm::Parameter(parameter)
            } else {
                let arguments = self.walk_generic_arguments(generic_arguments)?;
                self.type_symbol_reference_term(source, symbol, arguments)?
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
    ) -> CompilerResult<TypeTerm> {
        // return compiler intrinsic operation
        let item = self.check.environment.language.item(symbol);
        if let Some(item) = item
            && Self::is_memory_type_intrinsic(item)
        {
            return Ok(self.memory_intrinsic_type_operation(item, &arguments));
        }

        // return nominal or structural reference term
        let arguments = self.specialize_selected_generic_arguments(symbol, arguments)?;
        Ok(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments,
        })
    }

    /// Walk generic arguments after selecting their symbol.
    fn walk_selected_generic_arguments(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        let arguments = self.walk_generic_arguments(arguments)?;
        self.specialize_selected_generic_arguments(symbol, arguments)
    }

    /// Specialize generic arguments after selecting their symbol.
    fn specialize_selected_generic_arguments(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        if arguments.is_empty() {
            return Ok(arguments);
        }

        let Some(template) = self.check.inference.generic_template_by_symbol(symbol) else {
            let context = DumpContext::new(self.check).with_module(self.module);
            let symbol_label = context.symbol_label(symbol);
            let kind = self.check.symbol_kind(symbol);

            return Err(CompilerError::Internal {
                message: format!(
                    "generic arguments supplied for non-generic symbol {symbol_label} kind={kind:?}"
                ),
            });
        };
        self.check.specialize_generic_arguments(template, arguments)
    }

    /// Return an induced variable for one constraint type operand when needed.
    ///
    /// Example:
    /// ```ds
    /// writer: Writer
    /// ```
    pub(in crate::check) fn induce_constraint_type_operand(
        &mut self,
        source: dir::GlobalNodeIdAny,
        operand: TypeOperand,
        position: GenericInductionPosition,
        condition: Condition,
    ) -> CompilerResult<TypeOperand> {
        if !self.is_constraint_type_operand(operand) {
            return Ok(operand);
        }
        let variable = self
            .check
            .push_type_variable(source.module_id, Origin::Node(source));

        let induction = GenericInductionParameter::r#type("T", Some(operand), position.induction());
        self.check
            .inference
            .insert_generic_induction(variable, induction)?;
        self.check.constrain_type(
            Origin::Node(source),
            TypeRelation::Assignable,
            variable,
            operand,
            condition,
        );

        Ok(variable.into())
    }

    /// Return whether one operand induces a constrained type parameter.
    fn is_constraint_type_operand(&self, operand: TypeOperand) -> bool {
        match operand {
            TypeOperand::Term(term) => {
                self.is_constraint_type_term(self.check.inference.term(term))
            }
            TypeOperand::Variable(_) | TypeOperand::Type(_) => false,
        }
    }

    /// Return whether one type term induces a constrained type parameter.
    fn is_constraint_type_term(&self, term: &TypeTerm) -> bool {
        match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => arguments.is_empty() && self.is_constraint_type_symbol(*symbol),
            _ => false,
        }
    }

    /// Return whether one symbol names a constraint type.
    fn is_constraint_type_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        let kind = self.check.symbol_kind(symbol);

        matches!(
            kind,
            dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface,
        )
    }

    /// Return one memory intrinsic type operation.
    ///
    /// Example:
    /// ```ds
    /// LifetimeOf<T>
    /// ```
    fn memory_intrinsic_type_operation(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[GenericArgument],
    ) -> TypeTerm {
        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::Intrinsic {
                item,
                arguments: arguments.iter().cloned().collect(),
            });

        TypeTerm::Operation(operation)
    }

    /// Resolve one value generic parameter in type position.
    ///
    /// Example:
    /// ```ds
    /// Size
    /// ```
    fn resolve_value_parameter_type_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
    ) -> CompilerResult<Option<TypeTerm>> {
        // only single names can refer directly to value parameters
        let [name] = path.segments.as_slice() else {
            return Ok(None);
        };

        // resolve the name in value space under the current guard
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_name_by_name(self.module, id.into_any(), *name, dir::SymbolSpace::Value)
            .available_under(&guard);

        // reject ambiguous value names loudly
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

        // only generic value parameters can appear here
        if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericValueParameter {
            return Ok(None);
        }

        // record the resolved name
        let source = id.into_global_any(self.module);
        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))?;

        // return the generic parameter term
        let Some(parameter_id) = self.check.inference.generic_parameter_by_symbol(symbol) else {
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
            owner: receiver.owner,
            ty: receiver.ty,
        });

        Ok(Some(TypeTerm::Receiver(receiver)))
    }

    /// Select one resolved value name.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    pub(in crate::check) fn select_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        self.check
            .inference
            .select_name(source, dir::NameResolution::new(symbol))
    }

    /// Read one resolved value reference.
    ///
    /// Example:
    /// ```ds
    /// value
    /// ```
    fn read_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        anchor: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        self.capture_symbol_reference(symbol);
        self.select_value_reference(source, symbol)?;

        // validate active module bindings
        if symbol.module_id == source.module_id {
            self.report_unassigned_local_read(anchor, symbol);
        }

        Ok(())
    }

    /// Report one local binding read that is not definitely assigned.
    fn report_unassigned_local_read(
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
