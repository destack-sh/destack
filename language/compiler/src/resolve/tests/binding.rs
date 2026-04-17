use super::*;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Declaration, Declarator, Expression, FloatType, IntType, Pattern, PrimitiveType, ScalarLiteral,
    StaticKey, SymbolSpace, TypeLiteral,
};

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
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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

/// Ignore global augmentation symbols when building module exports.
#[test]
fn test_resolve_module_exports_ignore_global_augmentations() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "url.d.ts",
        r#"
export {};

declare global {
    interface URL {}
    var URL: { new(): URL };
}

export class URL {}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import { URL } from "./url";

const url = new URL();
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Ignore global augmentations inside declared module bindings when exporting names.
#[test]
fn test_resolve_module_binding_exports_ignore_global_augmentations() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "url" {
    class URL {}

    global {
        interface URL {}
        var URL: { new(): URL };
    }
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "./decl.d.ts";
import { URL } from "url";

const url = new URL();
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
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
    test.compile_check_clean();

    let outer_symbol_id = test.resolve_label_symbol("test.ds", "outer").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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
    test.compile_check_clean();

    let block_symbol_id = test.resolve_label_symbol("test.ds", "myblock").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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

    // ER201 = MissingTarget
    test.check_has_diagnostic("ER201");
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

    // ER201 = MissingTarget
    test.check_has_diagnostic("ER201");
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
    test.compile_check_clean();

    let middle_symbol_id = test.resolve_label_symbol("test.ds", "middle").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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
    test.compile_check_clean();

    let outer_symbol_id = test.resolve_label_symbol("test.ds", "outer").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
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
    test.compile_check_clean();

    let dir_b = test.dir_resolved(module_b_id);
    let tree_b = &dir_b.tree;

    // export let A = 1;
    let (a_symbol_id, _a_node_id) = test.resolve_to_node::<Pattern>("a.ds", "A").unwrap();

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
    const N: usize = 10;
    let test = TestProgram::memory_parallel();
    let initial_module_count = test.program.tracked_module_count();

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
    test.compile_check_clean();

    // verify all N modules were created (no duplicates from race conditions)
    let module_count = test.program.tracked_module_count();
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
    test.compile_check_clean();

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

/// Ensure type-only reexports produce type-space imports.
#[test]
fn test_resolve_type_only_reexport_import_space() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "types.ts",
        r#"
export type Options = { strict: boolean };
"#,
    );
    test.add_file(
        "module-b.ts",
        r#"
export { type Options } from "./types";
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ts",
        r#"
import type { Options } from "./module-b";
type Alias = Options;
"#,
    );
    test.resolve_module(consumer_id);
    test.compile_check_clean();

    let options_symbol_id = test.resolve_to_symbol("consumer.ts", "Options").unwrap();
    let options_symbol = test.symbol_by_id(options_symbol_id);
    assert_eq!(options_symbol.space, SymbolSpace::Type);
}

/// Ensure export-star forwarding keeps type-only imports in type space.
#[test]
fn test_resolve_export_star_type_only_import_space() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "types.ts",
        r#"
export type User = { name: string };
export const value = 1;
"#,
    );
    test.add_file(
        "module-b.ts",
        r#"
export * from "./types";
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ts",
        r#"
import { User } from "./module-b";
type Alias = User;
"#,
    );
    test.resolve_module(consumer_id);
    test.compile_check_clean();

    let user_symbol_id = test.resolve_to_symbol("consumer.ts", "User").unwrap();
    let user_symbol = test.symbol_by_id(user_symbol_id);
    assert_eq!(user_symbol.space, SymbolSpace::Type);
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
    test.compile_check_clean();

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
    let diagnostics = test.diagnostics();
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
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
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
            assert_string!(test.program, name.expect("expected member name"), "x");
        });
    });
    // let b = obj.y
    assert_node!(tree, b_let, Expression::Let { declarators, .. } => {
        let declarator = tree.get(declarators[0]);
        let Some(value) = declarator.value else { panic!("expected binding value") };
        assert_node!(tree, value, Expression::Member { name, .. } => {
            assert_string!(test.program, name.expect("expected member name"), "y");
        });
    });
}

/// Resolve second declarators against earlier declarators in one var statement.
#[test]
fn test_resolve_var_declarator_references_previous_declarator_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
var base = 1, mirror = base;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

    // resolve the base symbol for the target assertion
    let base_symbol = test.resolve_to_symbol("test.js", "base").unwrap();

    // locate the second declarator through the mirror pattern node
    let (_, mirror_node) = test
        .resolve_to_node::<Pattern>("test.js", "mirror")
        .unwrap();
    let mirror_declarator = tree
        .get_parent(mirror_node.id)
        .unwrap()
        .into_typed::<Declarator>();

    // assert that mirror resolves to the first declarator symbol
    assert_node!(tree, mirror_declarator, Declarator { value: Some(value), .. } => {
        let value_expression = tree.get(*value);
        let target_symbol = match value_expression {
            Expression::LocalReference { target_symbol, .. } => *target_symbol,
            Expression::ModuleReference { target_symbol, .. } => *target_symbol,
            other => panic!("expected local or module reference, got {other:?}"),
        };

        assert_eq!(target_symbol, base_symbol);
    });
}

/// Resolve runtime arguments inside JavaScript function bodies.
#[test]
fn test_resolve_arguments_inside_javascript_function_body() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
function pickFirst() {
    let first = arguments[0];
    return first;
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let symbols = &dir.symbols;
    let arguments_name = test.program.strings.intern("arguments");

    // scan resolved references and require one that targets the synthetic arguments binding
    let found_arguments_reference = tree.iter_nodes_of_type::<Expression>().any(|(_, expr)| {
        let target_symbol = match expr {
            Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
            Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };
        let Some(target_symbol) = target_symbol else {
            return false;
        };

        let symbol = symbols.get_symbol(target_symbol.into_local());
        symbol.key == Some(StaticKey::Name(arguments_name))
    });

    assert!(
        found_arguments_reference,
        "expected a resolved reference to function arguments"
    );
}

/// Resolve a declarator initializer closure reference to the declared binding.
#[test]
fn test_resolve_const_arrow_initializer_self_reference_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
const builder = (...arguments_) => arguments_.length === 1 ? builder : arguments_[0];
"#,
    );
    test.resolve_module(module_id);
    test.compile();
    test.check_no_diagnostic_code("ER101");
}

/// Resolve forward references to same-scope lexical bindings in JavaScript.
#[test]
fn test_resolve_forward_same_scope_lexical_reference_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
const make = () => value;
const value = 1;
"#,
    );
    test.resolve_module(module_id);
    test.compile();
    test.check_no_diagnostic_code("ER101");
}

/// Reject unresolved identifiers in runtime `typeof` probes.
#[test]
fn test_resolve_reject_unresolved_identifier_in_runtime_typeof() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.js", r#"typeof missing === "undefined";"#);
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Reject unresolved identifiers through parenthesized runtime `typeof` probes.
#[test]
fn test_resolve_reject_parenthesized_unresolved_identifier_in_runtime_typeof() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.js", r#"typeof (missing) === "undefined";"#);
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Reject unresolved member roots under runtime `typeof`.
#[test]
fn test_resolve_reject_unresolved_member_root_in_runtime_typeof() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.js", "typeof missing.member;");
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Reject commonjs wrapper global probes in JavaScript.
#[test]
fn test_resolve_reject_commonjs_wrapper_runtime_globals_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
var freeSelf = typeof self == "object" && self;

if (typeof define == "function" && define.amd) {
    define(function () {});
}
"#,
    );
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Reject unresolved namespace members in value paths.
#[test]
fn test_resolve_reject_unresolved_namespace_member_value_paths() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
namespace Box {
    export const ok = 1;
}

Box.missing;
"#,
    );
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Resolve `as const` assertions without unresolved `const` names.
#[test]
fn test_resolve_typescript_as_const_assertion_expression() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
const events = ["connect", "disconnect"] as const;

events;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Resolve function-scoped var bindings outside nested blocks in JavaScript.
#[test]
fn test_resolve_var_hoists_out_of_nested_block_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
function readAfterBlock() {
    if (true) {
        var separator = 1;
    }

    return separator;
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let symbols = &dir.symbols;
    let separator_name = test.program.strings.intern("separator");

    // locate a return expression that resolves to the hoisted separator symbol
    let found_hoisted_separator_reference =
        tree.iter_nodes_of_type::<Expression>().any(|(_, expr)| {
            let Expression::Return { value: Some(value) } = expr else {
                return false;
            };

            let target_symbol = match tree.get(*value) {
                Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
                Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            let Some(target_symbol) = target_symbol else {
                return false;
            };

            let symbol = symbols.get_symbol(target_symbol.into_local());
            symbol.key == Some(StaticKey::Name(separator_name))
        });

    assert!(
        found_hoisted_separator_reference,
        "expected return value to resolve to hoisted var separator"
    );
}

/// Resolve function declarations before textual declaration in JavaScript function bodies.
#[test]
fn test_resolve_function_declaration_hoists_in_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
function callBeforeDeclaration() {
    return verb();

    function verb() {
        return 1;
    }
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let symbols = &dir.symbols;
    let verb_name = test.program.strings.intern("verb");

    // locate a call expression whose callee resolves to the hoisted declaration symbol
    let found_hoisted_verb_reference = tree.iter_nodes_of_type::<Expression>().any(|(_, expr)| {
        let Expression::Call { left, .. } = expr else {
            return false;
        };

        let target_symbol = match tree.get(*left) {
            Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
            Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };
        let Some(target_symbol) = target_symbol else {
            return false;
        };

        let symbol = symbols.get_symbol(target_symbol.into_local());
        symbol.key == Some(StaticKey::Name(verb_name))
    });

    assert!(
        found_hoisted_verb_reference,
        "expected call target to resolve to hoisted function declaration"
    );
}

/// Keep named function expression bindings local to the function expression scope.
#[test]
fn test_resolve_named_function_expression_name_scope_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
var runInContext = (function runInContext(context) {
    return runInContext;
});
runInContext({});
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

    // collect the inner and outer runInContext references
    let mut inner_symbol = None;
    let mut outer_symbol = None;
    for (_, expression) in tree.iter_nodes_of_type::<Expression>() {
        // capture the named function expression self reference
        if let Expression::Return { value: Some(value) } = expression {
            let target_symbol = match tree.get(*value) {
                Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
                Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            if let Some(target_symbol) = target_symbol {
                inner_symbol = Some(target_symbol);
            }
        }

        // capture the outer call reference
        if let Expression::Call { left, .. } = expression {
            let target_symbol = match tree.get(*left) {
                Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
                Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            if let Some(target_symbol) = target_symbol {
                outer_symbol = Some(target_symbol);
            }
        }
    }

    // inner self name and outer variable must resolve to different symbols
    let inner_symbol = inner_symbol.expect("expected inner named function expression reference");
    let outer_symbol = outer_symbol.expect("expected outer runInContext call reference");
    assert_ne!(
        inner_symbol, outer_symbol,
        "expected inner named function reference to differ from outer var binding"
    );
}

/// Keep named function expression self names local in Destack.
#[test]
fn test_resolve_named_function_expression_name_scope_destack() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let create = (function inner() {
    return inner;
});

inner();
"#,
    );
    test.resolve_module(module_id);
    test.compile();
    test.check_has_diagnostic("ER101");
}

/// Resolve function-scoped var bindings captured inside nested functions.
#[test]
fn test_resolve_var_from_nested_block_inside_nested_function_javascript() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.js",
        r#"
function outer() {
    if (true) {
        var wrapper = 1;
    }

    return function inner() {
        return wrapper;
    };
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let symbols = &dir.symbols;
    let wrapper_name = test.program.strings.intern("wrapper");

    // require a nested return that resolves to the hoisted wrapper binding
    let has_wrapper_capture = tree
        .iter_nodes_of_type::<Expression>()
        .any(|(_, expression)| {
            let Expression::Return { value: Some(value) } = expression else {
                return false;
            };

            let target_symbol = match tree.get(*value) {
                Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
                Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            };
            let Some(target_symbol) = target_symbol else {
                return false;
            };

            let symbol = symbols.get_symbol(target_symbol.into_local());
            symbol.key == Some(StaticKey::Name(wrapper_name))
        });

    assert!(
        has_wrapper_capture,
        "expected nested function to resolve wrapper from outer function scope"
    );
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
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;

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
            assert_string!(test.program, name.expect("expected member name"), "value");
            assert_node!(tree, *left, Expression::Member { name: inner_name, .. } => {
                assert_string!(test.program, inner_name.expect("expected member name"), "inner");
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
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let symbols = &dir.symbols;

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
    test.compile_check_clean();

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
    test.compile_check_clean();

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
    test.compile_check_clean();

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
    test.compile_check_clean();

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

    // ER103 = CyclicSymbol (re-export chain forms a cycle)
    test.check_has_diagnostic("ER103");
}

/// Test that prelude items (like Add, Type) are available in user code.
#[test]
fn test_resolve_builtin_language_symbols() {
    let test = TestProgram::memory_sequential_with_prelude();

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

    test.resolve_language_environment();
    test.resolve_module(module_id);
    test.compile();
    test.check_clean();
    let profile = test.default_profile_id_for_root();
    let revision = test.program.current_revision();
    let environment = test
        .repository
        .language_environment(revision, profile)
        .unwrap_or_else(|| panic!("missing language environment for test profile"));

    for item in LanguageSymbol::all() {
        let _ = environment
            .item(item)
            .unwrap_or_else(|| panic!("missing language item {item:?}"));
    }
}

/// Type alias symbols point to their aliased type target symbols.
#[test]
fn test_resolve_type_alias_target_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {}
type Bar = Foo;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();
    let bar_symbol_id = test.resolve_to_symbol("test.ds", "Bar").unwrap();
    let bar_symbol = test.symbol_by_id(bar_symbol_id);

    assert!(
        bar_symbol.target_symbol.is_some(),
        "type alias symbol should have target symbol set"
    );
    assert_eq!(
        bar_symbol.target_symbol.unwrap(),
        foo_symbol_id,
        "type alias should point to Foo struct"
    );
}

/// Extension declarations point to their resolved target type symbols.
#[test]
fn test_resolve_extension_target_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {}
extension for Foo {
    bar() {}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let extensions: Vec<_> = tree
        .iter_node_ids_of_type::<Declaration>()
        .into_iter()
        .filter_map(|id| match tree.get(id) {
            Declaration::Extension(declaration) => Some(declaration.target_symbol),
            _ => None,
        })
        .collect();

    assert_eq!(extensions.len(), 1);
    let extension_target = extensions[0];
    assert!(
        extension_target.is_some(),
        "extension target symbol should be set"
    );
    assert_eq!(
        extension_target.unwrap(),
        foo_symbol_id,
        "extension should point to Foo struct"
    );
}

/// Extension declarations resolve namespace-qualified target types.
#[test]
fn test_resolve_extension_target_symbol_through_namespace_path() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
namespace Foo {
    export struct Bar {}
}

extension for Foo.Bar {
    baz() {}
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let bar_symbol_id = test.resolve_to_symbol("test.ds", "Foo.Bar").unwrap();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let extension_target = tree
        .iter_node_ids_of_type::<Declaration>()
        .into_iter()
        .find_map(|id| match tree.get(id) {
            Declaration::Extension(declaration) => declaration.target_symbol,
            _ => None,
        });

    assert_eq!(
        extension_target,
        Some(bar_symbol_id),
        "extension should point to Foo.Bar"
    );
}

/// Import aliases resolve namespace-qualified path targets.
#[test]
fn test_resolve_import_alias_target_symbol_through_namespace_path() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
namespace Foo {
    export class Bar {}
}

import Alias = Foo.Bar;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let alias_symbol_id = test.resolve_to_symbol("test.ds", "Alias").unwrap();
    let alias_symbol = test.symbol_by_id(alias_symbol_id);

    let bar_name = test.program.strings.intern("Bar");
    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let bar_symbol_id = tree
        .iter_node_ids_of_type::<Declaration>()
        .into_iter()
        .find_map(|id| match tree.get(id) {
            Declaration::Class(declaration)
                if declaration
                    .name
                    .is_some_and(|name| name.string() == bar_name) =>
            {
                Some(declaration.symbol.into_global(module_id))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected namespace class Bar declaration"));

    assert!(
        alias_symbol.target_symbol.is_some(),
        "import alias target symbol should be set"
    );
    assert_eq!(
        alias_symbol.target_symbol.unwrap(),
        bar_symbol_id,
        "import alias should point to namespace class Bar"
    );
}

/// Builtin type names resolve to type literal expressions in value position.
#[test]
fn test_resolve_builtin_types_in_value_position() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
int;
int32;
int64;
int68;
uint;
uint8;
uint16;
float;
float32;
float64;
boolean;
string;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let roots = &dir.roots;

    assert_node!(tree, roots[0], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))));
    });
    assert_node!(tree, roots[1], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))));
    });
    assert_node!(tree, roots[2], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int64))));
    });
    assert_node!(tree, roots[3], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Arbitrary { width: 68, is_signed: true }))));
    });
    assert_node!(tree, roots[4], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint32))));
    });
    assert_node!(tree, roots[5], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8))));
    });
    assert_node!(tree, roots[6], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint16))));
    });
    assert_node!(tree, roots[7], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64))));
    });
    assert_node!(tree, roots[8], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float32))));
    });
    assert_node!(tree, roots[9], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64))));
    });
    assert_node!(tree, roots[10], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Boolean)));
    });
    assert_node!(tree, roots[11], Expression::TypeLiteral { value } => {
            assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::String)));
    });
}

/// User bindings shadow builtin type names.
#[test]
fn test_variable_shadows_builtin_type() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let string: string = "hello";
string;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(module_id);
    let tree = &dir.tree;
    let roots = &dir.roots;

    let (string_symbol_id, _) = test
        .resolve_to_node::<Pattern>("test.ds", "string")
        .unwrap();
    assert_node!(tree, roots[1], Expression::ModuleReference { target_symbol, .. } => {
            assert_eq!(*target_symbol, string_symbol_id);
    });
}
