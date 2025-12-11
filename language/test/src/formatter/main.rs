use std::process::ExitCode;

use clap::Parser;

use destack_test::formatter::FormatterSuite;
use destack_test::harness::{Runner, TestOptions};

fn main() -> ExitCode {
    let options = TestOptions::parse();
    Runner::run_suite(&FormatterSuite, &options)
}
