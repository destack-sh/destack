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
    #[arg(short = 'j', long, alias = "test-threads", default_value_t = num_cpus())]
    pub jobs: usize,

    /// Parse timeout in milliseconds for conformance tests.
    #[arg(long, default_value_t = 1000)]
    pub parse_timeout_ms: u64,

    /// Early timeout in milliseconds for conformance tests.
    #[arg(long, default_value_t = 3000)]
    pub early_timeout_ms: u64,

    /// Timeout in milliseconds for mdtest suites (specification and query).
    #[arg(long, default_value_t = 15000)]
    pub mdtest_timeout_ms: u64,

    /// Show verbose output including failure details inline.
    #[arg(short, long)]
    pub verbose: bool,

    /// List tests without running them.
    #[arg(long)]
    pub list: bool,

    /// Continue running after timeouts (default is to abort).
    #[arg(long)]
    pub continue_on_timeout: bool,

    /// Update known-failures lists for mdtest suites.
    #[arg(long)]
    pub update_known_failures: bool,

    /// Run tests even when they are listed in known-failures files.
    #[arg(long)]
    pub include_known_failures: bool,

    /// Run tests even when they are explicitly marked as ignored/skipped.
    #[arg(long)]
    pub include_ignored: bool,

    /// Run all skipped tests (equivalent to include known failures and ignored).
    #[arg(long)]
    pub include_skipped: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            filter: None,
            no_parallel: false,
            jobs: num_cpus(),
            parse_timeout_ms: 1000,
            early_timeout_ms: 3000,
            mdtest_timeout_ms: 15000,
            verbose: false,
            list: false,
            continue_on_timeout: false,
            update_known_failures: false,
            include_known_failures: false,
            include_ignored: false,
            include_skipped: false,
        }
    }
}

impl TestOptions {
    /// Whether to run tests in parallel.
    pub fn parallel(&self) -> bool {
        !self.no_parallel
    }

    /// Whether timeouts should abort the entire run.
    pub fn abort_on_timeout(&self) -> bool {
        !self.continue_on_timeout
    }

    /// Whether known failures should be executed instead of auto-skipped.
    pub fn include_known_failures_effective(&self) -> bool {
        self.include_skipped || self.include_known_failures || self.update_known_failures
    }

    /// Whether explicitly skipped cases should be executed.
    pub fn include_ignored_effective(&self) -> bool {
        self.include_skipped || self.include_ignored
    }

    /// Get the parse timeout as a Duration.
    pub fn parse_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.parse_timeout_ms.max(1))
    }

    /// Get the early timeout as a Duration.
    pub fn early_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.early_timeout_ms.max(1))
    }

    /// Get the mdtest timeout as a Duration.
    pub fn mdtest_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.mdtest_timeout_ms.max(1))
    }
}

/// Get number of CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
