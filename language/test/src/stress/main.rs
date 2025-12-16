use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::stress::{
    EdgeCasesStressSuite, LargeFilesStressSuite, LargeProjectsStressSuite, MemoryStressSuite,
    PathologicalStressSuite,
};

/// CLI options for the `stress` test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "stress", about = "Run Destack stress tests")]
struct StressOptions {
    /// Run only large file stress tests.
    #[arg(long)]
    large_files: bool,

    /// Run only large project stress tests.
    #[arg(long)]
    large_projects: bool,

    /// Run only memory stress tests.
    #[arg(long)]
    memory: bool,

    /// Run only edge case stress tests.
    #[arg(long)]
    edge_cases: bool,

    /// Run only pathological stress tests.
    #[arg(long)]
    pathological: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let options = StressOptions::parse();

    // determine which tests to run (if no flags, run all)
    let any_specific = options.large_files
        || options.large_projects
        || options.memory
        || options.edge_cases
        || options.pathological;
    let run_large_files = options.large_files || !any_specific;
    let run_large_projects = options.large_projects || !any_specific;
    let run_memory = options.memory || !any_specific;
    let run_edge_cases = options.edge_cases || !any_specific;
    let run_pathological = options.pathological || !any_specific;

    let mut any_failed = false;

    if run_large_files {
        let result = Runner::run_suite(&LargeFilesStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_large_projects {
        let result = Runner::run_suite(&LargeProjectsStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_memory {
        let result = Runner::run_suite(&MemoryStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_edge_cases {
        let result = Runner::run_suite(&EdgeCasesStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_pathological {
        let result = Runner::run_suite(&PathologicalStressSuite, &options.test);
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
