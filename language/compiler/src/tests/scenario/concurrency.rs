use destack_artifact::ArtifactKey;

use crate::tests::scenario::{CompilerScenario, ScenarioStress, assert_ast_eq};

/// Deduplicate one artifact task under concurrent enqueue pressure.
#[test]
fn test_parallel_enqueues_share_one_ast_task() {
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

    // enqueue the same artifact from many threads
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    run.parallel(stress.parallelism, |_, run| {
        run.compiler().enqueue(ArtifactKey::ast(module_id));
    });

    // build the queued work once
    run.compiler().compile();

    // verify the artifact was published
    assert!(run.compiler().artifacts.ast(module_id).is_some());

    // verify the queue only created one ast task
    let task_count = run
        .compiler()
        .task_handles()
        .into_iter()
        .filter(|handle| handle.artifact_key == ArtifactKey::ast(module_id))
        .count();
    assert_eq!(task_count, 1);
}

/// Deduplicate shared prerequisites under mixed concurrent enqueue pressure.
#[test]
fn test_parallel_enqueues_share_one_ast_prerequisite() {
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

    // enqueue a mix of direct and dependent requests
    let run = workspace.open();
    let module_id = run.module_id("main.ts");
    run.parallel(stress.parallelism, |worker, run| {
        if worker % 2 == 0 {
            run.compiler().enqueue(ArtifactKey::ast(module_id));
        } else {
            run.compiler().enqueue(ArtifactKey::dir_base(module_id));
        }
    });

    // build the queued work once
    run.compiler().compile();

    // verify both artifacts were published
    assert!(run.compiler().artifacts.ast(module_id).is_some());
    assert!(run.compiler().artifacts.dir_base(module_id).is_some());

    // verify the shared ast prerequisite only exists once
    let ast_task_count = run
        .compiler()
        .task_handles()
        .into_iter()
        .filter(|handle| handle.artifact_key == ArtifactKey::ast(module_id))
        .count();
    let dir_base_task_count = run
        .compiler()
        .task_handles()
        .into_iter()
        .filter(|handle| handle.artifact_key == ArtifactKey::dir_base(module_id))
        .count();
    assert_eq!(ast_task_count, 1);
    assert_eq!(dir_base_task_count, 1);
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
        run.compiler()
            .run_to_completion(|compiler| compiler.process_ast(module_id))
            .unwrap_or_else(|error| panic!("failed to build baseline ast: {error:?}"));
        run.compiler()
            .artifacts
            .ast(module_id)
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
            run.compiler()
                .run_to_completion(|compiler| compiler.process_ast(module_id))
                .unwrap_or_else(|error| panic!("failed to build repeated ast: {error:?}"));
            run.compiler()
                .artifacts
                .ast(module_id)
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
