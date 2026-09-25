use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match one declaration node.
    pub(crate) fn match_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
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
            (dir::Declaration::Global(pattern), dir::Declaration::Global(candidate)) => {
                if pattern.is_ambient != candidate.is_ambient {
                    return Ok(false);
                }

                self.match_nodes(
                    nodes,
                    &pattern.expressions,
                    &candidate.expressions,
                    0,
                    0,
                    bindings,
                )
            }
            (dir::Declaration::Module(pattern), dir::Declaration::Module(candidate)) => self
                .match_nodes(
                    nodes,
                    &pattern.expressions,
                    &candidate.expressions,
                    0,
                    0,
                    bindings,
                ),
            (dir::Declaration::Type(pattern), dir::Declaration::Type(candidate)) => self
                .match_type_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Struct(pattern), dir::Declaration::Struct(candidate)) => self
                .match_struct_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Class(pattern), dir::Declaration::Class(candidate)) => self
                .match_class_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Enum(pattern), dir::Declaration::Enum(candidate)) => self
                .match_enum_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Interface(pattern), dir::Declaration::Interface(candidate)) => self
                .match_interface_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Extension(pattern), dir::Declaration::Extension(candidate)) => self
                .match_extension_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            (dir::Declaration::Function(pattern), dir::Declaration::Function(candidate)) => self
                .match_function_declaration(
                    nodes,
                    pattern_id,
                    candidate_id,
                    pattern,
                    candidate,
                    bindings,
                ),
            _ => Ok(false),
        }
    }

    /// Match one enum field node.
    pub(crate) fn match_enum_field(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::EnumField>,
        candidate_id: dir::LocalNodeId<dir::EnumField>,
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
        if !self.match_name(
            nodes,
            pattern_any,
            candidate_id.into_any(),
            pattern.name,
            candidate.name,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_optional_node(nodes, pattern.value, candidate.value, bindings)
    }

    /// Match one type declaration.
    fn match_type_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::TypeDeclaration,
        candidate: &dir::TypeDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_shared != candidate.is_shared
            || pattern.mutability != candidate.mutability
            || pattern.is_ambient != candidate.is_ambient
            || pattern.is_nominal != candidate.is_nominal
        {
            return Ok(false);
        }
        if !self.match_name(
            nodes,
            pattern_id.into_any(),
            candidate_id.into_any(),
            pattern.name,
            candidate.name,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_type_expression(nodes, pattern.value, candidate.value, bindings)
    }

    /// Match one struct declaration.
    fn match_struct_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::StructDeclaration,
        candidate: &dir::StructDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_shared != candidate.is_shared
            || pattern.is_ambient != candidate.is_ambient
            || !self.match_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
        {
            return Ok(false);
        }
        if !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.implements_types,
            &candidate.implements_types,
            0,
            0,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_nodes(nodes, &pattern.members, &candidate.members, 0, 0, bindings)
    }

    /// Match one class declaration.
    fn match_class_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::ClassDeclaration,
        candidate: &dir::ClassDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_shared != candidate.is_shared
            || pattern.is_ambient != candidate.is_ambient
            || pattern.is_abstract != candidate.is_abstract
            || pattern.is_final != candidate.is_final
            || !self.match_optional_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
        {
            return Ok(false);
        }
        if !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? || !self.match_optional_node(
            nodes,
            pattern.extends_type,
            candidate.extends_type,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.implements_types,
            &candidate.implements_types,
            0,
            0,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_nodes(nodes, &pattern.members, &candidate.members, 0, 0, bindings)
    }

    /// Match one enum declaration.
    fn match_enum_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::EnumDeclaration,
        candidate: &dir::EnumDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_shared != candidate.is_shared
            || pattern.is_ambient != candidate.is_ambient
            || !self.match_optional_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
        {
            return Ok(false);
        }
        if !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.implements_types,
            &candidate.implements_types,
            0,
            0,
            bindings,
        )? || !self.match_nodes(nodes, &pattern.fields, &candidate.fields, 0, 0, bindings)?
        {
            return Ok(false);
        }

        self.match_nodes(nodes, &pattern.members, &candidate.members, 0, 0, bindings)
    }

    /// Match one interface declaration.
    fn match_interface_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::InterfaceDeclaration,
        candidate: &dir::InterfaceDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_shared != candidate.is_shared
            || pattern.is_ambient != candidate.is_ambient
            || pattern.is_nominal != candidate.is_nominal
            || !self.match_optional_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
        {
            return Ok(false);
        }
        if !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.extends_types,
            &candidate.extends_types,
            0,
            0,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_nodes(nodes, &pattern.members, &candidate.members, 0, 0, bindings)
    }

    /// Match one extension declaration.
    fn match_extension_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::ExtensionDeclaration,
        candidate: &dir::ExtensionDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_ambient != candidate.is_ambient
            || !self.match_optional_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
        {
            return Ok(false);
        }
        if !self.match_nodes(
            nodes,
            &pattern.generic_parameters,
            &candidate.generic_parameters,
            0,
            0,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.where_clauses,
            &candidate.where_clauses,
            0,
            0,
            bindings,
        )? || !self.match_type_expression(
            nodes,
            pattern.target_type,
            candidate.target_type,
            bindings,
        )? || !self.match_nodes(
            nodes,
            &pattern.implements_types,
            &candidate.implements_types,
            0,
            0,
            bindings,
        )? {
            return Ok(false);
        }

        self.match_nodes(nodes, &pattern.members, &candidate.members, 0, 0, bindings)
    }

    /// Match one function declaration.
    fn match_function_declaration(
        &self,
        nodes: &PatternNodes<'_>,
        pattern_id: dir::LocalNodeId<dir::Declaration>,
        candidate_id: dir::LocalNodeId<dir::Declaration>,
        pattern: &dir::FunctionDeclaration,
        candidate: &dir::FunctionDeclaration,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.export != candidate.export
            || pattern.is_ambient != candidate.is_ambient
            || !self.match_optional_name(
                nodes,
                pattern_id.into_any(),
                candidate_id.into_any(),
                pattern.name,
                candidate.name,
                bindings,
            )?
            || !self.match_function_signature(
                nodes,
                &pattern.signature,
                &candidate.signature,
                bindings,
            )?
        {
            return Ok(false);
        }

        self.match_optional_node(nodes, pattern.body, candidate.body, bindings)
    }
}
