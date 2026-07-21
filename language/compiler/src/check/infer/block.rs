use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CauseId, FlowSite, PlaceUse, Relation, StaticGate, ValueCheck, ValueUse,
    answer,
};

impl BodyState<'_, '_> {
    /// Infer one block from its tail expression.
    pub(in crate::check) fn infer_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        let module = node.module_id;
        answer!(self.check_block_statements(module, block)?);
        let tail = self.block_value(module, block)?;
        let ty = match tail {
            Some(tail) => {
                let tail_site = self.node_site(tail.into_global_any(module))?;
                answer!(self.infer_node_type(tail_site, PlaceUse::Read)?)
            }
            None => self.end_type(module, block.into_any())?,
        };
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Check every leading statement of one block in source order.
    pub(in crate::check) fn check_block_statements(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Answer<()>> {
        let statements = self
            .module(module)
            .view()
            .get(block)
            .leading_expressions
            .clone();
        for statement in statements {
            let node = statement.into_global_any(module);
            if self.check.static_gate(node)? == StaticGate::Absent {
                continue;
            }
            let site = self.check.node_site(node)?;
            answer!(self.attempt_node(site, PlaceUse::Read, None)?);
        }

        Ok(Answer::Ready(()))
    }

    /// Return the implicit value type at one source node's end.
    pub(in crate::check) fn end_type(
        &mut self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = match self.check.module(module).unreachable_ends.contains(&node) {
            true => dir::Type::Never,
            false => dir::Type::Void,
        };

        self.intern_type(module, ty)
    }

    /// Check one block under an expected result type.
    pub(in crate::check) fn check_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let module = site.node.module_id;
        answer!(self.check_block_statements(module, block)?);
        let value = self.block_value(module, block)?;
        let check = match value {
            Some(value) => {
                let value_site = self.node_site(value.into_global_any(module))?;
                let check =
                    answer!(self.check_node_expected(value_site, target, relation, cause, use_)?);
                let value_type = answer!(self.node_type_at(value_site)?);
                self.commit_node_type(site.node, value_type)?;

                check
            }
            None => {
                let value = self.end_type(module, block.into_any())?;
                self.commit_node_type(site.node, value)?;
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                check
            }
        };

        Ok(Answer::Ready(check))
    }

    /// Return the statically present value expression of one block.
    fn block_value(
        &self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let value = self.module(module).view().get(block).value_expression();
        let value = match value {
            Some(value)
                if self.check.static_gate(value.into_global_any(module))?
                    == StaticGate::Present =>
            {
                Some(value)
            }
            Some(_) | None => None,
        };

        Ok(value)
    }
}
