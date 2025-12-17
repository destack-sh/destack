use std::process::ExitCode;

use clap::Parser;

use destack_test::formatter::FormatterSuite;
use destack_test::harness::{Runner, TestOptions};

fn main() -> ExitCode {
    let options = TestOptions::parse();
    let suite = FormatterSuite::load();
    Runner::run_suite(&suite, &options)
}
