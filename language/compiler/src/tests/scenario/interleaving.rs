use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use destack_artifact::ArtifactKey;

use crate::tests::scenario::{
    CompilerScenario, CompilerScenarioEvent, assert_ast_eq, assert_dir_resolved_eq,
};

/// One paused pre-commit gate for deterministic interleaving tests.
#[derive(Debug)]
struct CommitGate {
    /// The artifact key to block.
    target: Mutex<Option<ArtifactKey>>,
    /// The shared gate state.
    state: Mutex<CommitGateState>,
    /// The state transition condvar.
    condvar: Condvar,
}

/// One mutable pre-commit gate state.
#[derive(Debug, Default)]
struct CommitGateState {
    /// Whether the blocked task has reached the gate.
    is_reached: bool,
    /// Whether the blocked task may continue.
    is_released: bool,
}

impl CommitGate {
    /// Create one gate for a target artifact key.
    fn new() -> Arc<Self> {
        Arc::new(Self {
            target: Mutex::new(None),
            state: Mutex::new(CommitGateState::default()),
            condvar: Condvar::new(),
        })
    }

    /// Set the target artifact key to block.
    fn set_target(&self, target: ArtifactKey) {
        let mut current = self
            .target
            .lock()
            .unwrap_or_else(|error| panic!("failed to lock commit gate target: {error}"));
        *current = Some(target);
    }

    /// Build one test event handler for this gate.
    fn handler(self: &Arc<Self>) -> Arc<dyn Fn(CompilerScenarioEvent) + Send + Sync> {
        let gate = self.clone();

        Arc::new(move |event| {
            // ignore unrelated events
            let CompilerScenarioEvent::BeforeTaskCommit { artifact_key, .. } = event;
            let target = gate
                .target
                .lock()
                .unwrap_or_else(|error| panic!("failed to lock commit gate target: {error}"))
                .to_owned();
            if target.as_ref() != Some(&artifact_key) {
                return;
            }

            // mark the gate as reached once
            let mut state = gate
                .state
                .lock()
                .unwrap_or_else(|error| panic!("failed to lock commit gate: {error}"));

            // only block the first matching completion
            if state.is_reached {
                return;
            }
            state.is_reached = true;
            gate.condvar.notify_all();

            // wait until the test releases the blocked task
            while !state.is_released {
                state = gate
                    .condvar
                    .wait(state)
                    .unwrap_or_else(|error| panic!("failed to wait on commit gate: {error}"));
            }
        })
    }

    /// Wait until the blocked task reaches the gate.
    fn wait_until_reached(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|error| panic!("failed to lock commit gate: {error}"));

        while !state.is_reached {
            state = self
                .condvar
                .wait(state)
                .unwrap_or_else(|error| panic!("failed to wait on commit gate: {error}"));
        }
    }

    /// Release the blocked task.
    fn release(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|error| panic!("failed to lock commit gate: {error}"));
        state.is_released = true;
        self.condvar.notify_all();
    }
}

/// Drop a stale AST publish when the source changes before commit.
#[test]
fn test_source_edit_drops_stale_ast_publish_before_commit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_ast_interleaving")
        .disk_cache()
        .minimal_language_surface()
        .file("main.ts", "export const value: number = 1;")
        .root("main.ts")
        .materialize();

    // open one scenario run with a deterministic pre-commit gate
    let gate = CommitGate::new();
    let run = workspace.open_with_options(|options| {
        options.scenario_event_handler = Some(gate.handler());
    });
    let main = run.module("main.ts");
    let module_id = main.module_id();
    let artifact_key = main.ast_key();
    gate.set_target(artifact_key);

    // provide the target artifact and pause it before commit
    thread::scope(|scope| {
        let compile_run = run.clone();
        let compile_task = scope.spawn(move || {
            compile_run.provide(artifact_key);
        });

        // wait until the stale publish is paused before commit
        gate.wait_until_reached();
        main.replace("export const value: string = 'updated';");
        gate.release();
        compile_task
            .join()
            .unwrap_or_else(|_| panic!("scenario compile worker panicked"));
    });

    // the stale publish should not survive the interleaving
    assert!(
        !run.compiler()
            .artifact_key_is_available(run.current_revision(), &artifact_key)
    );
    assert!(
        run.program()
            .repository()
            .ast(run.current_revision(), module_id)
            .is_none()
    );

    // the next rebuild should converge to the fresh result
    main.require_ast();
    let rebuilt = main.ast().as_ref().clone();

    let fresh = run.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_ast();
    let expected = fresh_main.ast().as_ref().clone();

    // compare the rebuilt surface against a fresh session
    assert_ast_eq(expected, rebuilt);
}

/// Drop a stale dependent publish when one requirement changes before commit.
#[test]
fn test_dependency_edit_drops_stale_resolved_publish_before_commit() {
    let workspace = CompilerScenario::new()
        .with_prefix("scenario_resolved_interleaving")
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

    // open one scenario run with a deterministic pre-commit gate
    let gate = CommitGate::new();
    let run = workspace.open_with_options(|options| {
        options.scenario_event_handler = Some(gate.handler());
    });
    let main = run.module("main.ts");
    let dep = run.module("dep.ts");
    let main_module_id = main.module_id();
    let profile_id = main.profile_id();
    let artifact_key = main.dir_resolved_key();
    gate.set_target(artifact_key);

    // provide the dependent artifact and pause it before commit
    thread::scope(|scope| {
        let compile_run = run.clone();
        let compile_task = scope.spawn(move || {
            compile_run.provide(artifact_key);
        });

        // wait until the dependent publish is paused before commit
        gate.wait_until_reached();
        dep.replace("export const dep: string = 'updated';");
        gate.release();
        compile_task
            .join()
            .unwrap_or_else(|_| panic!("scenario compile worker panicked"));
    });

    // the stale dependent publish should not survive the interleaving
    assert!(
        !run.compiler()
            .artifact_key_is_available(run.current_revision(), &artifact_key)
    );
    assert!(
        run.program()
            .repository()
            .dir_resolved(run.current_revision(), main_module_id, profile_id)
            .is_none()
    );

    // the next rebuild should converge to the fresh result
    main.require_dir_resolved();
    let rebuilt = main.dir_resolved().as_ref().clone();

    let fresh = run.reopen_fresh();
    let fresh_main = fresh.module("main.ts");
    fresh_main.require_dir_resolved();
    let expected = fresh_main.dir_resolved().as_ref().clone();

    // compare the rebuilt surface against a fresh session
    assert_dir_resolved_eq(
        &fresh.program().strings,
        &run.program().strings,
        &expected,
        &rebuilt,
    );
}
