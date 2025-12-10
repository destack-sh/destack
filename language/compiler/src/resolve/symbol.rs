use destack_dir::{
    Argument, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId,
    LocalScopeMark, LocalSymbolId, NodeTree, NodeType, Path, Scope, ScopeKind, StaticKey,
    SymbolKind, SymbolTable,
};
use destack_workspace::Module;

use crate::{Compiler, ResolveError, ResolveResult, ResolveTask};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve an absolute symbol key.
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
            // find symbol
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
            // no more scopes
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

    /// Resolve an absolute path. Tries to resolve builtins if the path root segment couldn't be resolved.
    /// For non-namespace symbols with remaining path segments, creates Member expression chains.
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

        // resolve root symbol
        let symbol_id = self.resolve_absolute_symbol(
            module,
            node,
            scope,
            StaticKey::Name(first_segment),
            symbols,
        )?;
        let remaining_segments = &path.segments[1..];
        if remaining_segments.is_empty() {
            // simple case: just a single-segment path like `obj`
            return self.resolve_symbol_to_expression(
                module,
                node,
                symbol_id,
                path,
                static_arguments,
                symbols,
            );
        }

        // traverse remaining segments for namespace symbols
        let symbol = symbols.get_symbol(symbol_id);
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol(module, node, symbol_id, &remaining_path, symbols) {
                // fully resolved relative symbol
                Ok((resolved_id, None)) => {
                    return self.resolve_symbol_to_expression(
                        module,
                        node,
                        resolved_id,
                        path,
                        static_arguments,
                        symbols,
                    );
                }
                // partial resolution: namespace traversal found a non-namespace symbol
                // remaining segments should become Member chain
                Ok((resolved_id, Some(remaining))) => {
                    let resolved_path = path.slice(0..path.segments.len() - remaining.segments.len());
                    return self.build_member_chain_from_symbol(
                        module,
                        node,
                        expression_id,
                        resolved_id,
                        &resolved_path,
                        &remaining,
                        static_arguments,
                        symbols,
                        tree,
                    );
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![first_segment].into(),
        };
        let root_expr = self.resolve_symbol_to_expression(
            module, node, symbol_id, &root_path,
            None, // static_arguments go on the final member
            symbols,
        )?;

        // get scope info from the original expression
        let original_scope = tree.get_scope(expression_id);

        // create a new node for the root reference (don't reuse expression_id to avoid cycles)
        let root_node_id = tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            original_scope,
            Some(expression_id.into_any()),
        );
        tree.insert(root_node_id, root_expr);
        let mut current_id: LocalNodeId<Expression> = LocalNodeId::new(root_node_id.id);

        // create Member chain for remaining segments
        for (i, &segment) in remaining_segments.iter().enumerate() {
            let is_last = i == remaining_segments.len() - 1;

            // member (with static arguments if last segment)
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

            // return the final member expression (will replace original node)
            if is_last {
                return Ok(member_expression);
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

        unreachable!("remaining_segments is not empty")
    }

    /// Build a Member expression chain from a resolved symbol with remaining path segments.
    fn build_member_chain_from_symbol(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        expression_id: LocalNodeId<Expression>,
        symbol_id: LocalSymbolId,
        resolved_path: &Path,
        remaining_path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
    ) -> ResolveResult<Expression> {
        // create root expression from the resolved symbol
        let root_expr = self.resolve_symbol_to_expression(
            module,
            node,
            symbol_id,
            resolved_path,
            None, // static_arguments go on the final member
            symbols,
        )?;

        let original_scope = tree.get_scope(expression_id);

        // create a new node for the root reference
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

            if is_last {
                return Ok(member_expression);
            } else {
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

    /// Resolve a resolved symbol to an expression.
    pub(super) fn resolve_symbol_to_expression(
        &self,
        module: &Module,
        _node: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        symbols: &SymbolTable,
    ) -> ResolveResult<Expression> {
        let symbol = symbols.get_symbol(symbol_id);
        let scope = symbols.get_scope_by_id(symbol.scope.0);
        if symbol.module_id == module.id {
            if scope.kind == ScopeKind::Block {
                Ok(Expression::LocalReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol_id.into_global(module.id),
                })
            } else {
                Ok(Expression::ModuleReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol_id.into_global(module.id),
                })
            }
        } else {
            Ok(Expression::GlobalReference {
                path: path.clone(),
                static_arguments,
                target_symbol: symbol_id.into_global(module.id),
            })
        }
    }

    /// Follow a symbol's target chain to find the canonical (final) symbol.
    /// (This may yield if intermediate modules aren't resolved yet.)
    fn resolve_final_symbol_chain(
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

            // ensure the target module is resolved (may yield) - skip if it's the calling module
            if current.module_id != calling_module {
                self.require_task(ResolveTask::ResolveModule {
                    module: current.module_id,
                })?;
            }

            // get the symbol
            let module = self.program.modules.get(current.module_id);
            let module = module.read();
            let symbols = module.dir.symbols.read();
            let symbol = symbols.get_symbol(current.local_id);

            // if symbol already has final_symbol computed, use it (optimization)
            if let Some(final_symbol) = symbol.final_symbol {
                return Ok(final_symbol);
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

    /// Resolve and set the final_symbol for a symbol that has a target_symbol.
    /// (This should be called after setting target_symbol on a symbol.)
    pub(crate) fn resolve_final_symbol(
        &self,
        node: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
    ) -> ResolveResult<GlobalSymbolId> {
        // get the target_symbol
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            // no target, this symbol is its own final
            return Ok(symbol_id);
        };
        drop(symbols);
        drop(module);

        // follow the chain from target
        let final_symbol = self.resolve_final_symbol_chain(node, target_symbol)?;

        // set the final_symbol
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let mut symbols = module.dir.symbols.write();
        symbols.get_symbol_mut(symbol_id.local_id).final_symbol = Some(final_symbol);

        Ok(final_symbol)
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, Pattern, ScalarLiteral};

    use crate::{TestProgram, assert_node, assert_string};

    /// Resolve symbols at top level in a single module.
    #[test]
    fn test_resolve_symbol_in_single_module() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
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
        let tree = module.dir.tree.read();
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
        // add file_a to memory fs (will be imported transitively from file_b)
        test.add_file(
            "a.ds",
            r#"
export let A = 1;
            "#,
        );
        let module_b_id = test.register_module(
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
        let tree_a = module_a.dir.tree.read();
        let module_b = test.program.modules.get(module_b_id);
        let module_b = module_b.read();
        let tree_b = module_b.dir.tree.read();

        // export let A = 1;
        let (a_symbol_id, a_node_id) = test.resolve_to_node::<Pattern>("a.ds", "A").unwrap();
        let _a_node = tree_a
            .get_parent(a_node_id.id)
            .unwrap()
            .into_typed::<Expression>();

        // export let B = A + 1;
        let (_b_symbol_id, b_node_id) = test.resolve_to_node::<Pattern>("b.ds", "B").unwrap();
        let b_node = tree_b
            .get_parent(b_node_id.id)
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
            let module_id = test.register_module(&format!("m{N}.ds"), &content);
            test.resolve_module(module_id);
        }
        test.compile_dump_clean();

        // verify all N modules were created (no duplicates from race conditions)
        let module_count = test.program.modules.len();
        assert_eq!(module_count, N + 1,); // (+1 for the implicit root module)
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
        let module_b_id = test.register_module(
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
        // b.ds's A should also have final_symbol pointing to a.ds's A (canonical symbol)
        assert_eq!(b_import_symbol.final_symbol, Some(a_symbol_id));
    }

    /// Verify final_symbol chains through multi-level type aliases.
    #[test]
    fn test_resolve_symbol_multilevel_type_alias() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
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

        // Bar -> Foo (final_symbol, following the chain)
        assert_eq!(bar_symbol.final_symbol, Some(foo_symbol_id));

        // Baz -> Foo (target_symbol and final_symbol)
        let baz_symbol = test.symbol_by_id(baz_symbol_id);
        assert_eq!(baz_symbol.target_symbol, Some(foo_symbol_id));
        assert_eq!(baz_symbol.final_symbol, Some(foo_symbol_id));
    }

    /// Detect cyclic type alias reference (A -> B -> C -> A forms a cycle).
    #[test]
    fn test_detect_cyclic_type_alias() {
        let test = TestProgram::memory_parallel();
        let module_id = test.register_module(
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
            .any(|d| d.message.contains("cyclic symbol"));
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
        let module_b_id = test.register_module(
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
        let module_b_id = test.register_module(
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
        let module_c_id = test.register_module(
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

        // the import from c -> b should have final_symbol pointing to a's X
        // (this requires the re-export to be resolved as a transitive chain)
        let a_x_symbol_id = test.resolve_to_symbol("a.ds", "X").unwrap();
        assert_eq!(
            c_x_symbol.final_symbol,
            Some(a_x_symbol_id),
            "final_symbol should point to original symbol from a.ds"
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
        let consumer_id = test.register_module(
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
            value_a.final_symbol,
            Some(base_value_a),
            "VALUE_A final_symbol should point to base.ds"
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
            renamed_b.final_symbol,
            Some(base_value_b),
            "RENAMED_B final_symbol should point to base.ds VALUE_B"
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
            ns_x.final_symbol,
            Some(base_ns_x),
            "NS_X final_symbol should point to base.ds"
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
        let module_id = test.register_module(
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
        let tree = module.dir.tree.read();
        let (_a_symbol_id, a_node) = test.resolve_to_node::<Pattern>("test.ds", "a").unwrap();
        let a_let = tree.get_parent(a_node.id).unwrap().into_typed::<Expression>();
        let (_b_symbol_id, b_node) = test.resolve_to_node::<Pattern>("test.ds", "b").unwrap();
        let b_let = tree.get_parent(b_node.id).unwrap().into_typed::<Expression>();

        // let a = obj.x
        assert_node!(tree, a_let, Expression::Let { value: Some(value), .. } => {
            assert_node!(tree, *value, Expression::Member { name, .. } => {
                assert_string!(test.program, *name, "x");
            });
        });
        // let b = obj.y
        assert_node!(tree, b_let, Expression::Let { value: Some(value), .. } => {
            assert_node!(tree, *value, Expression::Member { name, .. } => {
                assert_string!(test.program, *name, "y");
            });
        });
    }

    /// Resolve chained member access like obj.inner.value.
    #[test]
    fn test_resolve_chained_member_access() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
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
        let tree = module.dir.tree.read();
        let (_a_symbol_id, a_node) = test.resolve_to_node::<Pattern>("test.ds", "a").unwrap();
        let a_let = tree.get_parent(a_node.id).unwrap().into_typed::<Expression>();

        // let a = obj.inner.value
        assert_node!(tree, a_let, Expression::Let { value: Some(value), .. } => {
            assert_node!(tree, *value, Expression::Member { left, name, .. } => {
                assert_string!(test.program, *name, "value");
                assert_node!(tree, *left, Expression::Member { name: inner_name, .. } => {
                    assert_string!(test.program, *inner_name, "inner");
                });
            });
        });
    }
}
