use destack_dir::{
    Argument, Expression, GlobalNodeIdAny, LocalNodeId, LocalScopeMark, LocalSymbolId, Module,
    Path, Scope, ScopeKind, Symbol, SymbolKey, SymbolTable,
};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve an absolute symbol key.
    pub(super) fn resolve_absolute_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        scope: (&Scope, LocalScopeMark),
        key: SymbolKey,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        let mut scope = scope;
        loop {
            // find symbol
            if let Some(symbol_id) = scope.0.find_up_to(key, scope.1) {
                return Ok(symbol_id);
            }
            // go to parent scope
            else if let Some((parent_scope_id, parent_mark)) = scope.0.parent {
                scope = (symbols.get_scope_by_id(parent_scope_id), parent_mark);
            }
            // no more scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingSymbol {
            node,
            scope: scope.0.id.into_global(module.id),
            via_module: None,
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
            if let Some(symbol_id) = scope.find(key) {
                symbol = symbols.get_symbol(symbol_id);
                scope = symbols.get_scope_by_symbol(symbol.id);
            } else {
                return Err(ResolveError::MissingSymbol {
                    node,
                    scope: scope.id.into_global(module.id),
                    via_module: None,
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
        scope: (&Scope, LocalScopeMark),
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
                    target_symbol: symbol.id.into_global(module.id),
                })
            } else {
                Ok(Expression::ModuleReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol.id.into_global(module.id),
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

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, Pattern, ScalarLiteral};
    use destack_source::Uri;

    use crate::{ImportTask, TestProgram, assert_node};

    /// Resolve symbols at top level in a single module.
    #[test]
    fn test_resolve_symbol_in_single_module() {
        let test = TestProgram::memory_sequential();
        let file = test.file(
            "test.ds",
            r#"
let x = 0;
let y = x;
let z = y;
"#,
        );
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let tree = module.tree.read();
        let (x_symbol_id, x_node) = test.resolve_to_node::<Pattern>("test.ds", "x").unwrap();
        let x_node = tree
            .get_parent(x_node.id)
            .unwrap()
            .into_typed::<Expression>();
        let (y_symbol_id, y_node) = test.resolve_to_node::<Pattern>("test.ds", "y").unwrap();
        let y_node = tree
            .get_parent(y_node.id)
            .unwrap()
            .into_typed::<Expression>();
        let (_z_symbol_id, z_node) = test.resolve_to_node::<Pattern>("test.ds", "z").unwrap();
        let z_node = tree
            .get_parent(z_node.id)
            .unwrap()
            .into_typed::<Expression>();

        // let x = 0;
        assert_node!(tree, x_node, Expression::Let { value: Some(value), ..} => {
            assert_node!(tree, *value, Expression::ScalarLiteral { value: ScalarLiteral::Integer(0) });
        });
        // let y = x;
        assert_node!(tree, y_node, Expression::Let { value: Some(value), ..} => {
            assert_node!(tree, *value, Expression::ModuleReference { target_symbol, .. } => {
                assert_eq!(*target_symbol, x_symbol_id);
            })
        });
        // let z = y;
        assert_node!(tree, z_node, Expression::Let { value: Some(value), ..} => {
            assert_node!(tree, *value, Expression::ModuleReference { target_symbol, .. } => {
                assert_eq!(*target_symbol, y_symbol_id);
            })
        });
    }

    /// Resolve symbols across two modules.
    #[test]
    fn test_resolve_symbol_across_two_modules() {
        let test = TestProgram::memory_parallel();
        let file_a = test.file(
            "a.ds",
            r#"
export let A = 1;
            "#,
        );
        let file_b = test.file(
            "b.ds",
            r#"
import { A } from "./a.ds";
export let B = A + 1;
            "#,
        );
        test.enqueue(ImportTask::ImportModuleFromFile { file: file_b.id });
        test.compile_dump_clean();

        let module_a = test.module_for_file(&file_a);
        let module_a = module_a.read();
        let tree_a = module_a.tree.read();
        let module_b = test.module_for_file(&file_b);
        let module_b = module_b.read();
        let tree_b = module_b.tree.read();

        // export let A = 1;
        let (a_symbol_id, a_node) = test.resolve_to_node::<Pattern>("a.ds", "A").unwrap();
        let _a_node = tree_a
            .get_parent(a_node.id)
            .unwrap()
            .into_typed::<Expression>();

        // export let B = A + 1;
        let (_b_symbol_id, b_node) = test.resolve_to_node::<Pattern>("b.ds", "B").unwrap();
        let b_node = tree_b
            .get_parent(b_node.id)
            .unwrap()
            .into_typed::<Expression>();
        assert_node!(tree_b, b_node, Expression::Let { value: Some(value), ..} => {
            assert_node!(tree_b, *value, Expression::Binary { left, right, .. } => {
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

        // create module 1: export let M1 = 1;
        test.file(
            "m1.ds",
            r#"
export let M1 = 1;
"#,
        );

        // create modules 2..N, each importing from all previous modules
        for i in 2..=N {
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
            test.file(&format!("m{i}.ds"), &content);
        }

        // enqueue the last module (will trigger imports of all others)
        let last_module_uri = format!("m{N}.ds");
        let last_file = test
            .program
            .files
            .get_by_uri(&Uri::from_string(&last_module_uri))
            .expect("last module file not found");
        test.enqueue(ImportTask::ImportModuleFromFile { file: last_file.id });
        test.compile_dump_clean();

        // verify all N modules were created (no duplicates from race conditions)
        let module_count = test.program.modules.len();
        assert_eq!(module_count, N + 1,); // (+1 for the implicit root module)
    }
}
