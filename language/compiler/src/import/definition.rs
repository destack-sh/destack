use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    BindingScope, DeclarationDescriptor, DeclarationKind, Definition, EnumField, Module, NodeId,
    ScopeId, ScopeKind, StructKind, SymbolId, SymbolKey, SymbolSpace,
};

impl<'a> Compiler<'a> {
    /// Lower declaration kind to DIR declaration kind.
    pub(super) fn lower_declaration_kind(&mut self, kind: ast::DeclarationKind) -> DeclarationKind {
        match kind {
            ast::DeclarationKind::Declaration => DeclarationKind::Declaration,
            ast::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Lower binding scope to DIR binding scope.
    pub(super) fn lower_binding_scope(&mut self, scope: ast::BindingScope) -> BindingScope {
        match scope {
            ast::BindingScope::Static => BindingScope::Static,
            ast::BindingScope::Instance => BindingScope::Instance,
        }
    }

    /// Lower expression to DIR definition (if it's maybe a definition).
    /// nocheckin: revisit lower_expression_to_definition_maybe
    pub(super) fn lower_expression_to_definition_maybe(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> Option<NodeId<Definition>> {
        let expression = module.get(expression_id);
        let definition_id = match expression {
            ast::Expression::Definition(definition_id) => {
                self.lower_definition(module, scope_id, *definition_id)
            }
            _ => return None,
        };
        self.session
            .tree
            .alias_from_source(module.id, expression_id.id, definition_id);
        Some(definition_id)
    }

    /// Lower AST definition descriptor into DIR definition descriptor.
    pub(super) fn lower_declaration_descriptor(
        &mut self,
        module: &Module,
        symbol_id: SymbolId,
        descriptor: &ast::DeclarationDescriptor,
    ) -> DeclarationDescriptor {
        let kind = self.lower_declaration_kind(descriptor.kind);
        let scope = self.lower_binding_scope(descriptor.scope);
        let name = descriptor.name.map(|name| {
            self.session
                .strings
                .intern_from(&module.strings, name.string())
        });
        let export = descriptor
            .export
            .map(|export| self.lower_export_type(export));
        DeclarationDescriptor {
            kind,
            scope,
            name,
            export,
            symbol: symbol_id,
        }
    }

    /// Lower a definition to a DIR definition.
    /// Lower an AST definition to a DIR definition.
    /// Handles modules, structs, and enums.
    pub(super) fn lower_definition(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        definition_id: ast::NodeId<ast::Definition>,
    ) -> NodeId<Definition> {
        let definition = module.get(definition_id);
        let (symbol_id, scope_id) = self.session.tree.create_symbol_with_scope(
            SymbolSpace::Value,
            None,
            ScopeKind::Block,
            scope_id,
        );
        let definition = match definition {
            ast::Definition::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.lower_generics(module, scope_id, generics);
                let definitions = expressions
                    .iter()
                    .flat_map(|expression| {
                        self.lower_expression_to_definition_maybe(module, scope_id, *expression)
                    })
                    .collect();
                Definition::Namespace {
                    descriptor,
                    generics,
                    scope: scope_id,
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
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let kind = match kind {
                    ast::StructKind::Struct => StructKind::Struct,
                    ast::StructKind::Class => StructKind::Class,
                };
                let generics = self.lower_generics(module, scope_id, generics);
                let heritage = self.lower_heritage(module, scope_id, heritage);
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, scope_id, *property))
                    .collect();
                Definition::Struct {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    scope: scope_id,
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
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.lower_generics(module, scope_id, generics);
                let heritage = self.lower_heritage(module, scope_id, heritage);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_enum_field(module, scope_id, *field))
                    .collect();
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, scope_id, *property))
                    .collect();
                Definition::Enum {
                    descriptor,
                    generics,
                    heritage,
                    scope: scope_id,
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
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.lower_generics(module, scope_id, generics);
                let heritage = self.lower_heritage(module, scope_id, heritage);
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, scope_id, *property))
                    .collect();
                Definition::Interface {
                    descriptor,
                    generics,
                    heritage,
                    scope: scope_id,
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
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.lower_generics(module, scope_id, generics);
                let target_type = self.lower_expression_to_type(module, scope_id, *target_type);
                let heritage = self.lower_heritage(module, scope_id, heritage);
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, scope_id, *property))
                    .collect();
                Definition::Implement {
                    descriptor,
                    generics,
                    target_type,
                    heritage,
                    scope: scope_id,
                    properties,
                }
            }
            ast::Definition::Function {
                descriptor,
                signature,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(module, symbol_id, descriptor);
                let signature = self.lower_function_signature(module, scope_id, signature);
                let body = body.map(|body| self.lower_expression(module, scope_id, body));
                Definition::Function {
                    descriptor,
                    signature,
                    scope: scope_id,
                    // nocheckin unpack definitions from function body DIR
                    definitions: Vec::new(),
                    body,
                }
            }
        };
        self.session.tree.insert_from_source_as_symbol(
            definition,
            module.id,
            definition_id,
            symbol_id,
        )
    }

    /// Lower an AST enum field into a DIR enum field.
    pub(super) fn lower_enum_field(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        field_id: ast::NodeId<ast::EnumField>,
    ) -> NodeId<EnumField> {
        let field = module.get(field_id);
        let name = self
            .session
            .strings
            .intern_from(&module.strings, field.name.string());
        let value = field
            .value
            .map(|value| self.lower_expression(module, scope_id, value));
        let symbol_id = self.session.tree.create_symbol(
            SymbolSpace::Value,
            Some(SymbolKey::Name(name)),
            scope_id,
        );
        let enum_field = EnumField {
            name,
            value,
            symbol: symbol_id,
        };
        self.session
            .tree
            .insert_from_source_as_symbol(enum_field, module.id, field_id, symbol_id)
    }
}
