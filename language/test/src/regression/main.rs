use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::regression::RegressionSuite;

/// Run Destack regression tests.
fn main() -> ExitCode {
    let options = RunOptions::parse();
    Runner::run_suite(RegressionSuite, &options)
}
