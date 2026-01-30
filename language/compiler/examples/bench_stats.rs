use std::collections::HashSet;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

use destack_compiler::bench::{BenchMode, BenchOptions, BenchOutputFormat, BenchRun, run_bench};

/// Command line arguments.
#[derive(Parser)]
#[command(name = "bench_stats", about = "Compiler builtin lib benchmarks")]
struct Args {
    /// Execution mode for the benchmark run.
    #[arg(long, value_enum, default_value_t = Mode::Parallel, global = true)]
    mode: Mode,

    /// Suppress per lib timing output.
    #[arg(long, global = true)]
    no_timings: bool,

    /// Output format for bench results.
    #[arg(long, value_enum, default_value_t = Output::Table, global = true)]
    output: Output,

    /// Disable ANSI color output.
    #[arg(long, global = true)]
    no_color: bool,

    /// How many timing entries to include in summaries.
    #[arg(long, default_value_t = 15, global = true)]
    top: usize,

    /// Write timing summaries to a CSV file.
    #[arg(long, global = true)]
    csv: Option<String>,

    /// Override the per phase timeout in milliseconds.
    #[arg(long, global = true)]
    timeout_ms: Option<u64>,

    /// Enable builtin lib validation during analysis.
    #[arg(long, global = true)]
    validate: bool,

    /// Restrict the lib set (comma-delimited).
    #[arg(long, value_delimiter = ',', global = true)]
    libs: Vec<String>,

    /// The benchmark command to execute.
    #[command(subcommand)]
    command: Command,
}

/// Bench commands.
#[derive(Subcommand)]
enum Command {
    /// Resolve declared lib symbols for all builtin libs.
    ResolveSymbols,
    /// Resolve all builtin libs without analysis.
    ResolveAll,
    /// Analyze builtin libs with fast settings.
    AnalyzeFast,
    /// Analyze builtin libs with full validation.
    AnalyzeFull,
    /// Analyze a combined lib set in one program.
    AnalyzeCombined,
}

/// CLI execution modes.
#[derive(ValueEnum, Clone, Copy, Debug)]
enum Mode {
    /// Run tasks sequentially.
    Sequential,
    /// Run tasks in parallel.
    Parallel,
    /// Run sequential and parallel modes.
    All,
}

/// CLI output formats.
#[derive(ValueEnum, Clone, Copy, Debug)]
enum Output {
    /// Human-readable table output.
    Table,
    /// Machine readable json output.
    Json,
    /// Machine readable csv output.
    Csv,
}

impl From<Output> for BenchOutputFormat {
    fn from(output: Output) -> Self {
        match output {
            Output::Table => BenchOutputFormat::Table,
            Output::Json => BenchOutputFormat::Json,
            Output::Csv => BenchOutputFormat::Csv,
        }
    }
}

impl From<Mode> for BenchMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Sequential => BenchMode::Sequential,
            Mode::Parallel => BenchMode::Parallel,
            Mode::All => BenchMode::Parallel,
        }
    }
}

fn main() {
    let args = Args::parse();
    let modes = match args.mode {
        Mode::Sequential => vec![BenchMode::Sequential],
        Mode::Parallel => vec![BenchMode::Parallel],
        Mode::All => vec![BenchMode::Sequential, BenchMode::Parallel],
    };

    // select the bench run
    let run = match args.command {
        Command::ResolveSymbols => BenchRun::ResolveSymbols,
        Command::ResolveAll => BenchRun::ResolveAll,
        Command::AnalyzeFast => BenchRun::AnalyzeFast,
        Command::AnalyzeFull => BenchRun::AnalyzeFull,
        Command::AnalyzeCombined => BenchRun::AnalyzeCombined,
    };

    let output = BenchOutputFormat::from(args.output);
    if matches!(args.mode, Mode::All) && !matches!(output, BenchOutputFormat::Table) {
        eprintln!("bench_stats: --mode all only supports table output");
        std::process::exit(2);
    }

    for (index, mode) in modes.iter().copied().enumerate() {
        // build bench options
        let mut options = BenchOptions::new(run, mode);
        if args.no_timings {
            options.report_timings = false;
        }
        options.output = output;
        if args.no_color {
            options.color = false;
        }
        options.report_top_n = args.top;
        if let Some(path) = args.csv.clone() {
            options.csv_path = Some(path);
        }
        if let Some(timeout_ms) = args.timeout_ms {
            options.timeout = Duration::from_millis(timeout_ms);
        }
        if args.validate {
            options.validate_builtin_libs = true;
        }

        // apply lib selection overrides
        let libs = args.libs.clone();
        if !libs.is_empty() {
            match run {
                BenchRun::AnalyzeCombined => {
                    options.combined_libs = Some(libs);
                }
                BenchRun::AnalyzeFast | BenchRun::AnalyzeFull => {
                    options.lib_filter = Some(libs.into_iter().collect::<HashSet<_>>());
                }
                BenchRun::ResolveSymbols | BenchRun::ResolveAll => {}
            }
        }

        if index > 0 {
            println!();
        }
        run_bench(&options);
    }
}
