use futures::executor::block_on;
use tspp_repository::TraceView;
use tspp_source::{DiagnosticTarget, FileId, Span};

use crate::command::{CheckInput, CommandInput, CommandOptions, CommandRevision};
use crate::tests::harness::TestWorkspace;

#[test]
fn test_check_command_reports_check_errors() {
    let test = TestWorkspace::new("check-command-diagnostics");
    let source = r#"
const wrong: string = 1;
"#;
    let main = test.write_text("main.tspp", source);
    test.apply_text(&main, source);

    let input = CheckInput::from((
        CommandRevision::Current,
        CommandOptions {
            inputs: vec![CommandInput::File { path: main }],
            ..CommandOptions::default()
        },
    ));
    let output = block_on(test.workspace.check(input, None)).expect("check command failed");

    let ids: Vec<&str> = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect();
    assert!(
        ids.contains(&"not-assignable"),
        "check errors never reached the command diagnostics: {ids:?}"
    );
}

#[test]
fn test_check_command_renders_cross_file_labels() {
    let test = TestWorkspace::new("check-command-cross-file");
    let util_source = r#"
export const helper: int32 = 1;
export const sibling: int32 = 2;
"#;
    let util = test.write_text("util.tspp", util_source);
    test.apply_text(&util, util_source);
    let main_source = r#"
import { helper } from "./util.tspp";

const second = sibling;
"#;
    let main = test.write_text("main.tspp", main_source);
    test.apply_text(&main, main_source);

    let input = CheckInput::from((
        CommandRevision::Current,
        CommandOptions {
            inputs: vec![CommandInput::File { path: main }],
            ..CommandOptions::default()
        },
    ));
    let output = block_on(test.workspace.check(input, None)).expect("check command failed");

    // label the exact imported declaration across files
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "unresolved-reference")
        .expect("missing the unresolved reference");
    let label = diagnostic
        .labels()
        .next()
        .expect("missing the cross-file label");
    assert_eq!(label.message.as_deref(), Some("'sibling' is declared here"));
    let declaration_start = util_source.find("sibling").expect("find declaration") as u32;
    assert_eq!(
        label.target,
        DiagnosticTarget::Span(Span::at(
            FileId::from_logical_str("util.tspp"),
            declaration_start,
            7,
        ))
    );

    // collect every file required to render the diagnostic
    let mut files = output
        .files
        .iter()
        .map(|image| image.name.as_str())
        .collect::<Vec<_>>();
    files.sort_unstable();

    assert_eq!(files, ["main.tspp", "util.tspp"]);
}

#[test]
fn test_check_command_requests_selected_module_and_program_lints() {
    let test = TestWorkspace::new("check-command-lints");
    let config_source = r#"{
  "name": "test",
  "targets": {
    "default": {
      "entry": ["main.tspp"]
    }
  },
  "defaultTarget": "default",
  "linter": {
    "only": ["no-debugger"]
  }
}
"#;
    let config = test.write_text("destack.json", config_source);
    test.apply_text(&config, config_source);
    let main_source = "export const value: int32 = 1;\n";
    let main = test.write_text("main.tspp", main_source);
    test.apply_text(&main, main_source);
    let selected_source = "debugger;\n";
    let selected = test.write_text("selected.tspp", selected_source);
    test.apply_text(&selected, selected_source);

    let mut input = CheckInput::from((
        CommandRevision::Current,
        CommandOptions {
            inputs: vec![CommandInput::File { path: selected }],
            ..CommandOptions::default()
        },
    ));
    input.trace = Some(TraceView::Detailed);
    let output = block_on(test.workspace.check(input, None)).expect("check command failed");

    // lint the explicit module even though it is outside the target graph
    let diagnostic_ids = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(diagnostic_ids, ["no-debugger"]);

    // request the selected module and its target program lint artifacts
    let trace = output.trace.expect("check trace");
    let mut lint_artifacts = trace
        .attempts
        .iter()
        .filter(|artifact| artifact.name.ends_with(".lint") && artifact.outcome == "built")
        .map(|artifact| (artifact.name.as_str(), artifact.label.as_deref()))
        .collect::<Vec<_>>();
    lint_artifacts.sort_unstable();

    assert_eq!(
        lint_artifacts,
        [
            ("module.lint", Some("file://selected.tspp")),
            ("program.lint", None),
        ]
    );
}
