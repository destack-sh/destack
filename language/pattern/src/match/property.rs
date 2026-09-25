use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one object property node.
    pub(crate) fn match_property(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Property>,
        candidate_id: dir::LocalNodeId<dir::Property>,
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
                dir::Property::Field {
                    name: pattern_name,
                    value: pattern_value,
                    is_shorthand: pattern_shorthand,
                },
                dir::Property::Field {
                    name: candidate_name,
                    value: candidate_value,
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

                self.match_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Property::Method {
                    name: pattern_name,
                    signature: pattern_signature,
                    body: pattern_body,
                },
                dir::Property::Method {
                    name: candidate_name,
                    signature: candidate_signature,
                    body: candidate_body,
                },
            ) => {
                if !self.match_optional_name(
                    nodes,
                    pattern_any,
                    candidate_id.into_any(),
                    *pattern_name,
                    *candidate_name,
                    bindings,
                )? || !self.match_function_signature(
                    nodes,
                    pattern_signature,
                    candidate_signature,
                    bindings,
                )? {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::Property::Spread {
                    value: pattern_value,
                },
                dir::Property::Spread {
                    value: candidate_value,
                },
            ) => self.match_expression(nodes, *pattern_value, *candidate_value, bindings),
            (dir::Property::Error, dir::Property::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one declaration member node.
    pub(crate) fn match_member(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Member>,
        candidate_id: dir::LocalNodeId<dir::Member>,
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
                dir::Member::AssociatedType {
                    name: pattern_name,
                    generic_parameters: pattern_generic_parameters,
                    where_clauses: pattern_where_clauses,
                    constraint: pattern_constraint,
                    value: pattern_value,
                    visibility: pattern_visibility,
                    is_ambient: pattern_ambient,
                    is_abstract: pattern_abstract,
                    is_override: pattern_override,
                },
                dir::Member::AssociatedType {
                    name: candidate_name,
                    generic_parameters: candidate_generic_parameters,
                    where_clauses: candidate_where_clauses,
                    constraint: candidate_constraint,
                    value: candidate_value,
                    visibility: candidate_visibility,
                    is_ambient: candidate_ambient,
                    is_abstract: candidate_abstract,
                    is_override: candidate_override,
                },
            ) => {
                if pattern_visibility != candidate_visibility
                    || pattern_ambient != candidate_ambient
                    || pattern_abstract != candidate_abstract
                    || pattern_override != candidate_override
                    || !self.match_string_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_nodes(
                        nodes,
                        pattern_generic_parameters,
                        candidate_generic_parameters,
                        0,
                        0,
                        bindings,
                    )?
                    || !self.match_nodes(
                        nodes,
                        pattern_where_clauses,
                        candidate_where_clauses,
                        0,
                        0,
                        bindings,
                    )?
                    || !self.match_optional_node(
                        nodes,
                        *pattern_constraint,
                        *candidate_constraint,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Member::AssociatedConst {
                    name: pattern_name,
                    declared_type: pattern_type,
                    value: pattern_value,
                    visibility: pattern_visibility,
                    is_ambient: pattern_ambient,
                    is_abstract: pattern_abstract,
                    is_override: pattern_override,
                },
                dir::Member::AssociatedConst {
                    name: candidate_name,
                    declared_type: candidate_type,
                    value: candidate_value,
                    visibility: candidate_visibility,
                    is_ambient: candidate_ambient,
                    is_abstract: candidate_abstract,
                    is_override: candidate_override,
                },
            ) => {
                if pattern_visibility != candidate_visibility
                    || pattern_ambient != candidate_ambient
                    || pattern_abstract != candidate_abstract
                    || pattern_override != candidate_override
                    || !self.match_string_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_optional_node(nodes, *pattern_type, *candidate_type, bindings)?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::Member::Field {
                    name: pattern_name,
                    declared_type: pattern_type,
                    default: pattern_default,
                    mutability: pattern_mutability,
                    visibility: pattern_visibility,
                    is_optional: pattern_optional,
                    is_readonly: pattern_readonly,
                    is_ambient: pattern_ambient,
                    is_abstract: pattern_abstract,
                    is_override: pattern_override,
                    is_static: pattern_static,
                    is_accessor: pattern_accessor,
                },
                dir::Member::Field {
                    name: candidate_name,
                    declared_type: candidate_type,
                    default: candidate_default,
                    mutability: candidate_mutability,
                    visibility: candidate_visibility,
                    is_optional: candidate_optional,
                    is_readonly: candidate_readonly,
                    is_ambient: candidate_ambient,
                    is_abstract: candidate_abstract,
                    is_override: candidate_override,
                    is_static: candidate_static,
                    is_accessor: candidate_accessor,
                },
            ) => {
                if pattern_mutability != candidate_mutability
                    || pattern_visibility != candidate_visibility
                    || pattern_optional != candidate_optional
                    || pattern_readonly != candidate_readonly
                    || pattern_ambient != candidate_ambient
                    || pattern_abstract != candidate_abstract
                    || pattern_override != candidate_override
                    || pattern_static != candidate_static
                    || pattern_accessor != candidate_accessor
                    || !self.match_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_optional_node(nodes, *pattern_type, *candidate_type, bindings)?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_default, *candidate_default, bindings)
            }
            (
                dir::Member::Method {
                    name: pattern_name,
                    signature: pattern_signature,
                    abstraction: pattern_abstraction,
                    body: pattern_body,
                    visibility: pattern_visibility,
                    is_optional: pattern_optional,
                    is_ambient: pattern_ambient,
                    is_override: pattern_override,
                    is_static: pattern_static,
                    is_accessor: pattern_accessor,
                },
                dir::Member::Method {
                    name: candidate_name,
                    signature: candidate_signature,
                    abstraction: candidate_abstraction,
                    body: candidate_body,
                    visibility: candidate_visibility,
                    is_optional: candidate_optional,
                    is_ambient: candidate_ambient,
                    is_override: candidate_override,
                    is_static: candidate_static,
                    is_accessor: candidate_accessor,
                },
            ) => {
                if pattern_abstraction != candidate_abstraction
                    || pattern_visibility != candidate_visibility
                    || pattern_optional != candidate_optional
                    || pattern_ambient != candidate_ambient
                    || pattern_override != candidate_override
                    || pattern_static != candidate_static
                    || pattern_accessor != candidate_accessor
                    || !self.match_optional_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_function_signature(
                        nodes,
                        pattern_signature,
                        candidate_signature,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::Member::StaticBlock { body: pattern },
                dir::Member::StaticBlock { body: candidate },
            )
            | (
                dir::Member::ConstBlock { body: pattern },
                dir::Member::ConstBlock { body: candidate },
            ) => self.match_expression(nodes, *pattern, *candidate, bindings),
            (dir::Member::Error, dir::Member::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one type member node.
    pub(crate) fn match_type_member(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::TypeMember>,
        candidate_id: dir::LocalNodeId<dir::TypeMember>,
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
                dir::TypeMember::Field {
                    name: pattern_name,
                    declared_type: pattern_type,
                    visibility: pattern_visibility,
                    is_static: pattern_static,
                    is_optional: pattern_optional,
                    is_readonly: pattern_readonly,
                },
                dir::TypeMember::Field {
                    name: candidate_name,
                    declared_type: candidate_type,
                    visibility: candidate_visibility,
                    is_static: candidate_static,
                    is_optional: candidate_optional,
                    is_readonly: candidate_readonly,
                },
            ) => {
                if pattern_static != candidate_static
                    || pattern_optional != candidate_optional
                    || pattern_readonly != candidate_readonly
                    || pattern_visibility != candidate_visibility
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

                self.match_optional_node(nodes, *pattern_type, *candidate_type, bindings)
            }
            (
                dir::TypeMember::Method {
                    name: pattern_name,
                    signature: pattern_signature,
                    body: pattern_body,
                    visibility: pattern_visibility,
                    is_static: pattern_static,
                    is_optional: pattern_optional,
                },
                dir::TypeMember::Method {
                    name: candidate_name,
                    signature: candidate_signature,
                    body: candidate_body,
                    visibility: candidate_visibility,
                    is_static: candidate_static,
                    is_optional: candidate_optional,
                },
            ) => {
                if pattern_static != candidate_static
                    || pattern_optional != candidate_optional
                    || pattern_visibility != candidate_visibility
                    || !self.match_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_function_signature(
                        nodes,
                        pattern_signature,
                        candidate_signature,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_body, *candidate_body, bindings)
            }
            (
                dir::TypeMember::CallSignature { signature: pattern },
                dir::TypeMember::CallSignature {
                    signature: candidate,
                },
            ) => self.match_function_type(nodes, pattern, candidate, bindings),
            (
                dir::TypeMember::ConstructSignature { signature: pattern },
                dir::TypeMember::ConstructSignature {
                    signature: candidate,
                },
            ) => self.match_constructor_type(nodes, pattern, candidate, bindings),
            (
                dir::TypeMember::IndexSignature {
                    name: pattern_name,
                    key_type: pattern_key,
                    value_type: pattern_value,
                    is_optional: pattern_optional,
                    is_readonly: pattern_readonly,
                },
                dir::TypeMember::IndexSignature {
                    name: candidate_name,
                    key_type: candidate_key,
                    value_type: candidate_value,
                    is_optional: candidate_optional,
                    is_readonly: candidate_readonly,
                },
            ) => {
                if pattern_optional != candidate_optional
                    || pattern_readonly != candidate_readonly
                    || !self.match_string_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_type_expression(nodes, *pattern_key, *candidate_key, bindings)?
                {
                    return Ok(false);
                }

                self.match_type_expression(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::TypeMember::AssociatedType {
                    name: pattern_name,
                    generic_parameters: pattern_generic_parameters,
                    where_clauses: pattern_where_clauses,
                    constraint: pattern_constraint,
                    value: pattern_value,
                    is_abstract: pattern_abstract,
                    is_override: pattern_override,
                },
                dir::TypeMember::AssociatedType {
                    name: candidate_name,
                    generic_parameters: candidate_generic_parameters,
                    where_clauses: candidate_where_clauses,
                    constraint: candidate_constraint,
                    value: candidate_value,
                    is_abstract: candidate_abstract,
                    is_override: candidate_override,
                },
            ) => {
                if pattern_abstract != candidate_abstract
                    || pattern_override != candidate_override
                    || !self.match_string_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_nodes(
                        nodes,
                        pattern_generic_parameters,
                        candidate_generic_parameters,
                        0,
                        0,
                        bindings,
                    )?
                    || !self.match_nodes(
                        nodes,
                        pattern_where_clauses,
                        candidate_where_clauses,
                        0,
                        0,
                        bindings,
                    )?
                    || !self.match_optional_node(
                        nodes,
                        *pattern_constraint,
                        *candidate_constraint,
                        bindings,
                    )?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_value, *candidate_value, bindings)
            }
            (
                dir::TypeMember::AssociatedConst {
                    name: pattern_name,
                    declared_type: pattern_type,
                    value: pattern_value,
                    is_abstract: pattern_abstract,
                    is_override: pattern_override,
                },
                dir::TypeMember::AssociatedConst {
                    name: candidate_name,
                    declared_type: candidate_type,
                    value: candidate_value,
                    is_abstract: candidate_abstract,
                    is_override: candidate_override,
                },
            ) => {
                if pattern_abstract != candidate_abstract
                    || pattern_override != candidate_override
                    || !self.match_string_name(
                        nodes,
                        pattern_any,
                        candidate_id.into_any(),
                        *pattern_name,
                        *candidate_name,
                        bindings,
                    )?
                    || !self.match_optional_node(nodes, *pattern_type, *candidate_type, bindings)?
                {
                    return Ok(false);
                }

                self.match_optional_node(nodes, *pattern_value, *candidate_value, bindings)
            }
            (dir::TypeMember::Error, dir::TypeMember::Error) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one string name stored on a containing DIR node.
    fn match_string_name(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_owner: dir::LocalNodeIdAny,
        candidate_owner: dir::LocalNodeIdAny,
        pattern: tspp_core::StringId,
        candidate: tspp_core::StringId,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_name(
            nodes,
            pattern_owner,
            candidate_owner,
            dir::Name::Identifier(pattern),
            dir::Name::Identifier(candidate),
            bindings,
        )
    }
}
