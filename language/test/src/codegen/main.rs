//! Codegen test runner.
//!
//! Usage:
//!   cargo test --test codegen                    # run all codegen tests
//!   cargo test --test codegen -- hello-world     # filter by test name
//!   cargo test --test codegen -- --list          # list tests without running

use std::process::ExitCode;

use clap::Parser;

use destack_test::codegen::run_codegen_tests;
use destack_test::harness::TestOptions;

fn main() -> ExitCode {
    let options = TestOptions::parse();
    run_codegen_tests(&options)
}
