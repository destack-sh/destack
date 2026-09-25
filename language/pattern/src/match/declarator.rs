use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one variable declarator.
    pub(crate) fn match_declarator(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declarator>,
        candidate_id: dir::LocalNodeId<dir::Declarator>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let pattern_any = pattern_id.into_any();
        if let Some(is_match) =
            self.match_metavariable(nodes, pattern_any, candidate_id.into_any(), bindings)?
        {
            return Ok(is_match);
        }
        if !self.match_decorators(nodes, pattern_any, candidate_id.into_any(), bindings)? {
            return Ok(false);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);
        if !self.match_pattern_node(nodes, pattern.pattern, candidate.pattern, bindings)? {
            return Ok(false);
        }
        if !self.match_optional_type_expression(nodes, pattern.ty, candidate.ty, bindings)? {
            return Ok(false);
        }

        self.match_optional_expression(nodes, pattern.value, candidate.value, bindings)
    }
}
