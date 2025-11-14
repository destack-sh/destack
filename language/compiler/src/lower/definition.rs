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

    /// Lower embedded definitions of a definition.
    /// Includes both super types and include types (with spread syntax).
    pub fn lower_embedded_definitions(
        &mut self,
        module: &Module,
        _definition_id: ast::NodeId<ast::Definition>,
        extends_types: &Option<Vec<ast::NodeId<ast::Expression>>>,
        implements_types: &Option<Vec<ast::NodeId<ast::Expression>>>,
        expressions: &[ast::NodeId<ast::Expression>],
    ) -> Vec<EmbeddedDefinition> {
        let mut embedded_definitions: Vec<EmbeddedDefinition> = vec![];
        if let Some(extends_types) = extends_types.as_ref() {
            embedded_definitions.extend(
                extends_types
                    .iter()
                    .map(|ty| self.lower_expression_to_type(module, *ty))
                    .map(|ty| EmbeddedDefinition::Extends { ty }),
            );
        }
        if let Some(implements_types) = implements_types.as_ref() {
            embedded_definitions.extend(
                implements_types
                    .iter()
                    .map(|ty| self.lower_expression_to_type(module, *ty))
                    .map(|ty| EmbeddedDefinition::Implements { ty }),
            );
        }
        // include types (from expressions)
        embedded_definitions.extend(expressions.iter().filter_map(|expression_id| {
            let expression = module.get(*expression_id);
            match expression {
                ast::Expression::Unary {
                    operator: ast::UnaryOperator::Spread,
                    right,
                } => {
                    let right = self.lower_expression_to_type(module, *right);
                    self.session
                        .tree
                        .alias_from_ast(module.id, expression_id.id, right);
                    Some(EmbeddedDefinition::Include { ty: right })
                }
                _ => None,
            }
        }));
        embedded_definitions
    }

    /// Lower AST definition meta into DIR definition meta.
    pub fn lower_declaration_descriptor(
        &mut self,
        module: &Module,
        meta: &ast::DeclarationDescriptor,
    ) -> DeclarationDescriptor {
        let kind = self.lower_declaration_kind(meta.kind);
        let name = meta.name.map(|name| {
            self.session
                .strings
                .intern_from(&module.strings, name.string())
        });
        let visibility = meta
            .visibility
            .map(|visibility| self.lower_visibility(visibility));
        let export = meta.export.map(|export| self.lower_export_type(export));
        DeclarationDescriptor {
            kind,
            name,
            visibility,
            export,
        }
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

    /// Lower function signature components into a DIR function signature.
    #[allow(clippy::too_many_arguments)]
    pub fn lower_function_signature(
        &mut self,
        module: &Module,
        abstraction: ast::FunctionAbstraction,
        asynchrony: ast::Asynchrony,
        cardinality: ast::FunctionCardinality,
        mode: Option<ast::FunctionMode>,
        kind: ast::FunctionKind,
        dynamic_parameters: &[ast::NodeId<ast::Parameter>],
        return_type: &Option<ast::NodeId<ast::Expression>>,
    ) -> FunctionSignature {
        let abstraction = self.lower_function_abstraction(abstraction);
        let asynchrony = self.lower_asynchrony(asynchrony);
        let cardinality = self.lower_function_cardinality(cardinality);
        let kind = self.lower_function_kind(kind);
        let mode = mode.map(|mode| self.lower_function_mode(mode));
        let dynamic_parameters = dynamic_parameters
            .iter()
            .map(|parameter| self.lower_parameter(module, *parameter))
            .collect();
        let return_type = return_type
            .as_ref()
            .map(|ty| self.lower_expression_to_type(module, *ty));
        FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            dynamic_parameters,
            return_type,
        }
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
                descriptor: meta,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let meta = self.lower_declaration_descriptor(module, meta);
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
                        descriptor: meta,
                        generics,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Struct definition
            ast::Definition::Struct {
                descriptor: meta,
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
                let meta = self.lower_declaration_descriptor(module, meta);
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
                let struct_name = meta.name;
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
                        descriptor: meta,
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
                descriptor: meta,
                extends_types,
                implements_types,
                static_parameters,
                tag_type,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_declaration_descriptor(module, meta);
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
                        descriptor: meta,
                        generics,
                        embedded_definitions,
                        variants,
                        definitions,
                    },
                    module.id,
                    definition_id,
                )
            }

            // Union definition
            ast::Definition::Union {
                meta,
                tag_type,
                representation_type,
                static_parameters,
                extends_types,
                implements_types,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_declaration_descriptor(module, meta);
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
                let representation_type = representation_type
                    .as_ref()
                    .map(|repr| self.lower_expression_to_type(module, *repr));
                let variants = self.lower_union_to_variant(
                    module,
                    definition_id,
                    representation_type,
                    tag_type,
                    fields,
                );
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(module, *expr))
                    .collect();
                self.session.tree.insert_from_ast(
                    Definition::Union {
                        meta,
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
                descriptor: meta,
                extends_types,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_declaration_descriptor(module, meta);
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
                        descriptor: meta,
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
                descriptor: meta,
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
                let meta = self.lower_declaration_descriptor(module, meta);
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
                        descriptor: meta,
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
                descriptor: meta,
                static_parameters,
                target_type,
                implements_types,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let meta = self.lower_declaration_descriptor(module, meta);
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
                        descriptor: meta,
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
