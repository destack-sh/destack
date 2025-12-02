use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    BindingAnchor, Declaration, DeclarationDescriptor, DeclarationKind, EnumField, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalScopeMark, Module, NodeTree, NodeType, ScopeKind, StaticKey,
    SymbolKind, SymbolSpace, SymbolTable, TypeTable,
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
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        kind: SymbolKind,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        let name = descriptor.name.map(|name| {
            self.program
                .strings
                .intern_from(&module.ast_strings, name.string())
        });
        let export = descriptor
            .export
            .map(|export| self.bind_dependency_mode(export));
        let (symbol_id, scope_id) = {
            let (symbol_id, _) = symbols.insert_symbol(
                kind,
                SymbolSpace::Value,
                name.map(StaticKey::Name),
                scope,
                export,
            );
            let scope_id = symbols.insert_scope(ScopeKind::Namespace, Some(scope), Some(symbol_id));
            (symbol_id, scope_id)
        };
        let kind = self.bind_declaration_kind(descriptor.kind);
        let anchor = self.bind_binding_anchor(descriptor.anchor);
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
        scope: (LocalScopeId, LocalScopeMark),
        ast_declaration_id: ast::LocalNodeId<ast::Declaration>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Declaration> {
        let ast_declaration = module.ast.get(ast_declaration_id);
        let declaration_id =
            tree.reserve_from_source(NodeType::Declaration, ast_declaration_id, scope, parent_id);
        let declaration = match ast_declaration {
            ast::Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let expressions = expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Namespace {
                    descriptor,
                    generics,
                    scope: scope_id,
                    expressions,
                }
            }
            ast::Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value,
            } => {
                let symbol_kind = if descriptor.export.is_some() {
                    SymbolKind::Item
                } else {
                    SymbolKind::Local
                };
                let (descriptor, _scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    symbol_kind,
                    symbols,
                );
                let kind = self.bind_type_kind(*kind);
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| {
                            self.bind_parameter(
                                module,
                                scope,
                                *param,
                                Some(declaration_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                let value = self.bind_expression(
                    module,
                    scope,
                    *value,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                Declaration::Type {
                    descriptor,
                    kind,
                    mutability,
                    static_parameters,
                    value,
                }
            }
            ast::Declaration::Struct {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *property,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Struct {
                    descriptor,
                    generics,
                    heritage,
                    scope: scope_id,
                    properties,
                }
            }
            ast::Declaration::Class {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *property,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Class {
                    descriptor,
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
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_enum_field(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *field,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *property,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
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
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *property,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
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
            ast::Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                properties,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let target_type = self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *target_type,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *property,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Extension {
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
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    symbols,
                );
                let signature = self.bind_function_signature(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    signature,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let body = body.map(|body| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        body,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Declaration::Function {
                    descriptor,
                    signature,
                    scope: scope_id,
                    body,
                }
            }
        };
        let symbol_id = declaration.symbol();
        let declaration_id = tree.insert(declaration_id, declaration);
        symbols
            .get_symbol_mut(symbol_id)
            .declare_primary(declaration_id);
        declaration_id
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_field_id: ast::LocalNodeId<ast::EnumField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<EnumField> {
        let ast_field = module.ast.get(ast_field_id);
        let field_id =
            tree.reserve_from_source(NodeType::EnumField, ast_field_id, scope, parent_id);
        let name = self
            .program
            .strings
            .intern_from(&module.ast_strings, ast_field.name.string());
        let value = ast_field.value.map(|value| {
            self.bind_expression(module, scope, value, Some(field_id), tree, symbols, types)
        });
        let (symbol_id, _) = self.bind_named_item(
            module,
            SymbolSpace::Value,
            StaticKey::Name(name),
            scope,
            None,
            symbols,
        );
        let enum_field = EnumField {
            name,
            value,
            symbol: symbol_id,
        };
        let enum_field_id = tree.insert(field_id, enum_field);
        symbols
            .get_symbol_mut(symbol_id)
            .declare_primary(enum_field_id);
        enum_field_id
    }
}
