use clap::Parser;
use std::time::Duration;

#[path = "../benches/program/mod.rs"]
mod program;

/// Command line arguments.
#[derive(Parser)]
#[command(name = "bench_stats", about = "Quick VM interpreter benchmarks")]
struct Args {
    /// Run validation instead of benchmarks.
    #[arg(short, long)]
    validate: bool,

    /// Show detailed stats (single run, no timing).
    #[arg(short, long)]
    stats: bool,

    /// Run timing benchmarks explicitly.
    #[arg(long)]
    timing: bool,

    /// Use more samples for lower variance (repeat=7, min=500ms, warmup=200ms).
    #[arg(short, long)]
    precise: bool,

    /// Benchmark profile (quick, standard, stress).
    #[arg(long, value_enum, default_value_t = program::BenchProfileKind::Quick)]
    profile: program::BenchProfileKind,

    /// Output format for timing results (table, json, csv).
    #[arg(long, value_enum, default_value_t = program::BenchOutputFormat::Table)]
    output: program::BenchOutputFormat,

    /// Enable perf counters when supported.
    #[arg(long)]
    perf: bool,

    /// Collect instruction profile samples (requires stats feature).
    #[arg(long)]
    instruction_profile: bool,

    /// Disable calibration of the scale axis.
    #[arg(long)]
    no_calibrate: bool,

    /// Disable time budget scaling for quick runs.
    #[arg(long)]
    no_budget: bool,

    /// Use deterministic scaling with fixed sizes.
    #[arg(long)]
    deterministic: bool,

    /// Disable runtime checks and stats for faster benchmarking.
    #[arg(long)]
    fast: bool,

    /// Number of timing repeats.
    #[arg(long)]
    repeat: Option<u32>,

    /// Minimum duration per program in milliseconds.
    #[arg(long)]
    min_duration_ms: Option<u64>,

    /// Warmup time per program in milliseconds.
    #[arg(long)]
    warmup_ms: Option<u64>,

    /// Target duration per program invocation in milliseconds.
    #[arg(long)]
    target_ms: Option<u64>,

    /// Total time budget for the benchmark run in milliseconds.
    #[arg(long)]
    time_budget_ms: Option<u64>,

    /// Filter programs by tag.
    #[arg(long, value_delimiter = ',')]
    tag: Vec<String>,

    /// Filter programs by name (substring match).
    filter: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // collect filter patterns
    let filter = if args.filter.is_empty() {
        None
    } else {
        Some(args.filter)
    };
    let tags = if args.tag.is_empty() {
        None
    } else {
        Some(args.tag)
    };

    // select timing parameters
    let mut profile = args.profile.defaults();
    if args.precise {
        profile.repeat = 7;
        profile.min_duration = Duration::from_millis(500);
        profile.warmup = Duration::from_millis(200);
        profile.target_duration = Duration::from_millis(50);
    }
    if let Some(repeat) = args.repeat {
        profile.repeat = repeat;
    }
    if let Some(min_duration_ms) = args.min_duration_ms {
        profile.min_duration = Duration::from_millis(min_duration_ms);
    }
    if let Some(warmup_ms) = args.warmup_ms {
        profile.warmup = Duration::from_millis(warmup_ms);
    }
    if let Some(target_ms) = args.target_ms {
        profile.target_duration = Duration::from_millis(target_ms);
    }

    // derive time budget setting
    let default_budget = if args.profile == program::BenchProfileKind::Quick {
        Some(Duration::from_secs(20))
    } else {
        None
    };
    let time_budget = if args.no_budget {
        None
    } else if let Some(time_budget_ms) = args.time_budget_ms {
        Some(Duration::from_millis(time_budget_ms))
    } else {
        default_budget
    };

    // derive calibration settings
    let calibrate = !args.no_calibrate && !args.deterministic;

    // build benchmark options
    let options = program::BenchOptions::new(
        filter.clone(),
        tags.clone(),
        args.output,
        profile,
        calibrate,
        args.perf,
        args.instruction_profile,
        time_budget,
        args.deterministic,
        args.fast,
    );

    if args.instruction_profile {
        #[cfg(not(feature = "stats"))]
        {
            println!("instruction profiling requires the stats feature");
        }
    }

    // run validation mode
    if args.validate {
        program::quick_check(filter, tags);
    }
    // otherwise show stats
    else if args.stats {
        program::print_stats(&options);
    }
    // otherwise run benchmarks
    else if args.timing || (!args.validate && !args.stats) {
        program::quick_bench_with_options(&options);
    }
}
