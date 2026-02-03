use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use destack_compiler::OptimizationLevel;

use destack_test::harness::{Runner, TestOptions};
use destack_test::optimize::{
    OptimizeBaselineSuite, OptimizeExecuteSuite, OptimizePerfSuite, OptimizeRunOptions,
    OptimizeValidateSuite, PerfOutputFormat,
};
use destack_test_mirbench::BenchProfileKind;

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

    /// Run perf suite over bench programs.
    #[arg(long)]
    perf: bool,

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

    /// Select the benchmark profile for execution.
    #[arg(long, default_value = "quick")]
    profile: String,

    /// Warmup iterations for perf runs.
    #[arg(long, default_value_t = 1)]
    perf_warmup: u32,

    /// Sample count for perf runs.
    #[arg(long, default_value_t = 3)]
    perf_samples: u32,

    /// Output path for perf reports.
    #[arg(long)]
    perf_out: Option<PathBuf>,

    /// Output format for perf reports.
    #[arg(long, default_value = "json")]
    perf_format: String,

    /// Enable A/B pipeline comparison for perf runs.
    #[arg(long)]
    perf_ab: bool,

    /// Comma-separated pass names to disable in the B pipeline.
    #[arg(long)]
    perf_ab_disable: Option<String>,

    /// Comma-separated pass names to keep in the B pipeline.
    #[arg(long)]
    perf_ab_only: Option<String>,

    /// Capture per-pass timing during perf runs.
    #[arg(long)]
    perf_pass_timing: bool,

    /// Named A/B preset for the B pipeline.
    #[arg(long)]
    perf_ab_preset: Option<String>,

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
        options.validate || (!options.execute && !options.baseline && !options.perf);
    let run_baseline = options.baseline;
    let run_execute = options.execute;
    let run_perf = options.perf;

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
    let perf_ab_preset = options.perf_ab_preset.as_deref();
    let mut perf_ab_disable = parse_pass_list(&options.perf_ab_disable);
    let perf_ab_only = parse_pass_list(&options.perf_ab_only);
    if perf_ab_preset.is_some() && (!perf_ab_disable.is_empty() || !perf_ab_only.is_empty()) {
        eprintln!("--perf-ab-preset cannot be combined with --perf-ab-disable/--perf-ab-only");
        return ExitCode::FAILURE;
    }
    if let Some(preset) = perf_ab_preset {
        match resolve_ab_preset(preset) {
            Some(disabled) => perf_ab_disable = disabled,
            None => {
                eprintln!(
                    "unknown perf ab preset '{preset}' (available: {})",
                    perf_ab_presets()
                );
                return ExitCode::FAILURE;
            }
        }
    }
    let perf_ab = options.perf_ab || perf_ab_preset.is_some();
    if perf_ab {
        if !perf_ab_disable.is_empty() && !perf_ab_only.is_empty() {
            eprintln!("--perf-ab-disable and --perf-ab-only are mutually exclusive");
            return ExitCode::FAILURE;
        }
        if perf_ab_disable.is_empty() && perf_ab_only.is_empty() {
            eprintln!("--perf-ab requires --perf-ab-disable, --perf-ab-only, or --perf-ab-preset");
            return ExitCode::FAILURE;
        }
    }

    let run_options = OptimizeRunOptions {
        level_filter,
        diagnostic: options.diagnostic,
        matrix: options.matrix,
        trace: options.trace,
        max_instruction_limit: options.max_instructions,
        bench_profile: match parse_bench_profile(&options.profile) {
            Some(profile) => profile,
            None => {
                eprintln!("unknown bench profile '{}'", options.profile);
                return ExitCode::FAILURE;
            }
        },
        perf_warmup: options.perf_warmup,
        perf_samples: options.perf_samples,
        perf_output: options.perf_out,
        perf_output_format: match parse_perf_format(&options.perf_format) {
            Some(format) => format,
            None => {
                eprintln!("unknown perf format '{}'", options.perf_format);
                return ExitCode::FAILURE;
            }
        },
        perf_ab,
        perf_ab_disable,
        perf_ab_only,
        perf_pass_timing: options.perf_pass_timing,
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
        let suite = OptimizeExecuteSuite::load(run_options.clone());
        let result = Runner::run_suite(&suite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }
    if run_perf {
        let suite = OptimizePerfSuite::load(run_options);
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

/// Parse a bench profile kind from a CLI value.
fn parse_bench_profile(value: &str) -> Option<BenchProfileKind> {
    match value {
        "quick" => Some(BenchProfileKind::Quick),
        "standard" => Some(BenchProfileKind::Standard),
        "stress" => Some(BenchProfileKind::Stress),
        _ => None,
    }
}

/// Parse a perf output format from a CLI value.
fn parse_perf_format(value: &str) -> Option<PerfOutputFormat> {
    match value {
        "json" => Some(PerfOutputFormat::Json),
        "csv" => Some(PerfOutputFormat::Csv),
        _ => None,
    }
}

/// Parse a comma-separated pass list.
fn parse_pass_list(value: &Option<String>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };

    value
        .split(',')
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

/// Return the available preset names.
fn perf_ab_presets() -> &'static str {
    "scalar,memory,loops,types,interproc,icombine,gvn,sccp,licm,inline"
}

/// Resolve a named preset into a disabled pass list.
fn resolve_ab_preset(name: &str) -> Option<Vec<String>> {
    match name {
        "scalar" => Some(preset_list(&[
            "ConstantFold",
            "InstructionCombine",
            "SimplifyCfg",
            "DeadCodeEliminate",
            "SparseConditionalConstantPropagation",
            "Reassociate",
            "CorrelatedValueProp",
            "ValueRangePropagation",
            "Narrow",
            "GuardEliminate",
            "PartialRedundancyElim",
            "GlobalValueNumbering",
            "CodeHoisting",
            "IfConvert",
            "LocalCse",
            "CopyPropagate",
        ])),
        "memory" => Some(preset_list(&[
            "Sroa",
            "Mem2Reg",
            "LoadPre",
            "StorePre",
            "LoadStoreForward",
            "MemCse",
            "StoreSink",
            "DeadStoreEliminate",
        ])),
        "loops" => Some(preset_list(&[
            "LoopSimplify",
            "LoopRotate",
            "LoopPeel",
            "InductionVariableSimplify",
            "LoopStrengthReduce",
            "LoopInterchange",
            "LoopDistribute",
            "LoopVersioning",
            "LoopIdiomRecognize",
            "Licm",
            "LoopUnswitch",
            "LoopUnroll",
            "LoopUnrollAndJam",
            "LoopDelete",
            "LoopFusion",
        ])),
        "types" => Some(preset_list(&[
            "LoopBoundsCheckEliminate",
            "BoundsCheckEliminate",
        ])),
        "interproc" => Some(preset_list(&[
            "FunctionAttrs",
            "InterproceduralConstantPropagation",
            "InterproceduralSccp",
            "ArgumentSpecialize",
            "DeadArgEliminate",
            "Inline",
            "GlobalOpt",
            "InterproceduralDceCleanup",
        ])),
        "icombine" => Some(preset_list(&["InstructionCombine"])),
        "gvn" => Some(preset_list(&["GlobalValueNumbering"])),
        "sccp" => Some(preset_list(&["SparseConditionalConstantPropagation"])),
        "licm" => Some(preset_list(&["Licm"])),
        "inline" => Some(preset_list(&["Inline"])),
        _ => None,
    }
}

/// Build a pass list from static names.
fn preset_list(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}
