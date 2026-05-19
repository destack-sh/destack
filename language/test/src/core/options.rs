use clap::Parser;

/// Options for running tests.
#[derive(Parser, Debug, Clone)]
#[command(name = "test", about = "Run Destack tests")]
pub struct RunOptions {
    /// Filter tests by name with substring matching.
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,

    /// Run tests sequentially.
    #[arg(long)]
    pub sequential: bool,

    /// Number of parallel jobs.
    #[arg(short = 'j', long, alias = "test-threads", default_value_t = num_cpus())]
    pub jobs: usize,

    /// Parse timeout in milliseconds for conformance tests.
    #[arg(long, default_value_t = 1000)]
    pub parse_timeout_ms: u64,

    /// Timeout in milliseconds for conformance error cases.
    #[arg(long, default_value_t = 3000)]
    pub error_timeout_ms: u64,

    /// Timeout in milliseconds for one first party suite case.
    #[arg(long, default_value_t = 15000)]
    pub case_timeout_ms: u64,

    /// Show verbose output including inline failure details.
    #[arg(short, long)]
    pub verbose: bool,

    /// List tests without running them.
    #[arg(long)]
    pub list: bool,

    /// Continue running after timeouts.
    #[arg(long)]
    pub continue_after_timeout: bool,

    /// Update known-failures lists for mdtest suites.
    #[arg(long)]
    pub update_known_failures: bool,

    /// Update exact output snapshots in place.
    #[arg(long)]
    pub update_snapshots: bool,

    /// Run tests even when they are listed in known failure files.
    #[arg(long)]
    pub run_known_failures: bool,

    /// Run all skipped tests.
    #[arg(long)]
    pub run_skipped: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            filter: None,
            sequential: false,
            jobs: num_cpus(),
            parse_timeout_ms: 1000,
            error_timeout_ms: 3000,
            case_timeout_ms: 15000,
            verbose: false,
            list: false,
            continue_after_timeout: false,
            update_known_failures: false,
            update_snapshots: false,
            run_known_failures: false,
            run_skipped: false,
        }
    }
}

impl RunOptions {
    /// Return whether the run should use parallel execution.
    pub fn runs_in_parallel(&self) -> bool {
        !self.sequential
    }

    /// Whether timeouts should abort the entire run.
    pub fn aborts_on_timeout(&self) -> bool {
        !self.continue_after_timeout
    }

    /// Whether known failures should be executed instead of auto-skipped.
    pub fn runs_known_failures(&self) -> bool {
        self.run_skipped || self.run_known_failures || self.update_known_failures
    }

    /// Whether skipped cases should be executed.
    pub fn runs_skipped(&self) -> bool {
        self.run_skipped
    }

    /// Get the parse timeout as a Duration.
    pub fn parse_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.parse_timeout_ms.max(1))
    }

    /// Get the error case timeout as a Duration.
    pub fn error_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.error_timeout_ms.max(1))
    }

    /// Get the suite case timeout as a Duration.
    pub fn case_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.case_timeout_ms.max(1))
    }
}

/// Get number of CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
