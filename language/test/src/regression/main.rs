use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::regression::RegressionSuite;

/// Run Destack regression tests.
fn main() -> ExitCode {
    let options = TestOptions::parse();
    Runner::run_suite(&RegressionSuite, &options)
}
