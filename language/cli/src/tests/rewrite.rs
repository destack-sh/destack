use clap::Parser;
use std::path::PathBuf;
use tspp_source::FileSystem;

use crate::app::{Cli, Command};
use crate::command::rewrite::{RewriteArgs, run as run_rewrite};
use crate::common::ReportArgs;

use super::tests::{TestProgram, assert_success, execute, input_args_from_path};

/// Parse rewrite patterns, replacements, files, and output mode compositionally.
#[test]
fn test_parse_rewrite_command() {
    let rewrite = Cli::try_parse_from([
        "tspp",
        "rewrite",
        "fetch($URL)",
        "client.fetch($URL)",
        "src/main.tspp",
        "--diff",
    ])
    .expect("parse rewrite command");
    let Command::Rewrite(rewrite) = rewrite.command else {
        panic!("expected rewrite command");
    };

    assert_eq!(rewrite.pattern, "fetch($URL)");
    assert_eq!(rewrite.replacement, "client.fetch($URL)");
    assert_eq!(rewrite.input.files, [PathBuf::from("src/main.tspp")]);
    assert!(rewrite.diff);

    let check = Cli::try_parse_from([
        "tspp",
        "rewrite",
        "fetch($URL)",
        "client.fetch($URL)",
        "src/main.tspp",
        "--check",
    ])
    .expect("parse rewrite check command");
    let Command::Rewrite(check) = check.command else {
        panic!("expected rewrite command");
    };
    assert!(check.check);
}

/// Rewrite one source file through the integrated Workspace command.
#[test]
fn test_rewrite_source_file() {
    let program = TestProgram::new("rewrite_source");
    let path = program.write_text("main.tspp", "fetch(\"/a\");\nkeep();\n");
    let args = RewriteArgs {
        pattern: "fetch($URL)".to_string(),
        replacement: "client.fetch($URL)".to_string(),
        input: input_args_from_path(path.clone()),
        predicates: Vec::new(),
        kind: None,
        write: true,
        diff: false,
        check: false,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    let code = execute(run_rewrite(&args));

    assert_success(code);
    assert_eq!(
        program
            .fs
            .read_to_string(&path)
            .expect("read rewritten source"),
        "client.fetch(\"/a\");\nkeep();\n"
    );
}

/// Return failure without changing source when check mode finds replacements.
#[test]
fn test_check_rewrite_source_file() {
    let program = TestProgram::new("check_rewrite_source");
    let source = "fetch(\"/a\");\nkeep();\n";
    let path = program.write_text("main.tspp", source);
    let args = RewriteArgs {
        pattern: "fetch($URL)".to_string(),
        replacement: "client.fetch($URL)".to_string(),
        input: input_args_from_path(path.clone()),
        predicates: Vec::new(),
        kind: None,
        write: false,
        diff: false,
        check: true,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    let code = execute(run_rewrite(&args));

    assert_eq!(code, 1);
    assert_eq!(
        program
            .fs
            .read_to_string(&path)
            .expect("read checked source"),
        source
    );
}
