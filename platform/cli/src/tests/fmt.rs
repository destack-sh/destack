use crate::command::fmt::{FmtArgs, run};
use crate::common::{DiagnosticArgs, ReportArgs};

use destack_source::FileSystem;

use super::tests::{TestProgram, assert_success};

/// Formats json files and updates them on disk.
#[test]
fn test_fmt_formats_json_file() {
    // set up a json file with minimal spacing
    let program = TestProgram::new("fmt_json");
    let path = program.write_source("config.json", r#"{"a":1}"#);

    // build formatter args
    let args = FmtArgs {
        files: vec![path.clone()],
        eval: None,
        check: false,
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert the file was updated
    assert_success(code);
    let formatted = program
        .fs
        .read_to_string(&path)
        .expect("formatted file should be readable");
    assert_eq!(
        formatted,
        r#"{"a": 1}
"#
    );
}
