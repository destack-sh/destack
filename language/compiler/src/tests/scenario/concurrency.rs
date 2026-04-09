use destack_artifact::ArtifactKey;

use crate::tests::scenario::{CompilerScenario, ScenarioStress, assert_ast_eq};

/// Provide one AST concurrently without losing the published artifact.
#[test]
fn test_parallel_provides_publish_one_ast() {
    let stress = ScenarioStress::new()
        .parallelism(8)
        .workers(4)
        .with_environment_overrides();
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_ast_task_dedup")
        .workers(stress.workers)
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // provide the same artifact from many threads
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    run.parallel(stress.parallelism, |_, run| {
        run.provide(ArtifactKey::ast(module_id));
    });

    // verify the artifact was published
    assert!(
        run.program()
            .repository()
            .ast(run.current_revision(), module_id)
            .is_some()
    );
}

/// Provide shared prerequisites under mixed concurrent requests.
#[test]
fn test_parallel_provides_publish_shared_prerequisites() {
    let stress = ScenarioStress::new()
        .parallelism(8)
        .workers(4)
        .with_environment_overrides();
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_shared_prerequisite_dedup")
        .workers(stress.workers)
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // provide a mix of direct and dependent requests
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    run.parallel(stress.parallelism, |worker, run| {
        if worker % 2 == 0 {
            run.provide(ArtifactKey::ast(module_id));
        } else {
            run.provide(ArtifactKey::dir_base(module_id));
        }
    });

    // verify both artifacts were published
    assert!(
        run.program()
            .repository()
            .ast(run.current_revision(), module_id)
            .is_some()
    );
    assert!(
        run.program()
            .repository()
            .dir_base(run.current_revision(), module_id)
            .is_some()
    );
}

/// Keep repeated parallel fresh AST runs semantically stable.
#[test]
fn test_parallel_fresh_ast_runs_stay_equivalent() {
    let stress = ScenarioStress::new()
        .repeats(3)
        .parallelism(4)
        .workers(4)
        .with_environment_overrides();
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_parallel_fresh_ast")
        .workers(stress.workers)
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // build baseline ASTs from fresh parallel sessions
    let baselines = workspace.parallel_fresh(stress.parallelism, |_, run| {
        let module_id = run.module_id("main.ts");
        run.provide(ArtifactKey::ast(module_id));
        run.repository()
            .ast(run.current_revision(), module_id)
            .unwrap_or_else(|| panic!("expected baseline ast"))
            .as_ref()
            .clone()
    });
    let expected = baselines
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("expected at least one baseline ast"));

    // repeat the same parallel workload on one live scenario
    let run = workspace.open();
    let repeated = run.repeat(stress.repeats, |_, run| {
        let asts = run.parallel(stress.parallelism, |_, run| {
            let module_id = run.module_id("main.ts");
            run.provide(ArtifactKey::ast(module_id));
            run.repository()
                .ast(run.current_revision(), module_id)
                .unwrap_or_else(|| panic!("expected repeated ast"))
                .as_ref()
                .clone()
        });

        // every repeated ast should match the fresh baseline
        for ast in asts {
            assert_ast_eq(expected.clone(), ast);
        }
    });

    // verify the requested repeat count ran to completion
    assert_eq!(repeated.len(), stress.repeats);
}
