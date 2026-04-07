use std::sync::Arc;
use std::time::Duration;

use destack_artifact::ArtifactKey;
use destack_compiler::{Compiler, CompilerOptions};
use destack_workspace::Repository;

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, current_workspace_revision,
    discover_file_cases, fixtures_dir, remember_default_profile_for_module,
};

/// Stress test suite for the type checker.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckerStressSuite;

impl Suite for CheckerStressSuite {
    fn name(&self) -> &'static str {
        "stress-checker"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let stress_dir = fixtures_dir().join("stress").join("checker");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        let extensions = &["ds"];
        discover_file_cases(&stress_dir, extensions, "destack_test::stress::checker")
            .unwrap_or_default()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_checker_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Run checker stress test.
fn run_checker_stress(test: &Case) -> CaseResult {
    let start = std::time::Instant::now();

    // read file content for stats
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(error) => {
            return CaseResult::Failed {
                message: format!("failed to read: {error}"),
            };
        }
    };
    let file_size = content.len();
    let line_count = content.lines().count();

    // set up compiler
    let cwd = test.path.parent().unwrap().to_path_buf();
    let repository = Arc::new(Repository::open_root(cwd.clone()));
    let program = repository.clone();
    let compiler = Arc::new(Compiler::new(
        repository.clone(),
        CompilerOptions::default(),
    ));

    // resolve module
    let revision = current_workspace_revision(&repository);
    let module_id = match compiler.resolve_path_to_module(revision, &test.path) {
        Ok(module_id) => module_id,
        Err(error) => {
            return CaseResult::Failed {
                message: format!("failed to resolve module: {error:?}"),
            };
        }
    };

    // analyze schedules import + bind + resolve automatically
    let profile = remember_default_profile_for_module(&program, &compiler, revision, module_id);
    compiler.enqueue(
        revision,
        ArtifactKey::DirAnalyzed {
            module: module_id,
            profile,
        },
    );
    compiler.compile();

    let elapsed = start.elapsed();
    eprintln!("  {file_size} bytes, {line_count} lines, checked in {elapsed:?}");
    CaseResult::Passed
}
