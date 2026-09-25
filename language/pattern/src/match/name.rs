use tspp_dir as dir;
use tspp_source::NodeSpanType;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one name stored on a containing DIR node.
    pub(crate) fn match_name(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        pattern: dir::Name,
        candidate: dir::Name,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_name_at(
            nodes,
            pattern_owner,
            candidate_owner,
            NodeSpanType::Main,
            pattern,
            candidate,
            bindings,
        )
    }

    /// Match one name at a typed source span owned by a DIR node.
    pub(crate) fn match_name_at(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        span_type: NodeSpanType,
        pattern: dir::Name,
        candidate: dir::Name,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let use_entry = nodes.uses().get_name(pattern_owner, span_type);
        let candidate_name = match candidate {
            dir::Name::Identifier(name) | dir::Name::String(name) => Some(name),
            dir::Name::Index(_) => None,
        };

        match use_entry {
            Some(use_entry) => self.bind_name(use_entry, candidate_owner, candidate_name, bindings),
            None => Ok(pattern == candidate),
        }
    }

    /// Match one optional name stored on a containing DIR node.
    pub(crate) fn match_optional_name(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        pattern: Option<dir::Name>,
        candidate: Option<dir::Name>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => self.match_name(
                nodes,
                pattern_owner,
                candidate_owner,
                pattern,
                candidate,
                bindings,
            ),
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }
}
