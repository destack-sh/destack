use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    DeclarationDescriptor, DeclarationKind, Definition, EnumField, Module, NodeId, StructKind,
    Visibility,
};

impl<'a> Compiler<'a> {
    /// Lower visibility into a DIR visibility.
    #[inline]
    pub fn lower_visibility(&self, visibility: ast::Visibility) -> Visibility {
        match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Protected => Visibility::Protected,
            ast::Visibility::Private => Visibility::Private,
        }
    }

    /// Lower declaration kind to DIR declaration kind.
    pub fn lower_declaration_kind(&mut self, kind: ast::DeclarationKind) -> DeclarationKind {
        match kind {
            ast::DeclarationKind::Declaration => DeclarationKind::Declaration,
            ast::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Lower expression to DIR definition (if it's maybe a definition).
    /// If the expression can't possibly resolve to a definition, returns `None`.
    /// (Like for a scalar literal)
    pub fn lower_expression_to_definition(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
        resolve_expressions: bool,
    ) -> Option<NodeId<Definition>> {
        let expression = module.get(expression_id);
        let definition_id = match expression {
            ast::Expression::Definition(definition_id) => {
                self.lower_definition(module, *definition_id)
            }
            ast::Expression::Import { .. } | ast::Expression::Export { .. } => {
                return None;
            }
            _ if resolve_expressions => {
                let definition = Definition::UnresolvedExpression {
                    expression: self.lower_expression(module, expression_id),
                };
                self.session
                    .tree
                    .insert_from_ast(definition, module.id, expression_id)
            }
            _ => {
                return None;
            }
        };
        self.session
            .tree
            .alias_from_ast(module.id, expression_id.id, definition_id);
        Some(definition_id)
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

    /// Lower a definition to a DIR definition.
    /// Lower an AST definition to a DIR definition.
    /// Handles modules, structs, and enums.
    pub fn lower_definition(
        &mut self,
        module: &Module,
        definition_id: ast::NodeId<ast::Definition>,
    ) -> NodeId<Definition> {
        let definition = module.get(definition_id);
        let definition = match definition {
            ast::Definition::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = generics
                    .as_ref()
                    .map(|generics| self.lower_generics(module, generics));
                let definitions = expressions
                    .iter()
                    .flat_map(|expression| {
                        self.lower_expression_to_definition(module, *expression, true)
                    })
                    .collect();
                Definition::Namespace {
                    descriptor,
                    generics,
                    definitions,
                }
            }
            ast::Definition::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let kind = match kind {
                    ast::StructKind::Struct => StructKind::Struct,
                    ast::StructKind::Class => StructKind::Class,
                };
                let generics = generics
                    .as_ref()
                    .map(|generics| self.lower_generics(module, generics));
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.lower_heritage(module, heritage));
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, *property))
                    .collect();
                Definition::Struct {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    properties,
                }
            }
            ast::Definition::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = generics
                    .as_ref()
                    .map(|generics| self.lower_generics(module, generics));
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.lower_heritage(module, heritage));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_enum_field(module, *field))
                    .collect();
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, *property))
                    .collect();
                Definition::Enum {
                    descriptor,
                    generics,
                    heritage,
                    fields,
                    properties,
                }
            }
            ast::Definition::Interface {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = generics
                    .as_ref()
                    .map(|generics| self.lower_generics(module, generics));
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.lower_heritage(module, heritage));
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, *property))
                    .collect();
                Definition::Interface {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            ast::Definition::Implement {
                descriptor,
                generics,
                target_type,
                heritage,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let generics = generics
                    .as_ref()
                    .map(|generics| self.lower_generics(module, generics));
                let target_type = self.lower_expression_to_type(module, *target_type);
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.lower_heritage(module, heritage));
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, *property))
                    .collect();
                Definition::Implement {
                    descriptor,
                    generics,
                    target_type,
                    heritage,
                    properties,
                }
            }
            ast::Definition::Function {
                descriptor,
                signature,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, descriptor);
                let signature = self.lower_function_signature(module, signature);
                let body = body.map(|body| self.lower_expression(module, body));
                Definition::Function {
                    descriptor,
                    signature,
                    // nocheckin unpack definitions from function body DIR
                    definitions: Vec::new(),
                    body,
                }
            }
        };
        self.session
            .tree
            .insert_from_ast(definition, module.id, definition_id)
    }

    /// Lower an AST enum field into a DIR enum field.
    pub fn lower_enum_field(
        &mut self,
        module: &Module,
        field_id: ast::NodeId<ast::EnumField>,
    ) -> NodeId<EnumField> {
        let field = module.get(field_id);
        let name = self
            .session
            .strings
            .intern_from(&module.strings, field.name.string());
        let value = field
            .value
            .map(|value| self.lower_expression(module, value));
        let enum_field = EnumField { name, value };
        self.session
            .tree
            .insert_from_ast(enum_field, module.id, field_id)
    }
}
