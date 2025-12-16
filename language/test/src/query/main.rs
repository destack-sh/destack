use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::query::QuerySuite;

#[derive(Parser, Debug, Clone)]
#[command(name = "query", about = "Run Destack LSP query tests")]
struct QueryOptions {
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = QueryOptions::parse();
    let suite = QuerySuite::load();
    Runner::run_suite(&suite, &options.test)
}
