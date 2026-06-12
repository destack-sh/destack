use crate::command::check::{CheckArgs, Format, Progress, run};
use crate::common::{DiagnosticArgs, ReportArgs};

use super::tests::{TestProgram, assert_success, input_args_from_path};

/// Checks a simple module without linting.
#[test]
fn test_check_compiles_single_file() {
    // set up a minimal source file
    let program = TestProgram::new("check_single");
    let path = program.write_text("main.ds", "export const answer = 42;\n");

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
        timings: false,
    };

    // run the check command
    let code = run(&args);

    // assert the check succeeded
    assert_success(code);
}
