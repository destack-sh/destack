use destack_dir as dir;
use destack_source::NodeSpanType;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match two optional type expressions.
    pub(crate) fn match_optional_type_expression(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<dir::TypeExpression>>,
        candidate: Option<dir::LocalNodeId<dir::TypeExpression>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_type_expression(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one type expression.
    pub(crate) fn match_type_expression(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TypeExpression>,
        candidate_id: dir::LocalNodeId<dir::TypeExpression>,
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
                dir::TypeExpression::Literal {
                    value: pattern_value,
                },
                dir::TypeExpression::Literal {
                    value: candidate_value,
                },
            ) => Ok(pattern_value == candidate_value),
            (
                dir::TypeExpression::Keyword {
                    value: pattern_value,
                },
                dir::TypeExpression::Keyword {
                    value: candidate_value,
                },
            ) => Ok(pattern_value == candidate_value),
            (
                dir::TypeExpression::Lifetime { name: pattern_name },
                dir::TypeExpression::Lifetime {
                    name: candidate_name,
                },
            ) => Ok(pattern_name == candidate_name),
            (dir::TypeExpression::Intrinsic, dir::TypeExpression::Intrinsic)
            | (dir::TypeExpression::Const, dir::TypeExpression::Const)
            | (dir::TypeExpression::This, dir::TypeExpression::This)
            | (dir::TypeExpression::Missing, dir::TypeExpression::Missing)
            | (dir::TypeExpression::Error, dir::TypeExpression::Error) => Ok(true),
            (
                dir::TypeExpression::Tuple {
                    form: pattern_form,
                    elements: pattern_elements,
                },
                dir::TypeExpression::Tuple {
                    form: candidate_form,
                    elements: candidate_elements,
                },
            ) => {
                if pattern_form != candidate_form {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_elements, candidate_elements, 0, 0, bindings)
            }
            (
                dir::TypeExpression::Array {
                    element: pattern_element,
                },
                dir::TypeExpression::Array {
                    element: candidate_element,
                },
            )
            | (
                dir::TypeExpression::Slice {
                    element: pattern_element,
                },
                dir::TypeExpression::Slice {
                    element: candidate_element,
                },
            ) => self.match_type_expression(nodes, *pattern_element, *candidate_element, bindings),
            (
                dir::TypeExpression::FixedArray {
                    element: pattern_element,
                    length: pattern_length,
                },
                dir::TypeExpression::FixedArray {
                    element: candidate_element,
                    length: candidate_length,
                },
            ) => {
                if !self.match_type_expression(
                    nodes,
                    *pattern_element,
                    *candidate_element,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_expression(nodes, *pattern_length, *candidate_length, bindings)
            }
            (
                dir::TypeExpression::Object {
                    members: pattern_members,
                },
                dir::TypeExpression::Object {
                    members: candidate_members,
                },
            ) => self.match_nodes(nodes, pattern_members, candidate_members, 0, 0, bindings),
            (dir::TypeExpression::Function(pattern), dir::TypeExpression::Function(candidate)) => {
                self.match_function_type(nodes, pattern, candidate, bindings)
            }
            (
                dir::TypeExpression::Constructor(pattern),
                dir::TypeExpression::Constructor(candidate),
            ) => self.match_constructor_type(nodes, pattern, candidate, bindings),
            (
                dir::TypeExpression::Reference {
                    path: pattern_path,
                    generic_arguments: pattern_arguments,
                },
                dir::TypeExpression::Reference {
                    path: candidate_path,
                    generic_arguments: candidate_arguments,
                },
            ) => {
                if pattern_path != candidate_path {
                    return Ok(false);
                }

                self.match_generic_arguments(
                    nodes,
                    pattern_arguments,
                    candidate_arguments,
                    bindings,
                )
            }
            (
                dir::TypeExpression::Member {
                    left: pattern_left,
                    name: pattern_name,
                    generic_arguments: pattern_arguments,
                },
                dir::TypeExpression::Member {
                    left: candidate_left,
                    name: candidate_name,
                    generic_arguments: candidate_arguments,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }
                let use_entry = nodes.uses().get_name(pattern_any, NodeSpanType::Main);
                let is_name_match = match use_entry {
                    Some(use_entry) => self.bind_name(
                        use_entry,
                        candidate_id.into_any(),
                        Some(*candidate_name),
                        bindings,
                    )?,
                    None => pattern_name == candidate_name,
                };
                if !is_name_match {
                    return Ok(false);
                }

                self.match_generic_arguments(
                    nodes,
                    pattern_arguments,
                    candidate_arguments,
                    bindings,
                )
            }
            (
                dir::TypeExpression::Range {
                    start: pattern_start,
                    end: pattern_end,
                    end_kind: pattern_end_kind,
                },
                dir::TypeExpression::Range {
                    start: candidate_start,
                    end: candidate_end,
                    end_kind: candidate_end_kind,
                },
            ) => {
                if pattern_end_kind != candidate_end_kind
                    || !self.match_optional_type_expression(
                        nodes,
                        *pattern_start,
                        *candidate_start,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_type_expression(nodes, *pattern_end, *candidate_end, bindings)
            }
            (
                dir::TypeExpression::Readonly {
                    target_type: pattern_target,
                },
                dir::TypeExpression::Readonly {
                    target_type: candidate_target,
                },
            )
            | (
                dir::TypeExpression::KeyOf {
                    target_type: pattern_target,
                },
                dir::TypeExpression::KeyOf {
                    target_type: candidate_target,
                },
            )
            | (
                dir::TypeExpression::Must {
                    target_type: pattern_target,
                },
                dir::TypeExpression::Must {
                    target_type: candidate_target,
                },
            )
            | (
                dir::TypeExpression::Not {
                    target_type: pattern_target,
                },
                dir::TypeExpression::Not {
                    target_type: candidate_target,
                },
            ) => self.match_type_expression(nodes, *pattern_target, *candidate_target, bindings),
            (
                dir::TypeExpression::TypeOf {
                    value: pattern_value,
                },
                dir::TypeExpression::TypeOf {
                    value: candidate_value,
                },
            )
            | (
                dir::TypeExpression::StaticValue {
                    expression: pattern_value,
                },
                dir::TypeExpression::StaticValue {
                    expression: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (
                dir::TypeExpression::OwnedOf {
                    mutability: pattern_mutability,
                    variance: pattern_variance,
                    target_type: pattern_target,
                },
                dir::TypeExpression::OwnedOf {
                    mutability: candidate_mutability,
                    variance: candidate_variance,
                    target_type: candidate_target,
                },
            ) => {
                if pattern_mutability != candidate_mutability
                    || pattern_variance != candidate_variance
                {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_target, *candidate_target, bindings)
            }
            (
                dir::TypeExpression::BorrowedOf {
                    lifetime: pattern_lifetime,
                    access: pattern_access,
                    variance: pattern_variance,
                    target_type: pattern_target,
                },
                dir::TypeExpression::BorrowedOf {
                    lifetime: candidate_lifetime,
                    access: candidate_access,
                    variance: candidate_variance,
                    target_type: candidate_target,
                },
            ) => {
                if pattern_access != candidate_access
                    || pattern_variance != candidate_variance
                    || !self.match_optional_type_expression(
                        nodes,
                        *pattern_lifetime,
                        *candidate_lifetime,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_target, *candidate_target, bindings)
            }
            (
                dir::TypeExpression::PointerOf {
                    mutability: pattern_mutability,
                    target_type: pattern_target,
                },
                dir::TypeExpression::PointerOf {
                    mutability: candidate_mutability,
                    target_type: candidate_target,
                },
            ) => {
                if pattern_mutability != candidate_mutability {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_target, *candidate_target, bindings)
            }
            (
                dir::TypeExpression::Union {
                    elements: pattern_elements,
                },
                dir::TypeExpression::Union {
                    elements: candidate_elements,
                },
            )
            | (
                dir::TypeExpression::Intersection {
                    elements: pattern_elements,
                },
                dir::TypeExpression::Intersection {
                    elements: candidate_elements,
                },
            ) => self.match_nodes(nodes, pattern_elements, candidate_elements, 0, 0, bindings),
            (
                dir::TypeExpression::Conditional {
                    left: pattern_left,
                    extends_type: pattern_extends,
                    then_type: pattern_then,
                    else_type: pattern_else,
                },
                dir::TypeExpression::Conditional {
                    left: candidate_left,
                    extends_type: candidate_extends,
                    then_type: candidate_then,
                    else_type: candidate_else,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_left, *candidate_left, bindings)?
                    || !self.match_type_expression(
                        nodes,
                        *pattern_extends,
                        *candidate_extends,
                        bindings,
                    )?
                    || !self.match_type_expression(
                        nodes,
                        *pattern_then,
                        *candidate_then,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_else, *candidate_else, bindings)
            }
            (
                dir::TypeExpression::Extends {
                    left: pattern_left,
                    right: pattern_right,
                },
                dir::TypeExpression::Extends {
                    left: candidate_left,
                    right: candidate_right,
                },
            )
            | (
                dir::TypeExpression::Implements {
                    left: pattern_left,
                    right: pattern_right,
                },
                dir::TypeExpression::Implements {
                    left: candidate_left,
                    right: candidate_right,
                },
            )
            | (
                dir::TypeExpression::Index {
                    left: pattern_left,
                    index: pattern_right,
                },
                dir::TypeExpression::Index {
                    left: candidate_left,
                    index: candidate_right,
                },
            ) => {
                if !self.match_type_expression(nodes, *pattern_left, *candidate_left, bindings)? {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_right, *candidate_right, bindings)
            }
            (
                dir::TypeExpression::Mapped {
                    parameter: pattern_parameter,
                    readonly: pattern_readonly,
                    optional: pattern_optional,
                    value: pattern_value,
                },
                dir::TypeExpression::Mapped {
                    parameter: candidate_parameter,
                    readonly: candidate_readonly,
                    optional: candidate_optional,
                    value: candidate_value,
                },
            ) => {
                if pattern_readonly != candidate_readonly
                    || pattern_optional != candidate_optional
                    || !self.match_type_mapped_parameter(
                        nodes,
                        *pattern_parameter,
                        *candidate_parameter,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_type_expression(
                    nodes,
                    *pattern_value,
                    *candidate_value,
                    bindings,
                )
            }
            (
                dir::TypeExpression::TemplateLiteral {
                    strings: pattern_strings,
                    spans: pattern_spans,
                },
                dir::TypeExpression::TemplateLiteral {
                    strings: candidate_strings,
                    spans: candidate_spans,
                },
            ) => {
                if pattern_strings != candidate_strings {
                    return Ok(false);
                }

                self.match_nodes(nodes, pattern_spans, candidate_spans, 0, 0, bindings)
            }
            (
                dir::TypeExpression::Infer {
                    form: pattern_form,
                    name: pattern_name,
                    constraint: pattern_constraint,
                },
                dir::TypeExpression::Infer {
                    form: candidate_form,
                    name: candidate_name,
                    constraint: candidate_constraint,
                },
            ) => {
                if pattern_form != candidate_form
                    || !self.match_node_name(
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

                self.match_optional_type_expression(
                    nodes,
                    *pattern_constraint,
                    *candidate_constraint,
                    bindings,
                )
            }
            _ => Ok(false),
        }
    }

    /// Match one mapped type parameter.
    pub(crate) fn match_type_mapped_parameter(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TypeMappedParameter>,
        candidate_id: dir::LocalNodeId<dir::TypeMappedParameter>,
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
        if !self.match_node_name(
            nodes,
            pattern_any,
            candidate_id.into_any(),
            Some(pattern.name),
            Some(candidate.name),
            bindings,
        )? || !self.match_type_expression(
            nodes,
            pattern.source_type,
            candidate.source_type,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_optional_type_expression(nodes, pattern.key_remap, candidate.key_remap, bindings)
    }
}
