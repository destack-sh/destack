use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::stress::{
    CheckerStressSuite, LspStressSuite, ParserStressSuite, QueryStressSuite,
};

#[derive(Parser, Debug, Clone)]
#[command(name = "stress", about = "run destack stress tests")]
struct StressOptions {
    /// run only parser stress tests
    #[arg(long)]
    parser: bool,

    /// run only checker stress tests
    #[arg(long)]
    checker: bool,

    /// run only query stress tests
    #[arg(long)]
    query: bool,

    /// run only lsp stress tests
    #[arg(long)]
    lsp: bool,

    #[command(flatten)]
    test: RunOptions,
}

fn main() -> ExitCode {
    let options = StressOptions::parse();

    let any_specific = options.parser || options.checker || options.query || options.lsp;
    let run_parser = options.parser || !any_specific;
    let run_checker = options.checker || !any_specific;
    let run_query = options.query || !any_specific;
    let run_lsp = options.lsp || !any_specific;

    let mut any_failed = false;

    if run_parser {
        let result = Runner::run_suite(ParserStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_checker {
        let result = Runner::run_suite(CheckerStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_query {
        let result = Runner::run_suite(QueryStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_lsp {
        let result = Runner::run_suite(LspStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
