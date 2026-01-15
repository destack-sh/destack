use std::process::ExitCode;

use clap::Parser;

use destack_compiler::OptimizationLevel;

use destack_test::harness::{Runner, TestOptions};
use destack_test::optimize::{
    OptimizeBaselineSuite, OptimizeExecuteSuite, OptimizeRunOptions, OptimizeValidateSuite,
};

/// CLI options for the `optimize` test binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "optimize", about = "Run optimizer bench tests")]
struct OptimizeOptions {
    /// Run validation suite over MIR fixtures.
    #[arg(long)]
    validate: bool,

    /// Run baseline suite over MIR bench programs without optimization.
    #[arg(long)]
    baseline: bool,

    /// Run execution suite over bench programs.
    #[arg(long)]
    execute: bool,

    /// Enable diagnostic mismatch isolation.
    #[arg(long)]
    diagnostic: bool,

    /// Enable matrix pass isolation.
    #[arg(long)]
    matrix: bool,

    /// Emit trace output for each case.
    #[arg(long)]
    trace: bool,

    /// Restrict to a single optimization level.
    #[arg(long)]
    level: Option<String>,

    /// Set the maximum instruction count for execution.
    #[arg(long)]
    max_instructions: Option<u64>,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

/// Run optimizer bench suites.
fn main() -> ExitCode {
    let options = OptimizeOptions::parse();

    // determine which suites to run
    let run_validate =
        options.validate || (!options.validate && !options.execute && !options.baseline);
    let run_baseline = options.baseline;
    let run_execute = options.execute;

    // parse the optional level filter
    let level_filter = match options.level.as_deref() {
        Some(value) => match parse_optimization_level(value) {
            Some(level) => Some(level),
            None => {
                eprintln!("unknown optimization level '{value}'");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    // build shared run options
    let run_options = OptimizeRunOptions {
        level_filter,
        diagnostic: options.diagnostic,
        matrix: options.matrix,
        trace: options.trace,
        max_instruction_limit: options.max_instructions,
    };

    // execute requested suites
    let mut any_failed = false;
    if run_validate {
        let suite = OptimizeValidateSuite::load(run_options.clone());
        let result = Runner::run_suite(&suite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }
    if run_baseline {
        let suite = OptimizeBaselineSuite::load(run_options.clone());
        let result = Runner::run_suite(&suite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }
    if run_execute {
        let suite = OptimizeExecuteSuite::load(run_options);
        let result = Runner::run_suite(&suite, &options.test);
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

/// Parse an optimization level from a CLI value.
fn parse_optimization_level(value: &str) -> Option<OptimizationLevel> {
    // match supported optimization levels
    match value {
        "O0" => Some(OptimizationLevel::O0),
        "O1" => Some(OptimizationLevel::O1),
        "O2" => Some(OptimizationLevel::O2),
        "O3" => Some(OptimizationLevel::O3),
        "O4" => Some(OptimizationLevel::O4),
        _ => None,
    }
}
