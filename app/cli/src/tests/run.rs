use std::cell::{Cell, RefCell};

use destack_source::{FileWatchEvent, FileWatchEventKind, MemoryFileWatcher};

use crate::command::run::{RunArgs, RunMode, RunRequest, run, run_watch_with_driver};
use crate::common::{
    DiagnosticArgs, InputArgs, ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs, WatchCompileReason,
};

use super::tests::{TestProgram, assert_exit, input_args_from_path, watch_loop_options_for_test};

/// Rejects run without input sources.
#[test]
fn test_run_requires_input() {
    // build run args without inputs
    let args = RunArgs {
        input: InputArgs::default(),
        program: ProgramArgs::default(),
        target: TargetArgs::default(),
        runtime: RuntimeArgs::default(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        entry: "main".to_string(),
        args: Vec::new(),
    };

    // run without input
    let code = run(&args);

    // assert the command fails
    assert_exit(code, 1);
}

/// Run watch mode handles source updates.
#[test]
fn test_run_watch_handles_update() {
    // set up a minimal source file
    let program = TestProgram::new("run_watch_update");
    let path = program.write_text("main.ds", "export function main(): number { return 0; }\n");

    // build a run request with watch enabled
    let mut request = RunRequest {
        command_name: "run",
        input: input_args_from_path(path.clone()),
        program: program.program_args(),
        target: TargetArgs::default(),
        runtime: RuntimeArgs::default(),
        report: ReportArgs::default(),
        entry: "main".to_string(),
        args: Vec::new(),
        mode: RunMode::Program,
    };
    request.program.watch = true;

    // configure the memory watcher
    let watcher = MemoryFileWatcher::new();
    let watch_options = watch_loop_options_for_test(watcher.clone());

    // capture the observed compile reason
    let observed_reason: RefCell<Option<WatchCompileReason>> = RefCell::new(None);

    // run the watch command until the first compile completes
    let compile_calls: Cell<usize> = Cell::new(0);

    let exit_code = run_watch_with_driver(
        &request,
        watch_options,
        || {
            // update the source file to trigger a watch event
            let _ = program.write_text("main.ds", "export function main(): number { return 1; }\n");
            watcher.emit(FileWatchEvent {
                path: path.clone(),
                previous_path: None,
                kind: FileWatchEventKind::Modified,
            });
        },
        |reason, updated, rescan| {
            assert!(updated);
            assert!(!rescan);
            observed_reason.replace(Some(reason));
        },
        |_, _, _, _, _, _reason, _, _, _, _| {
            compile_calls.set(compile_calls.get().saturating_add(1));
            0
        },
        true,
    );

    // check that the watch completes successfully
    assert_exit(exit_code, 0);
    assert_eq!(compile_calls.get(), 2);
    assert_eq!(
        observed_reason.into_inner(),
        Some(WatchCompileReason::Update)
    );
}
