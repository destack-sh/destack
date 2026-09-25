use tspp_dir as dir;
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one dependency item node.
    pub(crate) fn match_dependency_item(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::DependencyItem>,
        candidate_id: dir::LocalNodeId<dir::DependencyItem>,
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
                dir::DependencyItem::Binding {
                    binding: pattern_binding,
                    name: pattern_name,
                    alias: pattern_alias,
                    value: pattern_value,
                },
                dir::DependencyItem::Binding {
                    binding: candidate_binding,
                    name: candidate_name,
                    alias: candidate_alias,
                    value: candidate_value,
                },
            ) => {
                if pattern_binding != candidate_binding
                    || !self.match_optional_dependency_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_optional_dependency_alias(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_alias,
                        *candidate_alias,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_value, *candidate_value, bindings)
            }
            (dir::DependencyItem::Error, dir::DependencyItem::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one optional dependency source name.
    fn match_optional_dependency_name(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        pattern: Option<dir::Name>,
        candidate: Option<dir::Name>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => self.match_name_at(
                nodes,
                pattern_owner,
                candidate_owner,
                NodeSpanType::Region(NodeSpanRegion::Type),
                pattern,
                candidate,
                bindings,
            ),
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one optional dependency alias.
    fn match_optional_dependency_alias(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        pattern: Option<tspp_core::StringId>,
        candidate: Option<tspp_core::StringId>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => self.match_name(
                nodes,
                pattern_owner,
                candidate_owner,
                dir::Name::Identifier(pattern),
                dir::Name::Identifier(candidate),
                bindings,
            ),
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }
}
