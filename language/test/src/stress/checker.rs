use std::sync::Arc;
use std::time::Duration;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{FileSystem, PhysicalFileSystem};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, current_workspace_revision,
    default_profile_id_for_module, discover_file_cases, fixtures_dir, module_id_for_path,
    open_repository_with_options, provide_workspace_artifacts,
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
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
    let repository = open_repository_with_options(
        cwd.clone(),
        file_system,
        Default::default(),
        Default::default(),
    );
    let program = repository.clone();
    let compiler = Arc::new(Compiler::new(repository.clone()));

    // resolve module
    let revision = current_workspace_revision(&repository);
    let module_id = module_id_for_path(&repository, revision, &test.path);

    // check schedules import, bind, and export automatically
    let profile = default_profile_id_for_module(&program, revision, module_id);
    let artifact_keys = vec![ArtifactKey::DirChecked {
        module: module_id,
        profile,
    }];
    let _revision =
        provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys);

    let elapsed = start.elapsed();
    eprintln!("  {file_size} bytes, {line_count} lines, checked in {elapsed:?}");
    CaseResult::Passed
}
