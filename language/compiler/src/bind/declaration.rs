use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    BindingAnchor, Declaration, DeclarationDescriptor, DeclarationKind, EnumField, LocalNodeId,
    LocalScopeId, Module, NodeTree, ScopeKind, StructKind, SymbolKey, SymbolSpace, SymbolTable,
    TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind declaration kind to DIR declaration kind.
    pub(super) fn bind_declaration_kind(&self, kind: ast::DeclarationKind) -> DeclarationKind {
        match kind {
            ast::DeclarationKind::Declaration => DeclarationKind::Declaration,
            ast::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Bind binding anchor to DIR binding anchor.
    pub(super) fn bind_binding_anchor(&self, anchor: ast::BindingAnchor) -> BindingAnchor {
        match anchor {
            ast::BindingAnchor::Static => BindingAnchor::Static,
            ast::BindingAnchor::Instance => BindingAnchor::Instance,
        }
    }

    /// Bind AST declaration descriptor into DIR declaration descriptor (including symbol and scope).
    pub(super) fn bind_declaration_descriptor(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        descriptor: &ast::DeclarationDescriptor,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        let name = descriptor.name.map(|name| {
            self.program
                .strings
                .intern_from(&module.ast_strings, name.string())
        });
        let (symbol_id, scope_id) = symbols.insert_symbol_with_scope(
            SymbolSpace::Value,
            name.map(SymbolKey::Name),
            ScopeKind::Namespace,
            scope_id,
        );
        let kind = self.bind_declaration_kind(descriptor.kind);
        let anchor = self.bind_binding_anchor(descriptor.anchor);
        let export = descriptor
            .export
            .map(|export| self.bind_dependency_mode(export));
        let descriptor = DeclarationDescriptor {
            kind,
            anchor,
            name,
            export,
            symbol: symbol_id,
        };
        (descriptor, scope_id)
    }

    /// Bind an AST declaration into a DIR declaration.
    pub(super) fn bind_declaration(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        declaration_id: ast::LocalNodeId<ast::Declaration>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Declaration> {
        let declaration = module.get(declaration_id);
        let declaration = match declaration {
            ast::Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let generics = self.bind_generics(module, scope_id, generics, tree, symbols, types);
                let expressions = expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(module, scope_id, *expression, tree, symbols, types)
                    })
                    .collect();
                Declaration::Namespace {
                    descriptor,
                    generics,
                    scope: scope_id,
                    expressions,
                }
            }
            ast::Declaration::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties,
            } => {
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let kind = match kind {
                    ast::StructKind::Struct => StructKind::Struct,
                    ast::StructKind::Class => StructKind::Class,
                };
                let generics = self.bind_generics(module, scope_id, generics, tree, symbols, types);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree, symbols, types);
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(module, scope_id, *property, tree, symbols, types)
                    })
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
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let generics = self.bind_generics(module, scope_id, generics, tree, symbols, types);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree, symbols, types);
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_enum_field(module, scope_id, *field, tree, symbols, types)
                    })
                    .collect();
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(module, scope_id, *property, tree, symbols, types)
                    })
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
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let generics = self.bind_generics(module, scope_id, generics, tree, symbols, types);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree, symbols, types);
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(module, scope_id, *property, tree, symbols, types)
                    })
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
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let generics = self.bind_generics(module, scope_id, generics, tree, symbols, types);
                let target_type =
                    self.bind_expression(module, scope_id, *target_type, tree, symbols, types);
                let heritage = self.bind_heritage(module, scope_id, heritage, tree, symbols, types);
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(module, scope_id, *property, tree, symbols, types)
                    })
                    .collect();
                Declaration::Implement {
                    descriptor,
                    generics,
                    target_type,
                    target_symbol: None,
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
                let (descriptor, scope_id) =
                    self.bind_declaration_descriptor(module, scope_id, descriptor, symbols);
                let signature =
                    self.bind_function_signature(module, scope_id, signature, tree, symbols, types);
                let body = body
                    .map(|body| self.bind_expression(module, scope_id, body, tree, symbols, types));
                Declaration::Function {
                    descriptor,
                    signature,
                    scope: scope_id,
                    body,
                }
            }
        };
        let symbol_id = declaration.symbol();
        let declaration_id = tree.insert_from_source(declaration, declaration_id, scope_id);
        symbols.set_primary_declaration(symbol_id, declaration_id);
        declaration_id
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        field_id: ast::LocalNodeId<ast::EnumField>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<EnumField> {
        let field = module.get(field_id);
        let name = self
            .program
            .strings
            .intern_from(&module.ast_strings, field.name.string());
        let value = field
            .value
            .map(|value| self.bind_expression(module, scope_id, value, tree, symbols, types));
        let symbol_id =
            symbols.insert_symbol(SymbolSpace::Value, Some(SymbolKey::Name(name)), scope_id);
        let enum_field = EnumField {
            name,
            value,
            symbol: symbol_id,
        };
        let enum_field_id = tree.insert_from_source(enum_field, field_id, scope_id);
        symbols.set_primary_declaration(symbol_id, enum_field_id);
        enum_field_id
    }
}
