//! Formatter roundtrip test runner.

use std::process::ExitCode;

use clap::Parser;

use destack_test::formatter::run_formatter_tests;
use destack_test::harness::TestOptions;

fn main() -> ExitCode {
    let options = TestOptions::parse();
    run_formatter_tests(&options)
}
