//! Smoke test runner.
//!
//! Usage:
//!   cargo test --test smoke              # run all smoke tests
//!   cargo test --test smoke -- --parser  # run only parser tests
//!   cargo test --test smoke -- --compiler # run only compiler tests
//!   cargo test --test smoke -- smoke1    # filter by test name
//!   cargo test --test smoke -- --list    # list tests without running

use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::TestOptions;
use destack_test::smoke::{run_compiler_smoke_tests, run_parser_smoke_tests};

/// Smoke test specific options.
#[derive(Parser, Debug, Clone)]
#[command(name = "smoke", about = "Run Destack smoke tests")]
struct SmokeOptions {
    /// Run only parser smoke tests.
    #[arg(long)]
    parser: bool,

    /// Run only compiler smoke tests.
    #[arg(long)]
    compiler: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = SmokeOptions::parse();

    // determine which tests to run
    // if neither flag is set, run both
    let run_parser = options.parser || !options.compiler;
    let run_compiler = options.compiler || !options.parser;

    let mut any_failed = false;

    if run_parser {
        let result = run_parser_smoke_tests(&options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_compiler {
        let result = run_compiler_smoke_tests(&options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
