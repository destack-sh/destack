use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, MemoryFileSystem, PhysicalFileSystem, Uri, glob,
};
use destack_workspace::{LanguageOptions, Program};

use crate::harness::{RunContext, Runner, Suite, TestCase, TestOptions, TestResult, fixtures_dir};

use super::manifest::{EcosystemManifest, Tier};

const DEFAULT_INCLUDE_PATTERNS: &[&str] = &["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"];
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &["**/node_modules/**", "**/dist/**", "**/build/**"];

const MAX_ANALYZE_ENTRYPOINTS: usize = 50;

/// Configuration options for running ecosystem tests.
#[derive(Debug, Clone, Default)]
pub struct EcosystemRunOptions {
    /// The tier to run.
    pub tier: Option<Tier>,
    /// Override include patterns (if empty, use manifest defaults).
    pub include: Vec<String>,
    /// Additional exclude patterns.
    pub exclude: Vec<String>,
}

/// Configuration options for fetching ecosystem packages.
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchOptions {
    /// Whether to re-clone packages even if they already exist.
    pub refresh: bool,
}

/// A `Suite` implementation that runs ecosystem tests across a set of packages.
#[derive(Debug)]
pub struct EcosystemSuite {
    tier: Tier,
    include: Vec<String>,
    exclude: Vec<String>,
    manifests: Vec<EcosystemManifest>,
    manifests_by_name: HashMap<String, EcosystemManifest>,
    cache_dir: PathBuf,
    patches_dir: PathBuf,
}

impl EcosystemSuite {
    pub fn load(options: &EcosystemRunOptions) -> Self {
        let ecosystem_dir = fixtures_dir().join("ecosystem");
        let packages_dir = ecosystem_dir.join("packages");
        let cache_dir = ecosystem_dir.join("cache");
        let patches_dir = ecosystem_dir.join("patches");

        let tier = options.tier.unwrap_or(Tier::Parse);

        let mut manifests: Vec<EcosystemManifest> = Vec::new();
        let mut manifests_by_name = HashMap::new();

        for path in EcosystemManifest::discover_all(&packages_dir) {
            match EcosystemManifest::load(&path) {
                Ok(manifest) => {
                    manifests_by_name.insert(manifest.package.name.clone(), manifest.clone());
                    manifests.push(manifest);
                }
                Err(error) => {
                    eprintln!("warning: {error}");
                }
            }
        }

        Self {
            tier,
            include: options.include.clone(),
            exclude: options.exclude.clone(),
            manifests,
            manifests_by_name,
            cache_dir,
            patches_dir,
        }
    }

    fn run_package(&self, manifest: &EcosystemManifest) -> TestResult {
        let package_dir = self.cache_dir.join(&manifest.package.name);

        // check whether the package is present in the cache
        if !package_dir.exists() {
            return TestResult::Skipped {
                reason: "not fetched (run: ./language/test/fixtures/ecosystem/ecosystem-fetch.sh)"
                    .to_string(),
            };
        }

        // apply patches if they exist
        let package_patches_dir = self.patches_dir.join(&manifest.package.name);
        if package_patches_dir.exists()
            && let Err(error) = apply_patches(&package_patches_dir, &package_dir)
        {
            return TestResult::Failed {
                message: format!("failed to apply patches: {error}"),
            };
        }

        let files = discover_package_files(&package_dir, manifest, &self.include, &self.exclude);
        if files.is_empty() {
            return TestResult::Failed {
                message: "no files found matching include patterns".to_string(),
            };
        }

        let tier_result = match self.tier {
            Tier::Parse => run_parse_tier(&package_dir, &files),
            Tier::Analyze => run_analyze_tier(&package_dir, &files),
        };

        let expects_pass = manifest.tiers.expects_pass(self.tier);
        if expects_pass {
            return tier_result;
        }

        match tier_result {
            TestResult::Passed => TestResult::Passed,
            TestResult::Failed { message } => {
                let reason = manifest.tiers.get(self.tier).reason();
                let reason = reason
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| format!("{} tier not expected to pass", self.tier.name()));
                TestResult::Skipped {
                    reason: format!("expected failure: {reason}\n{message}"),
                }
            }
            TestResult::Skipped { reason } => TestResult::Skipped { reason },
            TestResult::Suite { .. } => tier_result,
        }
    }
}

impl Suite for EcosystemSuite {
    fn name(&self) -> &'static str {
        "ecosystem"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.manifests
            .iter()
            .map(|manifest| {
                TestCase::directory(
                    &manifest.package.name,
                    self.cache_dir.join(&manifest.package.name),
                    format!("destack_test::ecosystem::{}", self.tier.name()),
                )
            })
            .collect()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        let Some(manifest) = self.manifests_by_name.get(&case.name) else {
            return TestResult::Failed {
                message: "manifest not found".to_string(),
            };
        };

        self.run_package(manifest)
    }

    fn report(&self, results: &[(TestCase, TestResult)], _context: &RunContext<'_>) {
        let mut regressions: Vec<String> = Vec::new();
        let mut fixed: Vec<String> = Vec::new();

        for (case, result) in results {
            let Some(manifest) = self.manifests_by_name.get(&case.name) else {
                continue;
            };

            let expects_pass = manifest.tiers.expects_pass(self.tier);
            if expects_pass && result.is_failed() {
                regressions.push(case.name.clone());
            }
            if !expects_pass && result.is_passed() {
                fixed.push(case.name.clone());
            }
        }

        if regressions.is_empty() && fixed.is_empty() {
            return;
        }

        println!();
        println!("ecosystem: {}", self.tier.name());

        if !regressions.is_empty() {
            println!("regressions (expected to pass but failed):");
            for name in regressions {
                println!("  {name}");
            }
        }

        if !fixed.is_empty() {
            println!("fixed (expected to fail but passed):");
            for name in fixed {
                println!("  {name}");
            }
        }
        println!();
    }
}

pub fn run_ecosystem_tests(options: &TestOptions, run_options: &EcosystemRunOptions) -> ExitCode {
    let suite = EcosystemSuite::load(run_options);
    Runner::run_suite(&suite, options)
}

fn discover_package_files(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    include_override: &[String],
    exclude_override: &[String],
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let include_patterns: Vec<String> = if !include_override.is_empty() {
        include_override.to_vec()
    } else if manifest.discovery.include.is_empty() {
        DEFAULT_INCLUDE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        manifest.discovery.include.clone()
    };

    for pattern in &include_patterns {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        files.extend(matches);
    }

    let mut exclude_set = std::collections::HashSet::new();

    for pattern in &manifest.discovery.exclude {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    for pattern in exclude_override {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    for pattern in DEFAULT_EXCLUDE_PATTERNS {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    files.retain(|path| !exclude_set.contains(path));
    files.sort();
    files
}

fn run_parse_tier(package_dir: &Path, files: &[PathBuf]) -> TestResult {
    let mut failed = 0;
    let mut failure_messages: Vec<String> = Vec::new();

    for path in files {
        match parse_file(path) {
            Ok(()) => {}
            Err(error) => {
                failed += 1;
                let relative = path.strip_prefix(package_dir).unwrap_or(path);
                if failure_messages.len() < 10 {
                    failure_messages.push(format!("{}: {error}", relative.display()));
                }
            }
        }
    }

    if failed == 0 {
        return TestResult::Passed;
    }

    let message = if failure_messages.len() < failed {
        format!(
            "{failed} files failed to parse (showing first 10):\n  {}",
            failure_messages.join("\n  ")
        )
    } else {
        format!(
            "{failed} files failed to parse:\n  {}",
            failure_messages.join("\n  ")
        )
    };

    TestResult::Failed { message }
}

fn run_analyze_tier(package_dir: &Path, files: &[PathBuf]) -> TestResult {
    let files_registry = Arc::new(FileRegistry::new());
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let program = Arc::new(Program::new(
        LanguageOptions::default(),
        package_dir.to_path_buf(),
        file_system,
        files_registry,
    ));

    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );

    // N OTE: analyzing every file can be very expensive in large projects
    // we start with a bounded number of entrypoints to keep the suite practical
    for path in files.iter().take(MAX_ANALYZE_ENTRYPOINTS) {
        let module_id = match compiler.resolve_path_to_module(path) {
            Ok(id) => id,
            Err(error) => {
                return TestResult::Failed {
                    message: format!("failed to resolve module {}: {error:?}", path.display()),
                };
            }
        };
        compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
    }

    compiler.compile();
    drop(compiler);

    let diagnostics = program.diagnostics.collect();
    let errors: Vec<_> = diagnostics
        .iter()
        .into_iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .collect();
    if errors.is_empty() {
        return TestResult::Passed;
    }

    let mut messages: Vec<String> = Vec::new();
    for diag in errors.iter().take(10) {
        messages.push(diag.message.clone());
    }

    let message = if errors.len() > 10 {
        format!(
            "{} errors (showing first 10):\n  {}",
            errors.len(),
            messages.join("\n  ")
        )
    } else {
        format!("{} errors:\n  {}", errors.len(), messages.join("\n  "))
    };

    TestResult::Failed { message }
}

fn parse_file(path: &Path) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {e}"))?;

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let file_type = FileType::from_extension(extension).unwrap_or(FileType::TypeScript);
    let language_type = destack_source::LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);

    let uri = Uri::from_path(path);
    let files = Arc::new(FileRegistry::new());
    let file_system: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let program = Arc::new(Program::new(language, cwd, file_system, files));

    let file_id = program.files.next_id();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content,
    );
    program.files.insert(file);
    let file = program.files.get(file_id);

    let mut parser = Parser::lex_file(file, program.language.ty);
    let _ = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    let errors: Vec<_> = program
        .diagnostics
        .iter()
        .into_iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .collect();
    if errors.is_empty() {
        return Ok(());
    }

    Err(errors[0].message.clone())
}

fn apply_patches(patches_dir: &Path, package_dir: &Path) -> Result<(), String> {
    copy_dir_recursive(patches_dir, package_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path).map_err(|e| e.to_string())?;
            copy_dir_recursive(&src_path, &dst_path)?;
            continue;
        }

        std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn fetch_package(
    manifest: &EcosystemManifest,
    cache_dir: &Path,
    options: FetchOptions,
) -> Result<PathBuf, String> {
    let package_dir = cache_dir.join(&manifest.package.name);

    std::fs::create_dir_all(cache_dir).map_err(|e| format!("failed to create cache dir: {e}"))?;

    if package_dir.exists() && !options.refresh {
        // keep the existing clone and update it to the requested ref
        let status = Command::new("git")
            .args(["fetch", "--depth=1", "origin", &manifest.package.git_ref])
            .current_dir(&package_dir)
            .status()
            .map_err(|e| format!("git fetch failed: {e}"))?;
        if !status.success() {
            return Err("git fetch failed".to_string());
        }

        let status = Command::new("git")
            .args(["checkout", "--force", &manifest.package.git_ref])
            .current_dir(&package_dir)
            .status()
            .map_err(|e| format!("git checkout failed: {e}"))?;
        if !status.success() {
            return Err(format!("git checkout {} failed", manifest.package.git_ref));
        }

        let status = Command::new("git")
            .args(["reset", "--hard"])
            .current_dir(&package_dir)
            .status()
            .map_err(|e| format!("git reset failed: {e}"))?;
        if !status.success() {
            return Err("git reset failed".to_string());
        }

        return Ok(package_dir);
    }

    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir)
            .map_err(|e| format!("failed to remove existing dir: {e}"))?;
    }

    let status = Command::new("git")
        .args([
            "clone",
            "--no-checkout",
            "--filter=blob:none",
            &manifest.package.repo,
            &manifest.package.name,
        ])
        .current_dir(cache_dir)
        .status()
        .map_err(|e| format!("git clone failed: {e}"))?;
    if !status.success() {
        return Err("git clone failed".to_string());
    }

    let status = Command::new("git")
        .args(["checkout", &manifest.package.git_ref])
        .current_dir(&package_dir)
        .status()
        .map_err(|e| format!("git checkout failed: {e}"))?;
    if !status.success() {
        return Err(format!("git checkout {} failed", manifest.package.git_ref));
    }

    Ok(package_dir)
}

pub fn fetch_all_packages(options: FetchOptions) -> ExitCode {
    let ecosystem_dir = fixtures_dir().join("ecosystem");
    let packages_dir = ecosystem_dir.join("packages");
    let cache_dir = ecosystem_dir.join("cache");

    let manifest_paths = EcosystemManifest::discover_all(&packages_dir);
    println!("fetching {} packages...", manifest_paths.len());

    let mut any_failed = false;
    for path in manifest_paths {
        match EcosystemManifest::load(&path) {
            Ok(manifest) => {
                print!(
                    "  {}@{} ... ",
                    manifest.package.name, manifest.package.git_ref
                );
                match fetch_package(&manifest, &cache_dir, options) {
                    Ok(_) => println!("ok"),
                    Err(error) => {
                        println!("FAILED: {error}");
                        any_failed = true;
                    }
                }
            }
            Err(error) => {
                eprintln!("  error loading {}: {error}", path.display());
                any_failed = true;
            }
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
