use std::process::ExitCode;

use clap::Parser;

use destack_test::emit::EmitSuite;
use destack_test::harness::{Runner, TestOptions};

fn main() -> ExitCode {
    let options = TestOptions::parse();
    Runner::run_suite(&EmitSuite, &options)
}
