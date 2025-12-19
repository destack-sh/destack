use destack_dir::{
    Argument, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId,
    LocalScopeMark, LocalSymbolId, NodeTree, NodeType, Path, Scope, ScopeKind, StaticKey, StringId,
    SymbolKind, SymbolSpace, SymbolTable,
};
use destack_workspace::Module;

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build a Member expression chain from a root expression with remaining path segments.
    fn build_member_chain(
        &self,
        expression_id: LocalNodeId<Expression>,
        root_expr: Expression,
        remaining_path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        tree: &mut NodeTree,
    ) -> Expression {
        let original_scope = tree.get_scope(expression_id);

        // create a new node for the root expression
        let root_node_id = tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            original_scope,
            Some(expression_id.into_any()),
        );
        tree.insert(root_node_id, root_expr);
        let mut current_id: LocalNodeId<Expression> = LocalNodeId::new(root_node_id.id);

        // create Member chain for remaining segments
        let segments = &remaining_path.segments;
        for (i, &segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;
            let member_static_args = if is_last {
                static_arguments.clone()
            } else {
                None
            };
            let member_expression = Expression::Member {
                left: current_id,
                name: segment,
                static_arguments: member_static_args,
            };

            // return the final member expression
            if is_last {
                return member_expression;
            }
            // create intermediate member node
            else {
                let new_node_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    original_scope,
                    Some(expression_id.into_any()),
                );
                tree.insert(new_node_id, member_expression);
                current_id = LocalNodeId::new(new_node_id.id);
            }
        }

        unreachable!("remaining_path is not empty")
    }

    /// Resolve an absolute symbol key within local scopes only.
    /// Walks up the scope chain looking for the symbol.
    /// Does NOT check prelude - use resolve_absolute_path for that.
    pub(super) fn resolve_absolute_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        key: StaticKey,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        let mut scope = scope;
        loop {
            // find symbol in local scope
            if let Some(symbol_id) = scope.1.find_up_to(key, scope.2) {
                return Ok(symbol_id);
            }
            // go to parent scope
            else if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            }
            // no more local scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingSymbol {
            node,
            scope: scope.0.into_global(module.id),
            via_module: None,
            key,
        })
    }

    /// Resolve a symbol from the prelude by name.
    pub(super) fn resolve_prelude_symbol(
        &self,
        name: StringId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // check if prelude injection is enabled
        if !self.options.inject_prelude {
            return Ok(None);
        }

        // get the prelude module ID from builtins
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(None);
        };
        let prelude_module_id = builtins.prelude_module_id;

        // ensure the prelude module has been resolved (may yield)
        self.require_resolve_module(prelude_module_id)?;

        // get the prelude module
        let prelude_module = self.program.modules.get(prelude_module_id);
        let prelude_module = prelude_module.read();
        let prelude_dir = prelude_module.dir();
        let symbols = prelude_dir.symbols.read();

        // look up symbol by name in prelude's namespace scope
        let namespace_scope = symbols.get_scope_by_id(prelude_dir.namespace_scope);
        let key = StaticKey::Name(name);
        let Some(symbol_id) = namespace_scope.find(key) else {
            return Ok(None);
        };

        // verify it's exported
        let symbol = symbols.get_symbol(symbol_id);
        if symbol.export.is_none() {
            return Ok(None);
        }

        Ok(Some(symbol_id.into_global(prelude_module_id)))
    }

    /// Resolve a path starting from a prelude symbol.
    /// Similar to resolve_local_path but for symbols from the prelude module.
    fn resolve_prelude_path(
        &self,
        _module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        prelude_symbol: GlobalSymbolId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        tree: &mut NodeTree,
    ) -> ResolveResult<Expression> {
        let remaining_segments = &path.segments[1..];

        // single-segment path: just return the GlobalReference
        if remaining_segments.is_empty() {
            return Ok(Expression::GlobalReference {
                path: path.clone(),
                static_arguments,
                target_symbol: prelude_symbol,
            });
        }

        // multi-segment path: need to check if prelude symbol is a namespace
        let prelude_module_id = prelude_symbol.module_id;
        let prelude_module = self.program.modules.get(prelude_module_id);
        let prelude_module = prelude_module.read();
        let prelude_symbols = prelude_module.dir().symbols.read();

        let local_symbol_id = prelude_symbol.local_id;
        let symbol = prelude_symbols.get_symbol(local_symbol_id);

        if symbol.kind == SymbolKind::Namespace {
            // resolve remaining path within the prelude module's namespace
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol(
                &prelude_module,
                node,
                local_symbol_id,
                &remaining_path,
                &prelude_symbols,
            ) {
                Ok((resolved_id, None)) => {
                    // fully resolved within prelude
                    return Ok(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: resolved_id.into_global(prelude_module_id),
                    });
                }
                Ok((resolved_id, Some(remaining))) => {
                    // partially resolved, build Member chain for remaining
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = Expression::GlobalReference {
                        path: resolved_path,
                        static_arguments: None,
                        target_symbol: resolved_id.into_global(prelude_module_id),
                    };
                    return Ok(self.build_member_chain(
                        expression_id,
                        root_expr,
                        &remaining,
                        static_arguments,
                        tree,
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![path.first_segment().unwrap()].into(),
        };
        let root_expr = Expression::GlobalReference {
            path: root_path,
            static_arguments: None,
            target_symbol: prelude_symbol,
        };
        Ok(self.build_member_chain(
            expression_id,
            root_expr,
            &path.slice(1..),
            static_arguments,
            tree,
        ))
    }

    /// Resolve a relative path starting from a symbol.
    /// Returns the resolved symbol and any remaining path segments that couldn't be resolved
    /// (e.g., when hitting a non-namespace symbol with more segments to go).
    pub(super) fn resolve_relative_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
        path: &Path,
        symbols: &SymbolTable,
    ) -> ResolveResult<(LocalSymbolId, Option<Path>)> {
        let mut current_symbol_id = symbol_id;
        let segments = &path.segments;

        for (i, &segment) in segments.iter().enumerate() {
            let symbol = symbols.get_symbol(current_symbol_id);

            // stop traversing when we hit a non-namespace symbol
            if symbol.kind != SymbolKind::Namespace {
                let remaining = path.slice(i..);
                return Ok((current_symbol_id, Some(remaining)));
            }

            let key = StaticKey::Name(segment);
            let scope = symbols.get_scope_by_id(symbol.scope.0);
            if let Some(found_symbol_id) = scope.find(key) {
                current_symbol_id = found_symbol_id;
            } else {
                return Err(ResolveError::MissingSymbol {
                    node,
                    scope: symbol.scope.0.into_global(module.id),
                    via_module: None,
                    key,
                });
            }
        }

        Ok((current_symbol_id, None))
    }

    /// Resolve an absolute path.
    /// For non-namespace symbols with remaining path segments, creates Member expression chains.
    /// Falls back to prelude lookup if local lookup fails and inject_prelude is enabled.
    pub(super) fn resolve_absolute_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
    ) -> ResolveResult<Expression> {
        let first_segment = path.first_segment().expect("path is empty in {node:?}");
        let first_segment_str = self.program.strings.get(first_segment);

        // try to resolve root symbol locally
        let local_result = self.resolve_absolute_symbol(
            module,
            node,
            scope,
            StaticKey::Name(first_segment),
            symbols,
        );

        // if local lookup succeeded, use the local symbol
        if let Ok(local_id) = local_result {
            return self.resolve_local_path(
                module,
                expression_id,
                node,
                local_id,
                path,
                static_arguments,
                symbols,
                tree,
            );
        }

        // check for builtin types (boolean, int, string, etc.)
        if let Some(ty) = self.resolve_string_to_type(first_segment_str.as_str()) {
            let root_expr = Expression::TypeLiteral { value: ty };
            if path.segments.len() == 1 {
                return Ok(root_expr);
            }
            // multi-segment paths like `int.MAX` become member chains
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // resolve Self to enclosing type
        if first_segment_str.as_str() == "Self" {
            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expr = self
                .resolve_self_expression(module, scope, &root_path, None, symbols)
                .ok_or(ResolveError::MissingSelf { node })?;
            if path.segments.len() == 1 {
                return Ok(root_expr);
            }
            // multi-segment Self.X becomes member chain (e.g., for associated types)
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // try prelude if enabled
        if let Some(prelude_symbol) = self.resolve_prelude_symbol(first_segment)? {
            return self.resolve_prelude_path(
                module,
                expression_id,
                node,
                prelude_symbol,
                path,
                static_arguments,
                tree,
            );
        }

        // neither local nor prelude found, return the original error
        local_result.map(|_| unreachable!())
    }

    /// Resolve a path starting from a local symbol.
    fn resolve_local_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        local_id: LocalSymbolId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
    ) -> ResolveResult<Expression> {
        let first_segment = path.first_segment().expect("path is empty");
        let remaining_segments = &path.segments[1..];

        // single-segment path: just return the resolved expression
        if remaining_segments.is_empty() {
            return Ok(self.resolve_symbol_to_expression(
                module,
                local_id,
                path,
                static_arguments,
                symbols,
            ));
        }

        // multi-segment path: check if first segment is a namespace
        let symbol = symbols.get_symbol(local_id);
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol(module, node, local_id, &remaining_path, symbols) {
                Ok((resolved_id, None)) => {
                    return Ok(self.resolve_symbol_to_expression(
                        module,
                        resolved_id,
                        path,
                        static_arguments,
                        symbols,
                    ));
                }
                Ok((resolved_id, Some(remaining))) => {
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = self.resolve_symbol_to_expression(
                        module,
                        resolved_id,
                        &resolved_path,
                        None,
                        symbols,
                    );
                    return Ok(self.build_member_chain(
                        expression_id,
                        root_expr,
                        &remaining,
                        static_arguments,
                        tree,
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![first_segment].into(),
        };
        let root_expr =
            self.resolve_symbol_to_expression(module, local_id, &root_path, None, symbols);
        let remaining_path = path.slice(1..);
        Ok(self.build_member_chain(
            expression_id,
            root_expr,
            &remaining_path,
            static_arguments,
            tree,
        ))
    }

    /// Resolve a local symbol to an expression.
    pub(super) fn resolve_symbol_to_expression(
        &self,
        module: &Module,
        symbol_id: LocalSymbolId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
    ) -> Expression {
        let symbol = symbols.get_symbol(symbol_id);
        let scope = symbols.get_scope_by_id(symbol.scope.0);
        let global_id = symbol_id.into_global(module.id);
        if scope.kind == ScopeKind::Block {
            Expression::LocalReference {
                path: path.clone(),
                static_arguments,
                target_symbol: global_id,
            }
        } else {
            Expression::ModuleReference {
                path: path.clone(),
                static_arguments,
                target_symbol: global_id,
            }
        }
    }

    /// Resolve a label symbol by name, walking up scopes.
    /// Labels are in the Label symbol space and can only be found within the same module.
    pub(super) fn resolve_label_symbol(
        &self,
        _module: &Module,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        label: StringId,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        let key = StaticKey::Name(label);
        let mut scope = scope;
        loop {
            // search for label symbol in current scope
            for (candidate_key, symbol_id) in scope.1.named_symbols.iter() {
                if *candidate_key == key {
                    let symbol = symbols.get_symbol(*symbol_id);
                    if symbol.space == SymbolSpace::Label {
                        return Ok(*symbol_id);
                    }
                }
            }
            // go to parent scope
            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            }
            // no more scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingTarget {
            node,
            target: Some(label),
        })
    }

    /// Follow a symbol's target chain to find the canonical (final) symbol.
    fn resolve_canonical_symbol_chain(
        &self,
        node: GlobalNodeIdAny,
        start_symbol: GlobalSymbolId,
    ) -> ResolveResult<GlobalSymbolId> {
        // track the original calling module (from node)
        // (we don't need to require_task for this module since we're being called DURING its resolution)
        let calling_module = node.module_id;

        let mut current = start_symbol;
        let mut visited = Vec::new();
        loop {
            // detect cyclic symbol reference
            if visited.contains(&current) {
                return Err(ResolveError::CyclicSymbol {
                    node,
                    symbol: start_symbol,
                });
            }
            visited.push(current);

            // ensure the target module's direct symbols are resolved (may yield) - skip if it's the calling module
            if current.module_id != calling_module {
                self.require_resolve_module_direct(current.module_id)?;
            }

            // get the symbol
            let module = self.program.modules.get(current.module_id);
            let module = module.read();
            let symbols = module.dir().symbols.read();
            let symbol = symbols.get_symbol(current.local_id);

            // if symbol already has canonical_symbol computed, use it (optimization)
            if let Some(canonical_symbol) = symbol.canonical_symbol {
                return Ok(canonical_symbol);
            }

            // if it has a target, follow it
            if let Some(target) = symbol.target_symbol {
                current = target;
            } else {
                // no more targets, this is the final symbol
                return Ok(current);
            }
        }
    }

    /// Resolve and set the canonical_symbol for a symbol that has a target_symbol.
    pub(crate) fn resolve_canonical_symbol(
        &self,
        node: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
    ) -> ResolveResult<GlobalSymbolId> {
        // get the target_symbol
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir().symbols.read();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            // no target, this symbol is its own final
            return Ok(symbol_id);
        };
        drop(symbols);
        drop(module);

        // follow the chain from target
        let canonical_symbol = self.resolve_canonical_symbol_chain(node, target_symbol)?;

        // set the canonical_symbol
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let mut symbols = module.dir().symbols.write();
        symbols.get_symbol_mut(symbol_id.local_id).canonical_symbol = Some(canonical_symbol);

        Ok(canonical_symbol)
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, Pattern, ScalarLiteral};

    use crate::{TestProgram, assert_node, assert_string};

    /// Resolve labeled break to outer loop.
    #[test]
    fn test_resolve_labeled_break() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
outer: while (true) {
    while (true) {
        break outer;
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        // find the outer while loop's symbol
        let outer_symbol_id = test.resolve_label_symbol("test.ds", "outer").unwrap();

        // find the break expression and verify it resolved correctly
        let found_break = tree.iter_nodes_of_type::<Expression>().find(|(_, expr)| {
            matches!(
                expr,
                Expression::Break {
                    target_symbol: Some(_),
                    ..
                }
            )
        });
        assert!(found_break.is_some(), "expected resolved Break expression");
        let (_, break_expr) = found_break.unwrap();
        if let Expression::Break {
            target,
            target_symbol,
            ..
        } = break_expr
        {
            assert!(target.is_some(), "expected target label name");
            assert_eq!(
                *target_symbol,
                Some(outer_symbol_id),
                "break should target outer loop symbol"
            );
        }
    }

    /// Resolve labeled continue in nested loops.
    #[test]
    fn test_resolve_labeled_continue() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
outer: for (let i = 0; i < 10; i++) {
    for (let j = 0; j < 10; j++) {
        if (j == 5) {
            continue outer;
        }
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let outer_symbol_id = test.resolve_label_symbol("test.ds", "outer").unwrap();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        // find the continue expression
        let found_continue = tree.iter_nodes_of_type::<Expression>().find(|(_, expr)| {
            matches!(
                expr,
                Expression::Continue {
                    target_symbol: Some(_),
                    ..
                }
            )
        });
        assert!(
            found_continue.is_some(),
            "expected resolved Continue expression"
        );
        let (_, continue_expr) = found_continue.unwrap();
        if let Expression::Continue {
            target,
            target_symbol,
        } = continue_expr
        {
            assert!(target.is_some(), "expected target label name");
            assert_eq!(
                *target_symbol,
                Some(outer_symbol_id),
                "continue should target outer loop symbol"
            );
        }
    }

    /// Break from labeled block (not a loop).
    #[test]
    fn test_resolve_labeled_break_from_block() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
myblock: {
    if (true) {
        break myblock;
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let block_symbol_id = test.resolve_label_symbol("test.ds", "myblock").unwrap();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        let found_break = tree.iter_nodes_of_type::<Expression>().find(|(_, expr)| {
            matches!(
                expr,
                Expression::Break {
                    target_symbol: Some(_),
                    ..
                }
            )
        });
        assert!(found_break.is_some(), "expected resolved Break expression");
        let (_, break_expr) = found_break.unwrap();
        if let Expression::Break { target_symbol, .. } = break_expr {
            assert_eq!(
                *target_symbol,
                Some(block_symbol_id),
                "break should target labeled block"
            );
        }
    }

    /// Missing label error for break.
    #[test]
    fn test_resolve_missing_label_break() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
while (true) {
    break nonexistent;
}
"#,
        );
        test.resolve_module(module_id);
        test.compile();

        // ER010 = MissingTarget
        test.check_has_diagnostic("ER010");
    }

    /// Missing label error for continue.
    #[test]
    fn test_resolve_missing_label_continue() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
while (true) {
    continue nonexistent;
}
"#,
        );
        test.resolve_module(module_id);
        test.compile();

        // ER010 = MissingTarget
        test.check_has_diagnostic("ER010");
    }

    /// Multiple nested labeled loops with correct targeting.
    #[test]
    fn test_resolve_multiple_nested_labels() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
outer: while (true) {
    middle: while (true) {
        inner: while (true) {
            break middle;
        }
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let middle_symbol_id = test.resolve_label_symbol("test.ds", "middle").unwrap();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        let found_break = tree.iter_nodes_of_type::<Expression>().find(|(_, expr)| {
            matches!(
                expr,
                Expression::Break {
                    target_symbol: Some(_),
                    ..
                }
            )
        });
        assert!(found_break.is_some(), "expected resolved Break expression");
        let (_, break_expr) = found_break.unwrap();
        if let Expression::Break { target_symbol, .. } = break_expr {
            assert_eq!(
                *target_symbol,
                Some(middle_symbol_id),
                "break should target middle loop"
            );
        }
    }

    /// Labeled loop with break.
    #[test]
    fn test_resolve_labeled_loop_break() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
outer: loop {
    loop {
        break outer;
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let outer_symbol_id = test.resolve_label_symbol("test.ds", "outer").unwrap();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        let found_break = tree.iter_nodes_of_type::<Expression>().find(|(_, expr)| {
            matches!(
                expr,
                Expression::Break {
                    target_symbol: Some(_),
                    ..
                }
            )
        });
        assert!(found_break.is_some(), "expected resolved Break expression");
        let (_, break_expr) = found_break.unwrap();
        if let Expression::Break { target_symbol, .. } = break_expr {
            assert_eq!(
                *target_symbol,
                Some(outer_symbol_id),
                "break should target outer loop"
            );
        }
    }

    /// Resolve symbols at top level in a single module.
    #[test]
    fn test_resolve_symbol_in_single_module() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let x = 0;
let y = x;
let z = y;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();
        let (x_symbol_id, x_node) = test.resolve_to_node::<Pattern>("test.ds", "x").unwrap();

        // pattern -> declarator -> let
        let x_declarator = tree.get_parent(x_node.id).unwrap();
        let x_node = tree
            .get_parent(x_declarator.id)
            .unwrap()
            .into_typed::<Expression>();
        let (y_symbol_id, y_node) = test.resolve_to_node::<Pattern>("test.ds", "y").unwrap();
        let y_declarator = tree.get_parent(y_node.id).unwrap();
        let y_node = tree
            .get_parent(y_declarator.id)
            .unwrap()
            .into_typed::<Expression>();
        let (_z_symbol_id, z_node) = test.resolve_to_node::<Pattern>("test.ds", "z").unwrap();
        let z_declarator = tree.get_parent(z_node.id).unwrap();
        let z_node = tree
            .get_parent(z_declarator.id)
            .unwrap()
            .into_typed::<Expression>();

        // let x = 0;
        assert_node!(tree, x_node, Expression::Let { declarators, ..} => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::ScalarLiteral { value: ScalarLiteral::Integer(0) });
        });
        // let y = x;
        assert_node!(tree, y_node, Expression::Let { declarators, ..} => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::ModuleReference { target_symbol, .. } => {
                assert_eq!(*target_symbol, x_symbol_id);
            })
        });
        // let z = y;
        assert_node!(tree, z_node, Expression::Let { declarators, ..} => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::ModuleReference { target_symbol, .. } => {
                assert_eq!(*target_symbol, y_symbol_id);
            })
        });
    }

    /// Resolve symbols across two modules.
    #[test]
    fn test_resolve_symbol_across_two_modules() {
        let test = TestProgram::memory_parallel();
        // add file_a to memory fs (will be imported transitively from file_b)
        test.add_file(
            "a.ds",
            r#"
export let A = 1;
            "#,
        );
        let module_b_id = test.add_module(
            "b.ds",
            r#"
import { A } from "./a.ds";
export let B = A + 1;
            "#,
        );
        test.resolve_module(module_b_id);
        test.compile_dump_clean();

        let module_a = test.module("a.ds");
        let module_a = module_a.read();
        let tree_a = module_a.dir().tree.read();
        let module_b = test.program.modules.get(module_b_id);
        let module_b = module_b.read();
        let tree_b = module_b.dir().tree.read();

        // export let A = 1;
        let (a_symbol_id, a_node_id) = test.resolve_to_node::<Pattern>("a.ds", "A").unwrap();

        // pattern -> declarator -> let
        let a_declarator = tree_a.get_parent(a_node_id.id).unwrap();
        let _a_node = tree_a
            .get_parent(a_declarator.id)
            .unwrap()
            .into_typed::<Expression>();

        // export let B = A + 1;
        let (_b_symbol_id, b_node_id) = test.resolve_to_node::<Pattern>("b.ds", "B").unwrap();
        let b_declarator = tree_b.get_parent(b_node_id.id).unwrap();
        let b_node = tree_b
            .get_parent(b_declarator.id)
            .unwrap()
            .into_typed::<Expression>();
        assert_node!(tree_b, b_node, Expression::Let { declarators, ..} => {
            let declarator = tree_b.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree_b, value, Expression::Binary { left, right, .. } => {
                assert_node!(tree_b, *left, Expression::ModuleReference { target_symbol: target_symbol_id, .. } => {
                    let target_symbol = test.symbol_by_id(*target_symbol_id);
                    assert_eq!(target_symbol.target_symbol, Some(a_symbol_id));
                });
                assert_node!(tree_b, *right, Expression::ScalarLiteral { value: ScalarLiteral::Integer(1) });
            })
        });
    }

    /// Stress test: resolve symbols across N modules with overlapping imports.
    /// Module i imports from all modules 1..i, creating many concurrent imports to the same files.
    #[test]
    fn test_resolve_symbol_across_n_modules() {
        const N: usize = 20;
        let test = TestProgram::memory_parallel();
        let initial_module_count = test.program.modules.len();

        // add module 1 to fs: export let M1 = 1;
        test.add_file(
            "m1.ds",
            r#"
export let M1 = 1;
"#,
        );

        // add modules 2..N-1 to fs, each importing from all previous modules
        for i in 2..N {
            let mut imports = String::new();
            let mut sum_parts_str = Vec::new();

            // generate imports string
            for j in 1..i {
                imports.push_str(&format!("import {{ M{j} }} from \"./m{j}.ds\";\n"));
                sum_parts_str.push(format!("M{j}"));
            }

            // generate sum expression string
            let sum_expression_str = if sum_parts_str.is_empty() {
                "0".to_string()
            } else {
                sum_parts_str.join(" + ")
            };

            // generate module content
            let content = format!(
                r#"
{imports}
export let M{i} = {sum_expression_str} + 1;
"#
            );
            test.add_file(&format!("m{i}.ds"), &content);
        }

        // create and import module N (the last one, will trigger imports of all others)
        {
            let mut imports = String::new();
            let mut sum_parts_str = Vec::new();
            for j in 1..N {
                imports.push_str(&format!("import {{ M{j} }} from \"./m{j}.ds\";\n"));
                sum_parts_str.push(format!("M{j}"));
            }
            let sum_expression_str = sum_parts_str.join(" + ");
            let content = format!(
                r#"
{imports}
export let M{N} = {sum_expression_str} + 1;
"#
            );
            let module_id = test.add_module(&format!("m{N}.ds"), &content);
            test.resolve_module(module_id);
        }
        test.compile_dump_clean();

        // verify all N modules were created (no duplicates from race conditions)
        let module_count = test.program.modules.len();
        assert_eq!(module_count, initial_module_count + N);
    }

    /// Test import chain resolution to the canonical symbol.
    #[test]
    fn test_resolve_symbol_import_chain() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
export let A = 1;
            "#,
        );
        let module_b_id = test.add_module(
            "b.ds",
            r#"
import { A } from "./a.ds";
export let B = A + 1;
            "#,
        );
        test.resolve_module(module_b_id);
        test.compile_dump_clean();

        // get the original symbol from a.ds
        let (a_symbol_id, _) = test.resolve_to_node::<Pattern>("a.ds", "A").unwrap();

        // get the imported symbol from b.ds
        let b_import_symbol_id = test.resolve_to_symbol("b.ds", "A").unwrap();

        // check that b.ds's A symbol (import) has target_symbol pointing to a.ds's A
        let b_import_symbol = test.symbol_by_id(b_import_symbol_id);
        assert_eq!(b_import_symbol.target_symbol, Some(a_symbol_id));
        // b.ds's A should also have canonical_symbol pointing to a.ds's A (canonical symbol)
        assert_eq!(b_import_symbol.canonical_symbol, Some(a_symbol_id));
    }

    /// Verify canonical_symbol chains through multi-level type aliases.
    #[test]
    fn test_resolve_symbol_multilevel_type_alias() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {}
type Baz = Foo;
type Bar = Baz;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();
        let bar_symbol_id = test.resolve_to_symbol("test.ds", "Bar").unwrap();
        let baz_symbol_id = test.resolve_to_symbol("test.ds", "Baz").unwrap();

        // Bar -> Baz (target_symbol)
        let bar_symbol = test.symbol_by_id(bar_symbol_id);
        assert_eq!(bar_symbol.target_symbol, Some(baz_symbol_id));

        // Bar -> Foo (canonical_symbol, following the chain)
        assert_eq!(bar_symbol.canonical_symbol, Some(foo_symbol_id));

        // Baz -> Foo (target_symbol and canonical_symbol)
        let baz_symbol = test.symbol_by_id(baz_symbol_id);
        assert_eq!(baz_symbol.target_symbol, Some(foo_symbol_id));
        assert_eq!(baz_symbol.canonical_symbol, Some(foo_symbol_id));
    }

    /// Detect cyclic type alias reference (A -> B -> C -> A forms a cycle).
    #[test]
    fn test_detect_cyclic_type_alias() {
        let test = TestProgram::memory_parallel();
        let module_id = test.add_module(
            "test.ds",
            r#"
type A = B;
type B = C;
type C = A;
"#,
        );
        test.resolve_module(module_id);
        test.compile();

        // should produce a CyclicSymbol error
        let diagnostics = test.program.diagnostics.collect();
        let has_cyclic = diagnostics
            .iter()
            .into_iter()
            .any(|d| d.message.contains("cyclic reference"));
        assert!(
            has_cyclic,
            "expected CyclicSymbol error for cyclic type aliases"
        );
    }

    /// Resolve namespace import (`import * as foo from "bar"`).
    #[test]
    fn test_resolve_namespace_import() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
export let X = 1;
export let Y = 2;
"#,
        );
        let module_b_id = test.add_module(
            "b.ds",
            r#"
import * as A from "./a.ds";
let sum = A.X + A.Y;
"#,
        );
        test.resolve_module(module_b_id);
        test.compile_dump();

        // the namespace import A should target a.ds's namespace_symbol
        let b_a_symbol = test.resolve_to_symbol("b.ds", "A").unwrap();
        let b_a_symbol = test.symbol_by_id(b_a_symbol);
        assert!(
            b_a_symbol.target_symbol.is_some(),
            "namespace import should have target_symbol"
        );
    }

    /// Resolve default export and import (`export default foo`).
    #[test]
    fn test_resolve_default_export_import() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
let value = 42;
export default value;
"#,
        );
        let module_b_id = test.add_module(
            "b.ds",
            r#"
import DefaultValue from "./a.ds";
let x = DefaultValue;
"#,
        );
        test.resolve_module(module_b_id);
        test.compile();
        test.check_clean();
        test.dump();

        // the default import should target a.ds's default_symbol
        let b_default = test.resolve_to_symbol("b.ds", "DefaultValue").unwrap();
        let b_default_symbol = test.symbol_by_id(b_default);
        assert!(
            b_default_symbol.target_symbol.is_some(),
            "default import should have target_symbol"
        );
    }

    /// Resolve re-export (`export { X } from "foo"`).
    #[test]
    fn test_resolve_reexport() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
export let X = 1;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
export { X } from "./a.ds";
"#,
        );
        let module_c_id = test.add_module(
            "c.ds",
            r#"
import { X } from "./b.ds";
let y = X + 1;
"#,
        );
        test.resolve_module(module_c_id);
        test.compile();
        test.check_clean();
        test.dump();

        // c's X should resolve to b's re-export, which targets a's X
        let c_x_symbol_id = test.resolve_to_symbol("c.ds", "X").unwrap();
        let c_x_symbol = test.symbol_by_id(c_x_symbol_id);
        assert!(
            c_x_symbol.target_symbol.is_some(),
            "import from re-export should have target_symbol"
        );

        // the import from c -> b should have canonical_symbol pointing to a's X
        // (this requires the re-export to be resolved as a transitive chain)
        let a_x_symbol_id = test.resolve_to_symbol("a.ds", "X").unwrap();
        assert_eq!(
            c_x_symbol.canonical_symbol,
            Some(a_x_symbol_id),
            "canonical_symbol should point to original symbol from a.ds"
        );
    }

    /// Comprehensive test for multiple re-exports (most of JS/TS-style import/export surface).
    #[test]
    fn test_resolve_multiple_reexports() {
        let test = TestProgram::memory_parallel();

        // named exports: symbols X and Y
        test.add_file(
            "base.ds",
            r#"
// named exports
export let VALUE_A = 1;
export let VALUE_B = 2;

// will be exported as default
let defaultValue = 42;
export default defaultValue;

// for namespace re-export testing
export let NS_X = 10;
export let NS_Y = 20;
"#,
        );

        // default export and re-exports: symbol defaultValue
        test.add_file(
            "relay.ds",
            r#"
// re-export named
export { VALUE_A } from "./base.ds";

// re-export with rename
export { VALUE_B as RENAMED_B } from "./base.ds";

// re-export default as named
export { default as BaseDefault } from "./base.ds";
"#,
        );

        // namespace re-export: symbols NS_X and NS_Y
        test.add_file(
            "namespace_relay.ds",
            r#"
// true namespace export - re-exports all named exports from base.ds
export * from "./base.ds";
"#,
        );

        // namespace-as import: symbol Base
        test.add_file(
            "namespace_as.ds",
            r#"
// re-export namespace as named
export * as Base from "./base.ds";
"#,
        );

        // consumer: imports from all relay modules
        let consumer_id = test.add_module(
            "consumer.ds",
            r#"
// named import from relay
import { VALUE_A } from "./relay.ds";

// renamed import from relay
import { RENAMED_B } from "./relay.ds";

// default-as-named from relay
import { BaseDefault } from "./relay.ds";

// namespace import from base
import * as BaseNS from "./base.ds";

// default import from base
import DefaultFromBase from "./base.ds";

// from namespace re-export (export * from)
import { NS_X, NS_Y } from "./namespace_relay.ds";

// namespace-as import (export * as X from)
import { Base } from "./namespace_as.ds";

// use all imports to verify they resolve
let sum = VALUE_A + RENAMED_B + BaseDefault + BaseNS.VALUE_A + DefaultFromBase + NS_X + NS_Y;
"#,
        );

        test.resolve_module(consumer_id);
        test.compile();
        test.dump();
        test.check_clean();

        // consumer.ds VALUE_A -> relay.ds -> base.ds VALUE_A
        let value_a_symbol = test.resolve_to_symbol("consumer.ds", "VALUE_A").unwrap();
        let value_a = test.symbol_by_id(value_a_symbol);
        assert!(
            value_a.target_symbol.is_some(),
            "VALUE_A should have target_symbol"
        );
        let base_value_a = test.resolve_to_symbol("base.ds", "VALUE_A").unwrap();
        assert_eq!(
            value_a.canonical_symbol,
            Some(base_value_a),
            "VALUE_A canonical_symbol should point to base.ds"
        );

        // consumer.ds RENAMED_B -> relay.ds (VALUE_B as RENAMED_B) -> base.ds VALUE_B
        let renamed_b_symbol = test.resolve_to_symbol("consumer.ds", "RENAMED_B").unwrap();
        let renamed_b = test.symbol_by_id(renamed_b_symbol);
        assert!(
            renamed_b.target_symbol.is_some(),
            "RENAMED_B should have target_symbol"
        );
        let base_value_b = test.resolve_to_symbol("base.ds", "VALUE_B").unwrap();
        assert_eq!(
            renamed_b.canonical_symbol,
            Some(base_value_b),
            "RENAMED_B canonical_symbol should point to base.ds VALUE_B"
        );

        // consumer.ds DefaultFromBase -> base.ds default export
        let default_from_base = test
            .resolve_to_symbol("consumer.ds", "DefaultFromBase")
            .unwrap();
        let default_symbol = test.symbol_by_id(default_from_base);
        assert!(
            default_symbol.target_symbol.is_some(),
            "DefaultFromBase should have target_symbol"
        );

        // consumer.ds BaseNS -> base.ds namespace symbol
        let base_ns_symbol = test.resolve_to_symbol("consumer.ds", "BaseNS").unwrap();
        let base_ns = test.symbol_by_id(base_ns_symbol);
        assert!(
            base_ns.target_symbol.is_some(),
            "BaseNS namespace should have target_symbol"
        );

        // consumer.ds NS_X -> namespace_relay.ds (export * from) -> base.ds NS_X
        let ns_x_symbol = test.resolve_to_symbol("consumer.ds", "NS_X").unwrap();
        let ns_x = test.symbol_by_id(ns_x_symbol);
        assert!(
            ns_x.target_symbol.is_some(),
            "NS_X from namespace re-export should have target_symbol"
        );
        let base_ns_x = test.resolve_to_symbol("base.ds", "NS_X").unwrap();
        assert_eq!(
            ns_x.canonical_symbol,
            Some(base_ns_x),
            "NS_X canonical_symbol should point to base.ds"
        );

        // consumer.ds Base -> namespace_as.ds (export * as Base from) -> base.ds namespace
        let base_from_ns_as = test.resolve_to_symbol("consumer.ds", "Base").unwrap();
        let base_import = test.symbol_by_id(base_from_ns_as);
        assert!(
            base_import.target_symbol.is_some(),
            "Base from 'export * as Base' should have target_symbol"
        );
    }

    /// Resolve path expressions like obj.x to Member expressions.
    #[test]
    fn test_resolve_path_to_member() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();
        let (_a_symbol_id, a_node) = test.resolve_to_node::<Pattern>("test.ds", "a").unwrap();

        let a_declarator = tree.get_parent(a_node.id).unwrap();
        let a_let = tree
            .get_parent(a_declarator.id)
            .unwrap()
            .into_typed::<Expression>();
        let (_b_symbol_id, b_node) = test.resolve_to_node::<Pattern>("test.ds", "b").unwrap();
        let b_declarator = tree.get_parent(b_node.id).unwrap();
        let b_let = tree
            .get_parent(b_declarator.id)
            .unwrap()
            .into_typed::<Expression>();

        // let a = obj.x
        assert_node!(tree, a_let, Expression::Let { declarators, .. } => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::Member { name, .. } => {
                assert_string!(test.program, *name, "x");
            });
        });
        // let b = obj.y
        assert_node!(tree, b_let, Expression::Let { declarators, .. } => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::Member { name, .. } => {
                assert_string!(test.program, *name, "y");
            });
        });
    }

    /// Resolve chained member access like obj.inner.value.
    #[test]
    fn test_resolve_chained_member_access() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();

        let (_a_symbol_id, a_node) = test.resolve_to_node::<Pattern>("test.ds", "a").unwrap();
        let a_declarator = tree.get_parent(a_node.id).unwrap();
        let a_let = tree
            .get_parent(a_declarator.id)
            .unwrap()
            .into_typed::<Expression>();

        // let a = obj.inner.value
        assert_node!(tree, a_let, Expression::Let { declarators, .. } => {
            let declarator = tree.get(declarators[0]);
            let Some(value) = declarator.value else { panic!("expected binding value") };
            assert_node!(tree, value, Expression::Member { left, name, .. } => {
                assert_string!(test.program, *name, "value");
                assert_node!(tree, *left, Expression::Member { name: inner_name, .. } => {
                    assert_string!(test.program, *inner_name, "inner");
                });
            });
        });
    }

    /// Verify circular imports resolve correctly.
    #[test]
    fn test_resolve_circular_imports() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "a.ds",
            r#"
import { B } from "./b.ds";

export let A = 0;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
import { A } from "./a.ds";
export let B = 0;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { A } from "./a.ds";
import { B } from "./b.ds";

export let C = A + B;
"#,
        );

        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let symbols = dir.symbols.read();

        let a_symbol_id = test.resolve_to_symbol("a.ds", "A").unwrap();
        let b_symbol_id = test.resolve_to_symbol("b.ds", "B").unwrap();

        let main_a_symbol_id = test.resolve_to_symbol("main.ds", "A").unwrap();
        let main_b_symbol_id = test.resolve_to_symbol("main.ds", "B").unwrap();
        let main_a_symbol = symbols.get_symbol(main_a_symbol_id.into_local());
        let main_b_symbol = symbols.get_symbol(main_b_symbol_id.into_local());

        assert_eq!(main_a_symbol.canonical_symbol, Some(a_symbol_id));
        assert_eq!(main_b_symbol.canonical_symbol, Some(b_symbol_id));
    }

    /// Circular imports with re-exports (export { X } from).
    #[test]
    fn test_resolve_circular_imports_with_reexport() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
import { B } from "./b.ds";
export let A = 1;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
export { A } from "./a.ds";
export let B = 2;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { A, B } from "./b.ds";
export let C = A + B;
"#,
        );

        test.resolve_module(module_id);
        test.compile_dump_clean();

        let a_symbol_id = test.resolve_to_symbol("a.ds", "A").unwrap();
        let b_symbol_id = test.resolve_to_symbol("b.ds", "B").unwrap();

        let main_a_symbol_id = test.resolve_to_symbol("main.ds", "A").unwrap();
        let main_b_symbol_id = test.resolve_to_symbol("main.ds", "B").unwrap();
        let main_a_symbol = test.symbol_by_id(main_a_symbol_id);
        let main_b_symbol = test.symbol_by_id(main_b_symbol_id);

        assert_eq!(main_a_symbol.canonical_symbol, Some(a_symbol_id));
        assert_eq!(main_b_symbol.canonical_symbol, Some(b_symbol_id));
    }

    /// Circular imports with namespace re-exports (export * from).
    #[test]
    fn test_resolve_circular_imports_with_namespace_reexport() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
import { B } from "./b.ds";
export let A = 1;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
export * from "./a.ds";
export let B = 2;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { A, B } from "./b.ds";
export let C = A + B;
"#,
        );

        test.resolve_module(module_id);
        test.compile_dump_clean();

        let a_symbol_id = test.resolve_to_symbol("a.ds", "A").unwrap();
        let b_symbol_id = test.resolve_to_symbol("b.ds", "B").unwrap();

        let main_a_symbol_id = test.resolve_to_symbol("main.ds", "A").unwrap();
        let main_b_symbol_id = test.resolve_to_symbol("main.ds", "B").unwrap();
        let main_a_symbol = test.symbol_by_id(main_a_symbol_id);
        let main_b_symbol = test.symbol_by_id(main_b_symbol_id);

        assert_eq!(main_a_symbol.canonical_symbol, Some(a_symbol_id));
        assert_eq!(main_b_symbol.canonical_symbol, Some(b_symbol_id));
    }

    /// Three-way circular imports (A -> B -> C -> A).
    #[test]
    fn test_resolve_three_way_circular_imports() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "a.ds",
            r#"
import { C } from "./c.ds";
export let A = 1;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
import { A } from "./a.ds";
export let B = 2;
"#,
        );
        test.add_file(
            "c.ds",
            r#"
import { B } from "./b.ds";
export let C = 3;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { A } from "./a.ds";
import { B } from "./b.ds";
import { C } from "./c.ds";
export let D = A + B + C;
"#,
        );

        test.resolve_module(module_id);
        test.compile_dump_clean();

        let a_symbol_id = test.resolve_to_symbol("a.ds", "A").unwrap();
        let b_symbol_id = test.resolve_to_symbol("b.ds", "B").unwrap();
        let c_symbol_id = test.resolve_to_symbol("c.ds", "C").unwrap();

        let main_a = test.symbol_by_id(test.resolve_to_symbol("main.ds", "A").unwrap());
        let main_b = test.symbol_by_id(test.resolve_to_symbol("main.ds", "B").unwrap());
        let main_c = test.symbol_by_id(test.resolve_to_symbol("main.ds", "C").unwrap());

        assert_eq!(main_a.canonical_symbol, Some(a_symbol_id));
        assert_eq!(main_b.canonical_symbol, Some(b_symbol_id));
        assert_eq!(main_c.canonical_symbol, Some(c_symbol_id));
    }

    /// Mutual namespace re-exports resolve correctly (cycle handled by visited tracking).
    #[test]
    fn test_resolve_mutual_namespace_reexports() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
export * from "./b.ds";
export let X = 1;
"#,
        );
        test.add_file(
            "b.ds",
            r#"
export * from "./a.ds";
export let Y = 2;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { X, Y } from "./a.ds";
export let Z = X + Y;
"#,
        );

        test.resolve_module(module_id);
        test.compile_dump_clean();

        let a_x_symbol_id = test.resolve_to_symbol("a.ds", "X").unwrap();
        let b_y_symbol_id = test.resolve_to_symbol("b.ds", "Y").unwrap();

        let main_x = test.symbol_by_id(test.resolve_to_symbol("main.ds", "X").unwrap());
        let main_y = test.symbol_by_id(test.resolve_to_symbol("main.ds", "Y").unwrap());

        assert_eq!(main_x.canonical_symbol, Some(a_x_symbol_id));
        assert_eq!(main_y.canonical_symbol, Some(b_y_symbol_id));
    }

    /// Cyclic re-export chain should produce an error (a re-exports from b, b re-exports from a).
    #[test]
    fn test_detect_cyclic_reexport() {
        let test = TestProgram::memory_parallel();
        test.add_file(
            "a.ds",
            r#"
export { X } from "./b.ds";
"#,
        );
        test.add_file(
            "b.ds",
            r#"
export { X } from "./a.ds";
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { X } from "./a.ds";
"#,
        );

        test.resolve_module(module_id);
        test.compile();

        // ER008 = CyclicSymbol (re-export chain forms a cycle)
        test.check_has_diagnostic("ER008");
    }

    /// Test that prelude items (like Add, Type) are available in user code.
    #[test]
    fn test_resolve_prelude_items() {
        let test = TestProgram::memory_sequential_with_builtins();

        // code that uses prelude items without importing them
        let module_id = test.add_module(
            "test.ds",
            r#"
type MyAdd = Add;

@deprecated
function oldFunction() {}

@inline
function inlineFunction() {}

function printType(t: Type) {
    // ...
}
"#,
        );

        test.resolve_module(module_id);
        test.compile();
        test.check_clean();
    }
}
