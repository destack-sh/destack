use crate::command::targets::{TargetsArgs, run};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success, execute};
use serde_json::json;

/// Lists targets defined in package.json.
#[test]
fn test_targets_lists_configured_targets() {
    // set up a config with one target
    let program = TestProgram::new("targets");
    program.write_manifest_with_base(json!({
        "targets": {
            "js": {},
        },
    }));

    // build targets args
    let args = TargetsArgs {
        all: false,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    // run the targets command
    let code = execute(run(&args));

    // assert the command succeeded
    assert_success(code);
}
