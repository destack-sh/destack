use crate::command::fmt::{FmtArgs, run};
use crate::common::program::QuoteStyleArg;
use crate::common::{DiagnosticArgs, ReportArgs};
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_command_once};

use destack_daemon::protocol::{CommandFormatOptions, CommandFormatPayload, CommandPayload};
use destack_source::FileSystem;
use serde_json::json;

use super::tests::{TestProgram, assert_exit, assert_success};

/// Formats json files and updates them on disk.
#[test]
fn test_fmt_formats_json_file() {
    // set up a json file with minimal spacing
    let program = TestProgram::new("fmt_json");
    let path = program.write_text("config.json", r#"{"a":1}"#);

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

/// Formats destack files and updates them on disk.
#[test]
fn test_fmt_formats_destack_file() {
    // set up a source file with minimal spacing
    let program = TestProgram::new("fmt_destack");
    let path = program.write_text("main.ds", "const answer=42");

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
    assert_eq!(formatted, "const answer = 42;\n");
}

/// Formats typescript files during default directory scans.
#[test]
fn test_fmt_default_scan_includes_typescript() {
    // set up a typescript file in the root
    let program = TestProgram::new("fmt_scan_ts");
    let path = program.write_text("main.ts", "const answer=42");

    // build formatter args with default scan behavior
    let args = FmtArgs {
        files: Vec::new(),
        eval: None,
        check: false,
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert the file was updated from directory scan
    assert_success(code);
    let formatted = program
        .fs
        .read_to_string(&path)
        .expect("formatted file should be readable");
    assert_eq!(formatted, "const answer = 42;\n");
}

/// Skips build output during default directory scans.
#[test]
fn test_fmt_default_scan_skips_dist() {
    // set up a regular source and an output source
    let program = TestProgram::new("fmt_scan_ignore");
    let src_path = program.write_text("src/main.ts", "const answer=42");
    let output_path = program.write_text("dist/index.ts", "const output=1");

    // build formatter args with default scan behavior
    let args = FmtArgs {
        files: Vec::new(),
        eval: None,
        check: false,
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert the regular source is formatted
    assert_success(code);
    let src_formatted = program
        .fs
        .read_to_string(&src_path)
        .expect("formatted source should be readable");
    assert_eq!(src_formatted, "const answer = 42;\n");

    // assert output was ignored
    let output_content = program
        .fs
        .read_to_string(&output_path)
        .expect("output source should be readable");
    assert_eq!(output_content, "const output=1");
}

/// Returns check failure when formatting changes are needed.
#[test]
fn test_fmt_check_mode_returns_nonzero_on_change() {
    // set up an unformatted source file
    let program = TestProgram::new("fmt_check");
    let path = program.write_text("main.ds", "const answer=42");

    // build formatter args in check mode
    let args = FmtArgs {
        files: vec![path.clone()],
        eval: None,
        check: true,
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert check mode fails and does not mutate the file
    assert_exit(code, 1);
    let original = program
        .fs
        .read_to_string(&path)
        .expect("original source should be readable");
    assert_eq!(original, "const answer=42");
}

/// Continues formatting valid files when another file has parse errors.
#[test]
fn test_fmt_formats_valid_files_when_other_files_error() {
    // set up one valid file and one invalid file
    let program = TestProgram::new("fmt_mixed_errors");
    let good_path = program.write_text("good.ds", "const answer=42");
    let bad_path = program.write_text("bad.ds", "const broken =");

    // build formatter args for explicit files
    let args = FmtArgs {
        files: vec![good_path.clone(), bad_path.clone()],
        eval: None,
        check: false,
        program: program.program_args(),
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert the command fails due to the broken file
    assert_exit(code, 1);

    // assert the valid file was still formatted
    let good_content = program
        .fs
        .read_to_string(&good_path)
        .expect("formatted source should be readable");
    assert_eq!(good_content, "const answer = 42;\n");

    // assert the broken file was not mutated
    let bad_content = program
        .fs
        .read_to_string(&bad_path)
        .expect("broken source should be readable");
    assert_eq!(bad_content, "const broken =");
}

/// Applies formatter options from destack.json.
#[test]
fn test_fmt_applies_destack_config_formatter_options() {
    // set up source and formatter config
    let program = TestProgram::new("fmt_destack_config_options");
    program.write_destack_config(json!({
        "formatter": {
            "singleQuote": true
        }
    }));
    let path = program.write_text("main.ts", "const message=\"hello\"");

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

    // assert config options were applied
    assert_success(code);
    let formatted = program
        .fs
        .read_to_string(&path)
        .expect("formatted file should be readable");
    assert_eq!(formatted, "const message = 'hello';\n");
}

/// Preserves explicit CLI formatter overrides over destack.json options.
#[test]
fn test_fmt_cli_formatter_overrides_destack_config() {
    // set up source and formatter config
    let program = TestProgram::new("fmt_cli_overrides_destack_config");
    program.write_destack_config(json!({
        "formatter": {
            "singleQuote": true
        }
    }));
    let path = program.write_text("main.ts", "const message=\"hello\"");

    // build formatter args with an explicit cli formatter override
    let mut program_args = program.program_args();
    program_args.formatter.quote_style = Some(QuoteStyleArg::Double);
    let args = FmtArgs {
        files: vec![path.clone()],
        eval: None,
        check: false,
        program: program_args,
        diagnostics: DiagnosticArgs::default(),
        report: ReportArgs::default(),
    };

    // run the formatter
    let code = run(&args);

    // assert cli options win over destack.json
    assert_success(code);
    let formatted = program
        .fs
        .read_to_string(&path)
        .expect("formatted file should be readable");
    assert_eq!(formatted, "const message = \"hello\";\n");
}

/// Includes changed and error file paths in the format payload.
#[test]
fn test_fmt_payload_includes_changed_and_error_files() {
    // set up one valid and one invalid source file
    let program = TestProgram::new("fmt_payload_lists");
    let good_path = program.write_text("good.ds", "const answer=42");
    let bad_path = program.write_text("bad.ds", "const broken =");

    // build daemon command options
    let common = CommandOptionsBuilder::new(&program.program_args()).build();
    let payload = CommandPayload::Format(CommandFormatOptions {
        files: vec![good_path.clone(), bad_path.clone()],
        eval: None,
        check: false,
    });

    // run the daemon command directly so we can inspect payload data
    let result = run_root_command_once(&program.program_args(), common, payload, None)
        .expect("format command should return a response");

    // parse and decode the format payload
    let data = result
        .response
        .data
        .as_ref()
        .expect("format response should include payload data");
    let value = data
        .to_json_value()
        .expect("format payload should decode to json");
    let payload: CommandFormatPayload =
        serde_json::from_value(value).expect("payload should decode to format payload");

    // assert summary counts
    assert_eq!(payload.files, 2);
    assert_eq!(payload.changed, 1);
    assert_eq!(payload.errors, 1);

    // assert path lists are included and accurate
    let good_path = good_path.display().to_string();
    let bad_path = bad_path.display().to_string();
    assert_eq!(payload.changed_files, vec![good_path]);
    assert_eq!(payload.error_files, vec![bad_path]);
}
