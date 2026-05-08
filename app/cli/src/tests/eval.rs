use crate::command::eval::{EvalArgs, run};
use crate::common::{DiagnosticArgs, ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs};

use super::tests::assert_exit;

/// Rejects conflicting eval inputs.
#[test]
fn test_eval_rejects_conflicting_inputs() {
    // build conflicting eval args
    let args = EvalArgs {
        code: Some("1 + 1".to_string()),
        eval: Some("2 + 2".to_string()),
        stdin: false,
        file_type: None,
        print: false,
        entry: "main".to_string(),
        program: ProgramArgs::default(),
        target: TargetArgs::default(),
        runtime: RuntimeArgs::default(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
        args: Vec::new(),
    };

    // run eval with invalid input
    let code = run(&args);

    // assert the command fails
    assert_exit(code, 1);
}
