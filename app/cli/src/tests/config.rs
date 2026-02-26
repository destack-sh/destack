use crate::command::config::{ConfigArgs, run};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success};
use serde_json::json;

/// Loads configuration from dsconfig.
#[test]
fn test_config_reads_dsconfig() {
    // set up a basic dsconfig
    let program = TestProgram::new("config");
    program.write_dsconfig_with_base(json!({
        "include": ["src/**/*"],
    }));

    // build config args
    let args = ConfigArgs {
        path: None,
        full: false,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    // run the config command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
