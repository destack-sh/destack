use std::cell::RefCell;

use destack_source::{FileWatchEvent, FileWatchEventKind, MemoryFileWatcher};
use serde_json::json;

use crate::command::check::{CheckArgs, Format, Progress, run, run_watch_with_options};
use crate::common::{DiagnosticArgs, FormatOptions, ReportArgs, WatchCompileReason};

use super::tests::{
    TestProgram, assert_success, input_args_from_path, watch_loop_options_for_test,
};

/// Checks a simple module without linting.
#[test]
fn test_check_compiles_single_file() {
    // set up a minimal source file
    let program = TestProgram::new("check_single");
    let path = program.write_source("main.ds", "export const answer = 42;\n");

    // build check args
    let args = CheckArgs {
        input: input_args_from_path(path),
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        fix: false,
        unsafe_fixes: false,
        diff: false,
        no_lint: true,
        format: Format::Text,
        quiet: true,
        no_diagnostics: true,
        max_warnings: None,
        statistics: false,
        progress: Progress::Off,
    };

    // run the check command
    let code = run(&args);

    // assert the check succeeded
    assert_success(code);
}

/// Check watch mode handles source updates.
#[test]
fn test_check_watch_handles_update() {
    // set up a minimal source file
    let program = TestProgram::new("check_watch_update");
    let path = program.write_source("main.ds", "export const answer = 42;\n");

    // build check args with watch enabled
    let mut args = CheckArgs {
        input: input_args_from_path(path.clone()),
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        fix: false,
        unsafe_fixes: false,
        diff: false,
        no_lint: true,
        format: Format::Text,
        quiet: true,
        no_diagnostics: true,
        max_warnings: None,
        statistics: false,
        progress: Progress::Off,
    };
    args.program.watch = true;

    // build format options for watch output
    let format_options = FormatOptions {
        format: args.format.into(),
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        suppress_diagnostics: args.no_diagnostics,
    };

    // configure the memory watcher
    let watcher = MemoryFileWatcher::new();
    let watch_options = watch_loop_options_for_test(watcher.clone());

    // capture the observed compile reason
    let observed_reason: RefCell<Option<WatchCompileReason>> = RefCell::new(None);

    // run the watch command until the first compile completes
    let exit_code = run_watch_with_options(
        &args,
        "check",
        None,
        &format_options,
        None,
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

/// Check watch mode handles config rescans.
#[test]
fn test_check_watch_handles_config_rescan() {
    // set up a minimal source file
    let program = TestProgram::new("check_watch_config");
    let path = program.write_source("main.ds", "export const answer = 42;\n");

    // build check args with watch enabled
    let mut args = CheckArgs {
        input: input_args_from_path(path.clone()),
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        fix: false,
        unsafe_fixes: false,
        diff: false,
        no_lint: true,
        format: Format::Text,
        quiet: true,
        no_diagnostics: true,
        max_warnings: None,
        statistics: false,
        progress: Progress::Off,
    };
    args.program.watch = true;

    // build format options for watch output
    let format_options = FormatOptions {
        format: args.format.into(),
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        suppress_diagnostics: args.no_diagnostics,
    };

    // configure the memory watcher
    let watcher = MemoryFileWatcher::new();
    let watch_options = watch_loop_options_for_test(watcher.clone());
    let config_path = program.root.join("dsconfig.json");

    // capture the observed compile reason
    let observed_reason: RefCell<Option<WatchCompileReason>> = RefCell::new(None);

    // run the watch command until the first compile completes
    let exit_code = run_watch_with_options(
        &args,
        "check",
        None,
        &format_options,
        None,
        watch_options,
        || {
            program.write_dsconfig_with_base(json!({}));
            watcher.emit(FileWatchEvent {
                path: config_path.clone(),
                previous_path: None,
                kind: FileWatchEventKind::Created,
            });
        },
        |reason, updated, rescan| {
            assert!(rescan);
            let _ = updated;
            observed_reason.replace(Some(reason));
        },
        true,
    );

    // check that the watch completes successfully
    assert_success(exit_code);
    let reason = observed_reason.into_inner();
    assert!(matches!(
        reason,
        Some(WatchCompileReason::Rescan) | Some(WatchCompileReason::UpdateRescan)
    ));
}
