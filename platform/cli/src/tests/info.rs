use crate::command::info::{InfoArgs, run};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success};
use serde_json::json;

/// Shows workspace info with a config present.
#[test]
fn test_info_with_dsconfig() {
    // set up a config with a target
    let program = TestProgram::new("info");
    program.write_dsconfig_with_base(json!({
        "targets": {
            "js": {},
        },
    }));

    // build info args
    let args = InfoArgs {
        all: false,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    // run the info command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
