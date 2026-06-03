use crate::command::run::{RunArgs, run};
use crate::common::{DiagnosticArgs, InputArgs, ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs};

use super::tests::assert_exit;

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
