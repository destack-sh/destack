use crate::command::{CheckInput, CommandInput, CommandOptions, CommandRevision};
use crate::tests::harness::TestWorkspace;
use crate::workspace::Workspace;

/// Check errors must reach the check command's diagnostics.
///
/// Check attaches its diagnostics to the checked component artifact, while
/// the command requests per-module facades; the command query must follow
/// the dependency closure between them.
#[test]
fn test_check_command_reports_check_errors() {
    let test = TestWorkspace::new("check-command-diagnostics");
    let main = test.write_text("main.ds", "const wrong: string = 1;\n");
    test.apply_text(&main, "const wrong: string = 1;\n");

    let input = CheckInput::from((
        CommandRevision::Current,
        CommandOptions {
            inputs: vec![CommandInput::File { path: main }],
            ..CommandOptions::default()
        },
    ));
    let output = test
        .workspace
        .check(&test.roots[0], input, None)
        .expect("check command failed");

    let codes: Vec<&str> = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert!(
        codes.contains(&"EC200"),
        "check errors never reached the command diagnostics: {codes:?}"
    );
}
