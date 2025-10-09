use crate::{AstNodeId, Compiler};
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
            // NOTE #Incomplete: lower more Expressions to Definitions
            _ => None,
        }
    }

    /// Lower a definition to a DIR definition.
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
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let with_clauses = with_clauses.as_ref().map(|with_clauses| {
                    with_clauses
                        .iter()
                        .map(|with_clause| self.lower_with_clause(source_id, ast, *with_clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|where_clauses| {
                    where_clauses
                        .iter()
                        .map(|where_clause| self.lower_where_clause(source_id, ast, *where_clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expression| {
                        self.lower_expression_to_definition(source_id, ast, *expression)
                    })
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
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let super_types = super_types.as_ref().map(|super_types| {
                    super_types
                        .iter()
                        .map(|super_type| {
                            self.lower_expression_to_type(source_id, ast, *super_type)
                        })
                        .collect()
                });
                let with_clauses = with_clauses.as_ref().map(|with_clauses| {
                    with_clauses
                        .iter()
                        .map(|with_clause| self.lower_with_clause(source_id, ast, *with_clause))
                        .collect()
                });
                let where_clauses = where_clauses.as_ref().map(|where_clauses| {
                    where_clauses
                        .iter()
                        .map(|where_clause| self.lower_where_clause(source_id, ast, *where_clause))
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expression| {
                        self.lower_expression_to_definition(source_id, ast, *expression)
                    })
                    .collect();
                self.tree.allocate(
                    Definition::Struct {
                        name,
                        visibility,
                        super_types,
                        variant,
                        with_clauses,
                        where_clauses,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }
            _ => todo!(
                "Compiler.lower_definition not implemented for {:?}",
                definition
            ),
        }
    }
}
