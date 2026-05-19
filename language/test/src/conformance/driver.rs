use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::conformance::{ConformanceCapability, ConformanceEnvironment, status_json_path_for_dir};
use crate::core::RunOptions;

use super::{Case, CaseOutcome};

/// A shared conformance suite driver.
pub trait ConformanceDriver: Send + Sync + Clone {
    /// Return the suite display name.
    fn name(&self) -> &str;

    /// Return the suite metadata directory.
    fn suite_dir(&self) -> &Path;

    /// Return the suite tests directory.
    fn tests_dir(&self) -> &Path;

    /// Return the path to the structured status file.
    fn status_path(&self) -> PathBuf {
        status_json_path_for_dir(self.suite_dir())
    }

    /// Return the suite capability tag.
    fn capability(&self) -> ConformanceCapability {
        ConformanceCapability::Parse
    }

    /// Return the suite environment tag.
    fn environment(&self) -> ConformanceEnvironment {
        ConformanceEnvironment::Hostless
    }

    /// Return whether skipped cases should fail loudly if they pass.
    fn skipped_failures_are_strict(&self) -> bool {
        false
    }

    /// Discover all runnable cases in this suite.
    fn discover_cases(&self) -> Vec<Case>;

    /// Run one case and return the outcome.
    fn run(&self, case: &Case, show_diff: bool) -> CaseOutcome;

    /// Return instructions for fetching this suite.
    fn fetch_instructions(&self) -> String;

    /// Extract one reporting category from one case name.
    fn category_for_case(&self, case_name: &str) -> String {
        case_name.split('/').next().unwrap_or("unknown").to_string()
    }

    /// Select the timeout for one case.
    fn timeout_for_case(&self, case: &Case, options: &RunOptions) -> Duration {
        let scale = if options.runs_in_parallel() {
            options.jobs.max(1) as u64
        } else {
            1
        };

        let base_ms = if case.expect_error {
            options.error_timeout_ms
        } else {
            options.parse_timeout_ms
        };

        Duration::from_millis(base_ms.saturating_mul(scale).max(1))
    }
}

/// Build one category key from the first two path segments of one case name.
pub fn category_key_for_case_name(case_name: &str) -> String {
    let mut segments = case_name.split('/').filter(|segment| !segment.is_empty());

    let Some(first) = segments.next() else {
        return "unknown".to_string();
    };

    let Some(second) = segments.next() else {
        return first.to_string();
    };

    format!("{first}/{second}")
}

#[cfg(test)]
mod tests {
    use super::category_key_for_case_name;

    #[test]
    fn test_category_key_for_case_name_uses_first_two_segments() {
        let category = category_key_for_case_name("typescript/range/issue.ts");
        assert_eq!(category, "typescript/range");
    }

    #[test]
    fn test_category_key_for_case_name_handles_single_segment_paths() {
        let category = category_key_for_case_name("typescript");
        assert_eq!(category, "typescript");
    }

    #[test]
    fn test_category_key_for_case_name_handles_empty_input() {
        let category = category_key_for_case_name("");
        assert_eq!(category, "unknown");
    }
}
