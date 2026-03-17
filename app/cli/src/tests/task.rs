use crate::command::task::{TaskArgs, TaskCommand, run};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success};
use serde_json::json;

/// Lists tasks defined in destack.json.
#[test]
fn test_task_list_reads_tasks() {
    // set up a config with tasks
    let program = TestProgram::new("task_list");
    program.write_destack_config_with_base(json!({
        "tasks": {
            "build": "echo build",
        },
    }));

    // build task args
    let args = TaskArgs {
        program: program.program_args(),
        report: ReportArgs::default(),
        command: Some(TaskCommand::List),
    };

    // run the task command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}

/// Runs a task in dry run mode.
#[test]
fn test_task_run_dry_run() {
    // set up a config with tasks
    let program = TestProgram::new("task_run_dry");
    program.write_destack_config_with_base(json!({
        "tasks": {
            "build": "echo build",
        },
    }));

    // build task args with dry run
    let args = TaskArgs {
        program: program.program_args(),
        report: ReportArgs::default(),
        command: Some(TaskCommand::Run {
            name: "build".to_string(),
            args: Vec::new(),
            dry_run: true,
        }),
    };

    // run the task command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
