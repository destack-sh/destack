use destack_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one block.
    pub(crate) fn match_block(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Block>,
        candidate_id: dir::LocalNodeId<dir::Block>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let pattern_any = pattern_id.into_any();
        if let Some(use_entry) = nodes.uses().get_node(pattern_any) {
            return self.bind_node(use_entry, candidate_id.into_any(), bindings);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);
        if pattern.context != candidate.context
            || pattern.form != candidate.form
            || !self.match_nodes(
                nodes,
                &pattern.leading_expressions,
                &candidate.leading_expressions,
                0,
                0,
                bindings,
            )?
        {
            return Ok(false);
        }

        self.match_optional_expression(
            nodes,
            pattern.tail_expression,
            candidate.tail_expression,
            bindings,
        )
    }
}
