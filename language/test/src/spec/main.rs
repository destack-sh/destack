use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::spec::SpecSuite;

#[derive(Parser, Debug, Clone)]
#[command(name = "spec", about = "Run Destack specification tests")]
struct SpecOptions {
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = SpecOptions::parse();
    let suite = SpecSuite::load();
    Runner::run_suite(&suite, &options.test)
}
