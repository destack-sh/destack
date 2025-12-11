use std::process::ExitCode;

use clap::Parser;

use destack_test::conformance::{ConformanceHarnessSuite, ConformanceSelection};
use destack_test::harness::{Runner, TestOptions};

/// CLI options for the `conformance` test binary.
#[derive(Parser, Debug)]
#[command(name = "conformance", about = "Run parser conformance tests")]
struct Args {
    /// Update known-failures file with current failures.
    #[arg(long)]
    update_known_failures: bool,

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
    let selection = ConformanceSelection {
        test262: args.test262,
        babel: args.babel,
        swc: args.swc,
        biome: args.biome,
    };
    let suite = ConformanceHarnessSuite::new(selection, args.update_known_failures);
    Runner::run_suite(&suite, &args.test)
}
