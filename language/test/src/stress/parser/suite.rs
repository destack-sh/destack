use std::path::PathBuf;
use std::time::Duration;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::runner::run_parser_stress;
use crate::stress::suite::GeneratedStressSuite;
use crate::stress::target::StressTarget;

/// Stress test suite for the parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserStressSuite;

impl ParserStressSuite {
    /// Return the generated parser stress suite.
    fn suite() -> GeneratedStressSuite {
        GeneratedStressSuite::new(StressTarget::Parser, run_parser_stress)
    }

    /// Materialize parser stress fixtures and return the generated fixture count.
    pub fn generate() -> Result<usize, String> {
        Self::suite().generate()
    }

    /// Run one parser stress fixture inside a worker process.
    pub fn run_worker(path: PathBuf) -> CaseResult {
        Self::suite().run_worker(path)
    }
}

impl Suite for ParserStressSuite {
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
