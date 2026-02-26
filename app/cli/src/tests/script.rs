use super::tests::TestProgram;
use crate::pipeline::script::{ScriptSource, load_tasks, resolve_script_command};
use serde_json::json;

/// Resolves dsconfig tasks before package.json scripts.
#[test]
fn test_resolve_script_command_prefers_dsconfig() {
    // setup
    let program = TestProgram::new("script_dsconfig");
    program.write_dsconfig_with_base(json!({
        "tasks": {
            "build": "echo ds",
        },
    }));
    program.write_package_json(json!({
        "scripts": {
            "build": "echo pkg",
        },
    }));

    // resolve the script command
    let script = resolve_script_command(&program.program_args(), "build")
        .expect("script lookup should succeed")
        .expect("script should be found");

    // assert dsconfig task wins
    assert_eq!(script.source, ScriptSource::DsConfig);
    assert_eq!(script.command, "echo ds");
    assert_eq!(script.cwd, program.root);
}

/// Resolves package.json scripts when dsconfig tasks are absent.
#[test]
fn test_resolve_script_command_falls_back_to_package_json() {
    // setup
    let program = TestProgram::new("script_package");
    program.write_package_json(json!({
        "scripts": {
            "start": "echo pkg",
        },
    }));

    // resolve the script command
    let script = resolve_script_command(&program.program_args(), "start")
        .expect("script lookup should succeed")
        .expect("script should be found");

    // assert package.json script wins
    assert_eq!(script.source, ScriptSource::PackageJson);
    assert_eq!(script.command, "echo pkg");
    assert_eq!(script.cwd, program.root);
}

/// Resolves task working directories relative to dsconfig.
#[test]
fn test_load_tasks_resolves_relative_cwd() {
    // setup
    let program = TestProgram::new("script_tasks");
    program.write_dsconfig_with_base(json!({
        "tasks": {
            "serve": {
                "command": "echo ok",
                "cwd": "scripts",
            },
        },
    }));
    let dsconfig_path = program.root.join("dsconfig.json");

    // load tasks from the config
    let tasks = load_tasks(&program.resolver, &dsconfig_path).expect("task loading should succeed");
    let task = tasks
        .iter()
        .find(|task| task.name == "serve")
        .expect("task should be present");

    // assert the cwd was resolved
    assert_eq!(task.cwd, Some(program.root.join("scripts")));
}
