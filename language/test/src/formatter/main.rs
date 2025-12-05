//! Formatter roundtrip test runner.
//!
//! Usage:
//!   cargo test --test formatter              # run all formatter tests
//!   cargo test --test formatter -- fmt-0001  # filter by test name
//!   cargo test --test formatter -- --list    # list tests without running

use std::process::ExitCode;

use clap::Parser;

use destack_test::formatter::run_formatter_tests;
use destack_test::harness::TestOptions;

fn main() -> ExitCode {
    let options = TestOptions::parse();
    run_formatter_tests(&options)
}

