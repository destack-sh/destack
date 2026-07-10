use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::check::{
    Answer, CheckState, ConstructResult, Decision, Dependency, FlowSite, PlaceUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one expression node.
    pub(super) fn infer_expression(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        if self.node_type_maybe(node.into_any()).is_some() {
            return Ok(Answer::Ready(()));
        }

        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        match expression {
            dir::Expression::Identifier { .. } => {
                let Some(Decision::Name(resolution)) = self.decision(node.into_any()).cloned()
                else {
                    return Ok(Answer::pending([Dependency::Decision(node.into_any())]));
                };

                self.infer_name_expression(site, &resolution)
            }
            dir::Expression::Label { body, .. } => self.infer_transparent_expression(site, body),
            dir::Expression::Block(block) => self.infer_block(site, block),
            dir::Expression::Parenthesized { expression } if mode == InferMode::Const => {
                let expression_site = self.node_site(expression.into_global_any(node.module_id))?;
                answer!(self.infer_expression(expression_site, PlaceUse::Read, mode)?);
                let ty = answer!(self.node_type_at(expression_site)?);
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::Parenthesized { expression } => {
                self.infer_transparent_expression(site, expression)
            }
            dir::Expression::Comptime { body } => self.infer_transparent_expression(site, body),
            dir::Expression::MoveOf {
                mutability, right, ..
            } => self.infer_move_expression(site, mutability, right),
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => self.infer_borrow_expression(site, mutability, right),
            dir::Expression::SequenceExpression { expressions } => self.infer_sequence_expression(
                site,
                &expressions.into_iter().collect::<SmallVec<[_; 4]>>(),
            ),
            dir::Expression::If {
                then_expression,
                else_expression,
                ..
            } => self.infer_if_expression(site, then_expression, else_expression),
            dir::Expression::Try { body, catch, .. } => {
                self.infer_try_expression(site, body, catch)
            }
            dir::Expression::ScalarLiteral(value) => {
                let ty = self.scalar_literal_type(node, value)?;
                self.commit_node_type(node.into_any(), ty)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::TemplateExpression { value } => {
                if let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value {
                    for argument in arguments {
                        answer!(self.argument_value_type(site, argument)?);
                    }
                }

                let ty = self.intern_type(
                    node.module_id,
                    dir::Type::Primitive(dir::PrimitiveType::String),
                )?;
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
            dir::Expression::Match { cases, .. } => {
                self.infer_match_expression(site, &cases.into_iter().collect::<SmallVec<[_; 4]>>())
            }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                ..
            } => self.infer_for_each_expression(site, operator, binding, iterator),
            dir::Expression::Member { left, name }
            | dir::Expression::PrivateMember { left, name } => {
                if let Some(Decision::Name(resolution)) = self.decision(node.into_any()).cloned() {
                    self.infer_name_expression(site, &resolution)
                } else {
                    self.select_member(site, left, name)
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
                let target = answer!(self.node_type(ty.into_global_any(node.module_id))?);

                self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(target),
                )
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => self.select_call(
                site,
                left,
                &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
                &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
            ),
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
            ),
            dir::Expression::NewMaybe { ty, arguments } => self.select_construct(
                site,
                ty,
                &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                ConstructResult::Fallible,
            ),
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
            dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
            | dir::Expression::AwaitMaybe { expression: left }
            | dir::Expression::AwaitMust { expression: left } => {
                self.infer_try_projection_expression(site, left)
            }
            dir::Expression::Await { expression } => self.infer_await_expression(site, expression),
            dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Error => {
                self.commit_error_node(node.into_any())?;

                Ok(Answer::Ready(()))
            }
            expression => self.reject_expression_without_inference_owner(node, expression),
        }
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
            return Ok(Answer::Ready(()));
        };

        let ty = match self.static_value(*symbol) {
            Some(value) => value,
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
        // so the parent read overlays the narrowing itself
        let ty = answer!(self.node_type(child_site.node)?);
        self.commit_node_type(site.node, ty)?;

        Ok(Answer::Ready(()))
    }
}
