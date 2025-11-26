use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    LocalNodeId, LocalScopeId, LocalScopeMark, Module, NodeTree, Pattern, PatternField, SymbolKey, SymbolSpace,
    SymbolTable, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a pattern to a DIR pattern.
    pub(super) fn bind_pattern(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        pattern_id: ast::LocalNodeId<ast::Pattern>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Pattern> {
        let pattern = module.get(pattern_id);
        let pattern = match pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Rest { name } => Pattern::Rest {
                name: name.map(|name| self.program.strings.intern_from(&module.ast_strings, name)),
            },
            ast::Pattern::Maybe(pattern_id) => Pattern::Maybe(self.bind_pattern(
                module,
                scope,
                *pattern_id,
                tree,
                symbols,
                types,
            )),
            ast::Pattern::ReferenceOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(module, scope, *right_id, tree, symbols, types);
                Pattern::ReferenceOf { mutability, right }
            }
            ast::Pattern::ValueOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(module, scope, *right_id, tree, symbols, types);
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
                    self.bind_pattern(module, scope, pattern, tree, symbols, types)
                });
                let symbol = symbols.bind_named_local(
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                );
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                    symbol,
                }
            }
            ast::Pattern::Expression { value } => {
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start
                    .map(|start| self.bind_pattern(module, scope, start, tree, symbols, types));
                let end =
                    end.map(|end| self.bind_pattern(module, scope, end, tree, symbols, types));
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { ty, fields } => {
                let ty = ty
                    .as_ref()
                    .map(|ty| self.bind_expression(module, scope, *ty, tree, symbols, types));
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(module, scope, *field, tree, symbols, types)
                    })
                    .collect();
                if let Some(ty) = ty {
                    Pattern::UnresolvedTuple { ty, fields }
                } else {
                    Pattern::Tuple {
                        target_symbol: None,
                        fields,
                    }
                }
            }
            ast::Pattern::Slice { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(module, scope, *field, tree, symbols, types)
                    })
                    .collect();
                Pattern::Slice { fields }
            }
            ast::Pattern::Struct { ty, fields } => {
                let ty =
                    ty.map(|ty| self.bind_expression(module, scope, ty, tree, symbols, types));
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.bind_pattern_field(module, scope, *field, tree, symbols, types)
                    })
                    .collect();
                if let Some(ty) = ty {
                    Pattern::UnresolvedStruct { ty, fields }
                } else {
                    Pattern::Struct {
                        target_symbol: None,
                        fields,
                    }
                }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| self.bind_pattern(module, scope, *field, tree, symbols, types))
                    .collect();
                Pattern::Union { patterns }
            }
        };
        tree.insert_from_source(pattern, pattern_id, scope)
    }

    /// Bind a pattern field to a DIR pattern field.
    pub(super) fn bind_pattern_field(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<PatternField> {
        let pattern_field = module.get(pattern_field_id);
        let pattern_field = match pattern_field {
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
                    self.bind_pattern(module, scope, pattern, tree, symbols, types)
                });
                let default = default.map(|default| {
                    self.bind_expression(module, scope, default, tree, symbols, types)
                });
                let symbol = symbols.bind_named_local(
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                );
                PatternField::UnresolvedNamed {
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
                    self.bind_expression(module, scope, default, tree, symbols, types)
                });
                let symbol = symbols.bind_named_local(
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                );
                PatternField::UnresolvedAlias {
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
                let pattern =
                    self.bind_pattern(module, scope, *pattern_id, tree, symbols, types);
                PatternField::UnresolvedPositional { pattern }
            }
        };
        tree.insert_from_source(pattern_field, pattern_field_id, scope)
    }
}
