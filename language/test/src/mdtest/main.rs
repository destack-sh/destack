//! Markdown test runner.
//!
//! Usage:
//!   cargo test --test mdtest              # run all markdown tests
//!   cargo test --test mdtest -- variables # filter by test name
//!   cargo test --test mdtest -- --list    # list tests without running
//!   cargo test --test mdtest -- --verbose # show verbose output

use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::TestOptions;
use destack_test::mdtest::run_mdtests;

/// Markdown test specific options.
#[derive(Parser, Debug, Clone)]
#[command(name = "mdtest", about = "Run Destack markdown specification tests")]
struct MdTestOptions {
    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = MdTestOptions::parse();
    run_mdtests(&options.test)
}
