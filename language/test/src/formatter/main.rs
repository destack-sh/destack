use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::formatter::FormatterSuite;

fn main() -> ExitCode {
    let options = RunOptions::parse();
    let suite = match FormatterSuite::load() {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    Runner::run_suite(suite, &options)
}
