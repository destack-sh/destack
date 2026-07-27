use destack_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one decorator.
    pub(crate) fn match_decorator(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Decorator>,
        candidate_id: dir::LocalNodeId<dir::Decorator>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let pattern_any = pattern_id.into_any();
        if let Some(use_entry) = nodes.uses().get_node(pattern_any) {
            return self.bind_node(use_entry, candidate_id.into_any(), bindings);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);
        if pattern.position != candidate.position {
            return Ok(false);
        }

        self.match_expression(nodes, pattern.expression, candidate.expression, bindings)
    }
}
