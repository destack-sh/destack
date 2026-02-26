use std::cell::RefCell;

use destack_source::{FileWatchEvent, FileWatchEventKind, MemoryFileWatcher};

use crate::command::build::{BuildArgs, run, run_watch_with_options};
use crate::common::{DiagnosticArgs, ReportArgs, TargetArgs, WatchCompileReason};

use super::tests::{
    TestProgram, assert_success, input_args_from_path, watch_loop_options_for_test,
};

/// Builds a single source file in dry run mode.
#[test]
fn test_build_dry_run_single_file() {
    // set up a minimal source file
    let program = TestProgram::new("build_dry_run");
    let path = program.write_text("main.ds", "export const answer = 42;\n");

    // build args with dry run enabled
    let args = BuildArgs {
        input: input_args_from_path(path),
        target: TargetArgs::default(),
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        dry_run: true,
    };

    // run the build command
    let code = run(&args);

    // assert the build succeeded
    assert_success(code);
}

/// Build watch mode handles source updates.
#[test]
fn test_build_watch_handles_update() {
    // set up a minimal source file
    let program = TestProgram::new("build_watch_update");
    let path = program.write_text("main.ds", "export const answer = 42;\n");

    // build args with watch enabled
    let mut args = BuildArgs {
        input: input_args_from_path(path.clone()),
        target: TargetArgs::default(),
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        dry_run: false,
    };
    args.program.watch = true;

    // configure the memory watcher
    let watcher = MemoryFileWatcher::new();
    let watch_options = watch_loop_options_for_test(watcher.clone());

    // capture the observed compile reason
    let observed_reason: RefCell<Option<WatchCompileReason>> = RefCell::new(None);

    // run the watch command until the first compile completes
    let exit_code = run_watch_with_options(
        &args,
        "default",
        watch_options,
        || {
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
        true,
    );

    // check that the watch completes successfully
    assert_success(exit_code);
    assert_eq!(
        observed_reason.into_inner(),
        Some(WatchCompileReason::Update)
    );
}
