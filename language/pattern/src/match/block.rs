use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, MetavariableUse, PatternNodes};

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
        if pattern.context != candidate.context || pattern.form != candidate.form {
            return Ok(false);
        }

        // let an explicit repeated marker span the semantic tail split
        let repeated = pattern
            .tail_expression
            .and_then(|expression| nodes.uses().get_node(expression.into_any()))
            .is_some_and(|use_entry| matches!(use_entry, MetavariableUse::Nodes { .. }));
        if repeated {
            let mut patterns = pattern.leading_expressions.clone();
            patterns.extend(pattern.tail_expression);
            let mut candidates = candidate.leading_expressions.clone();
            candidates.extend(candidate.tail_expression);

            return self.match_nodes(nodes, &patterns, &candidates, 0, 0, bindings);
        }

        // otherwise preserve the leading and value producing tail distinction
        if !self.match_nodes(
            nodes,
            &pattern.leading_expressions,
            &candidate.leading_expressions,
            0,
            0,
            bindings,
        )? {
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
