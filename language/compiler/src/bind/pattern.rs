use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    DependencyMode, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree, NodeType,
    Pattern, PatternField, StaticKey, SymbolSpace, SymbolTable, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a pattern to a DIR pattern.
    pub(super) fn bind_pattern(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<DependencyMode>,
        ast_pattern_id: ast::LocalNodeId<ast::Pattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Pattern> {
        let ast_pattern = module.ast.get(ast_pattern_id);
        let pattern_id =
            tree.reserve_from_source(NodeType::Pattern, ast_pattern_id, scope, parent_id);
        let pattern = match ast_pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Maybe(ast_pattern_id) => Pattern::Maybe(self.bind_pattern(
                module,
                scope,
                export,
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
                    scope,
                    export,
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
                    scope,
                    export,
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
                let name = self.program.strings.intern_from(&module.ast_strings, *name);
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        scope,
                        export,
                        pattern,
                        Some(pattern_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol, _) = self.bind_named_symbol(
                    module,
                    SymbolSpace::Value,
                    StaticKey::Name(name),
                    scope,
                    export,
                    symbols,
                );
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
                    scope,
                    *value,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start.map(|start| {
                    self.bind_pattern(
                        module,
                        scope,
                        export,
                        start,
                        Some(pattern_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let end = end.map(|end| {
                    self.bind_pattern(
                        module,
                        scope,
                        export,
                        end,
                        Some(pattern_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            scope,
                            export,
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
                let ty = self.bind_expression(
                    module,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            scope,
                            export,
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
                            scope,
                            export,
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
                            scope,
                            export,
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
                let ty = self.bind_expression(
                    module,
                    scope,
                    *ty,
                    Some(pattern_id),
                    tree,
                    symbols,
                    types,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(
                            module,
                            scope,
                            export,
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
                            scope,
                            export,
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

    /// Bind a pattern field to a DIR pattern field.
    pub(super) fn bind_pattern_field(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<DependencyMode>,
        ast_pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<PatternField> {
        let ast_pattern_field = module.ast.get(ast_pattern_field_id);
        let pattern_field_id = tree.reserve_from_source(
            NodeType::PatternField,
            ast_pattern_field_id,
            scope,
            parent_id,
        );
        let pattern_field = match ast_pattern_field {
            ast::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let pattern = pattern.map(|pattern| {
                    self.bind_pattern(
                        module,
                        scope,
                        export,
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
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol, _) = self.bind_named_symbol(
                    module,
                    SymbolSpace::Value,
                    StaticKey::Name(name),
                    scope,
                    export,
                    symbols,
                );
                PatternField::Named {
                    mutability,
                    name,
                    pattern,
                    default,
                    symbol,
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
                    .program
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let alias = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, *alias);
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        scope,
                        default,
                        Some(pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol, _) = self.bind_named_symbol(
                    module,
                    SymbolSpace::Value,
                    StaticKey::Name(name),
                    scope,
                    export,
                    symbols,
                );
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
            } => {
                let pattern = self.bind_pattern(
                    module,
                    scope,
                    export,
                    *pattern_id,
                    Some(pattern_field_id),
                    tree,
                    symbols,
                    types,
                );
                PatternField::Positional { pattern }
            }
            ast::PatternField::Spread { mutability, name } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = name.map(|name| {
                    self.program
                        .strings
                        .intern_from(&module.ast_strings, name.string())
                });
                let symbol = if let Some(name) = name {
                    let (symbol, _) = self.bind_named_symbol(
                        module,
                        SymbolSpace::Value,
                        StaticKey::Name(name),
                        scope,
                        export,
                        symbols,
                    );
                    symbol
                } else {
                    let (symbol, _) =
                        self.bind_anonymous_local(module, SymbolSpace::Value, scope, symbols);
                    symbol
                };
                PatternField::Spread {
                    mutability,
                    name,
                    symbol,
                }
            }
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
