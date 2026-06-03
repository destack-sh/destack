use crate::command::build::{BuildArgs, run};
use crate::common::{DiagnosticArgs, ReportArgs, TargetArgs};

use super::tests::{TestProgram, assert_success, input_args_from_path};

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
