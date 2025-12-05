//! Command-line options for test runners.

use clap::Parser;

/// Options for running tests.
#[derive(Parser, Debug, Clone)]
#[command(name = "test", about = "Run Destack tests")]
pub struct TestOptions {
    /// Filter tests by name (substring match).
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,

    /// Run tests sequentially (default is parallel).
    #[arg(long)]
    pub no_parallel: bool,

    /// Number of parallel jobs.
    #[arg(short = 'j', long, default_value_t = num_cpus())]
    pub jobs: usize,

    /// Show verbose output including failure details inline.
    #[arg(short, long)]
    pub verbose: bool,

    /// List tests without running them.
    #[arg(long)]
    pub list: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            filter: None,
            no_parallel: false,
            jobs: num_cpus(),
            verbose: false,
            list: false,
        }
    }
}

impl TestOptions {
    /// Whether to run tests in parallel.
    pub fn parallel(&self) -> bool {
        !self.no_parallel
    }
}

/// Get number of CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

