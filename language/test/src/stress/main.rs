use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::stress::{CheckerStressSuite, ParserStressSuite, ResolverStressSuite};

#[derive(Parser, Debug, Clone)]
#[command(name = "stress", about = "run destack stress tests")]
struct StressOptions {
    /// run only parser stress tests
    #[arg(long)]
    parser: bool,

    /// run only resolver stress tests
    #[arg(long)]
    resolver: bool,

    /// run only checker stress tests
    #[arg(long)]
    checker: bool,

    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = StressOptions::parse();

    let any_specific = options.parser || options.resolver || options.checker;
    let run_parser = options.parser || !any_specific;
    let run_resolver = options.resolver || !any_specific;
    let run_checker = options.checker || !any_specific;

    let mut any_failed = false;

    if run_parser {
        let result = Runner::run_suite(&ParserStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_resolver {
        let result = Runner::run_suite(&ResolverStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_checker {
        let result = Runner::run_suite(&CheckerStressSuite, &options.test);
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
