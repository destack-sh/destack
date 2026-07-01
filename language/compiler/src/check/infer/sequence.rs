use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer};

impl CheckState<'_> {
    /// Infer one sequence expression from its final expression.
    pub(in crate::check) fn infer_sequence_expression(
        &mut self,
        site: FlowSite,
        expressions: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let ty = match expressions.last().copied() {
            Some(last) => {
                let last_site = self.node_site(last.into_global_any(node.module_id))?;
                answer!(self.infer_node_type(last_site, PlaceUse::Read)?)
            }
            None => self.push_type(node.module_id, dir::Type::Void, node.local_id.into_any())?,
        };
        self.commit_node_type(node.into_any(), ty)?;

        Ok(Answer::Ready(()))
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
