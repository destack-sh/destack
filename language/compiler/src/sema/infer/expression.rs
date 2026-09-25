use smallvec::SmallVec;
use tspp_dir as dir;

use super::InferMode;
use crate::sema::{
    AssignedPlace, Cause, CauseKind, CheckOutcome, CheckState, ElisionSite, Expectation,
    FailedCheck, FlowSite, PlaceUse, Relation, StoreTarget, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer a declaration qualifier or runtime receiver and its flow type.
    pub(in crate::sema) fn infer_receiver(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<(dir::GlobalTypeId, dir::GlobalTypeId)> {
        // reuse a complete receiver type
        let receiver = if let Some(ty) = self.own_node_type(site.node) {
            ty
        } else {
            // apply explicit arguments to the selected qualifier
            let expression = self
                .module(site.node.module_id)
                .view()
                .get(site.node.into_typed::<dir::Expression>().local_id)
                .clone();
            if let dir::Expression::Instantiation {
                left,
                generic_arguments,
            } = expression
            {
                let operand = self.visit_site(left.into_global_any(site.node.module_id))?;
                let (_, target) = self.infer_receiver(operand)?;
                let target = self.infer_instantiation(
                    site.node.into_typed(),
                    operand,
                    target,
                    &generic_arguments,
                )?;
                self.commit_node_type(site.node, target)?;
                target
            } else {
                // distinguish associated member qualifiers from ordinary value expressions
                let qualifier = match self.decide_reference(site.node)? {
                    Some(resolution) => {
                        let mut is_qualifier = resolution.denoted_type().is_some();
                        for symbol in resolution.symbols() {
                            let kind = self.symbol_kind(*symbol)?;
                            is_qualifier |= !kind.is_value() || kind == dir::SymbolKind::Class;
                        }
                        is_qualifier.then_some(resolution)
                    }
                    None => None,
                };
                match qualifier {
                    Some(resolution) => {
                        self.infer_declaration(site, &resolution)?;
                        self.require_node_type(site.node)?
                    }
                    None => self.infer_node(site, PlaceUse::Read, InferMode::Regular)?,
                }
            }
        };

        // narrow runtime receivers at the member access
        let narrowed = self.flow_type_at(site, receiver)?;
        self.commit_expression_place(site, narrowed)?;

        Ok((receiver, narrowed))
    }

    /// Infer one expression node.
    pub(in crate::sema) fn infer_expression(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // read the expression standing at this site
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        // infer by the expression kind
        match expression {
            dir::Expression::Identifier { name } => {
                // reuse a committed resolution, else resolve the reference
                let resolution = match self
                    .resolutions(node.module_id)
                    .name_resolution(node.into_any())
                    .cloned()
                {
                    Some(resolution) => resolution,
                    None => match self.decide_name_reference(node, name)? {
                        Some(resolution) => resolution,
                        None => return Ok(()),
                    },
                };

                // require local bindings assigned before this first-visit read
                if self.committed_node_type(node.into_any()).is_none()
                    && let [symbol] = resolution.symbols()
                {
                    // read the binding scope out of the module under check
                    let is_module_binding = if self.is_own_module(symbol.module_id) {
                        let module = &self.module;
                        module.symbol(symbol.local_id).scope.id == module.bindings.module_scope().id
                    }
                    // otherwise read it from the imported module's binding table
                    else {
                        let bindings = self.binding_table(symbol.module_id)?;
                        bindings.get_symbol(symbol.local_id).scope.id == bindings.module_scope().id
                    };

                    // reuse a module read at any point, a local read inside its frame
                    if is_module_binding || self.flow.current_function().is_some() {
                        self.report_unassigned_read(node.local_id.into_any(), *symbol);
                    }
                }

                self.infer_name_expression(site, &resolution, context)
            }
            dir::Expression::Block(block) => self.infer_block(site, block),
            dir::Expression::Const { body } => self.infer_transparent_expression(site, body),
            dir::Expression::BorrowOf { access, right, .. } => {
                self.infer_borrow_expression(site, access, right)
            }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                self.check_condition_operands(node.module_id, &condition)?;

                self.infer_if_expression(
                    site,
                    &condition,
                    then_expression,
                    else_expression,
                    context,
                )
            }
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => self.infer_try_expression(site, body, catch, finally),
            dir::Expression::Literal(value) => {
                let ty = self.literal_type(value)?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            // type T
            dir::Expression::Type { value } => {
                self.walk_body_type_expression(node.module_id, value, ElisionSite::Body)?;
                let represented = self
                    .committed_node_type(value.into_global_any(node.module_id))
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("type expression {value:?} committed no type"),
                    })?;
                let reflected = self.language_type(dir::LanguageItem::Type, &[represented])?;
                self.commit_node_type(node.into_any(), reflected)?;

                Ok(())
            }
            dir::Expression::TemplateExpression { value } => {
                let ty =
                    self.template_expression_type(site, value, mode == InferMode::Const, context)?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::ArrayExpression { elements } => {
                let ty = self.infer_array_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                    context,
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::FixedArrayExpression { value, length } => {
                // type the written length at its first visit
                let count = self.walk_body_static_term(node.module_id, length)?;

                self.infer_fixed_array_expression(site, value, count, mode, context)
            }
            dir::Expression::TupleExpression { elements } => {
                let ty = self.infer_tuple_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                    context,
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::Match { value, arms } => self.infer_match_expression(
                site,
                value,
                &arms.into_iter().collect::<SmallVec<[_; 4]>>(),
                context,
            ),
            dir::Expression::Switch { value, cases } => self.infer_switch_statement(
                site,
                value,
                &cases.into_iter().collect::<SmallVec<[_; 4]>>(),
            ),
            dir::Expression::ForEach {
                label,
                asynchrony,
                binding,
                iterator,
                body,
            } => self.infer_for_each_expression(site, label, asynchrony, binding, iterator, body),
            dir::Expression::Member {
                left,
                name,
                is_optional,
            } => {
                let resolution = self
                    .resolutions(node.module_id)
                    .name_resolution(node.into_any())
                    .cloned();
                let is_reference = self
                    .module(node.module_id)
                    .resolved
                    .references
                    .get(node.into_any())
                    .is_some();

                // reuse a committed qualified resolution
                if let Some(resolution) = resolution {
                    self.infer_name_expression(site, &resolution, context)
                }
                // decide a pre-resolved qualified reference at its first visit
                else if is_reference && let Some(name) = name {
                    self.decide_qualifier_segments(node.module_id, left)?;

                    match self.decide_name_reference(node, name)? {
                        Some(resolution) => self.infer_name_expression(site, &resolution, context),
                        // unresolved references already reported
                        None => Ok(()),
                    }
                }
                // select ordinary members through their receiver
                else {
                    self.select_member(site, left, name, is_optional)
                }
            }
            dir::Expression::ObjectExpression { properties } => {
                let ty = self.infer_object_expression(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                    context,
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::StructExpression { ty, properties } => {
                // require an aggregate type
                let target = self.construct_type(site.origin(), site.node.module_id, ty, None)?;
                if self
                    .require_aggregate_construct_target(site.node, site.origin(), target)?
                    .is_some()
                {
                    return Ok(());
                }

                // infer the aggregate fields
                let check = self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(target),
                )?;
                let cause = self.intern_cause(Cause::root(
                    site.origin(),
                    CauseKind::Write {
                        place: node.into_any(),
                    },
                ));
                let expectation = Expectation {
                    target,
                    relation: Relation::Storable,
                    cause,
                    use_: ValueUse::Store,
                    mode,
                    store: StoreTarget::Exact,
                };

                // complete the confirmed construction check at its authored value
                match check.outcome {
                    CheckOutcome::Holds => {
                        self.check_value(site, check.source, expectation)?;
                    }
                    // leave a pending conversion to the queue
                    CheckOutcome::Pending => {}
                    CheckOutcome::Fails(failure) => {
                        self.push_failure(FailedCheck {
                            cause,
                            relation: Relation::Storable,
                            use_: Some(ValueUse::Store),
                            source: check.source,
                            target,
                            failure,
                        })?;
                    }
                }

                Ok(())
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                is_optional,
                ..
            } => {
                self.select_call(
                    site,
                    left,
                    &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    is_optional,
                    None,
                )?;

                Ok(())
            }
            dir::Expression::Infer { .. } => {
                self.report_cannot_infer_node(node.into_any())?;
                self.commit_error_node(node.into_any())?;

                Ok(())
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::In,
                right,
            } => self.select_member_predicate(site, left, right),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.infer_binary_expression(site, left, operator, right, context),
            dir::Expression::Is { value, target_type } => {
                self.walk_body_type_expression(node.module_id, target_type, ElisionSite::Body)?;

                self.select_type_predicate(site, value, target_type)
            }
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => {
                self.walk_body_type_expression(node.module_id, target_type, ElisionSite::Body)?;

                self.infer_satisfies_expression(site, expression, target_type)
            }
            dir::Expression::As {
                expression,
                target_type,
            } => {
                // skip const assertions, which name no target type
                let is_const = matches!(
                    self.module(node.module_id).view().get(target_type),
                    dir::TypeExpression::Const
                );
                if !is_const {
                    self.walk_body_type_expression(node.module_id, target_type, ElisionSite::Body)?;
                }

                self.infer_as_expression(site, expression, target_type)
            }
            dir::Expression::InstanceOf { value, target } => {
                self.select_class_predicate(site, value, target)
            }
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => self.infer_range_expression(site, start, end, end_kind),
            dir::Expression::Unary { operator, right } => {
                self.select_unary_operator(site, operator, right, use_)
            }
            dir::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                self.select_construct(
                    site,
                    left,
                    &generic_arguments,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    context,
                )?;

                Ok(())
            }
            dir::Expression::Index {
                left,
                index,
                is_optional,
                ..
            } => self.select_index(site, left, index, is_optional, use_),
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                // specialize the selected function or class constructor
                let operand = self.visit_site(left.into_global_any(node.module_id))?;
                let (_, target) = self.infer_receiver(operand)?;
                let target = self.infer_instantiation(node, operand, target, &generic_arguments)?;

                // require a value from the specialized declaration
                if let Some(resolution) = self.decide_reference(node.into_any())?
                    && !self.check_value_reference(node.into_any(), &resolution)?
                {
                    self.commit_error_node(site.node)?;

                    return Ok(());
                }

                // bind class constructors at the selected signature
                if let dir::Type::Reference(reference) = self.ty(target)?
                    && self.symbol_kind(reference.symbol)? == dir::SymbolKind::Class
                {
                    self.infer_constructor_value(site, target, context)
                } else {
                    self.commit_node_type(site.node, target)
                }
            }
            dir::Expression::TaggedTemplateExpression { tag, .. } => {
                self.select_tagged_template(site, tag)
            }
            dir::Expression::TreeExpression { .. } => {
                self.check_tree_expression(site, None)?;

                Ok(())
            }
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => self.infer_assignment_expression(site, left, operator, right),
            dir::Expression::Chain { expression } => self.infer_chain_expression(site, expression),
            dir::Expression::Maybe { left, .. } => {
                self.infer_try_projection_expression(site, left, true)
            }
            dir::Expression::Must { left, .. } => {
                self.infer_try_projection_expression(site, left, false)
            }
            dir::Expression::AwaitMaybe { expression: left }
            | dir::Expression::AwaitMust { expression: left } => {
                // require the enclosing body's asynchrony
                if self
                    .flow
                    .current_function()
                    .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
                {
                    self.report_await_outside_async_context(
                        node.module_id,
                        node.local_id.into_any(),
                    );
                }

                let propagates = matches!(expression, dir::Expression::AwaitMaybe { .. });

                self.infer_try_projection_expression(site, left, propagates)
            }
            dir::Expression::Await { expression } => self.infer_await_expression(site, expression),
            dir::Expression::Missing | dir::Expression::Error => {
                self.commit_error_node(node.into_any())?;

                Ok(())
            }
            expression @ (dir::Expression::Let { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::LetElse { .. }
            | dir::Expression::Debugger
            | dir::Expression::Return { .. }
            | dir::Expression::Yield { .. }
            | dir::Expression::While { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }) => self.infer_statement(site, &expression, mode),
            dir::Expression::Declaration(declaration) => {
                self.infer_declaration_statement(site, declaration)
            }
            // read the receiver visible at the current frame
            dir::Expression::This => {
                let receiver =
                    self.commit_active_receiver_decision(node.into_any(), dir::ReceiverKind::This)?;
                match receiver {
                    Some(receiver) => {
                        if let Some(owner) = self.enclosing_initializes()
                            && self.class_has_base(owner)?
                            && !self.flow.is_assigned(AssignedPlace::Delegated)
                        {
                            self.report_this_before_super(node.module_id, node.local_id.into_any());
                        }
                        self.commit_node_type(node.into_any(), receiver.ty)?;
                    }
                    None => {
                        self.report_this_outside_receiver(node.module_id, node.local_id.into_any());
                        self.commit_error_node(node.into_any())?;
                    }
                }

                Ok(())
            }
            // read the receiver's declared heritage
            dir::Expression::Super => {
                let receiver = self
                    .commit_active_receiver_decision(node.into_any(), dir::ReceiverKind::Super)?;
                match receiver.and_then(|receiver| receiver.super_ty) {
                    Some(super_ty) => {
                        self.commit_node_type(node.into_any(), super_ty)?;
                    }
                    None => {
                        self.report_super_outside_class(node.module_id, node.local_id.into_any());
                        self.commit_error_node(node.into_any())?;
                    }
                }

                Ok(())
            }
            // type the module namespace as the metadata its interface declares
            dir::Expression::ImportMeta => {
                let meta = self.language_type(dir::LanguageItem::ImportMeta, &[])?;
                self.commit_node_type(node.into_any(), meta)?;

                Ok(())
            }
            // type module-form statements as void
            dir::Expression::Export { .. } | dir::Expression::Import { .. } => {
                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(node.into_any(), void)?;

                Ok(())
            }
            expression => Err(CompilerError::Internal {
                message: format!("cannot infer expression {node:?}: {expression:?}"),
            }),
        }
    }

    /// Return the type of one template expression, a template literal where the context asks.
    pub(in crate::sema) fn template_expression_type(
        &mut self,
        site: FlowSite,
        value: dir::TemplateLiteral,
        keeps_template: bool,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the template's chunks and interpolations
        let string = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;
        let (chunks, arguments) = match value {
            dir::TemplateLiteral::String { chunk } => {
                return match chunk.cooked {
                    Some(text) => self.intern_type(dir::Type::Literal(dir::Literal::String(text))),
                    None => Ok(string),
                };
            }
            dir::TemplateLiteral::InterpolatedString { chunks, arguments } => (chunks, arguments),
        };

        // walk the interpolation decorators, keeping the statically present arguments
        let arguments = self.walk_body_arguments(site.node.module_id, &arguments)?;

        // infer each interpolated value in source order
        let mut spans = Vec::with_capacity(arguments.len());
        let mut rendered = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let span = self.infer_argument_type(site, argument)?;
            rendered.push((argument, span));
            spans.push(self.template_span_type(span, string)?);
        }

        // record the calls this template renders and joins through
        self.select_template_calls(site, &rendered, string)?;

        // print an unrequested template as a fresh string
        if !keeps_template {
            return self.contextual_form(string, context);
        }

        // keep the template text around the spans
        let mut strings = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let Some(text) = chunk.cooked else {
                return Ok(string);
            };
            strings.push(text);
        }
        let strings = self.intern_strings(&strings)?;
        let spans = self.intern_type_ids(&spans)?;
        let template = self.intern_operation(dir::TypeOperation::TemplateLiteral(
            dir::TemplateLiteralType { strings, spans },
        ))?;

        // concatenate the template once every span prints
        let template = self.normalize(site.origin(), template)?;

        self.contextual_form(template, context)
    }

    /// Return the type one interpolated value contributes to a template literal type.
    fn template_span_type(
        &mut self,
        span: dir::GlobalTypeId,
        string: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the printed value beneath ownership and access forms
        let mut value = self.shallow_resolve(span)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.shallow_resolve(form.value)?;
        }

        // print values outside the template span domain as plain strings
        let prints = matches!(
            self.ty(value)?,
            dir::Type::Literal(_)
                | dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Operation(_)
                | dir::Type::Parameter(_)
                | dir::Type::Variable(_)
                | dir::Type::Union(_)
                | dir::Type::Primitive(_)
        );

        Ok(if prints { value } else { string })
    }

    /// Return whether one expected type asks a template expression for its literal text.
    pub(in crate::sema) fn contextualizes_template(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // ask by the head of the target
        let target = self.shallow_resolve(target)?;
        let contextualizes = match self.ty(target)? {
            dir::Type::Literal(dir::Literal::String(_)) => true,
            dir::Type::Operation(operation) => matches!(
                self.type_operation(target.module_id, operation)?,
                dir::TypeOperation::TemplateLiteral(_)
            ),
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
                let mut any = false;
                for element in elements {
                    any |= self.contextualizes_template(element)?;
                }

                any
            }
            _ => false,
        };

        Ok(contextualizes)
    }

    /// Infer one name expression by its resolution.
    pub(in crate::sema) fn infer_name_expression(
        &mut self,
        site: FlowSite,
        resolution: &dir::NameResolution,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // check the selected declaration's use before reading its type
        if !self.check_value_reference(site.node, resolution)? {
            self.commit_error_node(site.node)?;

            return Ok(());
        }

        // bind a class reference to its allocating function
        if let Some(symbol) = resolution.single_symbol()
            && self.symbol_kind(symbol)? == dir::SymbolKind::Class
        {
            let source = self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;

            return self.infer_constructor_value(site, source, context);
        }

        self.infer_declaration(site, resolution)
    }

    /// Infer the type selected by one declaration reference.
    pub(in crate::sema) fn infer_declaration(
        &mut self,
        site: FlowSite,
        resolution: &dir::NameResolution,
    ) -> CompilerResult<()> {
        // retain the selected type for an associated member qualifier
        let symbols = match resolution {
            dir::NameResolution::Type(denoted) => {
                return self.commit_node_type(site.node, *denoted);
            }
            dir::NameResolution::Symbols(symbols) => symbols.as_slice(),
        };

        // reject a plural declaration group referenced without a selecting call
        let [symbol] = symbols else {
            self.report_ambiguous_overload(site.node, symbols)?;
            let ty = self.intern_type(dir::Type::Error)?;
            self.commit_node_type(site.node, ty)?;

            return Ok(());
        };

        // require a written type for foreign values whose type needs another declaration
        if self.is_declaring()
            && !self.is_own_module(symbol.module_id)
            && self.symbol_kind(*symbol)?.is_value()
            && self.symbol_kind(*symbol)? != dir::SymbolKind::Class
        {
            self.report_export_type_not_derivable(site.node.module_id, site.node.local_id);

            // record the runtime access path while checking
            if self.symbol_kind(*symbol)?.is_binding() {
                self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
                self.commit_access_use(site.node, dir::BindingUse::READ);
            }
            let ty = self.intern_type(dir::Type::Error)?;
            self.commit_node_type(site.node, ty)?;

            return Ok(());
        }

        // read a const parameter as its representation type
        if let Some(parameter) = self.parameter_by_symbol(*symbol)?
            && let Some(binding) = self.generic_parameter(parameter)?
            && binding.is_const
            && binding.memory_parameter().is_none()
            && let Some(representation) = binding.constraint
        {
            self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
            self.commit_access_use(site.node, dir::BindingUse::READ);
            let representation = self.flow_type_at(site, representation)?;
            self.commit_node_type(site.node, representation)?;

            return Ok(());
        }

        // read identity statics as their declaration keys
        let identity = match self.static_value(*symbol)? {
            Some(value) if matches!(self.ty(value)?, dir::Type::Key(_)) => Some(value),
            _ => None,
        };
        let ty = match identity {
            Some(value) => value,
            // retain the declaration that selects static members and constructors
            None if matches!(
                self.symbol_kind(*symbol)?,
                dir::SymbolKind::TypeAlias | dir::SymbolKind::Class
            ) =>
            {
                self.intern_type(dir::Type::Reference(dir::TypeReference::new(*symbol)))?
            }
            None => self.symbol_type(*symbol)?,
        };

        // read a declared function as a callable handle
        let ty = if matches!(self.symbol_kind(*symbol)?, dir::SymbolKind::Function)
            && matches!(self.ty(ty)?, dir::Type::FunctionSignature(_))
        {
            self.function_type(ty)?
        } else {
            ty
        };

        // binding reads commit their runtime access path
        if self.symbol_kind(*symbol)?.is_binding() {
            self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
            self.commit_access_use(site.node, dir::BindingUse::READ);
        }

        // commit the selected function value for runtime consumers
        if matches!(self.symbol_kind(*symbol)?, dir::SymbolKind::Function) {
            let value = dir::FunctionValue {
                target: dir::CallableTarget::Symbol {
                    function: dir::FunctionTarget {
                        receiver: None,
                        generic_scope: None,
                        key: dir::InstanceKey::new(*symbol, Vec::new()),
                    },
                    dispatch: dir::FunctionDispatch::Direct,
                },
                callable_type: ty,
            };
            self.commit_decision(
                site.node,
                dir::Decision::Function(dir::OperationResolution::One(value)),
            )?;
        }

        // commit the narrowed value at this site
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(site.node, ty)?;

        Ok(())
    }

    /// Infer one expression whose type is exactly its child expression type.
    pub(in crate::sema) fn infer_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // infer the child value
        let module = site.node.module_id;
        let child_site = self.visit_site(child.into_global_any(module))?;
        let ty = self.infer_node(child_site, PlaceUse::Read, InferMode::Regular)?;

        // commit the raw child type, which the parent read narrows
        self.commit_node_type(site.node, ty)?;

        Ok(())
    }
}

impl CheckState<'_> {
    /// Require values from the selected declarations.
    pub(in crate::sema) fn check_value_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::NameResolution,
    ) -> CompilerResult<bool> {
        // reject primitive type names used as values
        if let dir::NameResolution::Type(denoted) = resolution {
            let name = self.format_type(*denoted);
            self.report_invalid_value_reference(source, name, None);

            return Ok(false);
        }

        // report a value read from a declaration without a value
        for symbol in resolution.symbols() {
            let kind = self.symbol_kind(*symbol)?;
            if !kind.is_value() {
                let name = self.format_symbol(*symbol);
                let help = match kind {
                    dir::SymbolKind::Struct => Some("construct structs with 'T { … }'"),
                    dir::SymbolKind::Newtype => Some("construct newtypes with 'T(…)'"),
                    _ => None,
                };
                self.report_invalid_value_reference(source, name, help);

                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide the bound qualifier segments of one reference chain.
    pub(in crate::sema) fn decide_qualifier_segments(
        &mut self,
        module: tspp_source::ModuleId,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // walk the qualifier chain outward
        let mut current = Some(left);
        while let Some(segment) = current {
            // read the segment name and its next qualifier
            let segment_node = segment.into_global(module);
            let expression = self.module(module).view().get(segment).clone();
            let next = match &expression {
                dir::Expression::Member { left, .. } => Some(*left),
                _ => None,
            };
            let segment_name = match &expression {
                dir::Expression::Identifier { name } => Some(*name),
                dir::Expression::Member { name, .. } => *name,
                _ => None,
            };

            // decide the segments a binding resolved
            let is_bound = matches!(
                self.module(module)
                    .resolved
                    .references
                    .get(segment_node.into_any()),
                Some(dir::Reference::Bound(_))
            );
            if is_bound
                && let Some(segment_name) = segment_name
                && self
                    .resolutions(module)
                    .name_resolution(segment_node.into_any())
                    .is_none()
            {
                self.decide_name_reference(segment_node, segment_name)?;
            }

            current = next;
        }

        Ok(())
    }

    /// Decide one reference node at its first visit, without reading it as a value.
    pub(in crate::sema) fn decide_reference(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        // reuse a committed resolution
        if let Some(resolution) = self.name_decision(node) {
            return Ok(Some(resolution.clone()));
        }

        // decide by the reference kind
        let module = node.module_id;
        let local = node.into_typed::<dir::Expression>().local_id;
        match self.module(module).view().get(local).clone() {
            dir::Expression::Identifier { name } => {
                self.decide_name_reference(node.into_typed(), name)
            }
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } if self.module(module).resolved.references.get(node).is_some() => {
                self.decide_qualifier_segments(module, left)?;
                self.decide_name_reference(node.into_typed(), name)
            }
            // leave every other expression undecided
            _ => Ok(None),
        }
    }

    /// Decide one pre-resolved identifier reference under static presence.
    pub(in crate::sema) fn decide_name_reference(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        // read the reference the resolver bound at this name
        let module = node.module_id;
        let source = node.into_any();
        let reference = self.module(module).resolved.references.get(source).cloned();

        // decide by the bound reference
        match reference {
            // take one declaration or the callable overload set
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.present_symbols(&symbols);
                let resolution = match symbols.as_slice() {
                    [symbol] => dir::NameResolution::new(*symbol),
                    _ => dir::NameResolution::from_symbols(symbols.to_vec()),
                };
                for symbol in resolution.symbols().iter().copied() {
                    self.capture_symbol_reference(source, symbol)?;
                }
                self.commit_name(source, resolution.clone())?;

                Ok(Some(resolution))
            }
            // report a conflicting lexical name
            Some(dir::Reference::Ambiguous(_)) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.report_ambiguous_reference(module, source.local_id, &path)?;
                self.commit_error_node(source)?;

                Ok(None)
            }
            // a type literal name resolves to its denoted type
            Some(dir::Reference::TypeLiteral(literal)) => {
                let denoted = self.intern_type(dir::Type::from(literal))?;
                let resolution = dir::NameResolution::new_type(denoted);
                self.commit_name(source, resolution.clone())?;

                Ok(Some(resolution))
            }

            // report a missing name, a namespace, or a projection used directly as a value
            Some(
                dir::Reference::Missing
                | dir::Reference::Namespace { .. }
                | dir::Reference::Projected { .. },
            )
            | None => {
                let path = self
                    .module(module)
                    .view()
                    .tree()
                    .reference_path(node.local_id)
                    .unwrap_or(dir::Path {
                        segments: smallvec::smallvec![name],
                    });
                self.report_unresolved_reference(module, source.local_id, &path)?;
                self.commit_error_node(source)?;

                Ok(None)
            }
        }
    }
}
