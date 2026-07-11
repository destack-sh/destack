use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CauseId, CheckAttempt, FlowSite, PlaceUse, Relation, ValueUse, answer,
};

impl BodyState<'_, '_> {
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
            None => self.intern_type(node.module_id, dir::Type::Void)?,
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
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let module = site.node.module_id;
        let check = match expressions.last().copied() {
            Some(value) => {
                let value_site = self.node_site(value.into_global_any(module))?;
                let check =
                    answer!(self.check_node_expected(value_site, target, relation, cause, use_)?);
                let value_type = answer!(self.node_type_at(value_site)?);
                self.commit_node_type(site.node, value_type)?;

                check
            }
            None => {
                let void = self.intern_type(module, dir::Type::Void)?;
                self.commit_node_type(site.node, void)?;
                let (_, check) =
                    answer!(self.check_node_value(site, relation, target, cause, Some(use_))?);

                check
            }
        };

        Ok(Answer::Ready(CheckAttempt::Checked(check)))
    }
}
