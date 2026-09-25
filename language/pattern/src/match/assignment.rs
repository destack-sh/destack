use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one assignment pattern.
    pub(crate) fn match_assign_pattern(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::AssignPattern>,
        candidate_id: dir::LocalNodeId<dir::AssignPattern>,
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
                dir::AssignPattern::Place {
                    expression: pattern,
                },
                dir::AssignPattern::Place {
                    expression: candidate,
                },
            ) => self.match_expression(nodes, *pattern, *candidate, bindings),
            (
                dir::AssignPattern::Default {
                    pattern: pattern_pattern,
                    value: pattern_value,
                },
                dir::AssignPattern::Default {
                    pattern: candidate_pattern,
                    value: candidate_value,
                },
            ) => {
                if !self.match_assign_pattern(
                    nodes,
                    *pattern_pattern,
                    *candidate_pattern,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::AssignPattern::Sequence {
                    fields: pattern_fields,
                },
                dir::AssignPattern::Sequence {
                    fields: candidate_fields,
                },
            )
            | (
                dir::AssignPattern::Tuple {
                    fields: pattern_fields,
                },
                dir::AssignPattern::Tuple {
                    fields: candidate_fields,
                },
            )
            | (
                dir::AssignPattern::Object {
                    fields: pattern_fields,
                },
                dir::AssignPattern::Object {
                    fields: candidate_fields,
                },
            ) => self.match_nodes(nodes, pattern_fields, candidate_fields, 0, 0, bindings),
            _ => Ok(false),
        }
    }

    /// Match one assignment pattern field.
    pub(crate) fn match_assign_pattern_field(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::AssignPatternField>,
        candidate_id: dir::LocalNodeId<dir::AssignPatternField>,
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
                dir::AssignPatternField::Named {
                    name: pattern_name,
                    pattern: pattern_pattern,
                    is_shorthand: pattern_shorthand,
                },
                dir::AssignPatternField::Named {
                    name: candidate_name,
                    pattern: candidate_pattern,
                    is_shorthand: candidate_shorthand,
                },
            ) => {
                if pattern_shorthand != candidate_shorthand
                    || !self.match_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_assign_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings)
            }
            (
                dir::AssignPatternField::Computed {
                    key: pattern_key,
                    pattern: pattern_pattern,
                },
                dir::AssignPatternField::Computed {
                    key: candidate_key,
                    pattern: candidate_pattern,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_key, *candidate_key, bindings)? {
                    return Ok(false);
                }

                self.match_assign_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings)
            }
            (
                dir::AssignPatternField::Positional {
                    pattern: pattern_pattern,
                },
                dir::AssignPatternField::Positional {
                    pattern: candidate_pattern,
                },
            ) => self.match_assign_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings),
            (
                dir::AssignPatternField::Rest {
                    pattern: pattern_pattern,
                },
                dir::AssignPatternField::Rest {
                    pattern: candidate_pattern,
                },
            ) => self.match_optional_assign_pattern(
                nodes,
                *pattern_pattern,
                *candidate_pattern,
                bindings,
            ),
            (dir::AssignPatternField::Elision, dir::AssignPatternField::Elision) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match two optional assignment patterns.
    fn match_optional_assign_pattern(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<dir::AssignPattern>>,
        candidate: Option<dir::LocalNodeId<dir::AssignPattern>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_assign_pattern(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }
}
