use crate::command::{CheckInput, CommandInput, CommandOptions, CommandRevision};
use crate::tests::harness::TestWorkspace;
use crate::workspace::Workspace;
use destack_repository::TraceView;

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

#[test]
fn test_check_command_lints_selected_module_and_program() {
    let test = TestWorkspace::new("check-command-lints");
    let config_source = r#"{
  "targets": {
    "default": {
      "entry": ["main.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;
    let config = test.write_text("destack.json", config_source);
    test.apply_text(&config, config_source);
    let main_source = "export const value: int32 = 1;\n";
    let main = test.write_text("main.ds", main_source);
    test.apply_text(&main, main_source);
    let selected_source = "debugger;\n";
    let selected = test.write_text("selected.ds", selected_source);
    test.apply_text(&selected, selected_source);

    let mut input = CheckInput::from((
        CommandRevision::Current,
        CommandOptions {
            inputs: vec![CommandInput::File { path: selected }],
            ..CommandOptions::default()
        },
    ));
    input.trace = TraceView::Detailed;
    let output = test
        .workspace
        .check(&test.roots[0], input, None)
        .expect("check command failed");

    // lint the explicit module even though it is outside the target graph
    let diagnostic_codes = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(diagnostic_codes, ["LU008"]);

    // run target program lints and both required module lint passes
    let mut lint_artifacts = output
        .data
        .trace
        .artifacts
        .iter()
        .filter(|artifact| artifact.name.ends_with(".lint") && artifact.outcome == "built")
        .map(|artifact| (artifact.name.as_str(), artifact.label.as_deref()))
        .collect::<Vec<_>>();
    lint_artifacts.sort_unstable();

    assert_eq!(
        lint_artifacts,
        [
            ("module.lint", Some("file://main.ds")),
            ("module.lint", Some("file://selected.ds")),
            ("program.lint", None),
        ]
    );
}
