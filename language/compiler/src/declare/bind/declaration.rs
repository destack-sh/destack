use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingTable, ClassDeclaration, Declaration, DeclaredModule, EnumDeclaration, EnumField,
    EnumKind, ExportKind, Expression, ExtensionDeclaration, FunctionDeclaration, GlobalDeclaration,
    InterfaceDeclaration, InterfaceHeritage, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, LocalSymbolId, ModuleDeclaration, Name, NamespaceDeclaration, NamespaceForm,
    NodeType, ScopeKind, StaticKey, StructDeclaration, SymbolBinding, SymbolForm, SymbolRole,
    SymbolSpace, Tree, TypeDeclaration, TypeTable,
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

    /// Bind an AST namespace form to a DIR namespace form.
    #[inline]
    pub(super) fn bind_namespace_form(&self, form: ast::NamespaceForm) -> NamespaceForm {
        match form {
            ast::NamespaceForm::Namespace => NamespaceForm::Namespace,
            ast::NamespaceForm::Module => NamespaceForm::Module,
        }
    }

    /// Return the symbol binding used for one declaration header.
    fn bind_declaration_binding(&self, module: &Module, is_ambient: bool) -> SymbolBinding {
        if module.is_declaration() || is_ambient {
            SymbolBinding::Ambient
        } else {
            SymbolBinding::Runtime
        }
    }

    /// Return the symbol space used for one declaration form.
    fn symbol_space_for_form(&self, _module: &Module, symbol_form: SymbolForm) -> SymbolSpace {
        match symbol_form {
            SymbolForm::TypeAlias | SymbolForm::Interface => SymbolSpace::Type,
            SymbolForm::Class
            | SymbolForm::Enum
            | SymbolForm::Struct
            | SymbolForm::Newtype
            | SymbolForm::Extension
            | SymbolForm::Function
            | SymbolForm::Variable => SymbolSpace::Value,
        }
    }

    /// Bind one named or anonymous declaration symbol.
    fn bind_declaration_symbol(
        &self,
        _module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        name: Option<Name>,
        export: Option<ExportKind>,
        role: SymbolRole,
        symbol_form: SymbolForm,
        binding: SymbolBinding,
        space: SymbolSpace,
        symbols: &mut BindingTable,
    ) -> LocalSymbolId {
        // declaration key
        let key = name.map(|name| StaticKey::Name(name.string()));

        // insert a fresh declaration symbol
        let symbol_id = symbols
            .insert_symbol(role, symbol_form, space, binding, key, scope, export)
            .0;

        symbol_id
    }

    /// Bind one named or anonymous declaration symbol with an owned scope.
    fn bind_declaration_symbol_with_scope(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        name: Option<Name>,
        export: Option<ExportKind>,
        role: SymbolRole,
        symbol_form: SymbolForm,
        binding: SymbolBinding,
        symbols: &mut BindingTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        // symbol space
        let space = self.symbol_space_for_form(module, symbol_form);

        // declaration key
        let key = name.map(|name| StaticKey::Name(name.string()));

        // insert a fresh declaration symbol
        let symbol_id = symbols
            .insert_symbol(role, symbol_form, space, binding, key, scope, export)
            .0;

        // function declarations own a function scope, other declarations own a namespace scope
        let scope_kind = if symbol_form == SymbolForm::Function {
            ScopeKind::Function
        } else {
            ScopeKind::Namespace
        };
        let scope_id = symbols.insert_scope(scope_kind, Some(scope), Some(symbol_id));

        // namespace symbols own their namespace scope
        if role == SymbolRole::Namespace {
            let scope_mark = symbols.get_scope_mark(scope_id);
            let symbol = symbols.get_symbol_mut(symbol_id);
            symbol.role = SymbolRole::Namespace;
            symbol.scope = (scope_id, scope_mark);
        }

        (symbol_id, scope_id)
    }

    /// Check whether a namespace scope includes runtime value symbols.
    fn namespace_scope_has_runtime_value_symbols(
        &self,
        symbols: &BindingTable,
        scope_id: LocalScopeId,
    ) -> bool {
        let scope = symbols.get_scope_by_id(scope_id);

        // named symbols
        for (_, symbol_id) in symbols.named_symbols(scope) {
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.binding == SymbolBinding::Runtime
                && symbol.space.conflicts_with(SymbolSpace::Value)
            {
                return true;
            }
        }

        // anonymous symbols
        for symbol_id in symbols.anonymous_symbols(scope) {
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
        symbol_form: SymbolForm,
        is_statement_declaration: bool,
    ) -> bool {
        if is_statement_declaration || name.is_none() {
            return false;
        }

        matches!(symbol_form, SymbolForm::Class | SymbolForm::Function)
    }

    /// Insert a self binding for one named declaration expression.
    fn bind_declaration_expression_self_name(
        &self,
        scope_id: LocalScopeId,
        name: Name,
        symbol_form: SymbolForm,
        symbols: &mut BindingTable,
    ) -> LocalSymbolId {
        let key = StaticKey::Name(name.string());
        let scope = (scope_id, LocalScopeMark::end());

        // keep one self binding per name
        let self_scope = symbols.get_scope_by_id(scope_id);
        if let Some(symbol_id) = symbols.find_symbol_up_to(self_scope, key, scope.1) {
            return symbol_id;
        }

        symbols
            .insert_symbol(
                SymbolRole::Local,
                symbol_form,
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
        export: Option<ast::ExportKind>,
        is_ambient: bool,
        symbol_form: SymbolForm,
        is_statement_declaration: bool,
        symbols: &mut BindingTable,
    ) -> (
        Option<Name>,
        Option<ExportKind>,
        SymbolBinding,
        LocalSymbolId,
        LocalScopeId,
        Option<LocalSymbolId>,
    ) {
        let name_is_self_scope_only = self.declaration_expression_name_is_self_scope_only(
            name,
            symbol_form,
            is_statement_declaration,
        );
        let name = name.map(|name| self.bind_name(ast, name));
        let export = export.map(|export| self.bind_export_kind(export));
        let binding = self.bind_declaration_binding(module, is_ambient);

        // named expressions do not publish an outer binding
        if name_is_self_scope_only {
            let (symbol_id, scope_id) = self.bind_declaration_symbol_with_scope(
                module,
                scope,
                None,
                export,
                SymbolRole::Item,
                symbol_form,
                binding,
                symbols,
            );
            let self_symbol = name.map(|name| {
                self.bind_declaration_expression_self_name(scope_id, name, symbol_form, symbols)
            });

            return (name, export, binding, symbol_id, scope_id, self_symbol);
        }

        // statement declarations and anonymous expressions publish normally
        let (symbol_id, scope_id) = self.bind_declaration_symbol_with_scope(
            module,
            scope,
            name,
            export,
            SymbolRole::Item,
            symbol_form,
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
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_declaration_id: ast::LocalNodeId<ast::Declaration>,
        is_statement_declaration: bool,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
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
        let mut declared_module_data = None;

        // declaration payload
        let declaration = match ast_declaration {
            ast::Declaration::Global(declaration) => {
                // symbol and declaration scope
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let symbol = self.bind_declaration_symbol(
                    module,
                    scope,
                    None,
                    None,
                    SymbolRole::Item,
                    SymbolForm::Variable,
                    binding,
                    SymbolSpace::Value,
                    symbols,
                );
                let declaration_scope = (global_scope, symbols.get_scope_mark(global_scope));

                // body
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();

                Declaration::Global(GlobalDeclaration {
                    is_ambient,
                    symbol,
                    scope: global_scope,
                    expressions,
                })
            }
            ast::Declaration::Module(declaration) => {
                // symbol
                let symbol = self.bind_declaration_symbol(
                    module,
                    scope,
                    None,
                    None,
                    SymbolRole::Item,
                    SymbolForm::Variable,
                    SymbolBinding::Runtime,
                    SymbolSpace::Value,
                    symbols,
                );
                let declaration_scope = (namespace_scope, symbols.get_scope_mark(namespace_scope));

                // body
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();

                Declaration::Module(ModuleDeclaration {
                    symbol,
                    scope: namespace_scope,
                    expressions,
                })
            }
            ast::Declaration::Namespace(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let form = self.bind_namespace_form(declaration.form);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    SymbolRole::Namespace,
                    SymbolForm::Variable,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();

                // runtime free namespaces degrade to is_ambient
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
                            SymbolRole::Namespace,
                            SymbolForm::Variable,
                            SymbolSpace::Value,
                            binding,
                            None,
                            (scope_id, scope_mark),
                            Some(ExportKind::Default),
                        )
                        .0;
                    let export_assignment_symbol = symbols
                        .insert_symbol(
                            SymbolRole::Namespace,
                            SymbolForm::Variable,
                            SymbolSpace::Value,
                            binding,
                            None,
                            (scope_id, scope_mark),
                            None,
                        )
                        .0;

                    declared_module_data = Some((
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
                    is_ambient,
                    symbol,
                    form,
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
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let symbol_kind = if declaration.export.is_some() {
                    SymbolRole::Item
                } else {
                    SymbolRole::Local
                };
                let symbol_form = if declaration.is_nominal {
                    SymbolForm::Newtype
                } else {
                    SymbolForm::TypeAlias
                };
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    symbol_kind,
                    symbol_form,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                    global_scope,
                    declared_modules,
                    declaration_scope,
                    declaration.value,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );

                Declaration::Type(TypeDeclaration {
                    name,
                    export,
                    is_ambient,
                    symbol,
                    scope: scope_id,
                    is_nominal: declaration.is_nominal,
                    mutability,
                    generic_parameters,
                    where_clauses,
                    value,
                })
            }
            ast::Declaration::Struct(declaration) => {
                // declaration header
                let name = self.bind_name(ast, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    Some(name),
                    export,
                    SymbolRole::Item,
                    SymbolForm::Struct,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
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
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
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
                            global_scope,
                            declared_modules,
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
                    is_ambient,
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
                        declaration.is_ambient,
                        SymbolForm::Class,
                        is_statement_declaration,
                        symbols,
                    );
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                        global_scope,
                        declared_modules,
                        declaration_scope,
                        expression,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });
                let extends_generic_arguments = declaration
                    .extends_generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *argument,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
                        )
                    })
                    .collect();
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
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
                            global_scope,
                            declared_modules,
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
                    is_ambient,
                    symbol,
                    self_symbol,
                    scope: scope_id,
                    is_abstract: declaration.is_abstract,
                    is_final: declaration.is_final,
                    generic_parameters,
                    where_clauses,
                    extends_expression,
                    extends_generic_arguments,
                    implements_types,
                    members,
                })
            }
            ast::Declaration::Enum(declaration) => {
                // declaration header
                let name = declaration.name.map(|name| self.bind_name(ast, name));
                let export = declaration
                    .export
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolRole::Item,
                    SymbolForm::Enum,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                    is_ambient,
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
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolRole::Item,
                    SymbolForm::Interface,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                let extends = declaration
                    .extends
                    .iter()
                    .map(|heritage| {
                        let expression = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            heritage.expression,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
                        );
                        let generic_arguments = heritage
                            .generic_arguments
                            .iter()
                            .map(|argument| {
                                self.bind_generic_argument(
                                    module,
                                    ast,
                                    namespace_scope,
                                    global_scope,
                                    declared_modules,
                                    declaration_scope,
                                    *argument,
                                    Some(declaration_id),
                                    tree,
                                    symbols,
                                    types,
                                    SymbolSpace::Type,
                                )
                            })
                            .collect();

                        InterfaceHeritage {
                            expression,
                            generic_arguments,
                        }
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
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *member,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
                        )
                    })
                    .collect();

                Declaration::Interface(InterfaceDeclaration {
                    name,
                    export,
                    is_ambient,
                    symbol,
                    scope: scope_id,
                    is_nominal: declaration.is_nominal,
                    generic_parameters,
                    where_clauses,
                    extends,
                    members,
                })
            }
            ast::Declaration::Extension(declaration) => {
                // declaration header
                let name = declaration.name.map(|name| self.bind_name(ast, name));
                let export = declaration
                    .export
                    .map(|export| self.bind_export_kind(export));
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let binding = self.bind_declaration_binding(module, declaration.is_ambient);
                let (symbol, scope_id) = self.bind_declaration_symbol_with_scope(
                    module,
                    scope,
                    name,
                    export,
                    SymbolRole::Item,
                    SymbolForm::Extension,
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
                            global_scope,
                            declared_modules,
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
                            global_scope,
                            declared_modules,
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
                    global_scope,
                    declared_modules,
                    declaration_scope,
                    declaration.target_type,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declaration_scope,
                            *ty,
                            Some(declaration_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
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
                            global_scope,
                            declared_modules,
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
                    is_ambient,
                    symbol,
                    scope: scope_id,
                    generic_parameters,
                    where_clauses,
                    target_type,
                    implements_types,
                    members,
                })
            }
            ast::Declaration::Function(declaration) => {
                // declaration header
                let binding_ambient = if declaration.body.is_none() && module.is_destack() {
                    true
                } else {
                    declaration.is_ambient
                };
                let (name, export, binding, symbol, scope_id, self_symbol) = self
                    .bind_expression_declaration_symbol_with_scope(
                        module,
                        ast,
                        scope,
                        declaration.name,
                        declaration.export,
                        binding_ambient,
                        SymbolForm::Function,
                        is_statement_declaration,
                        symbols,
                    );
                let is_ambient = self.bind_ambientness(declaration.is_ambient);
                let declaration_scope = (scope_id, symbols.get_scope_mark(scope_id));

                // signature and body
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    declaration_scope,
                    &declaration.signature,
                    Some(declaration_id),
                    tree,
                    symbols,
                    types,
                );
                let body_scope = (scope_id, symbols.get_scope_mark(scope_id));
                let body = declaration.body.map(|body| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        body_scope,
                        body,
                        Some(declaration_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Value,
                    )
                });

                // keep the effective declaration binding on the declaration symbol
                symbols.get_symbol_mut(symbol).binding = binding;

                Declaration::Function(FunctionDeclaration {
                    name,
                    export,
                    is_ambient,
                    symbol,
                    self_symbol,
                    scope: scope_id,
                    signature,
                    body,
                })
            }
        };

        let symbol_id = declaration.symbol();
        let declaration_id = tree.insert(declaration_id, declaration);

        // delayed declared module registration
        if let Some((specifier, scope_id, expressions, default_symbol, export_assignment_symbol)) =
            declared_module_data
        {
            let declared_module = DeclaredModule {
                specifier,
                declaration: declaration_id,
                scope: scope_id,
                expressions,
                default_symbol,
                export_assignment_symbol,
            };
            declared_modules.push(declared_module);
        }

        // attach the declaration to the symbol
        symbols.declare_symbol(symbol_id, declaration_id);

        declaration_id
    }

    /// Bind an AST enum field into a DIR enum field.
    pub(super) fn bind_enum_field(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_field_id: ast::LocalNodeId<ast::EnumField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<EnumField> {
        let ast_field = ast.tree.get(ast_field_id);
        let field_id =
            tree.reserve_from_source(NodeType::EnumField, ast_field_id.id, scope, parent_id);

        // field payload
        let name = self.bind_name(ast, ast_field.name);
        let key = StaticKey::Name(name.string());
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolRole::Item,
            SymbolForm::Variable,
            SymbolSpace::Value,
            self.bind_declaration_binding(module, false),
            Some(key),
            scope,
            None,
        );
        let value = ast_field.value.map(|value| {
            self.bind_expression(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                scope,
                value,
                Some(field_id),
                tree,
                symbols,
                types,
                SymbolSpace::Value,
            )
        });

        let enum_field = EnumField { name, value };

        let field_id = tree.insert(field_id, enum_field);
        symbols.declare_symbol(symbol_id, field_id);

        field_id
    }
}
