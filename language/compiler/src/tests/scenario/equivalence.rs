use crate::tests::scenario::{
    CompilerEditScript, CompilerScenario, assert_ast_eq, assert_dir_prepared_eq,
    assert_dir_resolved_eq,
};

/// Match fresh AST results after one same-process source edit.
#[test]
fn test_incremental_ast_matches_fresh_after_source_edit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_ast_equivalence")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();
    let script = CompilerEditScript::new()
        .replace_file("main.ts", "export const value: string = 'updated';");

    // rebuild incrementally after the source edit
    let incremental = workspace.open();
    let main = incremental.module("main.ts");
    main.require_ast();
    incremental.apply_script(&script);
    main.require_ast();
    let incremental_ast = main.ast().as_ref().clone();

    // rebuild the same final state from a fresh session
    let fresh = incremental.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_ast();
    let fresh_ast = fresh_main.ast().as_ref().clone();

    // compare the final semantic surface
    assert_ast_eq(fresh_ast, incremental_ast);
}

/// Match fresh prepared DIR results after one same-process source edit.
#[test]
fn test_incremental_dir_prepared_matches_fresh_after_source_edit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_dir_prepared_equivalence")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();
    let script = CompilerEditScript::new()
        .replace_file("main.ts", "export const value: string = 'updated';");

    // rebuild incrementally after the source edit
    let incremental = workspace.open();
    let main = incremental.module("main.ts");
    main.require_dir_prepared();
    incremental.apply_script(&script);
    main.require_dir_prepared();
    let incremental_dir = main.dir_prepared().as_ref().clone();

    // rebuild the same final state from a fresh session
    let fresh = incremental.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_dir_prepared();
    let fresh_dir = fresh_main.dir_prepared().as_ref().clone();

    // compare the final semantic surface
    assert_dir_prepared_eq(
        &fresh.program().strings,
        &incremental.program().strings,
        &fresh_dir,
        &incremental_dir,
    );
}

/// Match fresh resolved DIR results after one same-process source edit.
#[test]
fn test_incremental_dir_resolved_matches_fresh_after_source_edit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_dir_resolved_equivalence")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();
    let script = CompilerEditScript::new()
        .replace_file("main.ts", "export const value: string = 'updated';");

    // rebuild incrementally after the source edit
    let incremental = workspace.open();
    let main = incremental.module("main.ts");
    main.require_dir_resolved();
    incremental.apply_script(&script);
    main.require_dir_resolved();
    let incremental_dir = main.dir_resolved().as_ref().clone();

    // rebuild the same final state from a fresh session
    let fresh = incremental.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_dir_resolved();
    let fresh_dir = fresh_main.dir_resolved().as_ref().clone();

    // compare the final semantic surface
    assert_dir_resolved_eq(
        &fresh.program().strings,
        &incremental.program().strings,
        &fresh_dir,
        &incremental_dir,
    );
}

/// Match fresh resolved DIR results after one dependency edit.
#[test]
fn test_incremental_dir_resolved_matches_fresh_after_dependency_edit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_dependency_resolved_equivalence")
        .disk_cache()
        .minimal_language_surface()
        .file("dep.ts", "export const dep: number = 1;")
        .file(
            "main.ts",
            r#"
import { dep } from "./dep";

export const value: number = dep;
"#,
        )
        .root("main.ts")
        .materialize();

    // rebuild incrementally after the dependency edit
    let incremental = workspace.open();
    let dep = incremental.module("dep.ts");
    let main = incremental.module("main.ts");
    main.require_dir_resolved();
    dep.replace("export const dep: string = 'updated';");
    main.require_dir_resolved();
    let incremental_dir = main.dir_resolved().as_ref().clone();

    // rebuild the same final state from a fresh session
    let fresh = incremental.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_dir_resolved();
    let fresh_dir = fresh_main.dir_resolved().as_ref().clone();

    // compare the final semantic surface
    assert_dir_resolved_eq(
        &fresh.program().strings,
        &incremental.program().strings,
        &fresh_dir,
        &incremental_dir,
    );
}
