use dyst_dir::{
    Argument, Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, Module, Path, Scope,
    ScopeKind, Symbol, SymbolKey, SymbolTable,
};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve an absolute symbol key.
    pub(super) fn resolve_absolute_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        scope: &Scope,
        key: SymbolKey,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        let mut scope = scope;
        loop {
            // find symbol
            if let Some(symbol_id) = scope.find_symbol(key) {
                return Ok(symbol_id);
            }
            // go to parent scope
            else if let Some(parent_scope_id) = scope.parent_id {
                scope = symbols.get_scope_by_id(parent_scope_id);
            }
            // no more scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingSymbol {
            node,
            scope: scope.id.into_global(module.id),
            key,
        })
    }

    /// Resolve a sub path in a scope.
    pub(super) fn resolve_relative_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
        path: &Path,
        symbols: &SymbolTable,
    ) -> ResolveResult<(LocalSymbolId, Option<Path>)> {
        let mut symbol = symbols.get_symbol(symbol_id);
        let mut scope = symbols.get_scope_by_symbol(symbol.id);
        let mut remaining_path = path.clone();

        // resolve path segments
        while let Some(segment) = remaining_path.segments.pop() {
            let key = SymbolKey::Name(segment);
            if let Some(symbol_id) = scope.find_symbol(key) {
                symbol = symbols.get_symbol(symbol_id);
                scope = symbols.get_scope_by_symbol(symbol.id);
            } else {
                return Err(ResolveError::MissingSymbol {
                    node,
                    scope: scope.id.into_global(module.id),
                    key,
                });
            }
        }

        // return symbol and remainder (if any)
        if remaining_path.segments.is_empty() {
            Ok((symbol.id, None))
        } else {
            Ok((symbol.id, Some(remaining_path)))
        }
    }

    /// Resolve an absolute path. Tries to resolve builtins if the path root segment couldn't be resolved.
    pub(super) fn resolve_absolute_path(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        scope: &Scope,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
    ) -> ResolveResult<Expression> {
        let first_segment = path.first_segment().expect("path is empty in {node:?}");

        // resolve root symbol
        let symbol_id = self.resolve_absolute_symbol(
            module,
            node,
            scope,
            SymbolKey::Name(first_segment),
            symbols,
        )?;
        let remaining_path = path.slice(1..);

        // resolve remaining path
        let (symbol_id, remaining_path) =
            self.resolve_relative_symbol(module, node, symbol_id, &remaining_path, symbols)?;
        if let Some(remaining_path) = remaining_path {
            Ok(Expression::UnresolvedRelativePath {
                path: path.clone(),
                target_symbol: symbol_id,
                remaining_path,
                static_arguments,
            })
        } else {
            let symbol = symbols.get_symbol(symbol_id);
            self.resolve_symbol_to_expression(module, node, symbol, path, static_arguments, symbols)
        }
    }

    /// Resolve a relative path.
    pub(super) fn resolve_relative_path(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        symbol: &Symbol,
        path: &Path,
        remaining_path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
    ) -> ResolveResult<Expression> {
        // resolve relative symbol
        let (symbol_id, remaining_path) =
            self.resolve_relative_symbol(module, node, symbol.id, remaining_path, symbols)?;
        if let Some(remaining_path) = remaining_path {
            Ok(Expression::UnresolvedRelativePath {
                path: path.clone(),
                target_symbol: symbol_id,
                remaining_path,
                static_arguments,
            })
        } else {
            let symbol = symbols.get_symbol(symbol_id);
            self.resolve_symbol_to_expression(module, node, symbol, path, static_arguments, symbols)
        }
    }

    /// Resolve a resolved symbol to an expression.
    pub(super) fn resolve_symbol_to_expression(
        &self,
        module: &Module,
        _node: GlobalNodeIdAny,
        symbol: &Symbol,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
    ) -> ResolveResult<Expression> {
        let scope = symbols.get_scope_by_symbol(symbol.id);
        if symbol.module_id == module.id {
            if scope.kind == ScopeKind::Block {
                Ok(Expression::LocalReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol.id,
                })
            } else {
                Ok(Expression::ModuleReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol.id,
                })
            }
        } else {
            Ok(Expression::GlobalReference {
                path: path.clone(),
                static_arguments,
                target_symbol: symbol.id.into_global(module.id),
            })
        }
    }
}
