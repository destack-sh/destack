use std::path::{Path, PathBuf};

use super::parse::parse_file;
use crate::conformance::{
    Case, CaseOutcome, ConformanceDriver, ConformanceSuiteResult, run_conformance_driver,
    suite_fixtures_dir, suite_tests_dir,
};
use crate::core::RunOptions;

// pinned version of Biome parser tests
const BIOME_VERSION: &str = "1.x";
const BIOME_COMMIT: &str = "9f1b3b0"; // short sha

/// Biome parser conformance suite.
#[derive(Debug, Clone)]
pub struct BiomeSuite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

impl BiomeSuite {
    /// Create one Biome conformance suite.
    pub fn new() -> Self {
        let suite_dir = suite_fixtures_dir("biome");
        let tests_dir = suite_tests_dir("biome");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    // only include source files, not snapshots
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        let file_name = path.file_name().unwrap().to_string_lossy();
                        // skip .d.ts files
                        if file_name.ends_with(".d.ts") {
                            continue;
                        }
                        let name = format!("{prefix}/{file_name}");
                        tests.push(Case::valid(name));
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }
}

impl Default for BiomeSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceDriver for BiomeSuite {
    fn name(&self) -> &str {
        "biome"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn allows_undiscovered_status(&self, case_name: &str) -> bool {
        case_name.starts_with("error/")
    }

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();

        // Biome has ok/ (should pass) and error/ (should fail) directories
        let ok_dir = self.tests_dir.join("ok");
        if ok_dir.exists() {
            tests.extend(self.discover_in_dir(&ok_dir, "ok"));
        }

        tests
    }

    fn run(&self, test: &Case, show_diff: bool) -> CaseOutcome {
        let path = self.tests_dir.join(&test.name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return CaseOutcome::FailedRead,
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, show_diff);

        parse_outcome.case_outcome(test.source_validity)
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To refresh Biome parser tests (version {BIOME_VERSION}, commit {BIOME_COMMIT}):\n\
             \n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/biome\n"
        )
    }
}

/// Run Biome conformance tests.
pub fn run_biome(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = BiomeSuite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
