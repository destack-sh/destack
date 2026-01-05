use clap::Parser;
use std::time::Duration;

#[path = "../benches/program/mod.rs"]
mod program;

/// Command line arguments.
#[derive(Parser)]
#[command(name = "bench_stats", about = "Quick machine interpreter benchmarks")]
struct Args {
    /// Run validation instead of benchmarks.
    #[arg(short, long)]
    validate: bool,

    /// Show detailed stats (single run, no timing).
    #[arg(short, long)]
    stats: bool,

    /// Number of timing repeats.
    #[arg(long, default_value_t = 3)]
    repeat: u32,

    /// Minimum duration per program in milliseconds.
    #[arg(long, default_value_t = 100)]
    min_duration_ms: u64,

    /// Warmup time per program in milliseconds.
    #[arg(long, default_value_t = 50)]
    warmup_ms: u64,

    /// Filter programs by name (substring match).
    filter: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // run validation mode
    if args.validate {
        program::quick_check();
    }
    // otherwise show stats
    else if args.stats {
        program::print_stats();
    }
    // otherwise run benchmarks
    else {
        // collect filter patterns
        let filter = if args.filter.is_empty() {
            None
        } else {
            Some(args.filter)
        };

        // build benchmark options
        let options = program::BenchOptions::new(
            filter,
            args.repeat,
            Duration::from_millis(args.min_duration_ms),
            Duration::from_millis(args.warmup_ms),
        );

        // run benchmarks
        program::quick_bench_with_options(&options);
    }
}
