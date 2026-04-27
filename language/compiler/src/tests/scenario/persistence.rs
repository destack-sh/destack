use destack_artifact::ArtifactKey;
use destack_builtin::LanguageSymbol;

use crate::tests::scenario::{CompilerScenario, append_file_text, assert_ast_eq};

/// Reuse a persisted AST across a fresh compiler session.
#[test]
fn test_reuses_ast_across_fresh_scenario_sessions() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_ast_reuse")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the ast once
    let first = workspace.open();
    let main = first.module("main.ts");
    main.require_ast();
    let expected = main.ast().as_ref().clone();

    // reopen a fresh compiler session over the same workspace
    let fresh = first.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    let loaded = fresh_main
        .load_ast_image()
        .unwrap_or_else(|| panic!("expected persisted ast image"));

    // compare the persisted ast surface
    assert_ast_eq(expected, loaded);
}

/// Reuse a persisted language environment across a fresh compiler session.
#[test]
fn test_reuses_language_environment_across_fresh_scenario_sessions() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_language_environment_reuse")
        .disk_cache()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the language environment once
    let first = workspace.open();
    let module_id = first.module_id("main.ts");
    let profile_id = first
        .compiler()
        .context(first.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    first.provide(ArtifactKey::language_environment(profile_id));
    let expected = first
        .program()
        .repository()
        .language_environment(first.current_revision(), profile_id)
        .unwrap_or_else(|| panic!("expected published language environment"))
        .as_ref()
        .clone();

    // reopen a fresh compiler session over the same workspace
    let fresh = first.reopen_fresh();
    let fresh_module_id = fresh.module_id("main.ts");
    let fresh_profile_id = fresh
        .compiler()
        .context(fresh.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(fresh_module_id);
    let loaded = fresh
        .compiler()
        .load_language_environment_image(fresh.current_revision(), fresh_profile_id)
        .unwrap_or_else(|error| panic!("failed to load language environment image: {error}"))
        .unwrap_or_else(|| panic!("expected persisted language environment image"));

    // compare the persisted semantic surface
    assert_eq!(loaded.items, expected.items);
    assert_eq!(loaded.symbols, expected.symbols);
}

/// Reuse a persisted module graph across a fresh compiler session.
#[test]
fn test_reuses_module_graph_across_fresh_scenario_sessions() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_module_graph_reuse")
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

    // build the graph once through resolved module work
    let first = workspace.open();
    let dep = first.module("dep.ts");
    let main = first.module("main.ts");
    dep.require_dir_resolved();
    main.require_dir_resolved();
    let module_id = main.module_id();
    let profile_id = first
        .compiler()
        .context(first.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    let expected = first
        .repository()
        .module_graph(first.current_revision(), profile_id)
        .unwrap_or_else(|| panic!("expected published module graph"))
        .as_ref()
        .clone();

    // reopen a fresh compiler session over the same workspace
    let fresh = first.reopen_fresh();
    let fresh_module_id = fresh.module_id("main.ts");
    let fresh_profile_id = fresh
        .compiler()
        .context(fresh.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(fresh_module_id);
    let loaded = fresh
        .compiler()
        .load_module_graph_image(fresh.current_revision(), fresh_profile_id)
        .unwrap_or_else(|error| panic!("failed to load module graph image: {error}"))
        .unwrap_or_else(|| panic!("expected persisted module graph image"));

    // compare the graph shape without live profile ids
    assert_eq!(loaded.modules, expected.modules);
    assert_eq!(loaded.dependents, expected.dependents);
}

/// Reject a persisted language environment image after builtin source changes.
#[test]
fn test_language_environment_image_tracks_builtin_source_content() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_language_environment_source_hash")
        .disk_cache()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the language environment once
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    let profile_id = run
        .compiler()
        .context(run.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run.provide(ArtifactKey::language_environment(profile_id));
    assert!(
        run.compiler()
            .load_language_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to load language environment image: {error}"))
            .is_some()
    );

    // perturb one contributing builtin module
    let builtins = run.program().builtins();
    let builtin_module_id = builtins.module_for_item(LanguageSymbol::Add);
    let builtin_file_id = run.program().module_descriptor(builtin_module_id).file_id;
    append_file_text(
        run.program().as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the stale image should now be rejected
    assert!(
        run.compiler()
            .load_language_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to reload language environment image: {error}"))
            .is_none()
    );
}

/// Reject a persisted AST image after source changes.
#[test]
fn test_ast_image_tracks_source_content() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_ast_source_hash")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the ast once
    let run = workspace.open();
    let main = run.module("main.ts");
    main.require_ast();
    assert!(main.load_ast_image().is_some());

    // perturb the source file
    main.replace("export const value: string = 'updated';");

    // the stale image should now be rejected
    assert!(main.load_ast_image().is_none());
}

/// Reject a persisted module graph image after the reachable module set changes.
#[test]
fn test_module_graph_image_tracks_reachable_module_changes() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_module_graph_source_hash")
        .disk_cache()
        .minimal_language_surface()
        .file("dep.ts", "export const dep: number = 1;")
        .file("extra.ts", "export const extra: number = 2;")
        .file(
            "main.ts",
            r#"
import { dep } from "./dep";

export const value: number = dep;
"#,
        )
        .root("main.ts")
        .materialize();

    // persist the module graph once
    let run = workspace.open();
    let dep = run.module("dep.ts");
    let main = run.module("main.ts");
    dep.require_dir_resolved();
    main.require_dir_resolved();
    let profile_id = run
        .compiler()
        .context(run.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(main.module_id());
    assert!(
        run.compiler()
            .load_module_graph_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to load module graph image: {error}"))
            .is_some()
    );

    // expand the reachable closure by importing one extra module
    main.replace(
        r#"
import { dep } from "./dep";
import { extra } from "./extra";

export const value: number = dep + extra;
"#,
    );

    // the stale graph image should now be rejected
    assert!(
        run.compiler()
            .load_module_graph_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to reload module graph image: {error}"))
            .is_none()
    );
}

/// Reject a persisted library environment image after library source changes.
#[test]
fn test_library_environment_image_tracks_library_source_content() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_library_environment_source_hash")
        .disk_cache()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the library environment once
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    let profile_id = run
        .compiler()
        .context(run.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run.provide(ArtifactKey::library_environment(profile_id));
    assert!(
        run.compiler()
            .load_library_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to load library environment image: {error}"))
            .is_some()
    );

    // perturb one selected library module
    let environment = run
        .program()
        .repository()
        .library_environment(run.current_revision(), profile_id)
        .unwrap_or_else(|| panic!("expected published library environment"));
    let library_module_id = *environment
        .modules
        .first()
        .unwrap_or_else(|| panic!("expected selected library modules"));
    let library_file_id = run.program().module_descriptor(library_module_id).file_id;
    append_file_text(
        run.program().as_ref(),
        library_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the stale image should now be rejected
    assert!(
        run.compiler()
            .load_library_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to reload library environment image: {error}"))
            .is_none()
    );
}

/// Reject a persisted intrinsic environment image after builtin source changes.
#[test]
fn test_intrinsic_environment_image_tracks_builtin_source_content() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_intrinsic_environment_source_hash")
        .disk_cache()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // persist the intrinsic environment once
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    let profile_id = run
        .compiler()
        .context(run.current_revision())
        .unwrap_or_else(|message| panic!("{message}"))
        .default_profile_id_for_module(module_id);
    run.provide(ArtifactKey::intrinsic_environment(profile_id));
    assert!(
        run.compiler()
            .load_intrinsic_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to load intrinsic environment image: {error}"))
            .is_some()
    );

    // perturb one intrinsic builtin module
    let builtins = run.program().builtins();
    let builtin_module_id = *builtins
        .intrinsic_module_ids()
        .first()
        .unwrap_or_else(|| panic!("expected intrinsic builtin modules"));
    let builtin_file_id = run.program().module_descriptor(builtin_module_id).file_id;
    append_file_text(
        run.program().as_ref(),
        builtin_file_id,
        "\n// changed for cache invalidation\n",
    );

    // the stale image should now be rejected
    assert!(
        run.compiler()
            .load_intrinsic_environment_image(run.current_revision(), profile_id)
            .unwrap_or_else(|error| panic!("failed to reload intrinsic environment image: {error}"))
            .is_none()
    );
}
