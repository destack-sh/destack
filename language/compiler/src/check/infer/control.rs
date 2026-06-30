use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Constraint, ExpectedType, FlowSite, ForInSourceObligation, Obligation,
    Origin, Relation, Task, ValueUse, Widening, answer,
};

impl CheckState<'_> {
    /// Infer one block from its tail expression.
    pub(in crate::check) fn infer_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        let module = node.module_id;
        let tail = self.module(module).view().get(block).value_expression();
        let ty = match tail {
            Some(tail) => answer!(self.node_type_at(site.sibling(tail.into_global_any(module)))?),
            None => self.push_type(module, dir::Type::Void, node.local_id)?,
        };
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one expression whose type is exactly its child expression type.
    pub(in crate::check) fn infer_forward_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = site.node.module_id;
        let ty = answer!(self.node_type_at(site.sibling(child.into_global_any(module)))?);
        self.commit_node_type(site.node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one sequence expression from its final expression.
    pub(in crate::check) fn infer_sequence_expression(
        &mut self,
        site: FlowSite,
        expressions: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let ty = match expressions.last().copied() {
            Some(last) => {
                answer!(self.node_type_at(site.sibling(last.into_global_any(node.module_id)))?)
            }
            None => self.push_type(node.module_id, dir::Type::Void, node.local_id.into_any())?,
        };
        self.commit_node_type(node.into_any(), ty)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one if expression from its branches.
    pub(in crate::check) fn infer_if_expression(
        &mut self,
        site: FlowSite,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let source = node.local_id.into_any();
        let then_type =
            answer!(self.node_type_at(site.sibling(then_expression.into_global_any(module)))?);
        let result = if let Some(else_expression) = else_expression {
            let else_type =
                answer!(self.node_type_at(site.sibling(else_expression.into_global_any(module)))?);
            self.normalized_union_type(module, [then_type, else_type], source)?
        } else {
            let void = self.push_type(module, dir::Type::Void, source)?;
            self.normalized_union_type(module, [then_type, void], source)?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one for-each binding from its iterator expression.
    pub(in crate::check) fn infer_for_each_expression(
        &mut self,
        site: FlowSite,
        operator: dir::ForEachOperator,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = site.node.module_id;
        let source = site.node.local_id;
        let iterator_type =
            answer!(self.node_type_at(site.sibling(iterator.into_global_any(module)))?);
        let value_type = match operator {
            dir::ForEachOperator::Of => {
                let origin = Origin::Node(site.node);
                let variable = self.allocate_variable(module, origin, Widening::Preserve);
                let value = self.push_variable_type(variable, source)?;
                let unknown = self.push_type(module, dir::Type::Unknown, source)?;
                let iterable = self.push_language_type(
                    module,
                    source,
                    dir::LanguageItem::Iterable,
                    vec![value, unknown, unknown],
                )?;
                self.push_constraint(Constraint::check(
                    Relation::Assignable,
                    iterator_type,
                    iterable,
                    origin,
                ));

                value
            }
            dir::ForEachOperator::In => {
                self.push_obligation(Obligation::ForInSource(ForInSourceObligation {
                    source: site.node,
                    ty: iterator_type,
                }));
                self.push_type(
                    module,
                    dir::Type::Primitive(dir::PrimitiveType::String),
                    source,
                )?
            }
        };
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.queue_task(Task::Check {
            site: site.sibling(pattern.into_global_any(module)),
            expected: ExpectedType::Type(value_type),
            relation: Relation::Assignable,
            origin: Origin::Node(site.node),
            use_: ValueUse::Store,
        });

        Ok(Answer::Ready(()))
    }

    /// Infer one try expression from its body and catch branches.
    pub(in crate::check) fn infer_try_expression(
        &mut self,
        site: FlowSite,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let body_type = answer!(self.node_type_at(site.sibling(body.into_global_any(module)))?);
        let result = if let Some(catch) = catch {
            let catch_body = self.module(module).view().get(catch).body;
            let catch_type =
                answer!(self.node_type_at(site.sibling(catch_body.into_global_any(module)))?);

            self.normalized_union_type(module, [body_type, catch_type], node.local_id.into_any())?
        } else {
            body_type
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one match expression from its arm values.
    pub(in crate::check) fn infer_match_expression(
        &mut self,
        site: FlowSite,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut values = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        for case in cases {
            let body = match self.module(module).view().get(*case) {
                dir::MatchCase::Expression { body, .. } => body.into_global_any(module),
                dir::MatchCase::Block { body, .. } => body.into_global_any(module),
            };
            values.push(answer!(self.node_type_at(site.sibling(body))?));
        }

        let result = if values.is_empty() {
            self.push_type(module, dir::Type::Never, node.local_id.into_any())?
        } else {
            self.normalized_union_type(module, values, node.local_id.into_any())?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Check one expression whose value is exactly its child value.
    pub(in crate::check) fn check_forward_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let module = site.node.module_id;
        let () = answer!(self.check_node(
            site.sibling(child.into_global_any(module)),
            target,
            relation,
            origin,
            use_
        )?);
        let child_type = answer!(self.node_type_at(site.sibling(child.into_global_any(module)))?);
        self.commit_node_type(site.node, child_type)?;

        Ok(Answer::Ready(true))
    }

    /// Check one block expression under an expected result type.
    pub(in crate::check) fn check_block_expression(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let module = site.node.module_id;
        let value = self.module(module).view().get(block).value_expression();
        match value {
            Some(value) => {
                let value_node = value.into_global_any(module);
                let () = answer!(self.check_node(
                    site.sibling(value_node),
                    target,
                    relation,
                    origin,
                    use_
                )?);
                let value_type = answer!(self.node_type_at(site.sibling(value_node))?);
                self.commit_node_type(site.node, value_type)?;
            }
            None => {
                let void = self.push_type(module, dir::Type::Void, site.node.local_id)?;
                self.commit_node_type(site.node, void)?;
                self.push_constraint(Constraint::value(relation, void, target, origin, use_));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Check one sequence expression under an expected result type.
    pub(in crate::check) fn check_sequence_expression(
        &mut self,
        site: FlowSite,
        expressions: &[dir::LocalNodeId<dir::Expression>],
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let module = site.node.module_id;
        match expressions.last().copied() {
            Some(value) => {
                let value_node = value.into_global_any(module);
                let () = answer!(self.check_node(
                    site.sibling(value_node),
                    target,
                    relation,
                    origin,
                    use_
                )?);
                let value_type = answer!(self.node_type_at(site.sibling(value_node))?);
                self.commit_node_type(site.node, value_type)?;
            }
            None => {
                let void = self.push_type(module, dir::Type::Void, site.node.local_id)?;
                self.commit_node_type(site.node, void)?;
                self.push_constraint(Constraint::value(relation, void, target, origin, use_));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Check one conditional expression under an expected result type.
    pub(in crate::check) fn check_if_expression(
        &mut self,
        site: FlowSite,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let module = site.node.module_id;
        let source = site.node.local_id;
        let then_node = then_expression.into_global_any(module);
        let () =
            answer!(self.check_node(site.sibling(then_node), target, relation, origin, use_)?);
        let then_type = answer!(self.node_type_at(site.sibling(then_node))?);

        let result = if let Some(else_expression) = else_expression {
            let else_node = else_expression.into_global_any(module);
            let () = answer!(self.check_node(
                site.sibling(else_node),
                target,
                relation,
                origin,
                use_
            )?);
            let else_type = answer!(self.node_type_at(site.sibling(else_node))?);

            self.normalized_union_type(module, [then_type, else_type], source)?
        } else {
            let void = self.push_type(module, dir::Type::Void, site.node.local_id)?;
            self.push_constraint(Constraint::value(relation, void, target, origin, use_));
            self.normalized_union_type(module, [then_type, void], source)?
        };

        self.commit_node_type(site.node, result)?;

        Ok(Answer::Ready(true))
    }
}
