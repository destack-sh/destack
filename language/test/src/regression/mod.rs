use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, discover_test_directories, fixtures_dir,
};

/// The regression suite.
#[derive(Debug, Clone, Copy, Default)]
pub struct RegressionSuite;

impl Suite for RegressionSuite {
    fn name(&self) -> &'static str {
        "regression"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let regression_directory = fixtures_dir().join("regression");
        discover_test_directories(&regression_directory, "destack_test::regression")
            .expect("failed to discover regression tests")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        TestResult::Failed {
            message: format!(
                "regression case '{}' is not runnable yet: dedicated regression harness work is still pending",
                case.full_name()
            ),
        }
    }
}
