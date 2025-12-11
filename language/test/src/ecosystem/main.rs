use std::process::ExitCode;

use clap::Parser;

use destack_test::ecosystem::{
    EcosystemRunOptions, FetchOptions, Tier, fetch_all_packages, run_ecosystem_tests,
};
use destack_test::harness::TestOptions;

/// CLI options for the `ecosystem` test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "ecosystem", about = "Run Destack ecosystem tests")]
struct EcosystemOptions {
    /// Fetch all packages before running tests.
    #[arg(long)]
    fetch: bool,

    /// Refresh packages during fetch (re-clone even if already present).
    #[arg(long)]
    refresh: bool,

    /// Only run parse tier.
    #[arg(long)]
    parse: bool,

    /// Only run analyze tier.
    #[arg(long)]
    analyze: bool,

    /// Override include patterns (glob, relative to package root).
    #[arg(long, value_name = "PATTERN")]
    include: Vec<String>,

    /// Add extra exclude patterns (glob, relative to package root).
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = EcosystemOptions::parse();

    // fetch mode
    if options.fetch {
        return fetch_all_packages(FetchOptions {
            refresh: options.refresh,
        });
    }

    // determine tier filter
    let tier = if options.parse {
        Some(Tier::Parse)
    } else if options.analyze {
        Some(Tier::Analyze)
    } else {
        None // run default (parse)
    };

    let run_options = EcosystemRunOptions {
        tier,
        include: options.include,
        exclude: options.exclude,
    };

    run_ecosystem_tests(&options.test, &run_options)
}
