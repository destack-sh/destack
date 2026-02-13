use std::process::ExitCode;

use clap::Parser;

use destack_test::ecosystem::{
    EcosystemPhase, EcosystemRunOptions, EcosystemTscMode, EcosystemTscTool, FetchOptions,
    fetch_all_packages, run_ecosystem_tests,
};
use destack_test::harness::TestOptions;

/// CLI options for the ecosystem test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "ecosystem", about = "Run Destack ecosystem tests")]
struct EcosystemOptions {
    /// Fetch all packages before running tests.
    #[arg(long)]
    fetch: bool,

    /// Refresh packages during fetch.
    #[arg(long)]
    refresh: bool,

    /// Install package dependencies after fetch.
    #[arg(long)]
    install: bool,

    /// Select one or more phases.
    #[arg(long, value_enum, value_name = "PHASE")]
    phase: Vec<EcosystemPhase>,

    /// Run all phases in order.
    #[arg(long)]
    all_phases: bool,

    /// Override include patterns.
    #[arg(long, value_name = "PATTERN")]
    include: Vec<String>,

    /// Add extra exclude patterns.
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Override max discovered files per package and phase.
    #[arg(long, value_name = "COUNT")]
    max_files: Option<usize>,

    /// Control TypeScript TSC execution strategy.
    #[arg(long, value_enum, default_value_t = EcosystemTscMode::OnFailure)]
    tsc_mode: EcosystemTscMode,

    /// Select TypeScript TSC binary strategy.
    #[arg(long, value_enum, default_value_t = EcosystemTscTool::Auto)]
    tsc_tool: EcosystemTscTool,

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
            install: options.install,
        });
    }

    let phases = collect_phases(&options);

    let run_options = EcosystemRunOptions {
        phases,
        include: options.include,
        exclude: options.exclude,
        max_files: options.max_files,
        tsc_mode: options.tsc_mode,
        tsc_tool: options.tsc_tool,
    };

    run_ecosystem_tests(&options.test, &run_options)
}

/// Collect selected phases from cli flags.
fn collect_phases(options: &EcosystemOptions) -> Vec<EcosystemPhase> {
    if options.all_phases {
        return EcosystemPhase::all().to_vec();
    }

    if options.phase.is_empty() {
        return Vec::new();
    }

    EcosystemPhase::all()
        .iter()
        .copied()
        .filter(|phase| options.phase.contains(phase))
        .collect()
}
