use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::check::{
    BodyState, Cause, CauseKind, CheckOutcome, Expectation, FlowSite, PlaceUse, Relation, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one expression node.
    pub(in crate::check) fn infer_expression(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        match expression {
            dir::Expression::Identifier { name } => {
                // reuse a committed resolution, or resolve the reference now
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
                if self.check.committed_node_type(node.into_any()).is_none()
                    && let [symbol] = resolution.symbols()
                {
                    // replay a frameless read without re-checking it
                    let bindings = self.check.binding_table(symbol.module_id);
                    let is_module_binding =
                        bindings.get_symbol(symbol.local_id).scope.id == bindings.module_scope().id;
                    if is_module_binding || self.check.flow.current_function().is_some() {
                        self.check
                            .check_assigned_read(node.local_id.into_any(), *symbol);
                    }
                }

                self.infer_name_expression(site, &resolution)
            }
            dir::Expression::Block(block) => self.infer_block(site, block),
            dir::Expression::Comptime { body } => self.infer_transparent_expression(site, body),
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => self.infer_borrow_expression(site, mutability, right),
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                self.check_condition_operands(node.module_id, &condition)?;

                self.infer_if_expression(site, &condition, then_expression, else_expression)
            }
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => self.infer_try_expression(site, body, catch, finally),
            dir::Expression::ScalarLiteral(value) => {
                let ty = self.scalar_literal_type(node, value)?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::TemplateExpression { value } => {
                let ty = self.template_expression_type(site, value)?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::ArrayExpression { elements } => {
                let ty = self.infer_array_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::FixedArrayExpression { value, length } => {
                // type the written length at its first visit
                let count = self.walk_body_static_term(node.module_id, length)?;

                self.infer_fixed_array_expression(site, value, count, mode)
            }
            dir::Expression::TupleExpression { elements } => {
                let ty = self.infer_tuple_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::Match { value, arms } => self.infer_match_expression(
                site,
                value,
                &arms.into_iter().collect::<SmallVec<[_; 4]>>(),
            ),
            dir::Expression::Switch { value, cases } => self.infer_switch_statement(
                site,
                value,
                &cases.into_iter().collect::<SmallVec<[_; 4]>>(),
            ),
            dir::Expression::ForEach {
                label,
                operator,
                binding,
                iterator,
                body,
                ..
            } => self.infer_for_each_expression(site, label, operator, binding, iterator, body),
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
                    self.infer_name_expression(site, &resolution)
                }
                // decide a pre-resolved qualified reference at its first visit
                else if is_reference && let Some(name) = name {
                    self.decide_qualifier_segments(node.module_id, left)?;

                    match self.decide_name_reference(node, name)? {
                        Some(resolution) => self.infer_name_expression(site, &resolution),
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
                )?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(())
            }
            dir::Expression::StructExpression { ty, properties } => {
                let target = self.select_construct_target(site, ty, None)?;
                let check = self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(target),
                )?;
                let cause = self.check.intern_cause(Cause::root(
                    site.origin(),
                    CauseKind::Write {
                        place: node.into_any(),
                    },
                ));
                let expectation = Expectation {
                    target,
                    relation: Relation::Assignable,
                    cause,
                    use_: ValueUse::Store,
                    mode,
                };
                // complete the confirmed construction check at its authored value
                match check.outcome {
                    CheckOutcome::Holds => {
                        self.check_value(site, check.source, expectation)?;
                    }
                    CheckOutcome::Fails(failure) => {
                        self.record_failure(
                            cause,
                            Relation::Assignable,
                            Some(ValueUse::Store),
                            check.source,
                            target,
                            failure,
                        )?;
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
            } => self.select_binary_operator(site, operator, left, right, None),
            dir::Expression::Is { value, target_type } => {
                self.walk_body_guard_type_expression(node.module_id, target_type)?;

                self.select_type_predicate(site, value, target_type)
            }
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => {
                self.walk_body_type_expression(node.module_id, target_type)?;

                self.infer_satisfies_expression(site, expression, target_type)
            }
            dir::Expression::As {
                expression,
                target_type,
            } => {
                // const assertions carry no walkable target type
                let is_const = matches!(
                    self.module(node.module_id).view().get(target_type),
                    dir::TypeExpression::Const
                );
                if !is_const {
                    self.walk_body_type_expression(node.module_id, target_type)?;
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
            dir::Expression::New { ty, arguments } => {
                self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    None,
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
            } => self.select_instantiation(
                node,
                left,
                &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
            ),
            dir::Expression::TaggedTemplateExpression { tag, .. } => {
                self.select_tagged_template(site, tag)
            }
            dir::Expression::TreeExpression { .. } => {
                let _ = self.check_tree_expression(site, None)?;

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
                    .check
                    .flow
                    .current_function()
                    .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
                {
                    self.check.report_await_outside_async_context(
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
            | dir::Expression::Continue { .. }) => self.infer_statement(site, &expression),
            dir::Expression::Declaration(declaration) => {
                self.infer_declaration_statement(site, declaration)
            }
            // read the receiver visible at the current frame
            dir::Expression::This => {
                let receiver = self
                    .check
                    .commit_active_receiver_decision(node.into_any())?;
                match receiver {
                    Some(receiver) => {
                        self.check.commit_node_type(node.into_any(), receiver.ty)?;
                    }
                    None => {
                        self.check
                            .report_this_outside_receiver(node.module_id, node.local_id.into_any());
                        self.commit_error_node(node.into_any())?;
                    }
                }

                Ok(())
            }
            // read the receiver's declared heritage
            dir::Expression::Super => {
                let receiver = self
                    .check
                    .commit_active_receiver_decision(node.into_any())?;
                match receiver.and_then(|receiver| receiver.super_ty) {
                    Some(super_ty) => {
                        self.check.commit_node_type(node.into_any(), super_ty)?;
                    }
                    None => {
                        self.check
                            .report_super_outside_class(node.module_id, node.local_id.into_any());
                        self.commit_error_node(node.into_any())?;
                    }
                }

                Ok(())
            }
            // type module-form statements as void
            dir::Expression::Export { .. } | dir::Expression::Import { .. } => {
                let void = self.check.intern_type(dir::Type::Void)?;
                self.check.commit_node_type(node.into_any(), void)?;

                Ok(())
            }
            expression => self.reject_expression_without_inference_owner(node, expression),
        }
    }

    /// Return the type of one template expression after checking its arguments.
    pub(in crate::check) fn template_expression_type(
        &mut self,
        site: FlowSite,
        value: dir::TemplateLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value {
            for argument in arguments {
                self.infer_argument_type(site, argument)?;
            }
        }

        let ty = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;

        Ok(ty)
    }

    /// Reject expression inference that reached solve without a matching owner.
    fn reject_expression_without_inference_owner(
        &self,
        node: dir::GlobalNodeId<dir::Expression>,
        expression: dir::Expression,
    ) -> CompilerResult<()> {
        Err(CompilerError::Internal {
            message: format!("cannot infer expression {node:?}: {expression:?}"),
        })
    }

    /// Infer one expression that resolved to one lexical symbol.
    fn infer_name_expression(
        &mut self,
        site: FlowSite,
        resolution: &dir::NameResolution,
    ) -> CompilerResult<()> {
        let [symbol] = resolution.symbols() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "name expression at {:?} resolved to {} symbols",
                    site.node,
                    resolution.symbols().len(),
                ),
            });
        };

        // report foreign value reads while declaring
        if self.is_declaration() && !self.is_own_module(symbol.module_id) {
            self.report_export_type_not_derivable(site.node.module_id, site.node.local_id);
            // record the runtime access path while checking
            if self
                .symbol_kind_maybe(*symbol)?
                .is_some_and(dir::SymbolKind::is_binding)
            {
                self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
            }
            let ty = self.intern_type(dir::Type::Error)?;
            self.commit_node_type(site.node, ty)?;

            return Ok(());
        }

        // read a comptime value parameter as its carrier type
        if let Some(parameter) = self.check.parameter_by_symbol(*symbol)
            && let Some(binding) = self.check.generic_parameter(parameter)
            && matches!(binding.kind, dir::GenericParameterKind::Value)
            && let Some(carrier) = binding.constraint
        {
            self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
            let carrier = self.flow_type_at(site, carrier)?;
            self.commit_node_type(site.node, carrier)?;

            return Ok(());
        }

        // read identity statics as their declaration keys
        let identity = match self.static_value(*symbol) {
            Some(value) if matches!(self.ty(value)?, dir::Type::Key(_)) => Some(value),
            _ => None,
        };
        let ty = match identity {
            Some(value) => value,
            // type alias and class names as their written declaration reference
            None if matches!(
                self.symbol_kind_maybe(*symbol)?,
                Some(dir::SymbolKind::TypeAlias | dir::SymbolKind::Class)
            ) =>
            {
                self.intern_type(dir::Type::Reference(dir::TypeReference { symbol: *symbol }))?
            }
            None => self.symbol_type(*symbol)?,
        };

        // binding reads record their runtime access path
        if self
            .symbol_kind_maybe(*symbol)?
            .is_some_and(dir::SymbolKind::is_binding)
        {
            self.commit_access(site.node, dir::AccessPath::symbol(*symbol))?;
        }

        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(site.node, ty)?;

        Ok(())
    }

    /// Infer one expression whose type is exactly its child expression type.
    pub(in crate::check) fn infer_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let module = site.node.module_id;
        let child_site = self.visit_site(child.into_global_any(module))?;
        let ty = self.infer_node(child_site, PlaceUse::Read, InferMode::Exact)?;

        // commit the raw child type; the parent read narrows it
        self.commit_node_type(site.node, ty)?;

        Ok(())
    }
}

impl BodyState<'_, '_> {
    /// Decide the bound qualifier segments of one reference chain.
    pub(in crate::check) fn decide_qualifier_segments(
        &mut self,
        module: destack_source::ModuleId,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let mut current = Some(left);
        while let Some(segment) = current {
            // read the segment's own name and its next qualifier
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

            // decide only the segments a binding resolved
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
    pub(in crate::check) fn decide_reference(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        if let Some(resolution) = self.name_decision(node) {
            return Ok(Some(resolution.clone()));
        }

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
            _ => Ok(None),
        }
    }

    /// Decide one pre-resolved identifier reference under static presence.
    pub(in crate::check) fn decide_name_reference(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::NameResolution>> {
        let module = node.module_id;
        let source = node.into_any();
        let reference = self.module(module).resolved.references.get(source).cloned();

        match reference {
            // take one declaration or the callable overload set
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                let resolution = match symbols.as_slice() {
                    [symbol] => dir::NameResolution::new(*symbol),
                    _ => dir::NameResolution::from_symbols(symbols.to_vec()),
                };
                for symbol in resolution.symbols().iter().copied() {
                    self.check.capture_symbol_reference(symbol);
                }
                self.check.commit_name(source, resolution.clone())?;

                Ok(Some(resolution))
            }
            // report a conflicting lexical name
            Some(dir::Reference::Ambiguous(_)) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };
                self.check
                    .report_ambiguous_reference(module, source.local_id, &path);
                self.commit_error_node(source)?;

                Ok(None)
            }
            // report a missing name
            Some(dir::Reference::Missing) | None => {
                let path = self
                    .module(module)
                    .view()
                    .tree()
                    .reference_path(node.local_id)
                    .unwrap_or(dir::Path {
                        segments: smallvec::smallvec![name],
                    });
                self.check
                    .reject_unresolved_reference(module, source.local_id, &path);
                self.commit_error_node(source)?;

                Ok(None)
            }
            // reject namespaces and projections used directly as values
            Some(dir::Reference::Namespace { .. } | dir::Reference::Projected { .. }) => {
                let path = self
                    .module(module)
                    .view()
                    .tree()
                    .reference_path(node.local_id)
                    .unwrap_or(dir::Path {
                        segments: smallvec::smallvec![name],
                    });
                self.check
                    .reject_unresolved_reference(module, source.local_id, &path);
                self.commit_error_node(source)?;

                Ok(None)
            }
        }
    }
}
