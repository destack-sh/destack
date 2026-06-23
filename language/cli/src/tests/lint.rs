use crate::command::check::{Format, Progress};
use crate::command::lint::{LintArgs, run};
use crate::common::{InputArgs, ProgramArgs, ReportArgs};

use super::tests::assert_success;
/// Lists lint rules without requiring inputs.
#[test]
fn test_lint_list_rules() {
    // build lint args with list rules enabled
    let args = LintArgs {
        input: InputArgs::default(),
        program: ProgramArgs::default(),
        report: ReportArgs::default(),
        fix: false,
        unsafe_fixes: false,
        diff: false,
        format: Format::Text,
        quiet: false,
        no_diagnostics: false,
        max_warnings: None,
        statistics: false,
        progress: Progress::Off,
        list_rules: true,
    };

    // run lint list rules
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
