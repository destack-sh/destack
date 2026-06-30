use std::path::PathBuf;
use std::time::Duration;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::StressFixture;
use super::target::StressTarget;
use super::worker::run_stress_worker;

/// One generated stress suite.
#[derive(Debug, Clone, Copy)]
pub(super) struct GeneratedStressSuite {
    /// The generated stress target.
    target: StressTarget,
    /// The target worker evaluator.
    run_fixture: fn(&StressFixture) -> CaseResult,
}

impl GeneratedStressSuite {
    /// Create one generated stress suite.
    pub(super) const fn new(
        target: StressTarget,
        run_fixture: fn(&StressFixture) -> CaseResult,
    ) -> Self {
        Self {
            target,
            run_fixture,
        }
    }

    /// Materialize stress fixtures and return the generated fixture count.
    pub(super) fn generate(self) -> Result<usize, String> {
        self.target
            .materialize_fixtures()
            .map(|fixtures| fixtures.len())
    }

    /// Run one stress fixture inside a worker process.
    pub(super) fn run_worker(self, path: PathBuf) -> CaseResult {
        let stress_fixture = match StressFixture::from_path(path) {
            Ok(stress_fixture) => stress_fixture,
            Err(message) => return CaseResult::Failed { message },
        };

        (self.run_fixture)(&stress_fixture)
    }
}

impl Suite for GeneratedStressSuite {
    fn name(&self) -> &'static str {
        self.target.suite_name()
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.target
            .materialize_fixtures()
            .unwrap_or_else(|error| {
                eprintln!("{error}");
                Vec::new()
            })
            .into_iter()
            .map(|fixture| Case::file(fixture.name, fixture.path, self.target.category()))
            .collect()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let stress_fixture = match StressFixture::from_path(case.path.clone()) {
            Ok(stress_fixture) => stress_fixture,
            Err(message) => return CaseResult::Failed { message },
        };

        run_stress_worker(self.target, &stress_fixture, self.target.worker_timeout())
    }

    fn timeout(&self) -> Option<Duration> {
        Some(self.target.harness_timeout())
    }
}
