use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingCategory, ExportMode, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark,
    LocalSymbolId, ModuleBinding, Mutability, NodeTree, NodeType, Pattern, PatternField,
    ProvenanceReason, ScopeKind, StaticKey, StringId, SymbolBinding, SymbolKind, SymbolSpace,
    SymbolSpaceOrder, SymbolTable, SymbolType, TypeTable,
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
            let is_function_scope = scope.owner_id.is_some_and(|owner_symbol_id| {
                let owner_symbol = symbols.get_symbol(owner_symbol_id);
                owner_symbol.ty == SymbolType::Function
                    || (scope.kind == ScopeKind::Namespace
                        && owner_symbol.kind == SymbolKind::Item
                        && owner_symbol.ty == SymbolType::Void
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

    /// Select the scope where a binding should be introduced for the binding category.
    ///
    /// Function-scoped declarations (`var`) bind in the nearest owning scope
    /// (typically function scope, or module root when no function owner exists).
    fn binding_scope_for_category(
        &self,
        scope: (LocalScopeId, LocalScopeMark),
        binding: SymbolBinding,
        binding_category: Option<BindingCategory>,
        symbols: &SymbolTable,
    ) -> (LocalScopeId, LocalScopeMark) {
        // keep ambient and declaration bindings in their lexical scopes
        if binding != SymbolBinding::Runtime {
            return scope;
        }

        if binding_category != Some(BindingCategory::FunctionScoped) {
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

    /// Record binding category for a symbol when not already set.
    pub(super) fn apply_binding_category(
        &self,
        symbols: &mut SymbolTable,
        symbol_id: LocalSymbolId,
        category: BindingCategory,
    ) {
        let symbol = symbols.get_symbol_mut(symbol_id);
        if symbol.binding_category == BindingCategory::Unclassified {
            symbol.binding_category = category;
        }
    }

    /// Bind a pattern to a DIR pattern.
    pub(super) fn bind_pattern(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportMode>,
        binding: SymbolBinding,
        binding_mutability: Option<Mutability>,
        binding_category: Option<BindingCategory>,
        ast_pattern_id: ast::LocalNodeId<ast::Pattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Pattern> {
        let ast_pattern = ast.tree.get(ast_pattern_id);
        let pattern_id =
            tree.reserve_from_source(NodeType::Pattern, ast_pattern_id.id, scope, parent_id);
        let binding_scope =
            self.binding_scope_for_category(scope, binding, binding_category, symbols);
        let pattern = match ast_pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Must(ast_pattern_id) => Pattern::Must(self.bind_pattern(
                module,
                ast,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                scope,
                export,
                binding,
                binding_mutability,
                binding_category,
                *ast_pattern_id,
                Some(pattern_id),
                tree,
                symbols,
                types,
            )),
            ast::Pattern::ReferenceOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_category,
                    *right_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::ReferenceOf { mutability, right }
            }
            ast::Pattern::ValueOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_category,
                    *right_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::ValueOf { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self.repository.strings.intern_from(&ast.strings, *name);
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_category,
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
                    binding_scope,
                    export,
                    symbols,
                );
                let symbol_mutability =
                    self.resolve_binding_mutability(module, mutability, binding_mutability);
                self.apply_binding_mutability(symbols, symbol, symbol_mutability);
                if let Some(binding_category) = binding_category {
                    self.apply_binding_category(symbols, symbol, binding_category);
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
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                Pattern::Expression { value }
            }
            ast::Pattern::TypeExpression { value } => {
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
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
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
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
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
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
            ast::Pattern::Array { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
                            *field,
                            Some(pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Pattern::Array { fields }
            }
            ast::Pattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
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
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
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
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            export,
                            binding,
                            binding_mutability,
                            binding_category,
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
            symbols
                .get_symbol_mut(symbol_id)
                .declare_primary(pattern_id);
            pattern_id
        } else {
            tree.insert(pattern_id, pattern)
        }
    }

    /// Bind a shorthand object field to one explicit binding pattern.
    fn bind_shorthand_named_pattern_for_field(
        &self,
        module: &Module,
        ast: &Ast,
        _namespace_scope: LocalScopeId,
        _global_augmentation_scope: LocalScopeId,
        _module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportMode>,
        binding: SymbolBinding,
        binding_scope: (LocalScopeId, LocalScopeMark),
        binding_mutability: Option<Mutability>,
        binding_category: Option<BindingCategory>,
        field_mutability: Option<Mutability>,
        field_name: StringId,
        parent_id: LocalNodeIdAny,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<Pattern> {
        // bind the shorthand field name as a regular local symbol
        let (symbol, _) = self.bind_named_symbol_with_binding(
            module,
            ast,
            SymbolSpace::Value,
            StaticKey::Name(field_name),
            binding,
            binding_scope,
            export,
            symbols,
        );

        // apply binding metadata from the enclosing binding context
        let symbol_mutability =
            self.resolve_binding_mutability(module, field_mutability, binding_mutability);
        self.apply_binding_mutability(symbols, symbol, symbol_mutability);
        if let Some(binding_category) = binding_category {
            self.apply_binding_category(symbols, symbol, binding_category);
        }

        // canonicalize shorthand as an explicit nested binding pattern
        let pattern_id = tree.reserve_from(
            NodeType::Pattern,
            parent_id,
            scope,
            Some(parent_id),
            Some(ProvenanceReason::Bound),
        );
        let pattern = Pattern::Binding {
            mutability: field_mutability,
            name: field_name,
            pattern: None,
            symbol,
        };
        let pattern_id = tree.insert(pattern_id, pattern);
        symbols.get_symbol_mut(symbol).declare_primary(pattern_id);

        pattern_id
    }

    /// Bind a pattern field to a DIR pattern field.
    pub(super) fn bind_pattern_field(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportMode>,
        binding: SymbolBinding,
        binding_mutability: Option<Mutability>,
        binding_category: Option<BindingCategory>,
        ast_pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
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
        let binding_scope =
            self.binding_scope_for_category(scope, binding, binding_category, symbols);
        let pattern_field = match ast_pattern_field {
            ast::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self
                    .repository
                    .strings
                    .intern_from(&ast.strings, name.string());
                let pattern = if let Some(pattern) = pattern {
                    Some(self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_category,
                        *pattern,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    ))
                }
                // canonicalize shorthand object fields to explicit nested bindings
                else {
                    Some(self.bind_shorthand_named_pattern_for_field(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        export,
                        binding,
                        binding_scope,
                        binding_mutability,
                        binding_category,
                        mutability,
                        name,
                        pattern_field_id,
                        tree,
                        symbols,
                    ))
                };
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                PatternField::Named {
                    mutability,
                    name,
                    pattern,
                    default,
                }
            }
            ast::PatternField::Computed {
                mutability,
                key,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let key = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *key,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_category,
                        pattern,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                PatternField::Computed {
                    mutability,
                    key,
                    pattern,
                    default,
                }
            }
            ast::PatternField::Alias {
                mutability,
                name,
                alias,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self
                    .repository
                    .strings
                    .intern_from(&ast.strings, name.string());
                let alias = self.repository.strings.intern_from(&ast.strings, *alias);
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let (symbol, _) = self.bind_named_symbol_with_binding(
                    module,
                    ast,
                    SymbolSpace::Value,
                    StaticKey::Name(alias),
                    binding,
                    binding_scope,
                    export,
                    symbols,
                );
                let symbol_mutability =
                    self.resolve_binding_mutability(module, mutability, binding_mutability);
                self.apply_binding_mutability(symbols, symbol, symbol_mutability);
                if let Some(binding_category) = binding_category {
                    self.apply_binding_category(symbols, symbol, binding_category);
                }
                PatternField::Alias {
                    mutability,
                    name,
                    alias,
                    default,
                    symbol,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
                default,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    export,
                    binding,
                    binding_mutability,
                    binding_category,
                    *pattern_id,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                PatternField::Positional { pattern, default }
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
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        export,
                        binding,
                        binding_mutability,
                        binding_category,
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
            symbols
                .get_symbol_mut(symbol_id)
                .declare_primary(pattern_field_id);
            pattern_field_id
        } else {
            tree.insert(pattern_field_id, pattern_field)
        }
    }
}
