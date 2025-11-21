use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    BindingScope, DeclarationDescriptor, DeclarationKind, Definition, EnumField, Module, NodeId,
    NodeTree, ScopeId, ScopeKind, StructKind, SymbolId, SymbolKey, SymbolSpace,
};

impl<'a> Compiler<'a> {
    /// Bind declaration kind to DIR declaration kind.
    pub(super) fn bind_declaration_kind(&self, kind: ast::DeclarationKind) -> DeclarationKind {
        match kind {
            ast::DeclarationKind::Declaration => DeclarationKind::Declaration,
            ast::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Bind binding scope to DIR binding scope.
    pub(super) fn bind_binding_scope(&self, scope: ast::BindingScope) -> BindingScope {
        match scope {
            ast::BindingScope::Static => BindingScope::Static,
            ast::BindingScope::Instance => BindingScope::Instance,
        }
    }

    /// Bind expression to DIR definition (if it's maybe a definition).
    /// nocheckin: revisit bind_expression_to_definition_maybe
    pub(super) fn bind_expression_to_definition_maybe(
        &self,
        module: &Module,
        scope_id: ScopeId,
        expression_id: ast::NodeId<ast::Expression>,
        tree: &mut NodeTree,
    ) -> Option<NodeId<Definition>> {
        let expression = module.get(expression_id);
        let definition_id = match expression {
            ast::Expression::Definition(definition_id) => {
                self.bind_definition(module, scope_id, *definition_id, tree)
            }
            _ => return None,
        };
        tree.alias_from_source(module.id, expression_id.id, definition_id);
        Some(definition_id)
    }

    /// Bind AST definition descriptor into DIR definition descriptor.
    pub(super) fn bind_declaration_descriptor(
        &self,
        module: &Module,
        symbol_id: SymbolId,
        descriptor: &ast::DeclarationDescriptor,
    ) -> DeclarationDescriptor {
        let kind = self.bind_declaration_kind(descriptor.kind);
        let scope = self.bind_binding_scope(descriptor.scope);
        let name = descriptor.name.map(|name| {
            self.session
                .strings
                .intern_from(&module.ast_strings, name.string())
        });
        let export = descriptor
            .export
            .map(|export| self.bind_export_type(export));
        DeclarationDescriptor {
            kind,
            scope,
            name,
            export,
            symbol: symbol_id,
        }
    }

    /// Bind a definition to a DIR definition.
    /// Bind an AST definition to a DIR definition.
    /// Handles modules, structs, and enums.
    pub(super) fn bind_definition(
        &self,
        module: &Module,
        scope_id: ScopeId,
        definition_id: ast::NodeId<ast::Definition>,
        tree: &mut NodeTree,
    ) -> NodeId<Definition> {
        let definition = module.get(definition_id);
        let (symbol_id, scope_id) =
            tree.create_symbol_with_scope(SymbolSpace::Value, None, ScopeKind::Block, scope_id);
        let definition = match definition {
            ast::Definition::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let definitions = expressions
                    .iter()
                    .flat_map(|expression| {
                        self.bind_expression_to_definition_maybe(
                            module,
                            scope_id,
                            *expression,
                            tree,
                        )
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
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let kind = match kind {
                    ast::StructKind::Struct => StructKind::Struct,
                    ast::StructKind::Class => StructKind::Class,
                };
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
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
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let fields = fields
                    .iter()
                    .map(|field| self.bind_enum_field(module, scope_id, *field, tree))
                    .collect();
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
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
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
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
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let target_type =
                    self.bind_expression_to_type(module, scope_id, *target_type, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
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
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let signature = self.bind_function_signature(module, scope_id, signature, tree);
                let body = body.map(|body| self.bind_expression(module, scope_id, body, tree));
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
        tree.insert_from_source_as_symbol(definition, module.id, definition_id, symbol_id)
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        scope_id: ScopeId,
        field_id: ast::NodeId<ast::EnumField>,
        tree: &mut NodeTree,
    ) -> NodeId<EnumField> {
        let field = module.get(field_id);
        let name = self
            .session
            .strings
            .intern_from(&module.ast_strings, field.name.string());
        let value = field
            .value
            .map(|value| self.bind_expression(module, scope_id, value, tree));
        let symbol_id =
            tree.create_symbol(SymbolSpace::Value, Some(SymbolKey::Name(name)), scope_id);
        let enum_field = EnumField {
            name,
            value,
            symbol: symbol_id,
        };
        tree.insert_from_source_as_symbol(enum_field, module.id, field_id, symbol_id)
    }
}
