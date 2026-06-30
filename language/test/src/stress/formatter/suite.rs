use std::path::PathBuf;
use std::time::Duration;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::runner::run_formatter_stress;
use crate::stress::suite::GeneratedStressSuite;
use crate::stress::target::StressTarget;

/// Stress test suite for the formatter.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterStressSuite;

impl FormatterStressSuite {
    /// Return the generated formatter stress suite.
    fn suite() -> GeneratedStressSuite {
        GeneratedStressSuite::new(StressTarget::Formatter, run_formatter_stress)
    }

    /// Materialize formatter stress fixtures and return the generated fixture count.
    pub fn generate() -> Result<usize, String> {
        Self::suite().generate()
    }

    /// Run one formatter stress fixture inside a worker process.
    pub fn run_worker(path: PathBuf) -> CaseResult {
        Self::suite().run_worker(path)
    }
}

impl Suite for FormatterStressSuite {
    fn name(&self) -> &'static str {
        Self::suite().name()
    }

    fn discover(&self, options: &RunOptions) -> Vec<Case> {
        Self::suite().discover(options)
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        Self::suite().run(case, context)
    }

    fn timeout(&self) -> Option<Duration> {
        Self::suite().timeout()
    }
}
