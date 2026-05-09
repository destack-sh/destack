use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingTable, DeclaredModule, ExportKind, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, LocalSymbolId, Mutability, NodeType, Pattern, PatternField, StaticKey,
    StringId, SymbolBinding, SymbolSpace, Tree, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return the default mutability for bindings without explicit mutability.
    pub(super) fn default_binding_mutability(&self, _module: &Module) -> Mutability {
        Mutability::Mutable
    }

    /// Resolve the mutability for a binding symbol.
    fn resolve_binding_mutability(
        &self,
        module: &Module,
        binding_mutability: Option<Mutability>,
    ) -> Mutability {
        binding_mutability.unwrap_or_else(|| self.default_binding_mutability(module))
    }

    /// Record binding mutability for a symbol when provided.
    pub(super) fn apply_binding_mutability(
        &self,
        symbols: &mut BindingTable,
        symbol_id: LocalSymbolId,
        mutability: Mutability,
    ) {
        let symbol = symbols.get_symbol_mut(symbol_id);
        if symbol.binding_mutability.is_none() {
            symbol.binding_mutability = Some(mutability);
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
        ast_pattern_id: ast::LocalNodeId<ast::Pattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Pattern> {
        let ast_pattern = ast.tree.get(ast_pattern_id);
        let pattern_id =
            tree.reserve_from_source(NodeType::Pattern, ast_pattern_id.id, scope, parent_id);
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
                    *right_id,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::MoveOf { mutability, right }
            }
            ast::Pattern::Binding { name, pattern } => {
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
                    scope,
                    export,
                    symbols,
                );
                let symbol_mutability = self.resolve_binding_mutability(module, binding_mutability);
                self.apply_binding_mutability(symbols, symbol, symbol_mutability);
                Pattern::Binding {
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
            symbols.declare_symbol(symbol_id, pattern_id);
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
        field_name: StringId,
        symbols: &mut BindingTable,
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
        let symbol_mutability = self.resolve_binding_mutability(module, binding_mutability);
        self.apply_binding_mutability(symbols, symbol, symbol_mutability);

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
        ast_pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<PatternField> {
        let ast_pattern_field = ast.tree.get(ast_pattern_field_id);
        let pattern_field_id = tree.reserve_from_source(
            NodeType::PatternField,
            ast_pattern_field_id.id,
            scope,
            parent_id,
        );
        let pattern_field = match ast_pattern_field {
            ast::PatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
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
                        scope,
                        binding_mutability,
                        name,
                        symbols,
                    ))
                } else {
                    None
                };

                PatternField::Named {
                    name,
                    symbol,
                    is_shorthand: *is_shorthand,
                    pattern,
                }
            }
            ast::PatternField::Computed { key, pattern } => {
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
                    *pattern,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );

                PatternField::Computed { key, pattern }
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
                    *pattern_id,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );
                PatternField::Positional { pattern }
            }
            ast::PatternField::Spread { pattern } => {
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
                        pattern_id,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                PatternField::Spread { pattern }
            }
            ast::PatternField::Elision => PatternField::Elision,
        };

        // pattern field
        if let Some(symbol_id) = pattern_field.symbol() {
            let pattern_field_id = tree.insert(pattern_field_id, pattern_field);
            symbols.declare_symbol(symbol_id, pattern_field_id);
            pattern_field_id
        } else {
            tree.insert(pattern_field_id, pattern_field)
        }
    }
}
