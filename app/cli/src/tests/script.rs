use super::tests::TestProgram;
use crate::pipeline::script::{ScriptSource, load_tasks, resolve_script_command};
use serde_json::json;

/// Resolves destack.json tasks.
#[test]
fn test_resolve_script_command_reads_destack_config() {
    // setup
    let program = TestProgram::new("script_destack_config");
    program.write_destack_config_with_base(json!({
        "tasks": {
            "build": "echo ds",
        },
    }));

    // resolve the script command
    let script = resolve_script_command(&program.program_args(), "build")
        .expect("script lookup should succeed")
        .expect("script should be found");

    // assert destack.json task resolves
    assert_eq!(script.source, ScriptSource::Destack);
    assert_eq!(script.command, "echo ds");
    assert_eq!(script.cwd, program.root);
}

/// Resolves task working directories relative to destack.json.
#[test]
fn test_load_tasks_resolves_relative_cwd() {
    // setup
    let program = TestProgram::new("script_tasks");
    program.write_destack_config_with_base(json!({
        "tasks": {
            "serve": {
                "command": "echo ok",
                "cwd": "scripts",
            },
        },
    }));
    let destack_config_path = program.root.join("destack.json");
    let revision = program.current_revision();

    // load tasks from the config
    let tasks = load_tasks(&program.repository, revision, &destack_config_path)
        .expect("task loading should succeed");
    let task = tasks
        .iter()
        .find(|task| task.name == "serve")
        .expect("task should be present");

    // assert the cwd was resolved
    assert_eq!(task.cwd, Some(program.root.join("scripts")));
}
