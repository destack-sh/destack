use tspp_dir as dir;
use tspp_source::NodeSpanType;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one generic parameter.
    pub(crate) fn match_generic_parameter(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::GenericParameter>,
        candidate_id: dir::LocalNodeId<dir::GenericParameter>,
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
                dir::GenericParameter::Type {
                    name: pattern_name,
                    variance: pattern_variance,
                    constraint: pattern_constraint,
                    default: pattern_default,
                    is_const: pattern_const,
                },
                dir::GenericParameter::Type {
                    name: candidate_name,
                    variance: candidate_variance,
                    constraint: candidate_constraint,
                    default: candidate_default,
                    is_const: candidate_const,
                },
            )
            | (
                dir::GenericParameter::VariadicType {
                    name: pattern_name,
                    variance: pattern_variance,
                    constraint: pattern_constraint,
                    default: pattern_default,
                    is_const: pattern_const,
                },
                dir::GenericParameter::VariadicType {
                    name: candidate_name,
                    variance: candidate_variance,
                    constraint: candidate_constraint,
                    default: candidate_default,
                    is_const: candidate_const,
                },
            ) => {
                if pattern_variance != candidate_variance
                    || pattern_const != candidate_const
                    || !self.match_node_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        Some(*pattern_name),
                        Some(*candidate_name),
                        bindings,
                    )?
                    || !self.match_optional_type_expression(
                        nodes,
                        *pattern_constraint,
                        *candidate_constraint,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_type_expression(
                    nodes,
                    *pattern_default,
                    *candidate_default,
                    bindings,
                )
            }
            (
                dir::GenericParameter::Lifetime { name: pattern_name },
                dir::GenericParameter::Lifetime {
                    name: candidate_name,
                },
            ) => self.match_node_name(
                nodes,
                pattern_any,
                candidate_id.into_any(),
                Some(*pattern_name),
                Some(*candidate_name),
                bindings,
            ),
            (dir::GenericParameter::Error, dir::GenericParameter::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one callable parameter.
    pub(crate) fn match_parameter(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Parameter>,
        candidate_id: dir::LocalNodeId<dir::Parameter>,
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
                dir::Parameter::Named {
                    name: pattern_name,
                    declared_type: pattern_type,
                    default: pattern_default,
                    is_optional: pattern_optional,
                },
                dir::Parameter::Named {
                    name: candidate_name,
                    declared_type: candidate_type,
                    default: candidate_default,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_optional != candidate_optional
                    || !self.match_node_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        Some(*pattern_name),
                        Some(*candidate_name),
                        bindings,
                    )?
                    || !self.match_optional_type_expression(
                        nodes,
                        *pattern_type,
                        *candidate_type,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_expression(
                    nodes,
                    *pattern_default,
                    *candidate_default,
                    bindings,
                )
            }
            (
                dir::Parameter::Pattern {
                    pattern: pattern_pattern,
                    declared_type: pattern_type,
                    default: pattern_default,
                    is_optional: pattern_optional,
                },
                dir::Parameter::Pattern {
                    pattern: candidate_pattern,
                    declared_type: candidate_type,
                    default: candidate_default,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_optional != candidate_optional
                    || !self.match_pattern_node(
                        nodes,
                        *pattern_pattern,
                        *candidate_pattern,
                        bindings,
                    )?
                    || !self.match_optional_type_expression(
                        nodes,
                        *pattern_type,
                        *candidate_type,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_expression(
                    nodes,
                    *pattern_default,
                    *candidate_default,
                    bindings,
                )
            }
            (
                dir::Parameter::VariadicNamed {
                    name: pattern_name,
                    declared_type: pattern_type,
                },
                dir::Parameter::VariadicNamed {
                    name: candidate_name,
                    declared_type: candidate_type,
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

                self.match_optional_type_expression(nodes, *pattern_type, *candidate_type, bindings)
            }
            (
                dir::Parameter::VariadicPattern {
                    pattern: pattern_pattern,
                    declared_type: pattern_type,
                },
                dir::Parameter::VariadicPattern {
                    pattern: candidate_pattern,
                    declared_type: candidate_type,
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

                self.match_optional_type_expression(nodes, *pattern_type, *candidate_type, bindings)
            }
            (dir::Parameter::Error, dir::Parameter::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one tuple type element.
    pub(crate) fn match_tuple_element(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TupleElement>,
        candidate_id: dir::LocalNodeId<dir::TupleElement>,
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
                dir::TupleElement::Element {
                    label: pattern_label,
                    value: pattern_value,
                    is_optional: pattern_optional,
                    is_readonly: pattern_readonly,
                },
                dir::TupleElement::Element {
                    label: candidate_label,
                    value: candidate_value,
                    is_optional: candidate_optional,
                    is_readonly: candidate_readonly,
                },
            ) => {
                if pattern_optional != candidate_optional
                    || pattern_readonly != candidate_readonly
                    || !self.match_node_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_label,
                        *candidate_label,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::TupleElement::Spread {
                    label: pattern_label,
                    value: pattern_value,
                },
                dir::TupleElement::Spread {
                    label: candidate_label,
                    value: candidate_value,
                },
            ) => {
                if !self.match_node_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    *pattern_label,
                    *candidate_label,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (dir::TupleElement::Error, dir::TupleElement::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one name stored directly on a DIR node.
    pub(crate) fn match_node_name(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: dir::LocalNodeIdAny,
        candidate: dir::LocalNodeIdAny,
        pattern_name: Option<tspp_core::StringId>,
        candidate_name: Option<tspp_core::StringId>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let use_entry = nodes.uses().get_name(pattern, NodeSpanType::Main);
        match use_entry {
            Some(use_entry) => self.bind_name(use_entry, candidate, candidate_name, bindings),
            None => Ok(pattern_name == candidate_name),
        }
    }
}
