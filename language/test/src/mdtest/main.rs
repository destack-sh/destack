use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::mdtest::MdtestSuite;

/// CLI options for the `mdtest` test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "mdtest", about = "Run Destack markdown specification tests")]
struct MdTestOptions {
    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = MdTestOptions::parse();
    let suite = MdtestSuite::load();
    Runner::run_suite(&suite, &options.test)
}
