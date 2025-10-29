use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    DeclarationKind, Definition, DefinitionMeta, EmbeddedDefinition,
    FunctionSignature, Generics, NodeId, ReferenceStyle,
};
use dyst_source::SourceId;

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
                kind,
                asynchrony,
                target,
                alias,
                items,
                arguments,
            } => {
                let asynchrony = self.lower_asynchrony(*asynchrony);
                let items = self.lower_dependency_binding(
                    source_id,
                    ast,
                    expression_id,
                    *kind,
                    Some(target),
                    alias.as_ref().copied(),
                    items.as_ref().map(|items| items.as_slice()),
                );
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(source_id, ast, *argument))
                        .collect()
                });
                let kind = self.lower_dependency_kind(source_id, ast, *kind);
                let definition = Definition::Import {
                    kind,
                    asynchrony,
                    items,
                    arguments,
                };
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
        extends_types: &Option<Vec<ast::NodeId<ast::Expression>>>,
        implements_types: &Option<Vec<ast::NodeId<ast::Expression>>>,
        expressions: &[ast::NodeId<ast::Expression>],
    ) -> Vec<EmbeddedDefinition> {
        let mut embedded_definitions: Vec<EmbeddedDefinition> = vec![];
        if let Some(extends_types) = extends_types.as_ref() {
            embedded_definitions.extend(
                extends_types
                    .iter()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                    .map(|ty| EmbeddedDefinition::Extends { ty }),
            );
        }
        if let Some(implements_types) = implements_types.as_ref() {
            embedded_definitions.extend(
                implements_types
                    .iter()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty))
                    .map(|ty| EmbeddedDefinition::Implements { ty }),
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

    /// Lower AST definition meta into DIR definition meta.
    pub fn lower_definition_meta(
        &mut self,
        source_id: SourceId,
        meta: &ast::DefinitionMeta,
    ) -> DefinitionMeta {
        let kind = self.lower_declaration_kind(meta.kind);
        let name = meta
            .name
            .map(|name| self.intern_string(source_id, name.string()));
        let visibility = meta
            .visibility
            .map(|visibility| self.lower_visibility(visibility));
        let export = meta.export.map(|export| self.lower_export_mode(export));
        DefinitionMeta {
            kind,
            name,
            visibility,
            export,
        }
    }

    /// Lower AST definition generics into DIR definition generics.
    pub fn lower_generics(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
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
                    .map(|parameter| self.lower_parameter(source_id, ast, *parameter))
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
                    .map(|clause| self.lower_with_clause(source_id, ast, *clause))
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
                    .map(|clause| self.lower_where_clause(source_id, ast, *clause))
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
        source_id: SourceId,
        ast: &ast::NodeTree,
        runtime: ast::Runtime,
        abstraction: ast::FunctionAbstraction,
        asynchrony: ast::Asynchrony,
        cardinality: ast::FunctionCardinality,
        kind: Option<ast::FunctionKind>,
        style: ast::FunctionStyle,
        self_parameter: &Option<ast::SelfParameter>,
        dynamic_parameters: &[ast::NodeId<ast::Parameter>],
        return_type: &Option<ast::NodeId<ast::Expression>>,
    ) -> FunctionSignature {
        let runtime = self.lower_runtime(runtime);
        let abstraction = self.lower_function_abstraction(abstraction);
        let asynchrony = self.lower_asynchrony(asynchrony);
        let cardinality = self.lower_function_cardinality(cardinality);
        let kind = kind.map(|kind| self.lower_function_kind(kind));
        let style = self.lower_function_style(style);
        let self_parameter = self_parameter
            .as_ref()
            .map(|self_parameter| self.lower_self_parameter(source_id, ast, self_parameter));
        let dynamic_parameters = dynamic_parameters
            .iter()
            .map(|parameter| self.lower_parameter(source_id, ast, *parameter))
            .collect();
        let return_type = return_type
            .as_ref()
            .map(|ty| self.lower_expression_to_type(source_id, ast, *ty));
        FunctionSignature {
            runtime,
            abstraction,
            asynchrony,
            cardinality,
            kind,
            style,
            self_parameter,
            dynamic_parameters,
            return_type,
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
                meta,
                format: _,
                style: _,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    None,
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Module {
                        meta,
                        generics,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Struct definition
            ast::Definition::Struct {
                meta,
                style,
                kind,
                extends_types,
                implements_types,
                representation_type,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let style = match style {
                    ast::ReferenceStyle::Struct => ReferenceStyle::Struct,
                    ast::ReferenceStyle::Class => ReferenceStyle::Class,
                };
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    extends_types,
                    implements_types,
                    expressions,
                );
                let representation_type = representation_type
                    .as_ref()
                    .map(|repr| self.lower_expression_to_type(source_id, ast, *repr));
                let struct_name = meta.name;
                let variant = self.lower_struct_to_variant(
                    source_id,
                    ast,
                    definition_id,
                    struct_name,
                    *kind,
                    representation_type,
                    fields,
                );
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Struct {
                        meta,
                        style,
                        generics,
                        embedded_definitions,
                        variant,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Enum definition
            ast::Definition::Enum {
                meta,
                extends_types,
                implements_types,
                static_parameters,
                tag_type,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    extends_types,
                    implements_types,
                    expressions,
                );
                let tag_type = tag_type
                    .as_ref()
                    .map(|tag| self.lower_expression_to_type(source_id, ast, *tag));
                let variants =
                    self.lower_enum_to_variant(source_id, ast, definition_id, tag_type, fields);
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Enum {
                        meta,
                        generics,
                        embedded_definitions,
                        variants,
                        definitions,
                    },
                    source_id,
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
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    extends_types,
                    implements_types,
                    expressions,
                );
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
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Union {
                        meta,
                        generics,
                        embedded_definitions,
                        variants,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Interface definition
            ast::Definition::Interface {
                meta,
                extends_types,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let embedded_definitions = self.lower_embedded_definitions(
                    source_id,
                    ast,
                    definition_id,
                    extends_types,
                    &None,
                    expressions,
                );
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
                        meta,
                        generics,
                        embedded_definitions,
                        fields,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Function definition
            ast::Definition::Function {
                meta,
                runtime,
                abstraction,
                asynchrony,
                cardinality,
                kind,
                style,
                self_parameter,
                dynamic_parameters,
                return_type,
                static_parameters,
                with_clauses,
                where_clauses,
                body,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let signature = self.lower_function_signature(
                    source_id,
                    ast,
                    *runtime,
                    *abstraction,
                    *asynchrony,
                    *cardinality,
                    *kind,
                    *style,
                    self_parameter,
                    dynamic_parameters,
                    return_type,
                );
                let definitions = Vec::new();
                let body = body
                    .as_ref()
                    .map(|body| self.lower_expression(source_id, ast, *body));
                self.tree.insert_from_ast(
                    Definition::Function {
                        meta,
                        generics,
                        signature,
                        definitions,
                        body,
                    },
                    source_id,
                    definition_id,
                )
            }

            // Implement definition
            ast::Definition::Extension {
                meta,
                static_parameters,
                target_type,
                implements_types,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                let meta = self.lower_definition_meta(source_id, meta);
                let generics = self.lower_generics(
                    source_id,
                    ast,
                    static_parameters.as_ref(),
                    with_clauses.as_ref(),
                    where_clauses.as_ref(),
                );
                let target_type = self.lower_expression_to_type(source_id, ast, *target_type);
                let implements_types = implements_types.as_ref().map(|implements_types| {
                    implements_types
                        .iter()
                        .map(|implements_type| {
                            self.lower_expression_to_type(source_id, ast, *implements_type)
                        })
                        .collect()
                });
                let definitions = expressions
                    .iter()
                    .filter_map(|expr| self.lower_expression_to_definition(source_id, ast, *expr))
                    .collect();
                self.tree.insert_from_ast(
                    Definition::Extension {
                        meta,
                        generics,
                        target_type,
                        implements_types,
                        definitions,
                    },
                    source_id,
                    definition_id,
                )
            }
        }
    }
}
