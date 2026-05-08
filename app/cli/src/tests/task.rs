use crate::command::task::{TaskArgs, run};
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
        name: None,
        projects: Vec::new(),
        groups: Vec::new(),
        args: Vec::new(),
        dry_run: false,
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
        name: Some("build".to_string()),
        projects: Vec::new(),
        groups: Vec::new(),
        args: Vec::new(),
        dry_run: true,
    };

    // run the task command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}

/// Runs one workspace task for one selected project group.
#[test]
fn test_task_run_dry_run_for_workspace_group() {
    // set up a workspace with one named group
    let program = TestProgram::new("task_workspace_group");
    program.write_destack_config_with_base(json!({
        "workspace": {
            "members": ["apps/*"],
            "groups": {
                "product": ["apps/web"],
            },
        },
    }));
    program.write_json(
        "apps/web/destack.json",
        json!({
            "tasks": {
                "build": "echo web",
            },
        }),
    );
    program.write_json(
        "apps/api/destack.json",
        json!({
            "tasks": {
                "build": "echo api",
            },
        }),
    );

    // build one grouped task invocation
    let args = TaskArgs {
        program: program.program_args(),
        report: ReportArgs::default(),
        name: Some("build".to_string()),
        projects: Vec::new(),
        groups: vec!["product".to_string()],
        args: Vec::new(),
        dry_run: true,
    };

    // run the task command
    let code = run(&args);

    // assert the command succeeded
    assert_success(code);
}
