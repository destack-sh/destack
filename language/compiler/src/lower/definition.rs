use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    DeclarationDescriptor, DeclarationKind, Definition, EmbeddedDefinition, FunctionSignature,
    Generics, Module, NodeId, StructKind,
};

impl<'a> Compiler<'a> {
    /// Lower declaration kind to DIR declaration kind.
    pub fn lower_declaration_kind(&mut self, kind: ast::DeclarationKind) -> DeclarationKind {
        match kind {
            ast::DeclarationKind::Declaration => DeclarationKind::Declaration,
            ast::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Lower expression to DIR definition (if it's maybe a definition).
    /// If the expression can't possibly evaluate to a definition, returns `None`.
    /// (Like for a scalar literal)
    pub fn lower_expression_to_definition(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> Option<NodeId<Definition>> {
        let expression = module.get(expression_id);
        let definition = match expression {
            ast::Expression::Definition(definition_id) => {
                Some(self.lower_definition(module, *definition_id))
            }
            _ => None,
        };
        if let Some(definition) = definition {
            self.session
                .tree
                .alias_from_ast(module.id, expression_id.id, definition);
        }
        definition
    }

    /// Lower AST definition descriptor into DIR definition descriptor.
    pub fn lower_declaration_descriptor(
        &mut self,
        module: &Module,
        descriptor: &ast::DeclarationDescriptor,
    ) -> DeclarationDescriptor {
        let kind = self.lower_declaration_kind(descriptor.kind);
        let name = descriptor.name.map(|name| {
            self.session
                .strings
                .intern_from(&module.strings, name.string())
        });
        let export = descriptor
            .export
            .map(|export| self.lower_export_type(export));
        DeclarationDescriptor { kind, name, export }
    }

    /// Lower AST definition generics into DIR definition generics.
    pub fn lower_generics(
        &mut self,
        module: &Module,
        static_parameters: Option<&Vec<ast::NodeId<ast::Parameter>>>,
        with_clauses: Option<&Vec<ast::NodeId<ast::WithClause>>>,
        where_clauses: Option<&Vec<ast::NodeId<ast::WhereClause>>>,
    ) -> Option<Generics> {
        let static_parameters = static_parameters.and_then(|parameters| {
            if parameters.is_empty() {
                return None;
            }
            Some(
                parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(module, *parameter))
                    .collect(),
            )
        });
        let with_clauses = with_clauses.and_then(|clauses| {
            if clauses.is_empty() {
                return None;
            }
            Some(
                clauses
                    .iter()
                    .map(|clause| self.lower_with_clause(module, *clause))
                    .collect(),
            )
        });
        let where_clauses = where_clauses.and_then(|clauses| {
            if clauses.is_empty() {
                return None;
            }
            Some(
                clauses
                    .iter()
                    .map(|clause| self.lower_where_clause(module, *clause))
                    .collect(),
            )
        });
        if static_parameters.is_none() && with_clauses.is_none() && where_clauses.is_none() {
            return None;
        }
        Some(Generics {
            static_parameters,
            with_clauses,
            where_clauses,
        })
    }

    /// Lower a definition to a DIR definition.
    /// Lower an AST definition to a DIR definition.
    /// Handles modules, structs, and enums.
    pub fn lower_definition(
        &mut self,
        module: &Module,
        definition_id: ast::NodeId<ast::Definition>,
    ) -> NodeId<Definition> {
        let definition = module.get(definition_id);
        match definition {
            // Module definition
            ast::Definition::Namespace {
                descriptor,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = self.lower_generics(
                    module,
                    None,
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Namespace {
                        descriptor,
                        generics,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Struct definition
            ast::Definition::Struct {
                descriptor,
                kind,
                format,
                extends_types,
                implements_types,
                representation_type,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let kind = match kind {
                    ast::StructKind::Struct => StructKind::Struct,
                    ast::StructKind::Class => StructKind::Class,
                };
                let generics = self.lower_generics(
                    module,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    module,
                    definition_id,
                    extends_types,
                    implements_types,
                    expressions,
                );
                let representation_type = representation_type
                    .as_ref()
                    .map(|repr| self.lower_expression_to_type(module, *repr));
                let struct_name = descriptor.name;
                let variant = self.lower_struct_to_variant(
                    module,
                    definition_id,
                    struct_name,
                    *format,
                    representation_type,
                    fields,
                );
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Struct {
                        descriptor,
                        kind,
                        generics,
                        embedded_definitions,
                        variant,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Enum definition
            ast::Definition::Enum {
                descriptor,
                extends_types,
                implements_types,
                static_parameters,
                tag_type,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = self.lower_generics(
                    module,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    module,
                    definition_id,
                    extends_types,
                    implements_types,
                    expressions,
                );
                let tag_type = tag_type
                    .as_ref()
                    .map(|tag| self.lower_expression_to_type(module, *tag));
                let variants = self.lower_enum_to_variant(module, definition_id, tag_type, fields);
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Enum {
                        descriptor,
                        generics,
                        embedded_definitions,
                        variants,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Interface definition
            ast::Definition::Interface {
                descriptor,
                extends_types,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = self.lower_generics(
                    module,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    module,
                    definition_id,
                    extends_types,
                    &None,
                    expressions,
                );
                let fields = fields
                    .iter()
                    .map(|field| self.lower_field(module, *field))
                    .collect();
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Interface {
                        descriptor,
                        generics,
                        embedded_definitions,
                        fields,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Function definition
            ast::Definition::Function {
                descriptor,
                abstraction,
                asynchrony,
                cardinality,
                mode,
                kind,
                dynamic_parameters,
                return_type,
                static_parameters,
                with_clauses,
                where_clauses,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = self.lower_generics(
                    module,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let signature = self.lower_function_signature(
                    module,
                    *abstraction,
                    *asynchrony,
                    *cardinality,
                    *mode,
                    *kind,
                    dynamic_parameters,
                    return_type,
                );
                let definitions = Vec::new();
                let body = body
                    .as_ref()
                    .map(|body| self.lower_expression(module, *body));
                self.session.tree.insert_from_ast(
                    Definition::Function {
                        descriptor,
                        generics,
                        signature,
                        definitions,
                        body,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Implement definition
            ast::Definition::Implement {
                descriptor,
                static_parameters,
                target_type,
                implements_types,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = self.lower_generics(
                    module,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let target_type = self.lower_expression_to_type(module, *target_type);
                let implements_types = implements_types.as_ref().map(|implements_types| {
                    implements_types
                        .iter()
                        .map(|implements_type| {
                            self.lower_expression_to_type(module, *implements_type)
                        })
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Implement {
                        descriptor,
                        generics,
                        target_type,
                        implements_types,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }
        }
    }
}
