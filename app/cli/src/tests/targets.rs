use crate::command::targets::{TargetsArgs, run};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success};
use serde_json::json;

/// Lists targets defined in destack.json.
#[test]
fn test_targets_lists_configured_targets() {
    // set up a config with one target
    let program = TestProgram::new("targets");
    program.write_destack_config_with_base(json!({
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
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
