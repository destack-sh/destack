use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::specification::SpecSuite;

#[derive(Parser, Debug, Clone)]
#[command(name = "specification", about = "Run Destack specification tests")]
struct SpecificationOptions {
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = SpecificationOptions::parse();
    let suite = SpecSuite::load();
    Runner::run_suite(&suite, &options.test)
}
