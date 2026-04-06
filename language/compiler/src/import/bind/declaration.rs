use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingAnchor, BindingCategory, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    DeclarationKind, DependencyItem, DependencyKind, DependencyMode, DependencySource, EnumField,
    EnumKind, Expression, ImportAliasTarget, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, ModuleBinding, Name, NamespaceKind, NodeTree, NodeType, ScopeKind, StaticKey,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable, SymbolType, TypeTable,
};
use destack_workspace::Module;

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

    /// Bind namespace kind to DIR namespace kind.
    #[inline]
    pub(super) fn bind_namespace_kind(&self, kind: ast::NamespaceKind) -> NamespaceKind {
        match kind {
            ast::NamespaceKind::Namespace => NamespaceKind::Namespace,
            ast::NamespaceKind::Module => NamespaceKind::Module,
        }
    }

    /// Bind AST declaration descriptor into DIR declaration descriptor (including symbol and scope).
    pub(super) fn bind_declaration_descriptor(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        kind: SymbolKind,
        symbol_type: SymbolType,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        let space = match symbol_type {
            SymbolType::TypeAlias | SymbolType::Interface => SymbolSpace::Type,
            SymbolType::Class
            | SymbolType::Enum
            | SymbolType::Struct
            | SymbolType::Newtype
            | SymbolType::Extension => SymbolSpace::TypeValue,
            SymbolType::Function => {
                if module.language_type.is_destack() {
                    SymbolSpace::TypeValue
                } else {
                    SymbolSpace::Value
                }
            }
            SymbolType::Void => SymbolSpace::Value,
        };

        self.bind_declaration_descriptor_with_space(
            module,
            ast,
            scope,
            descriptor,
            kind,
            symbol_type,
            space,
            symbols,
        )
    }

    /// Bind AST declaration descriptor into DIR declaration descriptor with an explicit space.
    pub(super) fn bind_declaration_descriptor_with_space(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        kind: SymbolKind,
        symbol_type: SymbolType,
        space: SymbolSpace,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        // bind the name into the dir string pool
        let name = descriptor.name.map(|name| self.bind_name(ast, name));
        let export = descriptor
            .export
            .map(|export| self.bind_dependency_mode(export));

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
        let key = name.map(|name: Name| StaticKey::Name(name.string()));

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

    /// Check whether a namespace scope includes runtime value symbols.
    fn namespace_scope_has_runtime_value_symbols(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
    ) -> bool {
        let scope = symbols.get_scope_by_id(scope_id);

        // check named symbols for runtime value participation
        for (_, symbol_id) in symbols.active_named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.binding == SymbolBinding::Runtime
                && symbol.space.conflicts_with(SymbolSpace::Value)
            {
                return true;
            }
        }

        // check anonymous symbols for runtime value participation
        for symbol_id in symbols.active_anonymous_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.binding == SymbolBinding::Runtime
                && symbol.space.conflicts_with(SymbolSpace::Value)
            {
                return true;
            }
        }

        false
    }

    /// Return true when a declaration expression name should bind only in self scope.
    fn declaration_expression_name_is_self_scope_only(
        &self,
        _module: &Module,
        descriptor: &ast::DeclarationDescriptor,
        symbol_type: SymbolType,
        is_statement_declaration: bool,
    ) -> bool {
        // statement declarations publish names in the surrounding scope
        if is_statement_declaration {
            return false;
        }

        // only named class/function expressions keep a local self name
        if descriptor.name.is_none() {
            return false;
        }

        matches!(symbol_type, SymbolType::Class | SymbolType::Function)
    }

    /// Insert a self binding for named declaration expressions.
    fn bind_declaration_expression_self_name(
        &self,
        scope_id: LocalScopeId,
        name: Name,
        symbol_type: SymbolType,
        symbols: &mut SymbolTable,
    ) {
        let key = StaticKey::Name(name.string());
        let scope = (scope_id, LocalScopeMark::end());
        let has_binding = symbols
            .get_scope_by_id(scope_id)
            .find_up_to(key, scope.1)
            .is_some();

        // keep one self binding per name
        if has_binding {
            return;
        }

        symbols.insert_symbol(
            SymbolKind::Local,
            symbol_type,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            Some(key),
            scope,
            None,
        );
    }

    /// Bind a declaration descriptor with expression self-name semantics.
    fn bind_declaration_expression_descriptor(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        descriptor: &ast::DeclarationDescriptor,
        symbol_kind: SymbolKind,
        symbol_type: SymbolType,
        is_statement_declaration: bool,
        symbols: &mut SymbolTable,
    ) -> (DeclarationDescriptor, LocalScopeId) {
        // named class/function expressions keep their self name inside declaration scope
        let name_is_self_scope_only = self.declaration_expression_name_is_self_scope_only(
            module,
            descriptor,
            symbol_type,
            is_statement_declaration,
        );
        let expression_name = descriptor.name.map(|name| self.bind_name(ast, name));

        // hide outer name binding when the expression name is self-scope-only
        let mut descriptor_for_binding = *descriptor;
        if name_is_self_scope_only {
            descriptor_for_binding.name = None;
        }
        let (descriptor, scope_id) = self.bind_declaration_descriptor(
            module,
            ast,
            scope,
            &descriptor_for_binding,
            symbol_kind,
            symbol_type,
            symbols,
        );
        if !name_is_self_scope_only {
            return (descriptor, scope_id);
        }

        // insert one local self binding so recursion and self references resolve
        let Some(name) = expression_name else {
            return (descriptor, scope_id);
        };
        self.bind_declaration_expression_self_name(scope_id, name, symbol_type, symbols);

        let mut descriptor = descriptor;
        descriptor.name = Some(name);

        (descriptor, scope_id)
    }

    /// Bind AST declaration descriptor for a global augmentation.
    pub(super) fn bind_global_descriptor(
        &self,
        module: &Module,
        _ast: &Ast,
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
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_declaration_id: ast::LocalNodeId<ast::Declaration>,
        is_statement_declaration: bool,
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

        // track module binding data for module declarations
        let mut module_binding_data = None;

        // bind the declaration payload
        let declaration = match ast_declaration {
            ast::Declaration::Global {
                descriptor,
                expressions,
            } => {
                let descriptor =
                    self.bind_global_descriptor(module, ast, scope, descriptor, symbols);
                let global_scope_id = global_augmentation_scope;
                let global_scope = (global_scope_id, symbols.get_scope_mark(global_scope_id));
                let expressions: Vec<LocalNodeId<Expression>> = expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            global_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::ValueThenType,
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
                kind,
                generics,
                expressions,
            } => {
                let kind = self.bind_namespace_kind(*kind);
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    generics,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let expressions: Vec<LocalNodeId<Expression>> = expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::ValueThenType,
                        )
                    })
                    .collect();

                // type only namespaces do not emit runtime namespace objects
                if descriptor.kind == DeclarationKind::Definition
                    && !self.namespace_scope_has_runtime_value_symbols(symbols, scope_id)
                {
                    let symbol = symbols.get_symbol_mut(descriptor.symbol);
                    symbol.binding = SymbolBinding::Ambient;
                }

                // register module declarations for ambient module resolution
                if let Some(Name::String(specifier)) = descriptor.name {
                    // insert default and export assignment symbols for module declarations
                    let scope_mark = symbols.get_scope_mark(scope_id);
                    let scope = (scope_id, scope_mark);
                    let binding = symbols.get_symbol(descriptor.symbol).binding;
                    let (default_symbol, _) = symbols.insert_symbol(
                        SymbolKind::Namespace,
                        SymbolType::Void,
                        SymbolSpace::Value,
                        binding,
                        None,
                        scope,
                        Some(DependencyMode::Default),
                    );
                    let (export_assignment_symbol, _) = symbols.insert_symbol(
                        SymbolKind::Namespace,
                        SymbolType::Void,
                        SymbolSpace::Value,
                        binding,
                        None,
                        scope,
                        None,
                    );

                    module_binding_data = Some((
                        specifier,
                        scope_id,
                        expressions.clone(),
                        default_symbol,
                        export_assignment_symbol,
                    ));
                }

                Declaration::Namespace {
                    descriptor,
                    kind,
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
                let symbol_type = if *kind == ast::TypeKind::Nominal {
                    SymbolType::Newtype
                } else {
                    SymbolType::TypeAlias
                };
                let (descriptor, scope_id) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    symbol_kind,
                    symbol_type,
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
                                namespace_scope,
                                global_augmentation_scope,
                                module_bindings,
                                (scope_id, symbols.get_scope_mark(scope_id)),
                                SymbolSpace::Type,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                Declaration::Type {
                    descriptor,
                    kind,
                    mutability,
                    static_parameters,
                    value,
                }
            }

            ast::Declaration::ImportAlias {
                descriptor,
                kind,
                target,
            } => {
                let symbol_space = match kind {
                    ast::DependencyKind::Type => SymbolSpace::Type,
                    ast::DependencyKind::Value => SymbolSpace::TypeValue,
                };
                let (descriptor, _scope_id) = self.bind_declaration_descriptor_with_space(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Void,
                    symbol_space,
                    symbols,
                );
                let kind = self.bind_dependency_kind(*kind);
                let scope = (scope.0, symbols.get_scope_mark(scope.0));
                let symbol_id = descriptor.symbol;
                let alias_name = descriptor.name.map(|name| name.string());
                let target = match target {
                    ast::ImportAliasTarget::Require { target } => {
                        let target = self.repository.strings.intern_from(&ast.strings, *target);
                        ImportAliasTarget::Require { target }
                    }
                    ast::ImportAliasTarget::Path { value } => {
                        let space_order = match kind {
                            DependencyKind::Type => SymbolSpaceOrder::TypeThenValue,
                            DependencyKind::Value => SymbolSpaceOrder::ValueThenType,
                        };
                        let value = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            *value,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            space_order,
                        );
                        ImportAliasTarget::Path { value }
                    }
                };
                let require_target = match &target {
                    ImportAliasTarget::Require { target } => Some(*target),
                    ImportAliasTarget::Path { .. } => None,
                };
                let declaration = Declaration::ImportAlias {
                    descriptor,
                    kind,
                    target,
                };

                if let Some(target) = require_target
                    && let Some(name) = alias_name
                {
                    let dependency_id = tree.reserve_from(
                        NodeType::DependencyItem,
                        declaration_id,
                        scope,
                        Some(declaration_id),
                    );
                    let dependency = DependencyItem::UnresolvedRemote {
                        source: DependencySource::ImportEquals,
                        mode: DependencyMode::Namespace,
                        kind,
                        name: None,
                        alias: Some(name),
                        target,
                        target_module: None,
                        symbol: Some(symbol_id),
                    };
                    tree.insert(dependency_id, dependency);
                }

                declaration
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                let (descriptor, scope_id) = self.bind_declaration_expression_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    SymbolKind::Item,
                    SymbolType::Class,
                    is_statement_declaration,
                    symbols,
                );
                let generics = self.bind_generics(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *target_type,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                let heritage = self.bind_heritage(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
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
                // treat signature-only functions as declarations in mergeable languages
                let mut descriptor = *descriptor;
                if body.is_none()
                    && (module.language_type.supports_declaration_merging()
                        || module.language_type.is_destack())
                {
                    descriptor.kind = ast::DeclarationKind::Declaration;
                }
                let (descriptor, scope_id) = self.bind_declaration_expression_descriptor(
                    module,
                    ast,
                    scope,
                    &descriptor,
                    SymbolKind::Item,
                    SymbolType::Function,
                    is_statement_declaration,
                    symbols,
                );
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        body,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
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
        // classify declaration symbols for duplicate-binding checks
        let binding_category = match &declaration {
            Declaration::Function { .. } | Declaration::Class { .. } => {
                Some(BindingCategory::BlockScoped)
            }
            _ => None,
        };

        let symbol_id = declaration.symbol();
        let declaration_id = tree.insert(declaration_id, declaration);

        // register module bindings once the declaration id is stable
        if let Some((specifier, scope_id, expressions, default_symbol, export_assignment_symbol)) =
            module_binding_data
        {
            let module_binding = ModuleBinding {
                specifier,
                declaration: declaration_id,
                scope: scope_id,
                expressions,
                default_symbol,
                export_assignment_symbol,
            };
            module_bindings.push(module_binding);
        }

        // attach the declaration to the symbol
        let symbol_entry = symbols.get_symbol_mut(symbol_id);
        if symbol_entry.primary_declaration.is_some() {
            symbol_entry.declare_secondary(declaration_id);
        } else {
            symbol_entry.declare_primary(declaration_id);
        }

        // apply declaration category once the symbol is known
        if let Some(binding_category) = binding_category {
            self.apply_binding_category(symbols, symbol_id, binding_category);
        }

        declaration_id
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
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
            .repository
            .strings
            .intern_from(&ast.strings, ast_field.name.string());
        let value = ast_field.value.map(|value| {
            self.bind_expression(
                module,
                ast,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                scope,
                value,
                Some(field_id),
                tree,
                symbols,
                types,
                SymbolSpaceOrder::ValueThenType,
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
