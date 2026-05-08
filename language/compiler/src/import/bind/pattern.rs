use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingScope, DeclaredModule, ExportKind, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, LocalSymbolId, Mutability, NodeType, Pattern, PatternField, ScopeKind,
    StaticKey, StringId, SymbolBinding, SymbolForm, SymbolRole, SymbolSpace, SymbolTable, Tree,
    TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Find the nearest function or root scope for function-scoped bindings.
    fn function_scoped_binding_scope_id(
        &self,
        start_scope_id: LocalScopeId,
        symbols: &SymbolTable,
    ) -> LocalScopeId {
        let mut scope_id = start_scope_id;

        loop {
            let scope = symbols.get_scope_by_id(scope_id);

            // function-scoped bindings live on the owning function scope
            let is_function_scope = scope.owner.is_some_and(|owner_symbol_id| {
                let owner_symbol = symbols.get_symbol(owner_symbol_id);
                owner_symbol.form == SymbolForm::Function
                    || (scope.kind == ScopeKind::Namespace
                        && owner_symbol.role == SymbolRole::Item
                        && owner_symbol.form == SymbolForm::Value
                        && owner_symbol.binding == SymbolBinding::Runtime)
            });
            if is_function_scope {
                return scope_id;
            }

            // module roots also host function-scoped bindings
            let Some((parent_scope_id, _)) = scope.parent else {
                return scope_id;
            };
            scope_id = parent_scope_id;
        }
    }

    /// Return the default mutability for bindings without explicit mutability.
    pub(super) fn default_binding_mutability(&self, _module: &Module) -> Mutability {
        Mutability::Mutable
    }

    /// Resolve the mutability for a binding symbol.
    fn resolve_binding_mutability(
        &self,
        module: &Module,
        pattern_mutability: Option<Mutability>,
        binding_mutability: Option<Mutability>,
    ) -> Mutability {
        pattern_mutability
            .or(binding_mutability)
            .unwrap_or_else(|| self.default_binding_mutability(module))
    }

    /// Select the scope where a binding should be introduced for the binding scope.
    ///
    /// Function-scoped declarations (`var`) bind in the nearest owning scope
    /// (typically function scope, or module root when no function owner exists).
    fn scope_for_binding_scope(
        &self,
        scope: (LocalScopeId, LocalScopeMark),
        binding: SymbolBinding,
        binding_scope_form: Option<BindingScope>,
        symbols: &SymbolTable,
    ) -> (LocalScopeId, LocalScopeMark) {
        // keep ambient and declaration bindings in their lexical scopes
        if binding != SymbolBinding::Runtime {
            return scope;
        }

        if binding_scope_form != Some(BindingScope::Function) {
            return scope;
        }

        // route function-scoped bindings to their owning function or module scope
        let scope_id = self.function_scoped_binding_scope_id(scope.0, symbols);

        // keep visibility consistent across the full target scope
        (scope_id, LocalScopeMark::end())
    }

    /// Record binding mutability for a symbol when provided.
    pub(super) fn apply_binding_mutability(
        &self,
        symbols: &mut SymbolTable,
        symbol_id: LocalSymbolId,
        mutability: Mutability,
    ) {
        let symbol = symbols.get_symbol_mut(symbol_id);
        if symbol.binding_mutability.is_none() {
            symbol.binding_mutability = Some(mutability);
        }
    }

    /// Record binding scope for a symbol when not already set.
    pub(super) fn apply_binding_scope(
        &self,
        symbols: &mut SymbolTable,
        symbol_id: LocalSymbolId,
        binding_scope: BindingScope,
    ) {
        let symbol = symbols.get_symbol_mut(symbol_id);
        if symbol.binding_scope.is_none() {
            symbol.binding_scope = Some(binding_scope);
        }
    }

    /// Bind a pattern to a DIR pattern.
    pub(super) fn bind_pattern(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        binding: SymbolBinding,
        binding_mutability: Option<Mutability>,
        binding_scope_form: Option<BindingScope>,
        ast_pattern_id: ast::LocalNodeId<ast::Pattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Pattern> {
        let ast_pattern = ast.tree.get(ast_pattern_id);
        let pattern_id =
            tree.reserve_from_source(NodeType::Pattern, ast_pattern_id.id, scope, parent_id);
        let binding_target_scope =
            self.scope_for_binding_scope(scope, binding, binding_scope_form, symbols);
        let pattern = match ast_pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Must(ast_pattern_id) => Pattern::Must(self.bind_pattern(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                scope,
                export,
                binding,
                binding_mutability,
                binding_scope_form,
                *ast_pattern_id,
                Some(pattern_id),
                tree,
                symbols,
                types,
            )),
            ast::Pattern::Assign {
                pattern: ast_pattern_id,
                value,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_scope_form,
                    *ast_pattern_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                Pattern::Assign { pattern, value }
            }
            ast::Pattern::BorrowOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_scope_form,
                    *right_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::BorrowOf { mutability, right }
            }
            ast::Pattern::MoveOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_scope_form,
                    *right_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::MoveOf { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = *name;
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_scope_form,
                        pattern,
                        Some(pattern_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol, _) = self.bind_named_symbol_with_binding(
                    module,
                    ast,
                    SymbolSpace::Value,
                    StaticKey::Name(name),
                    binding,
                    binding_target_scope,
                    export,
                    symbols,
                );
                let symbol_mutability =
                    self.resolve_binding_mutability(module, mutability, binding_mutability);
                self.apply_binding_mutability(symbols, symbol, symbol_mutability);
                if let Some(binding_scope) = binding_scope_form {
                    self.apply_binding_scope(symbols, symbol, binding_scope);
                }
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                    symbol,
                }
            }
            ast::Pattern::Expression { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                Pattern::Expression { value }
            }
            ast::Pattern::TypeExpression { value } => {
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                Pattern::TypeExpression { value }
            }
            ast::Pattern::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::Tuple { fields }
            }
            ast::Pattern::TaggedTuple { ty, fields } => {
                let ty = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::TaggedTuple { ty, fields }
            }
            ast::Pattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::Sequence { fields }
            }
            ast::Pattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::Object { fields }
            }
            ast::Pattern::TaggedObject { ty, fields } => {
                let ty = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::TaggedObject { ty, fields }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| {
                        self.bind_pattern(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_scope_form,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::Union { patterns }
            }
        };

        // pattern
        if let Some(symbol_id) = pattern.symbol() {
            let pattern_id = tree.insert(pattern_id, pattern);
            symbols.get_symbol_mut(symbol_id).declare(pattern_id);
            pattern_id
        } else {
            tree.insert(pattern_id, pattern)
        }
    }

    /// Bind one named pattern field symbol.
    fn bind_named_pattern_field_symbol(
        &self,
        module: &Module,
        ast: &Ast,
        export: Option<ExportKind>,
        binding: SymbolBinding,
        scope: (LocalScopeId, LocalScopeMark),
        binding_mutability: Option<Mutability>,
        binding_scope_form: Option<BindingScope>,
        field_mutability: Option<Mutability>,
        field_name: StringId,
        symbols: &mut SymbolTable,
    ) -> LocalSymbolId {
        // symbol
        let (symbol, _) = self.bind_named_symbol_with_binding(
            module,
            ast,
            SymbolSpace::Value,
            StaticKey::Name(field_name),
            binding,
            scope,
            export,
            symbols,
        );

        // apply binding metadata from the enclosing binding context
        let symbol_mutability =
            self.resolve_binding_mutability(module, field_mutability, binding_mutability);
        self.apply_binding_mutability(symbols, symbol, symbol_mutability);
        if let Some(binding_scope) = binding_scope_form {
            self.apply_binding_scope(symbols, symbol, binding_scope);
        }

        symbol
    }

    /// Return the binding symbol introduced by one bound pattern.
    fn bound_pattern_symbol(
        &self,
        tree: &Tree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> Option<LocalSymbolId> {
        match tree.get(pattern_id) {
            Pattern::Binding { symbol, .. } => Some(*symbol),
            Pattern::Assign { pattern, .. } => self.bound_pattern_symbol(tree, *pattern),
            Pattern::Must(pattern)
            | Pattern::BorrowOf { right: pattern, .. }
            | Pattern::MoveOf { right: pattern, .. } => self.bound_pattern_symbol(tree, *pattern),
            Pattern::Wildcard
            | Pattern::Expression { .. }
            | Pattern::TypeExpression { .. }
            | Pattern::Tuple { .. }
            | Pattern::TaggedTuple { .. }
            | Pattern::Sequence { .. }
            | Pattern::Object { .. }
            | Pattern::TaggedObject { .. }
            | Pattern::Union { .. } => None,
        }
    }

    /// Bind a pattern field to a DIR pattern field.
    pub(super) fn bind_pattern_field(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        binding: SymbolBinding,
        binding_mutability: Option<Mutability>,
        binding_scope_form: Option<BindingScope>,
        ast_pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<PatternField> {
        let ast_pattern_field = ast.tree.get(ast_pattern_field_id);
        let pattern_field_id = tree.reserve_from_source(
            NodeType::PatternField,
            ast_pattern_field_id.id,
            scope,
            parent_id,
        );
        let binding_target_scope =
            self.scope_for_binding_scope(scope, binding, binding_scope_form, symbols);
        let pattern_field = match ast_pattern_field {
            ast::PatternField::Named {
                mutability,
                name,
                is_shorthand,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = name.string();
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_scope_form,
                        pattern,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });

                // symbol
                let symbol = if let Some(pattern) = pattern {
                    self.bound_pattern_symbol(tree, pattern)
                } else if *is_shorthand {
                    Some(self.bind_named_pattern_field_symbol(
                        module,
                        ast,
                        export,
                        binding,
                        binding_target_scope,
                        binding_mutability,
                        binding_scope_form,
                        mutability,
                        name,
                        symbols,
                    ))
                } else {
                    None
                };

                PatternField::Named {
                    mutability,
                    name,
                    symbol,
                    is_shorthand: *is_shorthand,
                    pattern,
                }
            }
            ast::PatternField::Computed {
                mutability,
                key,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let key = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *key,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_scope_form,
                    *pattern,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );

                PatternField::Computed {
                    mutability,
                    key,
                    pattern,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_scope_form,
                    *pattern_id,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );
                PatternField::Positional { pattern }
            }
            ast::PatternField::Spread {
                mutability,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let pattern = pattern.map(|pattern_id| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_scope_form,
                        pattern_id,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                PatternField::Spread {
                    mutability,
                    pattern,
                }
            }
            ast::PatternField::Elision => PatternField::Elision,
        };

        // pattern field
        if let Some(symbol_id) = pattern_field.symbol() {
            let pattern_field_id = tree.insert(pattern_field_id, pattern_field);
            symbols.get_symbol_mut(symbol_id).declare(pattern_field_id);
            pattern_field_id
        } else {
            tree.insert(pattern_field_id, pattern_field)
        }
    }
}
