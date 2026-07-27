use destack_dir as dir;
use destack_source::{NodeSpanRegion, NodeSpanType};

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
        if let Some(use_entry) = nodes.uses().get_node(pattern_any) {
            return self.bind_node(use_entry, candidate_id.into_any(), bindings);
        }
        let pattern = nodes.tree().get(pattern_id);
        let candidate = self.candidate.get(candidate_id);

        match (pattern, candidate) {
            (
                dir::DependencyItem::Binding {
                    binding: pattern_binding,
                    form: pattern_form,
                    name: pattern_name,
                    alias: pattern_alias,
                    value: pattern_value,
                },
                dir::DependencyItem::Binding {
                    binding: candidate_binding,
                    form: candidate_form,
                    name: candidate_name,
                    alias: candidate_alias,
                    value: candidate_value,
                },
            ) => {
                if pattern_binding != candidate_binding
                    || pattern_form != candidate_form
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
        pattern: Option<destack_core::StringId>,
        candidate: Option<destack_core::StringId>,
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
