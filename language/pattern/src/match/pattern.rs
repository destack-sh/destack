use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one destructuring or selection pattern.
    pub(crate) fn match_pattern_node(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        candidate_id: dir::LocalNodeId<dir::Pattern>,
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
            (dir::Pattern::Wildcard, dir::Pattern::Wildcard) => Ok(true),
            (dir::Pattern::Must(pattern), dir::Pattern::Must(candidate)) => {
                self.match_pattern_node(nodes, *pattern, *candidate, bindings)
            }
            (
                dir::Pattern::Default {
                    pattern: pattern_pattern,
                    value: pattern_value,
                },
                dir::Pattern::Default {
                    pattern: candidate_pattern,
                    value: candidate_value,
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

                self.match_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Pattern::BorrowOf {
                    access: pattern_access,
                    right: pattern_right,
                },
                dir::Pattern::BorrowOf {
                    access: candidate_access,
                    right: candidate_right,
                },
            ) => {
                if pattern_access != candidate_access {
                    return Ok(false);
                }

                self.match_pattern_node(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::Pattern::MoveOf {
                    mutability: pattern_mutability,
                    right: pattern_right,
                },
                dir::Pattern::MoveOf {
                    mutability: candidate_mutability,
                    right: candidate_right,
                },
            ) => {
                if pattern_mutability != candidate_mutability {
                    return Ok(false);
                }

                self.match_pattern_node(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::Pattern::DereferenceOf {
                    right: pattern_right,
                },
                dir::Pattern::DereferenceOf {
                    right: candidate_right,
                },
            ) => self.match_pattern_node(nodes, *pattern_right, *candidate_right, bindings),
            (
                dir::Pattern::Binding {
                    name: pattern_name,
                    pattern: pattern_pattern,
                },
                dir::Pattern::Binding {
                    name: candidate_name,
                    pattern: candidate_pattern,
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

                self.match_optional_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings)
            }
            (
                dir::Pattern::Expression {
                    value: pattern_value,
                },
                dir::Pattern::Expression {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::Pattern::Range {
                    start: pattern_start,
                    end: pattern_end,
                    end_kind: pattern_end_kind,
                },
                dir::Pattern::Range {
                    start: candidate_start,
                    end: candidate_end,
                    end_kind: candidate_end_kind,
                },
            ) => {
                if pattern_end_kind != candidate_end_kind
                    || !self.match_optional_expression(
                        nodes,
                        *pattern_start,
                        *candidate_start,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_expression(nodes, *pattern_end, *candidate_end, bindings)
            }
            (
                dir::Pattern::Tuple {
                    fields: pattern_fields,
                },
                dir::Pattern::Tuple {
                    fields: candidate_fields,
                },
            )
            | (
                dir::Pattern::Sequence {
                    fields: pattern_fields,
                },
                dir::Pattern::Sequence {
                    fields: candidate_fields,
                },
            )
            | (
                dir::Pattern::Object {
                    fields: pattern_fields,
                },
                dir::Pattern::Object {
                    fields: candidate_fields,
                },
            ) => self.match_pattern_fields(nodes, pattern_fields, candidate_fields, bindings),
            (
                dir::Pattern::NominalTuple {
                    ty: pattern_type,
                    fields: pattern_fields,
                },
                dir::Pattern::NominalTuple {
                    ty: candidate_type,
                    fields: candidate_fields,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_type, *candidate_type, bindings)? {
                    return Ok(false);
                }

                self.match_pattern_fields(nodes, pattern_fields, candidate_fields, bindings)
            }
            (
                dir::Pattern::NominalObject {
                    ty: pattern_type,
                    fields: pattern_fields,
                },
                dir::Pattern::NominalObject {
                    ty: candidate_type,
                    fields: candidate_fields,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_type, *candidate_type, bindings)? {
                    return Ok(false);
                }

                self.match_pattern_fields(nodes, pattern_fields, candidate_fields, bindings)
            }
            (
                dir::Pattern::Union {
                    patterns: pattern_patterns,
                },
                dir::Pattern::Union {
                    patterns: candidate_patterns,
                },
            ) => self.match_patterns(nodes, pattern_patterns, candidate_patterns, bindings),
            _ => Ok(false),
        }
    }

    /// Match two optional patterns.
    pub(crate) fn match_optional_pattern(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<dir::Pattern>>,
        candidate: Option<dir::LocalNodeId<dir::Pattern>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_pattern_node(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one ordered list of pattern fields.
    fn match_pattern_fields(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: &[dir::LocalNodeId<dir::PatternField>],
        candidates: &[dir::LocalNodeId<dir::PatternField>],
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_nodes(nodes, patterns, candidates, 0, 0, bindings)
    }

    /// Match one ordered list of patterns.
    fn match_patterns(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
        candidates: &[dir::LocalNodeId<dir::Pattern>],
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_nodes(nodes, patterns, candidates, 0, 0, bindings)
    }

    /// Match one destructuring pattern field.
    pub(crate) fn match_pattern_field(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::PatternField>,
        candidate_id: dir::LocalNodeId<dir::PatternField>,
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
                dir::PatternField::Named {
                    name: pattern_name,
                    pattern: pattern_pattern,
                    is_shorthand: pattern_shorthand,
                },
                dir::PatternField::Named {
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

                self.match_optional_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings)
            }
            (
                dir::PatternField::Computed {
                    key: pattern_key,
                    pattern: pattern_pattern,
                },
                dir::PatternField::Computed {
                    key: candidate_key,
                    pattern: candidate_pattern,
                },
            ) => {
                if !self.match_expression(nodes, *pattern_key, *candidate_key, bindings)? {
                    return Ok(false);
                }

                self.match_pattern_node(nodes, *pattern_pattern, *candidate_pattern, bindings)
            }
            (
                dir::PatternField::Positional {
                    pattern: pattern_pattern,
                },
                dir::PatternField::Positional {
                    pattern: candidate_pattern,
                },
            ) => self.match_pattern_node(nodes, *pattern_pattern, *candidate_pattern, bindings),
            (
                dir::PatternField::Rest {
                    pattern: pattern_pattern,
                },
                dir::PatternField::Rest {
                    pattern: candidate_pattern,
                },
            ) => self.match_optional_pattern(nodes, *pattern_pattern, *candidate_pattern, bindings),
            (dir::PatternField::Elision, dir::PatternField::Elision) => Ok(true),
            _ => Ok(false),
        }
    }
}
