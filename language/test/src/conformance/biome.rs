use std::path::{Path, PathBuf};

use super::parse::{ParseOptions, ParseOutcome, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, Test, TestOutcome, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of Biome parser tests
const BIOME_VERSION: &str = "1.x";
const BIOME_COMMIT: &str = "9f1b3b0"; // short sha

/// Biome parser conformance suite.
#[derive(Debug, Clone)]
pub struct BiomeSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl BiomeSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("biome");
        Self {
            root,
            conformance_dir,
        }
    }

    /// Tests to skip (cause stack overflow due to deep recursion).
    fn should_skip_test(name: &str) -> bool {
        // nocheckin #Broken: fix stack overflow from IR-driven recursion (probably both parse & compile)
        // many_empty_strings.js has 2500+ nested binary expressions that overflow the stack
        name == "ok/many_empty_strings.js"
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str, expect_error: bool) -> Vec<Test> {
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
                        if Self::should_skip_test(&name) {
                            continue;
                        }
                        let file_type = Test::file_type_from_name(&name);
                        tests.push(Test {
                            name,
                            file_type,
                            expect_error,
                        });
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

impl ConformanceSuite for BiomeSuite {
    fn name(&self) -> &str {
        "biome"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("biome-known-failures.txt")
    }

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();

        // Biome has ok/ (should pass) and error/ (should fail) directories
        let ok_dir = self.root.join("ok");
        if ok_dir.exists() {
            tests.extend(self.discover_in_dir(&ok_dir, "ok", false));
        }

        let error_dir = self.root.join("error");
        if error_dir.exists() {
            tests.extend(self.discover_in_dir(&error_dir, "error", true));
        }

        tests
    }

    fn run(&self, test: &Test) -> TestOutcome {
        let path = self.root.join(&test.name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, ParseOptions::default());

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => TestOutcome::Passed,
            (true, ParseOutcome::Ok) => TestOutcome::Failed,
            (false, ParseOutcome::Ok) => TestOutcome::Passed,
            (false, ParseOutcome::Error) => TestOutcome::Failed,
        }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download Biome parser tests (version {BIOME_VERSION}, commit {BIOME_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/biome-fetch.sh\n"
        )
    }
}

/// Run Biome conformance tests.
pub fn run_biome(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = BiomeSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
