use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingAnchor, BindingCategory, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    DeclarationKind, DependencyItem, DependencyKind, DependencyMode, DependencySource, EnumField,
    EnumKind, Expression, ImportAliasTarget, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, LocalSymbolId, ModuleBinding, Name, NamespaceKind, NodeTree, NodeType,
    ProvenanceReason, ScopeKind, StaticKey, SymbolBinding, SymbolKind, SymbolSpace,
    SymbolSpaceOrder, SymbolTable, SymbolType, TypeTable,
};
use destack_dir::{
    ClassDeclaration, EnumDeclaration, ExportMode, ExtensionDeclaration, FunctionDeclaration,
    GlobalDeclaration, ImportAliasDeclaration, ImportSource, InterfaceDeclaration,
    NamespaceDeclaration, StructDeclaration, TypeDeclaration,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind enum kind to DIR enum kind.
    #[inline]
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

    /// Return the symbol binding used for one declaration header.
    fn bind_declaration_binding(
        &self,
        module: &Module,
        ambient: ast::Ambientness,
    ) -> SymbolBinding {
        if module.language_type.is_declaration() || ambient == ast::Ambientness::Ambient {
            SymbolBinding::Ambient
        } else {
            SymbolBinding::Runtime
        }
    }

    /// Return the symbol space used for one declaration symbol type.
    fn bind_declaration_symbol_space(
        &self,
        module: &Module,
        symbol_type: SymbolType,
    ) -> SymbolSpace {
        match symbol_type {
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
        }
    }

    /// Bind one named or anonymous declaration symbol.
    fn bind_declaration_symbol(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        name: Option<Name>,
        export: Option<ExportMode>,
        kind: SymbolKind,
        symbol_type: SymbolType,
        binding: SymbolBinding,
        space: SymbolSpace,
        symbols: &mut SymbolTable,
    ) -> LocalSymbolId {
        // key and merge lookup
        let key = name.map(|name| StaticKey::Name(name.string()));
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

        // reuse or insert symbol
        let symbol_id = if let Some(symbol_id) = merge_symbol {
            if let Some(export) = export
                && symbols.get_symbol(symbol_id).export.is_none()
            {
                symbols.get_symbol_mut(symbol_id).export = Some(export);
            }

            symbol_id
        } else {
            symbols
                .insert_symbol(kind, symbol_type, space, binding, key, scope, export)
                .0
        };

        // attach merge group metadata
        if let Some(group_id) = merge_group
            && symbols.get_symbol(symbol_id).merge_group != Some(group_id)
        {
            symbols.add_to_merge_group(group_id, symbol_id);
        }

        symbol_id
    }

    /// Bind one named or anonymous declaration symbol with an owned scope.
    fn bind_declaration_symbol_with_scope(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        name: Option<Name>,
        export: Option<ExportMode>,
        kind: SymbolKind,
        symbol_type: SymbolType,
        binding: SymbolBinding,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        // symbol and merge handling
        let space = self.bind_declaration_symbol_space(module, symbol_type);
        let key = name.map(|name| StaticKey::Name(name.string()));
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
        let symbol_id = if let Some(symbol_id) = merge_symbol {
            if let Some(export) = export
                && symbols.get_symbol(symbol_id).export.is_none()
            {
                symbols.get_symbol_mut(symbol_id).export = Some(export);
            }

            symbol_id
        } else {
            symbols
                .insert_symbol(kind, symbol_type, space, binding, key, scope, export)
                .0
        };

        // namespace merges reuse their owned scope
        let reuse_scope = kind == SymbolKind::Namespace
            && merge_symbol.is_some_and(|symbol_id| {
                symbols.get_symbol(symbol_id).kind == SymbolKind::Namespace
            });
        let scope_id = if reuse_scope {
            symbols.get_symbol(symbol_id).scope.0
        } else {
            symbols.insert_scope(ScopeKind::Namespace, Some(scope), Some(symbol_id))
        };

        // namespace symbols own their namespace scope
        if kind == SymbolKind::Namespace {
            let scope_mark = symbols.get_scope_mark(scope_id);
            let symbol = symbols.get_symbol_mut(symbol_id);
            symbol.kind = SymbolKind::Namespace;
            symbol.scope = (scope_id, scope_mark);
        }

        // attach merge group metadata
        if let Some(group_id) = merge_group
            && symbols.get_symbol(symbol_id).merge_group != Some(group_id)
        {
            symbols.add_to_merge_group(group_id, symbol_id);
        }

        (symbol_id, scope_id)
    }

    /// Check whether a namespace scope includes runtime value symbols.
    fn namespace_scope_has_runtime_value_symbols(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
    ) -> bool {
        let scope = symbols.get_scope_by_id(scope_id);

        // named symbols
        for (_, symbol_id) in symbols.active_named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.binding == SymbolBinding::Runtime
                && symbol.space.conflicts_with(SymbolSpace::Value)
            {
                return true;
            }
        }

        // anonymous symbols
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

    /// Return true when a declaration expression name should live only in self scope.
    fn declaration_expression_name_is_self_scope_only(
        &self,
        name: Option<ast::Name>,
        symbol_type: SymbolType,
        is_statement_declaration: bool,
    ) -> bool {
        if is_statement_declaration || name.is_none() {
            return false;
        }

        matches!(symbol_type, SymbolType::Class | SymbolType::Function)
    }

    /// Insert a self binding for one named declaration expression.
    fn bind_declaration_expression_self_name(
        &self,
        scope_id: LocalScopeId,
        name: Name,
        symbol_type: SymbolType,
        symbols: &mut SymbolTable,
    ) -> LocalSymbolId {
        let key = StaticKey::Name(name.string());
        let scope = (scope_id, LocalScopeMark::end());

        // keep one self binding per name
        if let Some(symbol_id) = symbols.get_scope_by_id(scope_id).find_up_to(key, scope.1) {
            return symbol_id;
        }

        symbols
            .insert_symbol(
                SymbolKind::Local,
                symbol_type,
                SymbolSpace::Value,
                SymbolBinding::Runtime,
                Some(key),
                scope,
                None,
            )
            .0
    }

    /// Bind a class or function declaration symbol with expression self-name semantics.
    fn bind_expression_declaration_symbol_with_scope(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        name: Option<ast::Name>,
        export: Option<ast::ExportMode>,
        ambient: ast::Ambientness,
        symbol_type: SymbolType,
        is_statement_declaration: bool,
        symbols: &mut SymbolTable,
    ) -> (
        Option<Name>,
        Option<ExportMode>,
        SymbolBinding,
        LocalSymbolId,
        LocalScopeId,
        Option<LocalSymbolId>,
    ) {
        let name_is_self_scope_only = self.declaration_expression_name_is_self_scope_only(
            name,
            symbol_type,
            is_statement_declaration,
        );
        let name = name.map(|name| self.bind_name(ast, name));
        let export = export.map(|export| self.bind_export_mode(export));
        let binding = self.bind_declaration_binding(module, ambient);

        // named expressions do not publish an outer binding
        if name_is_self_scope_only {
            let (symbol_id, scope_id) = self.bind_declaration_symbol_with_scope(
                module,
                scope,
                None,
                export,
                SymbolKind::Item,
                symbol_type,
                binding,
                symbols,
            );
            let self_symbol = name.map(|name| {
                self.bind_declaration_expression_self_name(scope_id, name, symbol_type, symbols)
            });

            return (name, export, binding, symbol_id, scope_id, self_symbol);
        }

        // statement declarations and anonymous expressions publish normally
        let (symbol_id, scope_id) = self.bind_declaration_symbol_with_scope(
            module,
            scope,
            name,
            export,
            SymbolKind::Item,
            symbol_type,
            binding,
            symbols,
        );

        (name, export, binding, symbol_id, scope_id, None)
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

        // module declarations need a follow-up binding record once the declaration id is stable
        let mut module_binding_data = None;

        // declaration payload
        let declaration = match ast_declaration {
            ast::Declaration::Global(declaration) => {
                // symbol and declaration scope
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let symbol = self.bind_declaration_symbol(
                    module,
                    scope,
                    None,
                    None,
                    SymbolKind::Item,
                    SymbolType::Void,
                    binding,
                    SymbolSpace::Value,
                    symbols,
                );
                let declaration_scope = (
                    global_augmentation_scope,
                    symbols.get_scope_mark(global_augmentation_scope),
                );

                // body
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::ValueThenType,
                        )
                    })
                    .collect();

                Declaration::Global(GlobalDeclaration {
                    ambient,
                    symbol,
                    scope: global_augmentation_scope,
                    expressions,
                })
            }
            ast::Declaration::Namespace(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let kind = self.bind_namespace_kind(declaration.kind);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    SymbolKind::Namespace,
                    SymbolType::Void,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // body
                let expressions: Vec<LocalNodeId<Expression>> = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::ValueThenType,
                        )
                    })
                    .collect();

                // runtime free namespaces degrade to ambient
                if binding == SymbolBinding::Runtime
                    && !self.namespace_scope_has_runtime_value_symbols(symbols, scope_id)
                {
                    symbols.get_symbol_mut(symbol).binding = SymbolBinding::Ambient;
                }

                // string named modules produce a module-binding entry
                if let Name::String(specifier) = name {
                    let scope_mark = symbols.get_scope_mark(scope_id);
                    let binding = symbols.get_symbol(symbol).binding;
                    let default_symbol = symbols
                        .insert_symbol(
                            SymbolKind::Namespace,
                            SymbolType::Void,
                            SymbolSpace::Value,
                            binding,
                            None,
                            (scope_id, scope_mark),
                            Some(ExportMode::Default),
                        )
                        .0;
                    let export_assignment_symbol = symbols
                        .insert_symbol(
                            SymbolKind::Namespace,
                            SymbolType::Void,
                            SymbolSpace::Value,
                            binding,
                            None,
                            (scope_id, scope_mark),
                            None,
                        )
                        .0;

                    module_binding_data = Some((
                        specifier,
                        scope_id,
                        expressions.clone(),
                        default_symbol,
                        export_assignment_symbol,
                    ));
                }

                Declaration::Namespace(NamespaceDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    kind,
                    generic_parameters,
                    where_clauses,
                    scope: scope_id,
                    expressions,
                })
            }
            ast::Declaration::Type(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let symbol_kind = if declaration.export.is_some() {
                    SymbolKind::Item
                } else {
                    SymbolKind::Local
                };
                let symbol_type = if declaration.is_nominal {
                    SymbolType::Newtype
                } else {
                    SymbolType::TypeAlias
                };
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    symbol_kind,
                    symbol_type,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // value
                let mutability = declaration
                    .mutability
                    .map(|mutability| self.bind_mutability(mutability));
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    declaration_scope,
                    declaration.value,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );

                Declaration::Type(TypeDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    scope: scope_id,
                    is_nominal: declaration.is_nominal,
                    mutability,
                    generic_parameters,
                    where_clauses,
                    value,
                })
            }
            ast::Declaration::ImportAlias(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let kind = self.bind_dependency_kind(declaration.kind);
                let space = match declaration.kind {
                    ast::DependencyKind::Type => SymbolSpace::Type,
                    ast::DependencyKind::Value => SymbolSpace::TypeValue,
                };
                let symbol = self.bind_declaration_symbol(
                    module,
                    scope,
                    Some(name),
                    export,
                    SymbolKind::Item,
                    SymbolType::Void,
                    binding,
                    space,
                    symbols,
                );

                // alias target
                let target = match &declaration.target {
                    ast::ImportAliasTarget::Require { target } => {
                        let target = self.repository.strings.intern_from(&ast.strings, *target);
                        ImportAliasTarget::Require { target }
                    }
                    ast::ImportAliasTarget::Path { path } => {
                        let path = self.bind_path(module, ast, path);
                        ImportAliasTarget::Path { path }
                    }
                };

                // require aliases also synthesize a namespace dependency item
                if let ImportAliasTarget::Require { target } = target
                    && let Some(alias) = Some(name.string())
                {
                    let dependency_id = tree.reserve_from(
                        NodeType::DependencyItem,
                        declaration_id,
                        scope,
                        Some(declaration_id),
                        Some(ProvenanceReason::Bound),
                    );
                    let dependency = DependencyItem::UnresolvedRemote {
                        source: ImportSource::ImportEquals,
                        mode: destack_dir::DependencyMode::Namespace,
                        kind,
                        name: None,
                        alias: Some(alias),
                        target,
                        target_module: None,
                        symbol: Some(symbol),
                    };
                    tree.insert(dependency_id, dependency);

                    let target = ImportAliasTarget::Require { target };

                    Declaration::ImportAlias(ImportAliasDeclaration {
                        name,
                        export,
                        ambient,
                        symbol,
                        kind,
                        target,
                    })
                } else {
                    Declaration::ImportAlias(ImportAliasDeclaration {
                        name,
                        export,
                        ambient,
                        symbol,
                        kind,
                        target,
                    })
                }
            }
            ast::Declaration::Struct(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    SymbolKind::Item,
                    SymbolType::Struct,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // relations and body
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let embedded_types = declaration
                    .embedded_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                Declaration::Struct(StructDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    scope: scope_id,
                    generic_parameters,
                    where_clauses,
                    implements_types,
                    embedded_types,
                    members,
                })
            }
            ast::Declaration::Class(declaration) => {
                // declaration header
                let (name, export, ambient_binding, symbol, scope_id, self_symbol) = self
                    .bind_expression_declaration_symbol_with_scope(
                        module,
                        ast,
                        scope,
                        declaration.name,
                        declaration.export,
                        declaration.ambient,
                        SymbolType::Class,
                        is_statement_declaration,
                        symbols,
                    );
                let ambient = self.bind_ambientness(declaration.ambient);
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // relations and body
                let extends_expression = declaration.extends_expression.map(|expression| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        declaration_scope,
                        expression,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // ensure the declaration symbol keeps the resolved binding mode
                symbols.get_symbol_mut(symbol).binding = ambient_binding;

                Declaration::Class(ClassDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    self_symbol,
                    scope: scope_id,
                    is_abstract: declaration.is_abstract,
                    generic_parameters,
                    where_clauses,
                    extends_expression,
                    implements_types,
                    members,
                })
            }
            ast::Declaration::Enum(declaration) => {
                // declaration header
                let name = declaration.name.map(|name| self.bind_name(ast, name));
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolKind::Item,
                    SymbolType::Enum,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // relations and body
                let kind = self.bind_enum_kind(declaration.kind);
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let fields = declaration
                    .fields
                    .iter()
                    .map(|field| {
                        self.bind_enum_field(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *field,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                Declaration::Enum(EnumDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    scope: scope_id,
                    kind,
                    generic_parameters,
                    where_clauses,
                    implements_types,
                    fields,
                    members,
                })
            }
            ast::Declaration::Interface(declaration) => {
                // declaration header
                let name = declaration.name.map(|name| self.bind_name(ast, name));
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolKind::Item,
                    SymbolType::Interface,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // relations and body
                let extends_types = declaration
                    .extends_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.bind_type_member(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();

                Declaration::Interface(InterfaceDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    scope: scope_id,
                    is_nominal: declaration.is_nominal,
                    generic_parameters,
                    where_clauses,
                    extends_types,
                    members,
                })
            }
            ast::Declaration::Extension(declaration) => {
                // declaration header
                let name = declaration.name.map(|name| self.bind_name(ast, name));
                let export = declaration
                    .export
                    .map(|export| self.bind_export_mode(export));
                let ambient = self.bind_ambientness(declaration.ambient);
                let binding = self.bind_declaration_binding(module, declaration.ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolKind::Item,
                    SymbolType::Extension,
                    binding,
                    symbols,
                );
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *parameter,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *where_clause,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                // relations and body
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    declaration_scope,
                    declaration.target_type,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::TypeThenValue,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.bind_member(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                Declaration::Extension(ExtensionDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    scope: scope_id,
                    generic_parameters,
                    where_clauses,
                    target_type,
                    target_symbol: None,
                    implements_types,
                    members,
                })
            }
            ast::Declaration::Function(declaration) => {
                // declaration header
                let binding_ambient = if declaration.body.is_none()
                    && (module.language_type.supports_declaration_merging()
                        || module.language_type.is_destack())
                {
                    ast::Ambientness::Ambient
                } else {
                    declaration.ambient
                };
                let (name, export, binding, symbol, scope_id, self_symbol) = self
                    .bind_expression_declaration_symbol_with_scope(
                        module,
                        ast,
                        scope,
                        declaration.name,
                        declaration.export,
                        binding_ambient,
                        SymbolType::Function,
                        is_statement_declaration,
                        symbols,
                    );
                let ambient = self.bind_ambientness(declaration.ambient);
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // signature and body
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    declaration_scope,
                    &declaration.signature,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let body = declaration.body.map(|body| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        declaration_scope,
                        body,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });

                // keep the effective declaration binding on the declaration symbol
                symbols.get_symbol_mut(symbol).binding = binding;

                Declaration::Function(FunctionDeclaration {
                    name,
                    export,
                    ambient,
                    symbol,
                    self_symbol,
                    scope: scope_id,
                    signature,
                    body,
                })
            }
        };

        // duplicate-binding category
        let binding_category = match &declaration {
            Declaration::Function(_) | Declaration::Class(_) => Some(BindingCategory::BlockScoped),
            _ => None,
        };

        let symbol_id = declaration.symbol();
        let declaration_id = tree.insert(declaration_id, declaration);

        // delayed module binding registration
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

        // apply declaration category
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

        // field payload
        let name = self.bind_name(ast, ast_field.name);
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

        let enum_field = EnumField { name, value };

        tree.insert(field_id, enum_field)
    }
}
