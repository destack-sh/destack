use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Decision, Dependency, Origin, Widening};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the meaning of one source node once its inputs allow.
    pub(in crate::check) fn run_select(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<()>> {
        // skip nodes that already decided
        if self.decisions.get(node).is_some() {
            return Ok(Answer::Ready(()));
        }

        // select under the node's recorded @if guard assumptions
        let predicates = self
            .inputs
            .node_condition(node)
            .iter()
            .copied()
            .collect::<SmallVec<[_; 2]>>();
        let mark = self.assume(&predicates)?;

        // narrow the queue currency to the node's typed kind
        let answer = match node.local_id.ty {
            dir::NodeType::Expression => self.select_expression(node.into_typed()),
            dir::NodeType::Pattern => self.select_pattern(node.into_typed()),
            other => Err(CompilerError::Internal {
                message: format!("check node {node:?} has no selector for {other:?}"),
            }),
        };
        self.release_assumptions(mark);

        answer
    }

    /// Select the meaning of one expression node by its syntactic shape.
    fn select_expression(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // read each shape's payload once and dispatch with it
        let view = self.module(module).view();
        match view.get(node.local_id) {
            dir::Expression::Member { left, name } => {
                let (left, name) = (*left, *name);

                self.select_member(node, left, name)
            }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_property_merge(node, &properties, None)
            }
            dir::Expression::StructExpression { properties, .. } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                let Some(target) = self.inputs.node_type(node.into_any()) else {
                    return Err(CompilerError::Internal {
                        message: format!("struct literal {node:?} has no declared target"),
                    });
                };

                self.select_property_merge(node, &properties, Some(target))
            }
            dir::Expression::Call {
                left, arguments, ..
            } => {
                let callee = *left;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_call(node, callee, &arguments)
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let (operator, left, right) = (*operator, *left, *right);

                self.select_binary_operator(node, operator, left, right)
            }
            dir::Expression::Unary { operator, right } => {
                let (operator, operand) = (*operator, *right);

                self.select_unary_operator(node, operator, operand)
            }
            dir::Expression::New { ty, arguments }
            | dir::Expression::NewMaybe { ty, arguments } => {
                let ty = *ty;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_construct(node, ty, &arguments)
            }
            dir::Expression::Index { left, index, .. } => {
                let (left, index) = (*left, *index);

                self.select_index(node, left, index)
            }
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left = *left;
                let arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();

                self.select_instantiation(node, left, &arguments)
            }
            dir::Expression::TaggedTemplateExpression { tag, .. } => {
                let tag = *tag;

                self.select_tagged_template(node, tag)
            }
            dir::Expression::TreeExpression { .. } => self.select_tree(node),
            // compound assignments resolve their operator over the
            // place's read type and the written value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let (left, operator, right) = (*left, *operator, *right);
                let Some(binary) = operator.binary_operator() else {
                    return Err(CompilerError::Internal {
                        message: format!("assign node {node:?} queued without a compound operator"),
                    });
                };
                let dir::AssignPattern::Expression { value: target } = view.get(left) else {
                    return Err(CompilerError::Internal {
                        message: format!("compound assignment {node:?} has no place target"),
                    });
                };
                let target = *target;

                self.select_binary_operator(node, binary, target, right)
            }
            other => Err(CompilerError::Internal {
                message: format!("check expression {node:?} has no selector: {other:?}"),
            }),
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
        let symbol = match self.decisions.get(left_node) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                // overloaded instantiations stay for their call sites
                _ => {
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
            let Some(ty) = self.inputs.node_type(argument) else {
                return Err(CompilerError::Internal {
                    message: format!("generic argument {argument:?} has no input type"),
                });
            };
            applied.push(ty);
        }

        // reject applications with more arguments than parameters
        let parameters = self
            .generics
            .template_by_symbol(symbol)
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        if applied.len() > parameters.len() {
            let name = self.format_symbol(symbol);
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            let error = crate::CheckError::WrongGenericArity {
                anchor,
                module,
                name,
                expected: parameters.len(),
                supplied: applied.len(),
            };
            self.module_mut(module).diagnostics.push(error.into());
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // fill declared positions, opening holes for missing arguments
        let mut canonical = Vec::with_capacity(parameters.len().max(applied.len()));
        let mut written = applied.iter().copied();
        for _parameter in &parameters {
            let argument = match written.next() {
                Some(argument) => argument,
                None => {
                    let variable = self.allocate_variable(module, origin, Widening::Preserve);

                    self.push_variable_type(variable, source)?
                }
            };
            canonical.push(argument);
        }

        // write the applied reference and record the selection
        let reference = self.push_type(
            module,
            dir::Type::Reference(dir::GenericInstance {
                symbol,
                arguments: canonical,
            }),
            source,
        )?;
        self.record_decision(node, Decision::Name(dir::NameResolution::new(symbol)))?;
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, reference)?;
        }

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
        let Some(tag_type) = self.inputs.node_type(tag_node) else {
            return Err(CompilerError::Internal {
                message: format!("template tag {tag_node:?} has no input type"),
            });
        };
        let tag_type = match self.evaluate_root(origin, tag_type)? {
            Answer::Ready(tag_type) => tag_type,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let return_type = match self.ty(tag_type)? {
            dir::Type::Function(function) => function.return_type,
            _ => {
                let ty = self.format_type(tag_type);
                let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                let error = crate::CheckError::NotCallable { anchor, module, ty };
                self.module_mut(module).diagnostics.push(error.into());
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }
        };

        // the application produces the tag's return value
        let result = match return_type {
            Some(return_type) => return_type,
            None => self.push_type(module, dir::Type::Void, source)?,
        };
        let resolution = dir::CallResolution::new(
            dir::CallTarget::Expression {
                arguments: Vec::new(),
            },
            Vec::new(),
            result,
        );
        self.record_decision(node, Decision::Call(resolution))?;
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, result)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Select one tree expression.
    fn select_tree(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        // TODO(check): select tree constructions through the configured
        // tree builder once tree checking lands.
        let module = node.module_id;
        self.report_invalid_control_flow(
            module,
            node.local_id.into_any(),
            "tree expressions are not checked yet",
        );
        self.record_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
