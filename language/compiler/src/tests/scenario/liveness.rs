use crate::tests::scenario::{CompilerScenario, ScenarioStress};

/// Complete a repeated parallel resolved workload without stalling.
#[test]
fn test_parallel_resolved_workload_reaches_completion() {
    let stress = ScenarioStress::new()
        .repeats(2)
        .parallelism(4)
        .workers(4)
        .with_environment_overrides();
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_parallel_resolved_liveness")
        .workers(stress.workers)
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

    // stress fresh compiler sessions with inner compiler parallelism
    let results = workspace.parallel_fresh(stress.parallelism, |_, run| {
        run.repeat(stress.repeats, |_, run| {
            let module_id = run.module_id("main.ts");
            let profile_id = run.program().default_profile_id_for_module(module_id);

            // drive resolved work to completion
            run.compiler()
                .run_to_completion(|compiler| compiler.require_dir_resolved(module_id, profile_id))
                .unwrap_or_else(|error| panic!("failed to resolve stressed module: {error:?}"));

            // report whether the resolved artifact published
            run.program()
                .artifacts
                .dir_resolved(module_id, profile_id)
                .is_some()
        })
    });

    // every stressed run should complete and publish the resolved artifact
    for results in results {
        for completed in results {
            assert!(completed);
        }
    }
}
