use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    BindingAnchor, Declaration, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    EnumField, EnumKind, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree,
    NodeType, ScopeKind, StaticKey, SymbolBinding, SymbolKind, SymbolSpace, SymbolTable,
    SymbolType, TypeTable,
};
use destack_workspace::{Module, ModuleAst};

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

    /// Bind declaration abstraction to DIR declaration abstraction.
    pub(super) fn bind_declaration_abstraction(
        &self,
        abstraction: ast::DeclarationAbstraction,
    ) -> DeclarationAbstraction {
        match abstraction {
            ast::DeclarationAbstraction::Abstract => DeclarationAbstraction::Abstract,
            ast::DeclarationAbstraction::Concrete => DeclarationAbstraction::Concrete,
        }
    }

    /// Bind enum kind to DIR enum kind.
    pub(super) fn bind_enum_kind(&self, kind: ast::EnumKind) -> EnumKind {
        match kind {
            ast::EnumKind::Enum => EnumKind::Enum,
            ast::EnumKind::Const => EnumKind::Const,
        }
    }

    /// Bind AST declaration descriptor into DIR declaration descriptor (including symbol and scope).
    pub(super) fn bind_declaration_descriptor(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        kind: SymbolKind,
        symbol_type: SymbolType,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        let name = descriptor.name.map(|name| {
            self.program
                .strings
                .intern_from(&ast.strings, name.string())
        });
        let export = descriptor
            .export
            .map(|export| self.bind_dependency_mode(export));
        let space = match symbol_type {
            SymbolType::TypeAlias | SymbolType::Interface => SymbolSpace::Type,
            SymbolType::Class
            | SymbolType::Enum
            | SymbolType::Function
            | SymbolType::Struct
            | SymbolType::Newtype
            | SymbolType::Extension => SymbolSpace::TypeValue,
            SymbolType::Void => SymbolSpace::Value,
        };

        // declaration kind is always a declaration in declaration files
        let mut declaration_kind = self.bind_declaration_kind(descriptor.kind);
        if module.language_type.is_declaration() {
            declaration_kind = DeclarationKind::Declaration;
        }

        // map the declaration kind into a binding mode
        let binding = match declaration_kind {
            DeclarationKind::Declaration => SymbolBinding::Ambient,
            DeclarationKind::Definition => SymbolBinding::Runtime,
        };
        let key = name.map(StaticKey::Name);

        // check for mergeable symbols in the current scope
        let (merge_symbol, merge_group) = key
            .map(|key| {
                self.select_merge_candidate(
                    module,
                    scope,
                    key,
                    kind,
                    symbol_type,
                    binding,
                    space,
                    symbols,
                )
            })
            .unwrap_or((None, None));

        // bind or reuse a symbol id
        let symbol_id = if let Some(existing_id) = merge_symbol {
            if let Some(export) = export
                && symbols.get_symbol(existing_id).export.is_none()
            {
                symbols.get_symbol_mut(existing_id).export = Some(export);
            }
            existing_id
        } else {
            let (symbol_id, _) =
                symbols.insert_symbol(kind, symbol_type, space, binding, key, scope, export);
            symbol_id
        };

        // select a declaration scope
        let reuse_scope = kind == SymbolKind::Namespace
            && merge_symbol.is_some_and(|existing_id| {
                symbols.get_symbol(existing_id).kind == SymbolKind::Namespace
            });

        // reuse the existing scope for namespace merges
        let scope_id = if reuse_scope {
            symbols.get_symbol(symbol_id).scope.0
        }
        // create a fresh scope for this declaration
        else {
            let scope_kind = ScopeKind::Namespace;
            symbols.insert_scope(scope_kind, Some(scope), Some(symbol_id))
        };

        // promote namespace merges to namespace symbols and point to the namespace scope
        if kind == SymbolKind::Namespace {
            let scope_mark = symbols.get_scope_mark(scope_id);
            let symbol = symbols.get_symbol_mut(symbol_id);
            symbol.kind = SymbolKind::Namespace;
            symbol.scope = (scope_id, scope_mark);
        }

        // attach the new symbol to a merge group if needed
        if let Some(group_id) = merge_group
            && symbols.get_symbol(symbol_id).merge_group != Some(group_id)
        {
            symbols.add_to_merge_group(group_id, symbol_id);
        }
        let abstraction = self.bind_declaration_abstraction(descriptor.abstraction);
        let anchor = self.bind_binding_anchor(descriptor.anchor);
        let descriptor = DeclarationDescriptor {
            kind: declaration_kind,
            abstraction,
            anchor,
            name,
            export,
            symbol: symbol_id,
        };
        (descriptor, scope_id)
    }

    /// Bind AST declaration descriptor for a global augmentation.
    pub(super) fn bind_global_descriptor(
        &self,
        module: &Module,
        _ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        symbols: &mut SymbolTable,
    ) -> DeclarationDescriptor {
        let export = descriptor
            .export
            .map(|export| self.bind_dependency_mode(export));

        // declare bindings are implicit in declaration files
        let mut declaration_kind = self.bind_declaration_kind(descriptor.kind);
        if module.language_type.is_declaration() {
            declaration_kind = DeclarationKind::Declaration;
        }

        // map the declaration kind into a binding mode
        let binding = match declaration_kind {
            DeclarationKind::Declaration => SymbolBinding::Ambient,
            DeclarationKind::Definition => SymbolBinding::Runtime,
        };
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Item,
            SymbolType::Void,
            SymbolSpace::Value,
            binding,
            None,
            scope,
            export,
        );
        let kind = declaration_kind;
        let abstraction = self.bind_declaration_abstraction(descriptor.abstraction);
        let anchor = self.bind_binding_anchor(descriptor.anchor);

        DeclarationDescriptor {
            kind,
            abstraction,
            anchor,
            name: None,
            export,
            symbol: symbol_id,
        }
    }

    /// Bind an AST declaration into a DIR declaration.
    pub(super) fn bind_declaration(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_declaration_id: ast::LocalNodeId<ast::Declaration>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Declaration> {
        let ast_declaration = ast.tree.get(ast_declaration_id);
        let declaration_id = tree.reserve_from_source(
            NodeType::Declaration,
            ast_declaration_id.id,
            scope,
            parent_id,
        );
        let declaration = match ast_declaration {
            ast::Declaration::Global {
                descriptor,
                expressions,
            } => {
                let descriptor =
                    self.bind_global_descriptor(module, ast, scope, descriptor, symbols);
                let global_scope_id = module.dir_base().global_augmentation_scope;
                let global_scope = (global_scope_id, symbols.get_scope_mark(global_scope_id));
                let expressions = expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            global_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Global {
                    descriptor,
                    scope: global_scope_id,
                    expressions,
                }
            }
            ast::Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Namespace,
                    SymbolType::Void,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
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
                            ast,
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
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    symbol_kind,
                    SymbolType::TypeAlias,
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
                                ast,
                                (scope_id, symbols.get_scope_mark(scope_id)),
                                *param,
                                Some(declaration_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                let scope = (scope_id, symbols.get_scope_mark(scope_id));
                let value = self.bind_expression(
                    module,
                    ast,
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
                members,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Struct,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *member,
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
                    members,
                }
            }
            ast::Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Class,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *member,
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
                    members,
                }
            }
            ast::Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Enum,
                    symbols,
                );
                let kind = self.bind_enum_kind(*kind);
                let generics = self.bind_generics(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
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
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *field,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Enum {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    scope: scope_id,
                    fields,
                    members,
                }
            }
            ast::Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Interface,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Declaration::Interface {
                    descriptor,
                    kind: self.bind_type_kind(*kind),
                    generics,
                    heritage,
                    scope: scope_id,
                    members,
                }
            }
            ast::Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Extension,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let target_type = self.bind_expression(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *target_type,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    heritage,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *member,
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
                    members,
                }
            }
            ast::Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Function,
                    symbols,
                );
                let signature = self.bind_function_signature(
                    module,
                    ast,
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
                        ast,
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
        // attach the declaration to the symbol
        let symbol_entry = symbols.get_symbol_mut(symbol_id);
        if symbol_entry.primary_declaration.is_some() {
            symbol_entry.declare_secondary(declaration_id);
        } else {
            symbol_entry.declare_primary(declaration_id);
        }
        declaration_id
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_field_id: ast::LocalNodeId<ast::EnumField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<EnumField> {
        let ast_field = ast.tree.get(ast_field_id);
        let field_id =
            tree.reserve_from_source(NodeType::EnumField, ast_field_id.id, scope, parent_id);
        let name = self
            .program
            .strings
            .intern_from(&ast.strings, ast_field.name.string());
        let value = ast_field.value.map(|value| {
            self.bind_expression(
                module,
                ast,
                scope,
                value,
                Some(field_id),
                tree,
                symbols,
                types,
            )
        });
        let (symbol_id, _) = self.bind_named_item(
            module,
            ast,
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
