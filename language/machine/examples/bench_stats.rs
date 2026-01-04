use clap::Parser;

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

    /// Filter programs by name (substring match).
    filter: Vec<String>,
}

fn main() {
    let args = Args::parse();

    if args.validate {
        program::quick_check();
    } else if args.stats {
        program::print_stats();
    } else {
        let filter: Vec<&str> = args.filter.iter().map(|s| s.as_str()).collect();
        let filter = if filter.is_empty() {
            None
        } else {
            Some(&filter[..])
        };
        program::quick_bench(filter);
    }
}
