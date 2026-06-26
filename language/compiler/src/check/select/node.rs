use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Decision, Dependency, GenericArgumentMode, Origin, Relation, Selection,
    SelectionId, SelectionState, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one queued operation once its inputs allow.
    pub(in crate::check) fn run_select(&mut self, id: SelectionId) -> CompilerResult<Answer<()>> {
        if self.solver.selections.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        // copy the selection before selector calls mutate solver state
        let selection = self.solver.selections.get(id)?.clone();
        let node = selection.node();
        if self.solver.decision(node).is_some() {
            self.solver.set_selection_state(id, SelectionState::Done)?;

            return Ok(Answer::Ready(()));
        }

        // select under the node's recorded @if guard assumptions
        let predicates = self
            .node_condition(node)
            .iter()
            .copied()
            .collect::<SmallVec<[_; 2]>>();
        let mark = self.assume(&predicates)?;

        // run the selected operation
        let answer = self.select(selection);
        self.release_assumptions(mark);

        match answer? {
            Answer::Ready(()) => {
                self.solver.set_selection_state(id, SelectionState::Done)?;

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Select the meaning of one queued operation.
    fn select(&mut self, selection: Selection) -> CompilerResult<Answer<()>> {
        match selection {
            Selection::Member { node, left, name } => self.select_member(node, left, name),
            Selection::ObjectMerge { node, properties } => {
                let properties = properties.into_iter().collect::<SmallVec<[_; 4]>>();

                self.select_property_merge(node, &properties, None)
            }
            Selection::StructMerge { node, properties } => {
                let properties = properties.into_iter().collect::<SmallVec<[_; 4]>>();
                let target = answer!(self.node_type_answer(node.into_any())?);

                self.select_property_merge(node, &properties, Some(target))
            }
            Selection::Call {
                node,
                callee,
                generic_arguments,
                arguments,
            } => {
                let generic_arguments = generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>();
                let arguments = arguments.into_iter().collect::<SmallVec<[_; 4]>>();

                self.select_call(node, callee, &generic_arguments, &arguments)
            }
            Selection::MemberPredicate { node, left, right } => {
                self.select_member_predicate(node, left, right)
            }
            Selection::BinaryOperator {
                node,
                operator,
                left,
                right,
            } => self.select_binary_operator(node, operator, left, right),
            Selection::TypePredicate {
                node,
                value,
                target,
            } => self.select_type_predicate(node, value, target),
            Selection::ClassPredicate {
                node,
                value,
                target,
            } => self.select_class_predicate(node, value, target),
            Selection::UnaryOperator {
                node,
                operator,
                operand,
                use_,
            } => self.select_unary_operator_with_use(node, operator, operand, use_),
            Selection::Construct {
                node,
                ty,
                arguments,
                result,
            } => {
                let arguments = arguments.into_iter().collect::<SmallVec<[_; 4]>>();

                self.select_construct(node, ty, &arguments, result)
            }
            Selection::Subscript {
                node,
                left,
                index,
                use_,
            } => self.select_index_with_use(node, left, index, use_),
            Selection::Instantiation {
                node,
                left,
                arguments,
            } => {
                let arguments = arguments.into_iter().collect::<SmallVec<[_; 2]>>();

                self.select_instantiation(node, left, &arguments)
            }
            Selection::TaggedTemplate { node, tag } => self.select_tagged_template(node, tag),
            Selection::Tree { node } => self.select_tree(node),
            Selection::Pattern { node } => self.select_pattern(node),
            Selection::AssignPattern { node } => self.select_assign_pattern(node),
        }
    }

    /// Select one explicit instantiation once its target name decides.
    fn select_instantiation(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let source = node.local_id.into_any();
        let node = node.into_any();
        let origin = Origin::Node(node);

        // read the decided target name
        let left_node = left.into_global_any(module);
        let symbol = match self.solver.decision(left_node) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => {
                    let Some(path) = self.module(module).view().tree().reference_path(left) else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "overloaded instantiation target {left_node:?} has no reference path"
                            ),
                        });
                    };
                    self.report_ambiguous_reference(module, left.into_any(), &path);
                    self.record_decision(node, Decision::Rejected)?;

                    return Ok(Answer::Ready(()));
                }
            },
            Some(Decision::Rejected) => {
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }
            Some(other) => {
                return Err(CompilerError::Internal {
                    message: format!("instantiation target {left_node:?} decided as {other:?}"),
                });
            }
            None => return Ok(Answer::pending([Dependency::Decision(left_node)])),
        };

        // collect the written argument types
        let mut applied = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for argument in arguments {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.node_type_answer(argument)?);
            applied.push(ty);
        }

        // read the selected declaration template
        let template = self.symbol_template(symbol);
        if template.is_none() && !applied.is_empty() {
            let name = self.format_symbol(symbol);
            self.report_wrong_generic_arity(module, source, name, 0, applied.len());
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // specialize the selected value type by the applied arguments
        let declared = self
            .symbol_type_maybe(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("instantiated symbol {symbol:?} has no declared type"),
            })?;
        let specialized = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template);
                let Some(substitution) = self.instantiate_template(
                    origin,
                    template,
                    &applied,
                    GenericArgumentMode::Default,
                )?
                else {
                    let name = self.format_symbol(symbol);
                    self.report_wrong_generic_arity(
                        module,
                        source,
                        name,
                        parameters.len(),
                        applied.len(),
                    );
                    self.record_decision(node, Decision::Rejected)?;

                    return Ok(Answer::Ready(()));
                };

                // check written arguments against declared constraints
                for (parameter, argument) in parameters
                    .iter()
                    .copied()
                    .zip(substitution.arguments.iter().copied())
                {
                    let constraint = self
                        .generic_parameter(parameter)
                        .and_then(|binding| binding.constraint);
                    let Some(constraint) = constraint else {
                        continue;
                    };
                    let constraint =
                        self.fold_type(module, source, constraint, substitution.rewrite())?;
                    let condition = self.node_static_condition(node);

                    if !answer!(self.constrain_generic_argument(
                        origin, node, condition, argument, constraint
                    )?) {
                        self.relate(origin, Relation::Assignable, None, argument, constraint)?;
                        self.record_decision(node, Decision::Rejected)?;

                        return Ok(Answer::Ready(()));
                    }
                }

                match self.ty(declared)? {
                    dir::Type::Reference(reference) if reference.symbol == symbol => self
                        .push_type(
                            module,
                            dir::Type::Instance(dir::GenericInstance {
                                symbol,
                                arguments: substitution.arguments.to_vec(),
                            }),
                            source,
                        )?,
                    _ => self.fold_type(module, source, declared, substitution.rewrite())?,
                }
            }
            None => declared,
        };

        let arguments = self.symbol_generic_argument_bindings(symbol, &applied)?;
        let resolution = dir::InstantiationResolution::new(symbol, arguments);
        self.record_decision(node, Decision::Instantiation(resolution))?;
        self.bind_node_type(node, specialized)?;

        Ok(Answer::Ready(()))
    }

    /// Select one tagged template through its tag's callable value.
    fn select_tagged_template(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        tag: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let source = node.local_id.into_any();
        let node = node.into_any();
        let origin = Origin::Node(node);

        // close the tag's callable shape
        let tag_node = tag.into_global_any(module);
        let tag_type = answer!(self.node_type_answer(tag_node)?);
        let tag_type = answer!(self.reduce_type_root(origin, tag_type)?);
        let signature = match self.ty(tag_type)? {
            dir::Type::FunctionSignature(_) => Some(tag_type),
            _ => self.callable_signature(tag_type)?,
        };
        let Some(signature) = signature else {
            self.report_not_callable(origin, tag_type)?;
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };
        let return_type = match self.ty(signature)? {
            dir::Type::FunctionSignature(function) => function.return_type,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("tagged template signature {signature:?} is not callable"),
                });
            }
        };

        // the application produces the tag's return value
        let result = match return_type {
            Some(return_type) => return_type,
            None => self.push_type(module, dir::Type::Void, source)?,
        };
        let resolution = dir::CallResolution::new(
            dir::CallTarget::Expression {
                generic_arguments: Vec::new(),
            },
            Some(signature),
            Vec::new(),
            Vec::new(),
            result,
        );
        self.record_decision(node, Decision::Call(resolution))?;
        self.bind_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }

    /// Select one tree expression.
    fn select_tree(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        // TODO #Incomplete: select tree constructions through the configured tree builder
        let module = node.module_id;
        self.report_unsupported_tree_expression(module, node.local_id.into_any());
        self.record_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
