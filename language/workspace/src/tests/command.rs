use crate::command::{CheckInput, CommandInput, CommandOptions, CommandRevision};
use crate::tests::harness::TestWorkspace;
use crate::workspace::Workspace;

#[test]
fn test_check_command_reports_check_errors() {
    let test = TestWorkspace::new("check-command-diagnostics");
    let source = r#"
const wrong: string = 1;
"#;
    let main = test.write_text("main.ds", source);
    test.apply_text(&main, source);

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

#[test]
fn test_check_command_renders_cross_file_labels() {
    let test = TestWorkspace::new("check-command-cross-file");
    let util_source = r#"
export const helper: int32 = 1;
export const sibling: int32 = 2;
"#;
    let util = test.write_text("util.ds", util_source);
    test.apply_text(&util, util_source);
    let main_source = r#"
import { helper } from "./util.ds";

const second = sibling;
"#;
    let main = test.write_text("main.ds", main_source);
    test.apply_text(&main, main_source);

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

    // the sibling hint labels the declaring module across files
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "EC308")
        .expect("missing the unresolved reference");
    let label = diagnostic
        .labels()
        .next()
        .expect("missing the cross-file label");
    assert_eq!(
        label.message.as_deref(),
        Some("'sibling' is declared in this module")
    );

    // the render path resolves every labeled file without degrading
    let files = output
        .files
        .iter()
        .map(|image| image.name.as_str())
        .collect::<Vec<_>>();
    assert!(
        files.contains(&"util.ds"),
        "cross-file label source was not collected: {files:?}"
    );
}
