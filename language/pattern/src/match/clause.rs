use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one catch clause.
    pub(crate) fn match_catch(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Catch>,
        candidate_id: dir::LocalNodeId<dir::Catch>,
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
        if !self.match_optional_pattern(nodes, pattern.pattern, candidate.pattern, bindings)? {
            return Ok(false);
        }
        if !self.match_optional_type_expression(nodes, pattern.ty, candidate.ty, bindings)? {
            return Ok(false);
        }

        self.match_expression(nodes, pattern.body, candidate.body, bindings)
    }

    /// Match one where clause.
    pub(crate) fn match_where_clause(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::WhereClause>,
        candidate_id: dir::LocalNodeId<dir::WhereClause>,
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
        if pattern.relation != candidate.relation
            || !self.match_type_expression(nodes, pattern.left, candidate.left, bindings)?
        {
            return Ok(false);
        }

        self.match_type_expression(nodes, pattern.right, candidate.right, bindings)
    }

    /// Match one switch case.
    pub(crate) fn match_switch_case(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::SwitchCase>,
        candidate_id: dir::LocalNodeId<dir::SwitchCase>,
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
        let is_selector_match = match (pattern.selector, candidate.selector) {
            (dir::SwitchSelector::Case(pattern), dir::SwitchSelector::Case(candidate)) => {
                self.match_expression(nodes, pattern, candidate, bindings)?
            }
            (dir::SwitchSelector::Default, dir::SwitchSelector::Default) => true,
            _ => false,
        };
        if !is_selector_match {
            return Ok(false);
        }

        self.match_block(nodes, pattern.body, candidate.body, bindings)
    }
}
