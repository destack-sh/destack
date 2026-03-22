use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, discover_directory_cases, fixtures_dir,
};

/// The regression suite.
#[derive(Debug, Clone, Copy, Default)]
pub struct RegressionSuite;

impl Suite for RegressionSuite {
    fn name(&self) -> &'static str {
        "regression"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let regression_directory = fixtures_dir().join("regression");

        discover_directory_cases(&regression_directory, "destack_test::regression")
            .expect("failed to discover regression tests")
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        CaseResult::Failed {
            message: format!(
                "regression case '{}' is not runnable yet: dedicated regression harness work is still pending",
                case.full_name()
            ),
        }
    }
}
