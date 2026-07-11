use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CauseId, CheckOutcome, FlowSite, PlaceUse, Relation, ValueUse, answer,
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
        self.check_block_statements(module, block)?;
        let tail = self.module(module).view().get(block).value_expression();
        let ty = match tail {
            Some(tail) => {
                let tail_site = self.node_site(tail.into_global_any(module))?;
                answer!(self.infer_node_type(tail_site, PlaceUse::Read)?)
            }
            None => self.block_end_type(module, block)?,
        };
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Check every leading statement of one block in source order.
    pub(in crate::check) fn check_block_statements(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let statements = self
            .module(module)
            .view()
            .get(block)
            .leading_expressions
            .clone();
        for statement in statements {
            // the binder records no flow site for statically absent nodes
            let node = statement.into_global_any(module);
            if !self.check.module(module).node_flows.contains_key(&node) {
                continue;
            }
            let site = self.check.node_site(node)?;
            self.check_node(site, PlaceUse::Read, None)?;
        }

        Ok(())
    }

    /// Return the value type of one block that ends without a tail.
    fn block_end_type(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = match self
            .check
            .module(module)
            .unreachable_ends
            .contains(&block.into_any())
        {
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
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let module = site.node.module_id;
        self.check_block_statements(module, block)?;
        let value = self.module(module).view().get(block).value_expression();
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
                let value = self.block_end_type(module, block)?;
                self.commit_node_type(site.node, value)?;
                let (_, check) =
                    answer!(self.check_node_value(site, relation, target, cause, Some(use_))?);

                check
            }
        };

        Ok(Answer::Ready(check))
    }
}
