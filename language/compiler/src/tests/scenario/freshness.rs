use crate::tests::scenario::CompilerScenario;

/// Invalidate only the edited module AST without touching unrelated module ASTs.
#[test]
fn test_source_edit_invalidates_only_local_ast() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_local_ast_freshness")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .file("sibling.ts", "export const sibling: number = 2;")
        .root("main.ts")
        .root("sibling.ts")
        .materialize();

    // publish both module ASTs first
    let run = workspace.open();
    let main = run.module("main.ts");
    let sibling = run.module("sibling.ts");
    main.require_ast();
    sibling.require_ast();

    // verify both artifacts are currently available
    assert!(main.ast_is_available());
    assert!(sibling.ast_is_available());

    // edit only the main module
    main.replace("export const value: string = 'updated';");

    // only the edited module should become unavailable
    assert!(!main.ast_is_available());
    assert!(sibling.ast_is_available());
}

/// Invalidate dependency and dependent resolved artifacts without touching unrelated modules.
#[test]
fn test_dependency_edit_invalidates_dependents_without_touching_unrelated_modules() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_dependency_freshness")
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
        .file("sibling.ts", "export const sibling: number = 2;")
        .root("main.ts")
        .root("sibling.ts")
        .materialize();

    // publish the resolved artifacts first
    let run = workspace.open();
    let dep = run.module("dep.ts");
    let main = run.module("main.ts");
    let sibling = run.module("sibling.ts");
    dep.require_dir_resolved();
    main.require_dir_resolved();
    sibling.require_dir_resolved();

    // verify the resolved artifacts are currently available
    assert!(dep.dir_resolved_is_available());
    assert!(main.dir_resolved_is_available());
    assert!(sibling.dir_resolved_is_available());

    // edit the imported dependency
    dep.replace("export const dep: string = 'updated';");

    // dependency and dependent should invalidate together
    assert!(!dep.dir_resolved_is_available());
    assert!(!main.dir_resolved_is_available());

    // unrelated modules should remain available
    assert!(sibling.dir_resolved_is_available());
}
