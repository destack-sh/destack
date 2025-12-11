use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::smoke::{CompilerSmokeSuite, ParserSmokeSuite};

/// CLI options for the `smoke` test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "smoke", about = "Run Destack smoke tests")]
struct SmokeOptions {
    /// Run only parser smoke tests.
    #[arg(long)]
    parser: bool,

    /// Run only compiler smoke tests.
    #[arg(long)]
    compiler: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = SmokeOptions::parse();

    // determine which tests to run
    // if neither flag is set, run both
    let run_parser = options.parser || !options.compiler;
    let run_compiler = options.compiler || !options.parser;

    // actually run the tests
    let mut any_failed = false;
    if run_parser {
        let result = Runner::run_suite(&ParserSmokeSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }
    if run_compiler {
        let result = Runner::run_suite(&CompilerSmokeSuite, &options.test);
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
