use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one tree attribute node.
    pub(crate) fn match_tree_attribute(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TreeAttribute>,
        candidate_id: dir::LocalNodeId<dir::TreeAttribute>,
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
                dir::TreeAttribute::Named {
                    name: pattern_name,
                    value: pattern_value,
                },
                dir::TreeAttribute::Named {
                    name: candidate_name,
                    value: candidate_value,
                },
            ) => {
                if !self.match_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    *pattern_name,
                    *candidate_name,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_optional_tree_attribute_value(
                    nodes,
                    pattern_value.as_ref(),
                    candidate_value.as_ref(),
                    bindings,
                )
            }
            (
                dir::TreeAttribute::Spread {
                    value: pattern_value,
                },
                dir::TreeAttribute::Spread {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (dir::TreeAttribute::Error, dir::TreeAttribute::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one tree child node.
    pub(crate) fn match_tree_child(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TreeChild>,
        candidate_id: dir::LocalNodeId<dir::TreeChild>,
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
                dir::TreeChild::Text {
                    value: pattern_value,
                },
                dir::TreeChild::Text {
                    value: candidate_value,
                },
            ) => Ok(pattern_value == candidate_value),
            (
                dir::TreeChild::Expression {
                    value: pattern_value,
                },
                dir::TreeChild::Expression {
                    value: candidate_value,
                },
            )
            | (
                dir::TreeChild::Spread {
                    value: pattern_value,
                },
                dir::TreeChild::Spread {
                    value: candidate_value,
                },
            )
            | (
                dir::TreeChild::Tree {
                    value: pattern_value,
                },
                dir::TreeChild::Tree {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (dir::TreeChild::Empty, dir::TreeChild::Empty)
            | (dir::TreeChild::Error, dir::TreeChild::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match two optional tree attribute values.
    fn match_optional_tree_attribute_value(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<&dir::TreeAttributeValue>,
        candidate: Option<&dir::TreeAttributeValue>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (
                Some(dir::TreeAttributeValue::String(pattern)),
                Some(dir::TreeAttributeValue::String(candidate)),
            ) => Ok(pattern == candidate),
            (
                Some(dir::TreeAttributeValue::Expression(pattern)),
                Some(dir::TreeAttributeValue::Expression(candidate)),
            ) => self.match_expression(nodes, *pattern, *candidate, bindings),
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one template literal.
    pub(crate) fn match_template_literal(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::TemplateLiteral,
        candidate: &dir::TemplateLiteral,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (
                dir::TemplateLiteral::String { chunk: pattern },
                dir::TemplateLiteral::String { chunk: candidate },
            ) => Ok(pattern == candidate),
            (
                dir::TemplateLiteral::InterpolatedString {
                    chunks: pattern_chunks,
                    arguments: pattern_arguments,
                },
                dir::TemplateLiteral::InterpolatedString {
                    chunks: candidate_chunks,
                    arguments: candidate_arguments,
                },
            ) => {
                if pattern_chunks != candidate_chunks {
                    return Ok(false);
                }

                self.match_nodes(
                    nodes,
                    pattern_arguments,
                    candidate_arguments,
                    0,
                    0,
                    bindings,
                )
            }
            _ => Ok(false),
        }
    }
}
