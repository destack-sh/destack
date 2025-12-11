//! Ecosystem test runner.
//!
//! Usage:
//!   cargo test --test ecosystem              # run all ecosystem tests
//!   cargo test --test ecosystem -- ms        # run specific package
//!   cargo test --test ecosystem -- --fetch   # fetch packages
//!   cargo test --test ecosystem -- --list    # list packages

use std::process::ExitCode;

use clap::Parser;

use destack_test::ecosystem::{run_ecosystem_tests, Tier};
use destack_test::ecosystem::runner::fetch_all_packages;
use destack_test::harness::TestOptions;

#[derive(Parser, Debug, Clone)]
#[command(name = "ecosystem", about = "Run Destack ecosystem tests")]
struct EcosystemOptions {
    /// Fetch all packages before running tests.
    #[arg(long)]
    fetch: bool,

    /// Only run parse tier.
    #[arg(long)]
    parse: bool,

    /// Only run analyze tier.
    #[arg(long)]
    analyze: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = EcosystemOptions::parse();

    // fetch mode
    if options.fetch {
        return fetch_all_packages();
    }

    // determine tier filter
    let tier = if options.parse {
        Some(Tier::Parse)
    } else if options.analyze {
        Some(Tier::Analyze)
    } else {
        None // run default (parse)
    };

    run_ecosystem_tests(&options.test, tier)
}
