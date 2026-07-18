use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::check::{
    Answer, BodyState, Cause, CauseKind, ConstructResult, Decision, Expectation, FlowSite,
    PlaceUse, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one expression node.
    pub(in crate::check) fn infer_expression(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        if self.committed_node_type(node.into_any()).is_some() {
            return Ok(Answer::Ready(()));
        }

        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        match expression {
            dir::Expression::Identifier { .. } => {
                // name references decide during the walk
                let resolution = self
                    .resolutions(node.module_id)
                    .name_resolution(node.into_any())
                    .cloned();
                let Some(resolution) = resolution else {
                    return Err(CompilerError::Internal {
                        message: format!("identifier {node:?} has no name resolution"),
                    });
                };

                self.infer_name_expression(site, &resolution)
            }
            dir::Expression::Label { body, .. } => {
                // labeled blocks own their output; labeled loops forward transparently
                if let Some(result) = self.check.control_results.get(&node.into_any()).copied() {
                    let body_site = self.node_site(body.into_global_any(node.module_id))?;
                    let expectation = Expectation::assignable(
                        result,
                        self.check.intern_cause(Cause::root(
                            body_site.origin(),
                            CauseKind::Return { annotation: None },
                        )),
                        ValueUse::Output,
                    );
                    answer!(self.attempt_node(body_site, PlaceUse::Read, Some(expectation))?);
                    self.commit_node_type(node.into_any(), result)?;

                    return Ok(Answer::Ready(()));
                }

                self.infer_transparent_expression(site, body)
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
                answer!(self.check_condition_operands(node.module_id, &condition)?);

                self.infer_if_expression(site, then_expression, else_expression)
            }
            dir::Expression::Try { body, catch, .. } => {
                self.infer_try_expression(site, body, catch)
            }
            dir::Expression::ScalarLiteral(value) => {
                let ty = self.scalar_literal_type(node, value)?;
                let ty = if mode == InferMode::Widen {
                    self.widen_type(ty)?
                } else {
                    ty
                };
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::TemplateExpression { value } => {
                let ty = answer!(self.template_expression_type(site, value)?);
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::ArrayExpression { elements } => {
                let ty = answer!(self.infer_array_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                )?);
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::FixedArrayExpression { value, length } => {
                let count = answer!(self.node_type(length.into_global_any(node.module_id))?);

                self.infer_fixed_array_expression(site, value, count)
            }
            dir::Expression::TupleExpression { elements } => {
                let ty = answer!(self.infer_tuple_expression(
                    site,
                    &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                )?);
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
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
                operator,
                binding,
                iterator,
                body,
                ..
            } => self.infer_for_each_expression(site, operator, binding, iterator, body),
            dir::Expression::Member {
                left,
                name,
                is_optional,
            } => {
                let resolution = self
                    .resolutions(node.module_id)
                    .name_resolution(node.into_any())
                    .cloned();
                if let Some(resolution) = resolution {
                    self.infer_name_expression(site, &resolution)
                } else {
                    self.select_member(site, left, name, is_optional)
                }
            }
            dir::Expression::ObjectExpression { properties } => {
                let ty = answer!(self.infer_object_expression(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    mode,
                )?);
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::StructExpression { ty, properties } => {
                let target = answer!(self.select_construct_target(site, ty, None)?);
                answer!(self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(target),
                )?);

                Ok(Answer::Ready(()))
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                answer!(self.select_call(
                    site,
                    left,
                    &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    None,
                )?);

                Ok(Answer::Ready(()))
            }
            dir::Expression::Infer { .. } => {
                self.report_cannot_infer_node(node.into_any())?;
                self.commit_error_node(node.into_any())?;

                Ok(Answer::Ready(()))
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
                self.select_type_predicate(site, value, target_type)
            }
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => self.infer_satisfies_expression(site, expression, target_type),
            dir::Expression::As {
                expression,
                target_type,
            } => self.infer_as_expression(site, expression, target_type),
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
            dir::Expression::New { ty, arguments } => self.select_construct(
                site,
                ty,
                &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                ConstructResult::Direct,
                None,
            ),
            dir::Expression::NewMaybe { ty, arguments } => {
                let () = answer!(self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    ConstructResult::Fallible,
                    None,
                )?);
                let value = answer!(self.check.node_type(node.into_any())?);
                self.propagate_try_residual(node.into_any(), value, site)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::Index { left, index, .. } => {
                self.select_index(site, left, index, use_)
            }
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
                self.report_missing_tree_builder(node.module_id, node.local_id.into_any());
                self.commit_decision(node.into_any(), Decision::Rejected)?;
                self.commit_error_node(node.into_any())?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => self.infer_assignment_expression(site, left, operator, right),
            dir::Expression::Chain { expression } => self.infer_chain_expression(site, expression),
            dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
            | dir::Expression::AwaitMaybe { expression: left }
            | dir::Expression::AwaitMust { expression: left } => {
                self.infer_try_projection_expression(site, left)
            }
            dir::Expression::Await { expression } => self.infer_await_expression(site, expression),
            dir::Expression::Missing | dir::Expression::Error => {
                self.commit_error_node(node.into_any())?;

                Ok(Answer::Ready(()))
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
            | dir::Expression::Throw { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }) => self.infer_statement(site, &expression),
            expression => self.reject_expression_without_inference_owner(node, expression),
        }
    }

    /// Return the type of one template expression after checking its arguments.
    pub(in crate::check) fn template_expression_type(
        &mut self,
        site: FlowSite,
        value: dir::TemplateLiteral,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value {
            for argument in arguments {
                answer!(self.infer_argument_type(site, argument)?);
            }
        }

        let ty = self.intern_type(
            site.node.module_id,
            dir::Type::Primitive(dir::PrimitiveType::String),
        )?;

        Ok(Answer::Ready(ty))
    }

    /// Reject expression inference that reached solve without a matching owner.
    fn reject_expression_without_inference_owner(
        &self,
        node: dir::GlobalNodeId<dir::Expression>,
        expression: dir::Expression,
    ) -> CompilerResult<Answer<()>> {
        Err(CompilerError::Internal {
            message: format!("cannot infer expression {node:?}: {expression:?}"),
        })
    }

    /// Infer one expression that resolved to one lexical symbol.
    fn infer_name_expression(
        &mut self,
        site: FlowSite,
        resolution: &dir::NameResolution,
    ) -> CompilerResult<Answer<()>> {
        let [symbol] = resolution.symbols() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "name expression at {:?} resolved to {} symbols",
                    site.node,
                    resolution.symbols().len(),
                ),
            });
        };

        let ty = match self.static_value(*symbol) {
            Some(value) => value,
            // alias and class names type as their written declaration reference
            None if matches!(
                self.symbol_kind(*symbol),
                dir::SymbolKind::TypeAlias | dir::SymbolKind::Class
            ) =>
            {
                self.intern_type(
                    site.node.module_id,
                    dir::Type::Reference(dir::TypeReference { symbol: *symbol }),
                )?
            }
            None => answer!(self.symbol_type(*symbol)?),
        };
        let ty = answer!(self.flow_type_at(site, ty)?);
        self.commit_node_type(site.node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one expression whose type is exactly its child expression type.
    pub(in crate::check) fn infer_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = site.node.module_id;
        let child_site = self.node_site(child.into_global_any(module))?;
        answer!(self.infer_node(child_site, PlaceUse::Read)?);
        // commit the raw child type: both nodes share one flow path,
        //  so the parent read overlays the narrowing itself
        let ty = answer!(self.node_type(child_site.node)?);
        self.commit_node_type(site.node, ty)?;

        Ok(Answer::Ready(()))
    }
}
