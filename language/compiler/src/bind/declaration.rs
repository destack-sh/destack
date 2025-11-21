use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    BindingScope, Declaration, DeclarationDescriptor, DeclarationKind, EnumField, LocalNodeId,
    LocalScopeId, LocalSymbolId, Module, NodeTree, ScopeKind, StructKind, SymbolKey, SymbolSpace,
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

    /// Bind expression to DIR declaration (if it's maybe a declaration).
    /// nocheckin: revisit bind_expression_to_declaration_maybe
    pub(super) fn bind_expression_to_declaration_maybe(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        expression_id: ast::LocalNodeId<ast::Expression>,
        tree: &mut NodeTree,
    ) -> Option<LocalNodeId<Declaration>> {
        let expression = module.get(expression_id);
        let declaration_id = match expression {
            ast::Expression::Declaration(declaration_id) => {
                self.bind_declaration(module, scope_id, *declaration_id, tree)
            }
            _ => return None,
        };
        tree.alias_from_source(module.id, expression_id.id, declaration_id);
        Some(declaration_id)
    }

    /// Bind AST declaration descriptor into DIR declaration descriptor.
    pub(super) fn bind_declaration_descriptor(
        &self,
        module: &Module,
        symbol_id: LocalSymbolId,
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

    /// Bind a declaration to a DIR declaration.
    /// Bind an AST declaration to a DIR declaration.
    /// Handles modules, structs, and enums.
    pub(super) fn bind_declaration(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        declaration_id: ast::LocalNodeId<ast::Declaration>,
        tree: &mut NodeTree,
    ) -> LocalNodeId<Declaration> {
        let declaration = module.get(declaration_id);
        let declaration = match declaration {
            ast::Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Namespace,
                    scope_id,
                );
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let declarations = expressions
                    .iter()
                    .flat_map(|expression| {
                        self.bind_expression_to_declaration_maybe(
                            module,
                            scope_id,
                            *expression,
                            tree,
                        )
                    })
                    .collect();
                Declaration::Namespace {
                    descriptor,
                    generics,
                    scope: scope_id,
                    declarations,
                }
            }
            ast::Declaration::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Type,
                    scope_id,
                );
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
                Declaration::Struct {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    scope: scope_id,
                    properties,
                }
            }
            ast::Declaration::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                properties,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Type,
                    scope_id,
                );
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
                Declaration::Enum {
                    descriptor,
                    generics,
                    heritage,
                    scope: scope_id,
                    fields,
                    properties,
                }
            }
            ast::Declaration::Interface {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Type,
                    scope_id,
                );
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
                    .collect();
                Declaration::Interface {
                    descriptor,
                    generics,
                    heritage,
                    scope: scope_id,
                    properties,
                }
            }
            ast::Declaration::Implement {
                descriptor,
                generics,
                target_type,
                heritage,
                properties,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Type,
                    scope_id,
                );
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let generics = self.bind_generics(module, scope_id, generics, tree);
                let target_type =
                    self.bind_expression_to_type(module, scope_id, *target_type, tree);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree);
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
                    .collect();
                Declaration::Implement {
                    descriptor,
                    generics,
                    target_type,
                    heritage,
                    scope: scope_id,
                    properties,
                }
            }
            ast::Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let descriptor = self.bind_declaration_descriptor(module, symbol_id, descriptor);
                let signature = self.bind_function_signature(module, scope_id, signature, tree);
                let body = body.map(|body| self.bind_expression(module, scope_id, body, tree));
                Declaration::Function {
                    descriptor,
                    signature,
                    scope: scope_id,
                    // nocheckin unpack declarations from function body DIR
                    declarations: Vec::new(),
                    body,
                }
            }
        };
        let symbol_id = declaration.symbol();
        tree.insert_from_source_as_symbol(declaration, module.id, declaration_id, symbol_id)
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        field_id: ast::LocalNodeId<ast::EnumField>,
        tree: &mut NodeTree,
    ) -> LocalNodeId<EnumField> {
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
