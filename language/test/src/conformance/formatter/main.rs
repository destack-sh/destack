use std::process::ExitCode;

use clap::Parser;

use destack_test::conformance::formatter::{
    FormatterConformanceSelection, FormatterConformanceSuite,
};
use destack_test::core::{RunOptions, Runner};

/// CLI options for the `conformance-formatter` test binary.
#[derive(Parser, Debug)]
#[command(
    name = "conformance-formatter",
    about = "Run formatter conformance tests"
)]
struct Args {
    /// Filter tests inside a selected conformance suite.
    #[arg(long)]
    suite_filter: Option<String>,

    /// Run oxfmt formatter suite.
    #[arg(long)]
    oxfmt: bool,

    /// Common test options.
    #[command(flatten)]
    test: RunOptions,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let mut test_options = args.test;
    test_options.continue_after_timeout = true;
    let selection = FormatterConformanceSelection { oxfmt: args.oxfmt };
    let suite = FormatterConformanceSuite::new(
        selection,
        test_options.update_known_failures,
        args.suite_filter,
    );
    Runner::run_suite(suite, &test_options)
}
