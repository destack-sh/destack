use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one function signature.
    pub(crate) fn match_function_signature(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::FunctionSignature,
        candidate: &dir::FunctionSignature,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.asynchrony != candidate.asynchrony
            || pattern.role != candidate.role
            || pattern.form != candidate.form
            || pattern.phase != candidate.phase
            || pattern.this_form != candidate.this_form
            || pattern.is_abstract != candidate.is_abstract
            || pattern.is_override != candidate.is_override
            || pattern.is_generator != candidate.is_generator
            || !self.match_nodes(
                nodes,
                &pattern.generic_parameters,
                &candidate.generic_parameters,
                0,
                0,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.where_clauses,
                &candidate.where_clauses,
                0,
                0,
                bindings,
            )?
            || !self.match_optional_parameter(
                nodes,
                pattern.this_parameter,
                candidate.this_parameter,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.parameters,
                &candidate.parameters,
                0,
                0,
                bindings,
            )?
        {
            return Ok(false);
        }

        self.match_optional_type_expression(
            nodes,
            pattern.return_type,
            candidate.return_type,
            bindings,
        )
    }

    /// Match one function type expression.
    pub(crate) fn match_function_type(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::FunctionTypeExpression,
        candidate: &dir::FunctionTypeExpression,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.this_form != candidate.this_form
            || !self.match_nodes(
                nodes,
                &pattern.generic_parameters,
                &candidate.generic_parameters,
                0,
                0,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.where_clauses,
                &candidate.where_clauses,
                0,
                0,
                bindings,
            )?
            || !self.match_optional_parameter(
                nodes,
                pattern.this_parameter,
                candidate.this_parameter,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.parameters,
                &candidate.parameters,
                0,
                0,
                bindings,
            )?
        {
            return Ok(false);
        }

        self.match_optional_type_expression(
            nodes,
            pattern.return_type,
            candidate.return_type,
            bindings,
        )
    }

    /// Match one constructor type expression.
    pub(crate) fn match_constructor_type(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::ConstructorType,
        candidate: &dir::ConstructorType,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.is_abstract != candidate.is_abstract
            || !self.match_nodes(
                nodes,
                &pattern.generic_parameters,
                &candidate.generic_parameters,
                0,
                0,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.where_clauses,
                &candidate.where_clauses,
                0,
                0,
                bindings,
            )?
            || !self.match_nodes(
                nodes,
                &pattern.parameters,
                &candidate.parameters,
                0,
                0,
                bindings,
            )?
        {
            return Ok(false);
        }

        self.match_optional_type_expression(
            nodes,
            pattern.return_type,
            candidate.return_type,
            bindings,
        )
    }

    /// Match two optional parameters.
    fn match_optional_parameter(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<dir::Parameter>>,
        candidate: Option<dir::LocalNodeId<dir::Parameter>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_parameter(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }
}
