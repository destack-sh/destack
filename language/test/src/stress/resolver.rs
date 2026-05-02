use std::sync::Arc;
use std::time::Duration;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{FileSystem, PhysicalFileSystem};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, current_workspace_revision,
    default_profile_id_for_module, fixtures_dir, module_id_for_path, open_repository_with_options,
    provide_workspace_artifacts,
};

/// Stress test suite for the resolver.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolverStressSuite;

impl Suite for ResolverStressSuite {
    fn name(&self) -> &'static str {
        "stress-resolver"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let stress_dir = fixtures_dir().join("stress").join("resolver");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        discover_project_entry_points(&stress_dir, "destack_test::stress::resolver")
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_resolver_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Discover project entry points under one stress directory.
fn discover_project_entry_points(dir: &std::path::Path, category: &str) -> Vec<Case> {
    let mut cases = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let index = path.join("index.ds");
            if !index.exists() {
                continue;
            }

            let name = path.file_name().unwrap().to_string_lossy().to_string();
            cases.push(Case::file(name, index, category));
        }
    }

    cases
}

/// Run resolver stress test.
fn run_resolver_stress(test: &Case) -> CaseResult {
    let project_dir = test.path.parent().unwrap();
    let start = std::time::Instant::now();

    // count files in project
    let file_count = std::fs::read_dir(project_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ds"))
                .count()
        })
        .unwrap_or(0);

    // set up compiler with physical file system access to the project
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
    let repository = open_repository_with_options(
        project_dir.to_path_buf(),
        file_system,
        Default::default(),
        Default::default(),
    );
    let program = repository.clone();
    let compiler = Arc::new(Compiler::new(repository.clone()));

    // resolve entry module
    let revision = current_workspace_revision(&repository);
    let module_id = module_id_for_path(&repository, revision, &test.path);

    // export schedules import and bind automatically
    let revision = current_workspace_revision(&repository);
    let profile = default_profile_id_for_module(&program, revision, module_id);
    let artifact_keys = vec![ArtifactKey::DirExported {
        module: module_id,
        profile,
    }];
    let _revision =
        provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys);

    let elapsed = start.elapsed();
    eprintln!("  {file_count} files, resolved in {elapsed:?}");
    CaseResult::Passed
}
