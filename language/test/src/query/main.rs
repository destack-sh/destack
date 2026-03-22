use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::query::QuerySuite;

#[derive(Parser, Debug, Clone)]
#[command(name = "query", about = "Run Destack LSP query tests")]
struct QueryOptions {
    #[command(flatten)]
    test: RunOptions,
}

fn main() -> ExitCode {
    let options = QueryOptions::parse();
    let suite = match QuerySuite::load() {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    Runner::run_suite(suite, &options.test)
}
