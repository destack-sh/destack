use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::check::{Answer, CheckState, Constraint, FlowSite, Origin, Relation, answer};

impl CheckState<'_> {
    /// Infer one borrow expression from its borrowed value and provenance.
    pub(in crate::check) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let value =
            answer!(self.node_type_at(site.sibling(right.into_global_any(node.module_id)))?);
        let lifetime = self.borrow_provenance(node, right)?;
        let access = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.push_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Access(access)),
            node.local_id.into_any(),
        )?;
        let borrowed = self.push_type(
            node.module_id,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed { lifetime, access },
                value,
            }),
            node.local_id.into_any(),
        )?;
        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(Answer::Ready(()))
    }

    /// Return the lifetime flowing through one borrow expression.
    pub(in crate::check) fn borrow_provenance(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = node.module_id;
        let mut steps = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        if let Some(ty) = self.committed_node_type_maybe(right.into_global_any(module)) {
            steps.push(ty);
        }

        let mut current = right;
        let root = loop {
            let expression = self.module(module).view().get(current).clone();
            match expression {
                dir::Expression::Member { left, .. }
                | dir::Expression::PrivateMember { left, .. }
                | dir::Expression::Index { left, .. } => {
                    if let Some(ty) = self.committed_node_type_maybe(left.into_global_any(module)) {
                        steps.push(ty);
                    }
                    current = left;
                }
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    right,
                    ..
                } => {
                    if let Some(ty) = self.committed_node_type_maybe(right.into_global_any(module))
                    {
                        steps.push(ty);
                    }
                    current = right;
                }
                dir::Expression::Identifier { .. } => {
                    break self.borrow_root_lifetime(node, current)?;
                }
                _ => break self.temporary_lifetime(node)?,
            }
        };

        let mut provenance = root;
        for step in steps.into_iter().rev() {
            provenance = self.push_language_type(
                module,
                node.local_id.into_any(),
                dir::LanguageItem::LifetimeOr,
                vec![step, provenance],
            )?;
        }

        Ok(provenance)
    }

    /// Return the lifetime of one identifier root place.
    pub(in crate::check) fn borrow_root_lifetime(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let global = root.into_global_any(node.module_id);
        let Some(symbol) = self.single_resolved_symbol(global) else {
            return self.temporary_lifetime(node);
        };

        let module = self.module(symbol.module_id);
        let is_static = module
            .binding_table()
            .get_symbol_maybe(symbol.local_id)
            .is_some_and(|declared| declared.scope.id == module.bound.namespace_scope);
        let lifetime = if is_static {
            dir::Lifetime::Static
        } else {
            dir::Lifetime::Symbol(symbol)
        };

        self.push_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)),
            node.local_id.into_any(),
        )
    }

    /// Return the lifetime of one borrowed temporary.
    pub(in crate::check) fn temporary_lifetime(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            node.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
            node.local_id.into_any(),
        )
    }

    /// Infer one `satisfies` expression from its value while checking the target.
    pub(in crate::check) fn infer_satisfies_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_type = answer!(self.node_type_at(site.sibling(value.into_global_any(module)))?);
        let target = answer!(self.committed_node_type(target_type.into_global_any(module))?);
        self.push_constraint(Constraint::check(
            Relation::Satisfies,
            value_type,
            target,
            Origin::Node(node.into_any()),
        ));
        self.commit_node_type(node.into_any(), value_type)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one cast or const assertion expression.
    pub(in crate::check) fn infer_as_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = site.sibling(value.into_global_any(module));
        let value_type = answer!(self.node_type_at(value_site)?);

        let is_const_assertion = matches!(
            self.module(module).view().get(target_type),
            dir::TypeExpression::Const
        );
        if is_const_assertion {
            let ty = answer!(self.infer_expression_type(
                value_site,
                value.into_global(module),
                InferMode::Const
            )?);
            self.commit_node_type(node.into_any(), ty)?;

            return Ok(Answer::Ready(()));
        }

        let target = answer!(self.committed_node_type(target_type.into_global_any(module))?);
        self.push_constraint(Constraint::check(
            Relation::Castable,
            value_type,
            target,
            Origin::Node(node.into_any()),
        ));
        self.commit_node_type(node.into_any(), target)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one range expression from its written bounds.
    pub(in crate::check) fn infer_range_expression(
        &mut self,
        site: FlowSite,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut bounds = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        if let Some(start) = start {
            bounds.push(answer!(
                self.node_type_at(site.sibling(start.into_global_any(module)))?
            ));
        }
        if let Some(end) = end {
            bounds.push(answer!(
                self.node_type_at(site.sibling(end.into_global_any(module)))?
            ));
        }
        let element = match bounds.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_union_type(module, bounds, node.local_id.into_any())?),
        };
        let item = match (start, end, end_kind) {
            (Some(_), Some(_), dir::RangeEnd::Open) => dir::LanguageItem::Range,
            (Some(_), Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeInclusive,
            (Some(_), None, _) => dir::LanguageItem::RangeFrom,
            (None, Some(_), dir::RangeEnd::Open) => dir::LanguageItem::RangeTo,
            (None, Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeToInclusive,
            (None, None, _) => dir::LanguageItem::RangeFull,
        };
        let arguments = element.into_iter().collect();
        let range = self.push_language_type(module, node.local_id.into_any(), item, arguments)?;
        self.commit_node_type(node.into_any(), range)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one try projection expression from its carrier value.
    pub(in crate::check) fn infer_try_projection_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let value =
            answer!(self.node_type_at(site.sibling(value.into_global_any(node.module_id)))?);
        let output = answer!(self.reduce_operation_type(
            Origin::Node(node.into_any()),
            dir::TypeOperation::TryOutput { value },
        )?);
        self.commit_node_type(node.into_any(), output)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one awaited expression through the awaited type operation.
    pub(in crate::check) fn infer_await_expression(
        &mut self,
        site: FlowSite,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = Origin::Node(node.into_any());
        let value = answer!(self.node_type_at(site.sibling(awaited.into_global_any(module)))?);
        let result = answer!(self.reduce_operation_type(
            origin,
            dir::TypeOperation::Awaited(dir::UnaryType { target: value }),
        )?);
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }
}
