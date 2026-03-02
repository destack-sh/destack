use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::parser::conformance::{
    ParserConformanceHarnessSuite, ParserConformanceSelection,
};

/// CLI options for the `parser-conformance` test binary.
#[derive(Parser, Debug)]
#[command(name = "parser-conformance", about = "Run parser conformance tests")]
struct Args {
    /// Filter tests inside a selected conformance suite.
    #[arg(long)]
    suite_filter: Option<String>,

    /// Run test262 suite.
    #[arg(long)]
    test262: bool,

    /// Run Babel parser suite.
    #[arg(long)]
    babel: bool,

    /// Run SWC parser suite.
    #[arg(long)]
    swc: bool,

    /// Run Biome parser suite.
    #[arg(long)]
    biome: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let selection = ParserConformanceSelection {
        test262: args.test262,
        babel: args.babel,
        swc: args.swc,
        biome: args.biome,
    };
    let suite = ParserConformanceHarnessSuite::new(
        selection,
        args.test.update_known_failures,
        args.suite_filter,
    );
    Runner::run_suite(&suite, &args.test)
}
