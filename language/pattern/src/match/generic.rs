use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one ordered list of generic arguments.
    pub(crate) fn match_generic_arguments(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: &[dir::LocalNodeId<dir::GenericArgument>],
        candidates: &[dir::LocalNodeId<dir::GenericArgument>],
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_nodes(nodes, patterns, candidates, 0, 0, bindings)
    }

    /// Match one generic argument.
    pub(crate) fn match_generic_argument(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::GenericArgument>,
        candidate_id: dir::LocalNodeId<dir::GenericArgument>,
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
                dir::GenericArgument::Type {
                    value: pattern_value,
                },
                dir::GenericArgument::Type {
                    value: candidate_value,
                },
            )
            | (
                dir::GenericArgument::SpreadType {
                    value: pattern_value,
                },
                dir::GenericArgument::SpreadType {
                    value: candidate_value,
                },
            ) => self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::GenericArgument::AssociatedType {
                    name: pattern_name,
                    value: pattern_value,
                },
                dir::GenericArgument::AssociatedType {
                    name: candidate_name,
                    value: candidate_value,
                },
            )
            | (
                dir::GenericArgument::AssociatedConst {
                    name: pattern_name,
                    value: pattern_value,
                },
                dir::GenericArgument::AssociatedConst {
                    name: candidate_name,
                    value: candidate_value,
                },
            ) => {
                if !self.match_node_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    Some(*pattern_name),
                    Some(*candidate_name),
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (dir::GenericArgument::Error, dir::GenericArgument::Error) => Ok(true),
            _ => Ok(false),
        }
    }
}
