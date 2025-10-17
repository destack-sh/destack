use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Definition, EmbeddedDefinition, NodeId};
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
        let definition = match expression {
            ast::Expression::Definition(definition_id) => {
                Some(self.lower_definition(source_id, ast, *definition_id))
            }
            ast::Expression::Import {
                target,
                alias,
                items,
            } => {
                let items = self.lower_import_binding(
                    source_id,
                    ast,
                    expression_id,
                    Some(target),
                    alias.as_ref().copied(),
                    items.as_ref().map(|items| items.as_slice()),
                );
                let definition = Definition::Import { items };
                Some(
                    self.tree
                        .insert_from_ast(definition, source_id, expression_id),
                )
            }
            _ => None,
        };
        if let Some(definition) = definition {
            self.tree
                .alias_from_ast(source_id, expression_id.id, definition);
        }
        definition
    }

    /// Lower embedded definitions of a definition.
    /// Includes both super types and include types (with spread syntax).
    pub fn lower_embedded_definitions(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        _definition_id: ast::NodeId<ast::Definition>,
        super_types: &Option<Vec<ast::NodeId<ast::Expression>>>,
        expressions: &[ast::NodeId<ast::Expression>],
    ) -> Vec<EmbeddedDefinition> {
        let mut embedded_definitions: Vec<EmbeddedDefinition> = vec![];
        // super types
        if let Some(super_types) = super_types.as_ref() {
            embedded_definitions.extend(
                super_types
                    .iter()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                    .map(|ty| EmbeddedDefinition::Super { ty }),
            );
        }
        // include types (from expressions)
        embedded_definitions.extend(expressions.iter().filter_map(|expression_id| {
            let expression = ast.get(*expression_id);
            match expression {
                ast::Expression::Unary {
                    operator: ast::UnaryOperator::Spread,
                    expression: right,
                } => {
                    let right = self.lower_expression_to_type(source_id, ast, *right);
                    self.tree.alias_from_ast(source_id, expression_id.id, right);
                    Some(EmbeddedDefinition::Include { ty: right })
                }
                _ => None,
            }
        }));
        embedded_definitions
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
                export,
                format: _,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let export = export.map(|e| self.lower_export_mode(e));
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
                self.tree.insert_from_ast(
                    Definition::Module {
                        name,
                        export,
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
                export,
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
                let export = export.map(|e| self.lower_export_mode(e));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    super_types,
                    expressions,
                );
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
                self.tree.insert_from_ast(
                    Definition::Struct {
                        name,
                        visibility,
                        export,
                        static_parameters,
                        embedded_definitions,
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
                export,
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
                let export = export.map(|e| self.lower_export_mode(e));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    super_types,
                    expressions,
                );
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
                self.tree.insert_from_ast(
                    Definition::Enum {
                        name,
                        visibility,
                        export,
                        static_parameters,
                        embedded_definitions,
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
                export,
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
                let export = export.map(|e| self.lower_export_mode(e));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    super_types,
                    expressions,
                );
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
                self.tree.insert_from_ast(
                    Definition::Union {
                        name,
                        visibility,
                        export,
                        static_parameters,
                        embedded_definitions,
                        variants,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Interface definition
            ast::Definition::Interface {
                name,
                visibility,
                export,
                super_types,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let export = export.map(|e| self.lower_export_mode(e));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    super_types,
                    expressions,
                );
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
                let fields = fields
                    .iter()
                    .map(|field| self.lower_variant_field(source_id, ast, *field))
                    .collect();
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Interface {
                        name,
                        visibility,
                        export,
                        static_parameters,
                        embedded_definitions,
                        with_clauses,
                        where_clauses,
                        fields,
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
                export,
                runtime,
                style,
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
                let style = self.lower_function_style(*style);
                let export = export.map(|e| self.lower_export_mode(e));
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
                let definitions = vec![]; // not sure?
                let body = body
                    .as_ref()
                    .map(|body| self.lower_expression(source_id, ast, *body));
                self.tree.insert_from_ast(
                    Definition::Function {
                        name,
                        visibility,
                        export,
                        runtime,
                        style,
                        static_parameters,
                        self_parameter,
                        dynamic_parameters,
                        return_type,
                        with_clauses,
                        where_clauses,
                        definitions,
                        body,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Implement definition
            ast::Definition::Implement {
                export,
                visibility,
                static_parameters,
                target_type,
                super_type,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let export = export.map(|e| self.lower_export_mode(e));
                let visibility = visibility.map(|v| self.lower_visibility(v));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(source_id, ast, *param))
                        .collect()
                });
                let target_type = self.lower_expression_to_type(source_id, ast, *target_type);
                let super_type = super_type
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
                self.tree.insert_from_ast(
                    Definition::Implement {
                        export,
                        visibility,
                        static_parameters,
                        target_type,
                        super_type,
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
