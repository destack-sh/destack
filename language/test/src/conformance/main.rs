use std::process::ExitCode;

use clap::Parser;

use destack_test::conformance::{
    print_summary, run_babel as babel_suite, run_biome as biome_suite, run_swc as swc_suite,
    run_test262 as test262_suite, SuiteResult,
};
use destack_test::harness::TestOptions;

/// Conformance test specific options.
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

    // if no suite specified, run all suites
    let no_suite_specified = !args.test262 && !args.babel && !args.swc && !args.biome;

    let run_test262 = args.test262 || no_suite_specified;
    let run_babel = args.babel || no_suite_specified;
    let run_swc = args.swc || no_suite_specified;
    let run_biome = args.biome || no_suite_specified;

    let mut results: Vec<SuiteResult> = Vec::new();

    if run_test262 {
        if let Some(r) = test262_suite(&args.test, args.update_known_failures) {
            results.push(r);
        }
    }

    if run_babel {
        if let Some(r) = babel_suite(&args.test, args.update_known_failures) {
            results.push(r);
        }
    }

    if run_swc {
        if let Some(r) = swc_suite(&args.test, args.update_known_failures) {
            results.push(r);
        }
    }

    if run_biome {
        if let Some(r) = biome_suite(&args.test, args.update_known_failures) {
            results.push(r);
        }
    }

    // print summary if multiple suites ran
    print_summary(&results);

    // check for any regressions
    let any_regressions = results.iter().any(|r| r.result.has_regressions());

    if any_regressions {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
