use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, FlowSite, ForInSourceObligation, Obligation, Origin, PlaceUse, Relation,
    ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one if expression from its branches.
    pub(in crate::check) fn infer_if_expression(
        &mut self,
        site: FlowSite,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let then_site = self.node_site(then_expression.into_global_any(module))?;
        let then_type = answer!(self.infer_node_type(then_site, PlaceUse::Read)?);
        let result = if let Some(else_expression) = else_expression {
            let else_site = self.node_site(else_expression.into_global_any(module))?;
            let else_type = answer!(self.infer_node_type(else_site, PlaceUse::Read)?);
            self.normalized_union_type(module, [then_type, else_type])?
        } else {
            let void = self.intern_type(module, dir::Type::Void)?;
            self.normalized_union_type(module, [then_type, void])?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Check one if expression under an expected result type.
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

        // check the then branch against the incoming expectation
        let then_site = self.node_site(then_expression.into_global_any(module))?;
        let () = answer!(self.check_node(then_site, target, relation, origin, use_)?);
        let then_type = answer!(self.node_type_at(then_site)?);
        let mut should_relate_result = false;

        // check an else branch, or make the missing branch explicit as void
        let result = if let Some(else_expression) = else_expression {
            let else_site = self.node_site(else_expression.into_global_any(module))?;
            let () = answer!(self.check_node(else_site, target, relation, origin, use_)?);
            let else_type = answer!(self.node_type_at(else_site)?);

            self.normalized_union_type(module, [then_type, else_type])?
        } else {
            let void = self.intern_type(module, dir::Type::Void)?;
            should_relate_result = true;

            self.normalized_union_type(module, [then_type, void])?
        };
        self.commit_node_type(site.node, result)?;

        // relate the result when branch checks did not cover every arm
        if should_relate_result {
            let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
        }

        Ok(Answer::Ready(true))
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
        let body_site = self.node_site(body.into_global_any(module))?;
        let body_type = answer!(self.infer_node_type(body_site, PlaceUse::Read)?);
        let result = if let Some(catch) = catch {
            let catch_body = self.module(module).view().get(catch).body;
            let catch_site = self.node_site(catch_body.into_global_any(module))?;
            let catch_type = answer!(self.infer_node_type(catch_site, PlaceUse::Read)?);

            self.normalized_union_type(module, [body_type, catch_type])?
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
            let body_site = self.node_site(body)?;
            values.push(answer!(self.infer_node_type(body_site, PlaceUse::Read)?));
        }

        let result = if values.is_empty() {
            self.intern_type(module, dir::Type::Never)?
        } else {
            self.normalized_union_type(module, values)?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one for-in or for-of expression.
    pub(in crate::check) fn infer_for_each_expression(
        &mut self,
        site: FlowSite,
        operator: dir::ForEachOperator,
        binding: dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let iterator_site = self.node_site(iterator.into_global_any(module))?;
        let iterator_type = answer!(self.infer_node_type(iterator_site, PlaceUse::Read)?);
        let target =
            answer!(self.for_each_value_type(site.origin(), site.node, operator, iterator_type)?);

        // check the binding against the value produced by the iteration source
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => pattern,
        };
        let pattern_site = self.node_site(pattern.into_global_any(module))?;
        answer!(self.check_node(
            pattern_site,
            target,
            Relation::Assignable,
            site.origin(),
            ValueUse::Store
        )?);

        // for-in and for-of evaluate to void
        let void = self.intern_type(module, dir::Type::Void)?;
        self.commit_node_type(site.node, void)?;

        Ok(Answer::Ready(()))
    }

    /// Return the value type bound by one for-in or for-of source.
    fn for_each_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::ForEachOperator,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match operator {
            dir::ForEachOperator::In => self.for_in_value_type(origin, source, iterator_type),
            dir::ForEachOperator::Of => self.for_of_value_type(origin, source, iterator_type),
        }
    }

    /// Return the key type bound by one for-in source.
    fn for_in_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let scope = self.origin_scope(origin);
        self.push_obligation(
            Obligation::ForInSource(ForInSourceObligation {
                source,
                ty: iterator_type,
            }),
            scope,
        );
        let string = self.intern_type(
            source.module_id,
            dir::Type::Primitive(dir::PrimitiveType::String),
        )?;

        Ok(Answer::Ready(string))
    }

    /// Return the yielded value type of one for-of source.
    fn for_of_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let protocol = self.language_symbol(dir::LanguageItem::Iterable);
        let implementation = answer!(self.select_protocol_implementation(
            self.origin_at(origin, source),
            iterator_type,
            iterator_type,
            protocol,
        )?);
        let Some(implementation) = implementation else {
            self.report_for_of_source_not_iterable(source);
            let error = self.intern_type(source.module_id, dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        };
        let Some(value) = implementation.arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "Iterable protocol implementation has no value argument".to_owned(),
            });
        };

        Ok(Answer::Ready(value))
    }
}
