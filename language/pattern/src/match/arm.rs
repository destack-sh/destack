use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one match arm.
    pub(crate) fn match_arm(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::MatchArm>,
        candidate_id: dir::LocalNodeId<dir::MatchArm>,
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

        match (pattern, candidate) {
            (
                dir::MatchArm::Expression {
                    pattern: pattern_pattern,
                    guard: pattern_guard,
                    body: pattern_body,
                },
                dir::MatchArm::Expression {
                    pattern: candidate_pattern,
                    guard: candidate_guard,
                    body: candidate_body,
                },
            ) => {
                if !self.match_pattern_node(
                    nodes,
                    *pattern_pattern,
                    *candidate_pattern,
                    bindings,
                )? {
                    return Ok(false);
                }
                if !self.match_optional_condition(
                    nodes,
                    pattern_guard.as_ref(),
                    candidate_guard.as_ref(),
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::MatchArm::Block {
                    pattern: pattern_pattern,
                    guard: pattern_guard,
                    body: pattern_body,
                },
                dir::MatchArm::Block {
                    pattern: candidate_pattern,
                    guard: candidate_guard,
                    body: candidate_body,
                },
            ) => {
                if !self.match_pattern_node(
                    nodes,
                    *pattern_pattern,
                    *candidate_pattern,
                    bindings,
                )? {
                    return Ok(false);
                }
                if !self.match_optional_condition(
                    nodes,
                    pattern_guard.as_ref(),
                    candidate_guard.as_ref(),
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_block(nodes, *pattern_body, *candidate_body, bindings)
            }
            _ => Ok(false),
        }
    }

    /// Match two optional expressions.
    pub(crate) fn match_optional_expression(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<dir::Expression>>,
        candidate: Option<dir::LocalNodeId<dir::Expression>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_expression(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }
}
