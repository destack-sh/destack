use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer};

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
            Some(tail) => {
                let tail_site = self.node_site(tail.into_global_any(module))?;
                answer!(self.infer_node_type(tail_site, PlaceUse::Read)?)
            }
            None => self.push_type(module, dir::Type::Void, node.local_id)?,
        };
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
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
                let value_site = self.node_site(value.into_global_any(module))?;
                let () = answer!(self.check_node(value_site, target, relation, origin, use_)?);
                let value_type = answer!(self.node_type_at(value_site)?);
                self.commit_node_type(site.node, value_type)?;
            }
            None => {
                let void = self.push_type(module, dir::Type::Void, site.node.local_id)?;
                self.commit_node_type(site.node, void)?;
                let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
            }
        }

        Ok(Answer::Ready(true))
    }
}
