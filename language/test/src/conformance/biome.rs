//! Biome parser conformance tests.

use std::path::{Path, PathBuf};

use super::parse::{ParseOptions, ParseOutcome, file_type_from_path, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, TestOutcome, run_conformance_suite};
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

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    // only include source files, not snapshots
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        let name = path.file_name().unwrap().to_string_lossy();
                        // skip .d.ts files
                        if name.ends_with(".d.ts") {
                            continue;
                        }
                        tests.push(format!("{prefix}/{name}"));
                    }
                }
            }
        }

        tests.sort();
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

    fn discover_tests(&self) -> Vec<String> {
        let mut tests = Vec::new();

        // Biome has ok/ (should pass) and error/ (should fail) directories
        let ok_dir = self.root.join("ok");
        if ok_dir.exists() {
            tests.extend(self.discover_in_dir(&ok_dir, "ok"));
        }

        let error_dir = self.root.join("error");
        if error_dir.exists() {
            tests.extend(self.discover_in_dir(&error_dir, "error"));
        }

        tests
    }

    fn run_test(&self, name: &str) -> TestOutcome {
        let path = self.root.join(name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let file_type = file_type_from_path(&path);
        let parse_outcome = parse_file(&path, &content, file_type, ParseOptions::default());

        // ok/ should pass, error/ should fail
        let should_fail = name.starts_with("error/");

        match (should_fail, parse_outcome) {
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
