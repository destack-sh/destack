use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Definition, NodeId};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower expression to DIR definition (if it's maybe a definition).
    /// If the expression can't possibly evaluate to a definition, returns `None`.
    /// (Like for a scalar literal)
    pub fn lower_expression_to_definition(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> Option<NodeId<Definition>> {
        let expression = ast.get(expression_id);
        match expression {
            ast::Expression::Definition(definition_id) => {
                Some(self.lower_definition(source_id, ast, *definition_id))
            }
            ast::Expression::Use {
                visibility,
                clauses,
                body,
            } if body.is_none() => {
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let items = clauses
                    .iter()
                    .flat_map(|clause| self.lower_use_clause(source_id, ast, *clause))
                    .collect();
                let definition = Definition::Use { visibility, items };
                Some(self.tree.allocate(definition, source_id, expression_id))
            }

            // nocheckin #Incomplete: lower more Expressions to Definitions
            _ => None,
        }
    }

    /// Lower a definition to a DIR definition.
    /// Lower an AST definition to a DIR definition.
    /// Handles modules, structs, and enums.
    pub fn lower_definition(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        definition_id: ast::NodeId<ast::Definition>,
    ) -> NodeId<Definition> {
        let definition = ast.get(definition_id);
        match definition {
            // Module definition
            ast::Definition::Module {
                name,
                visibility,
                format: _,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                // lower all expressions to definitions, skipping non-definitions
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.allocate(
                    Definition::Module {
                        name,
                        visibility,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Struct definition
            ast::Definition::Struct {
                name,
                visibility,
                style,
                super_types,
                representation_type,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let super_types = super_types.as_ref().map(|supers| {
                    supers
                        .iter()
                        .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                        .collect()
                });
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                let representation_type = representation_type
                    .as_ref()
                    .map(|repr| self.lower_expression_to_type(source_id, ast, *repr));
                let variant = self.lower_struct_to_variant(
                    source_id,
                    ast,
                    definition_id,
                    name,
                    *style,
                    representation_type,
                    fields,
                );
                self.tree.allocate(
                    Definition::Struct {
                        name,
                        visibility,
                        super_types,
                        static_parameters,
                        variant,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Enum definition
            ast::Definition::Enum {
                name,
                visibility,
                super_types,
                static_parameters,
                tag_type,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let super_types = super_types.as_ref().map(|supers| {
                    supers
                        .iter()
                        .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                        .collect()
                });
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                let tag_type = tag_type
                    .as_ref()
                    .map(|tag| self.lower_expression_to_type(source_id, ast, *tag));
                let variants =
                    self.lower_enum_to_variant(source_id, ast, definition_id, tag_type, fields);
                self.tree.allocate(
                    Definition::Enum {
                        name,
                        visibility,
                        super_types,
                        static_parameters,
                        variants,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Union definition
            ast::Definition::Union {
                name,
                visibility,
                tag_type,
                representation_type,
                static_parameters,
                super_types,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let super_types = super_types.as_ref().map(|supers| {
                    supers
                        .iter()
                        .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                        .collect()
                });
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                let tag_type = tag_type
                    .as_ref()
                    .map(|tag| self.lower_expression_to_type(source_id, ast, *tag));
                let representation_type = representation_type
                    .as_ref()
                    .map(|repr| self.lower_expression_to_type(source_id, ast, *repr));
                let variants = self.lower_union_to_variant(
                    source_id,
                    ast,
                    definition_id,
                    representation_type,
                    tag_type,
                    fields,
                );
                self.tree.allocate(
                    Definition::Union {
                        name,
                        visibility,
                        super_types,
                        static_parameters,
                        variants,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Trait definition
            ast::Definition::Trait {
                name,
                visibility,
                super_types,
                static_parameters,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let super_types = super_types.as_ref().map(|supers| {
                    supers
                        .iter()
                        .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                        .collect()
                });
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.allocate(
                    Definition::Trait {
                        name,
                        visibility,
                        super_types,
                        static_parameters,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Function definition
            ast::Definition::Function {
                name,
                visibility,
                runtime,
                style: _,
                self_parameter,
                dynamic_parameters,
                return_type,
                static_parameters,
                with_clauses,
                where_clauses,
                body,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let runtime = self.lower_runtime(*runtime);
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let self_parameter = self_parameter
                    .as_ref()
                    .map(|param| self.lower_self_parameter(source_id, ast, param));
                let dynamic_parameters = dynamic_parameters
                    .iter()
                    .map(|param| self.lower_parameter(source_id, ast, *param))
                    .collect();
                let return_type = return_type
                    .as_ref()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty));
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = {
                    if let Some(body) = body {
                        let body = ast.get(*body);
                        body.expressions
                            .iter()
                            .filter_map(|expr| {
                                self.lower_expression_to_definition(source_id, ast, *expr)
                            })
                            .collect()
                    } else {
                        vec![]
                    }
                };
                self.tree.allocate(
                    Definition::Function {
                        name,
                        visibility,
                        runtime,
                        static_parameters,
                        self_parameter,
                        dynamic_parameters,
                        return_type,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Implement definition
            ast::Definition::Implement {
                static_parameters,
                receiver,
                for_type,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let receiver = self.lower_expression_to_type(source_id, ast, *receiver);
                let for_type = for_type
                    .as_ref()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty));
                let with_clauses = with_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|clauses| {
                    clauses
                        .iter()
                        .map(|clause| self.lower_where_clause(source_id, ast, *clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.allocate(
                    Definition::Implement {
                        static_parameters,
                        receiver,
                        for_type,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }
        }
    }
}
