//! Ecosystem test runner.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri, glob,
};
use destack_workspace::Program;

use crate::harness::{TestCase, TestOptions, TestResult, TestSummary, fixtures_dir, print_result, print_summary, filter_tests, print_test_list};

use super::manifest::{EcosystemManifest, Tier};

// nocheckin: ecosystem tests!

/// Run all ecosystem tests.
pub fn run_ecosystem_tests(options: &TestOptions, tier_filter: Option<Tier>) -> ExitCode {
    let ecosystem_dir = fixtures_dir().join("ecosystem");
    let packages_dir = ecosystem_dir.join("packages");
    let cache_dir = ecosystem_dir.join("cache");
    let patches_dir = ecosystem_dir.join("patches");

    // discover package manifests
    let manifest_paths = EcosystemManifest::discover_all(&packages_dir);
    if manifest_paths.is_empty() {
        println!("no ecosystem packages found in {}", packages_dir.display());
        return ExitCode::SUCCESS;
    }

    // load manifests
    let mut packages: Vec<(EcosystemManifest, PathBuf)> = Vec::new();
    for path in manifest_paths {
        match EcosystemManifest::load(&path) {
            Ok(manifest) => packages.push((manifest, path)),
            Err(e) => {
                eprintln!("warning: {e}");
            }
        }
    }

    // build test cases (one per package)
    let tests: Vec<TestCase> = packages
        .iter()
        .map(|(manifest, _)| {
            TestCase::directory(
                &manifest.package.name,
                cache_dir.join(&manifest.package.name),
                "ecosystem",
            )
        })
        .collect();

    let filtered = filter_tests(tests, options.filter.as_deref());

    if options.list {
        print_test_list(&filtered);
        return ExitCode::SUCCESS;
    }

    println!();
    println!("running {} ecosystem packages", filtered.len());

    let summary = TestSummary::new();
    let start = std::time::Instant::now();

    // run each package test
    for test in &filtered {
        let test_start = std::time::Instant::now();

        // find the manifest for this test
        let manifest = packages
            .iter()
            .find(|(m, _)| m.package.name == test.name)
            .map(|(m, _)| m);

        let result = if let Some(manifest) = manifest {
            run_package_test(manifest, &cache_dir, &patches_dir, tier_filter)
        } else {
            TestResult::Failed {
                message: "manifest not found".to_string(),
            }
        };

        let duration = test_start.elapsed();
        summary.record(&result);
        print_result(test, &result, duration, options.verbose);
    }

    let total_duration = start.elapsed();
    print_summary(&summary, total_duration);

    if summary.all_passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Run tests for a single package.
fn run_package_test(
    manifest: &EcosystemManifest,
    cache_dir: &Path,
    patches_dir: &Path,
    tier_filter: Option<Tier>,
) -> TestResult {
    let package_dir = cache_dir.join(&manifest.package.name);

    // check if package is fetched
    if !package_dir.exists() {
        return TestResult::Skipped {
            reason: format!("not fetched (run: just ecosystem-fetch)"),
        };
    }

    // apply patches if they exist
    let package_patches = patches_dir.join(&manifest.package.name);
    if package_patches.exists() {
        if let Err(e) = apply_patches(&package_patches, &package_dir) {
            return TestResult::Failed {
                message: format!("failed to apply patches: {e}"),
            };
        }
    }

    // discover files
    let files = discover_package_files(&package_dir, manifest);
    if files.is_empty() {
        return TestResult::Failed {
            message: "no files found matching include patterns".to_string(),
        };
    }

    // determine which tier to test
    let tier = tier_filter.unwrap_or(Tier::Parse);

    // only run if this tier is expected to pass
    if !manifest.tiers.expects_pass(tier) {
        return TestResult::Skipped {
            reason: format!("tier {} not expected to pass yet", tier.name()),
        };
    }

    // run the tier
    match tier {
        Tier::Parse => run_parse_tier(&package_dir, &files),
        Tier::Check => TestResult::Skipped {
            reason: "check tier not implemented".to_string(),
        },
        Tier::CompileJs => TestResult::Skipped {
            reason: "compile_js tier not implemented".to_string(),
        },
        Tier::CompileNative => TestResult::Skipped {
            reason: "compile_native tier not implemented".to_string(),
        },
    }
}

/// Discover files in package matching include/exclude patterns.
fn discover_package_files(package_dir: &Path, manifest: &EcosystemManifest) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // default patterns if none specified
    let include_patterns = if manifest.discovery.include.is_empty() {
        vec!["**/*.ts".to_string(), "**/*.tsx".to_string()]
    } else {
        manifest.discovery.include.clone()
    };

    // glob each include pattern
    for pattern in &include_patterns {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        files.extend(matches);
    }

    // build exclude set
    let mut exclude_set = std::collections::HashSet::new();
    for pattern in &manifest.discovery.exclude {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    // always exclude node_modules and common test directories
    let default_excludes = ["**/node_modules/**", "**/dist/**", "**/build/**"];
    for pattern in default_excludes {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    files.retain(|f| !exclude_set.contains(f));
    files.sort();
    files
}

/// Run parse tier on all files.
fn run_parse_tier(package_dir: &Path, files: &[PathBuf]) -> TestResult {
    let mut failed = 0;
    let mut failure_messages: Vec<String> = Vec::new();

    for file in files {
        match parse_file(file) {
            Ok(()) => {}
            Err(e) => {
                failed += 1;
                let relative = file.strip_prefix(package_dir).unwrap_or(file);
                if failure_messages.len() < 10 {
                    failure_messages.push(format!("{}: {e}", relative.display()));
                }
            }
        }
    }

    if failed == 0 {
        TestResult::Passed
    } else {
        let message = if failure_messages.len() < failed {
            format!(
                "{} files failed to parse (showing first 10):\n  {}",
                failed,
                failure_messages.join("\n  ")
            )
        } else {
            format!(
                "{} files failed to parse:\n  {}",
                failed,
                failure_messages.join("\n  ")
            )
        };
        TestResult::Failed { message }
    }
}

/// Parse a single file, return Ok if no errors.
fn parse_file(path: &Path) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {e}"))?;

    // determine file type from extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let file_type = FileType::from_extension(ext).unwrap_or(FileType::TypeScript);
    let language_type = destack_source::LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);

    let uri = Uri::from_path(path);
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let program = Arc::new(Program::new(language, cwd, fs, files));

    let file_id = program.files.next_id();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let file = File::from_text(file_id, name, uri, Some(path.to_path_buf()), file_type, content);
    program.files.insert(file);
    let file = program.files.get(file_id);

    let mut parser = Parser::lex_file(file, program.language);
    let _ = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    // check for errors
    let diagnostics = program.diagnostics.iter();
    let errors: Vec<_> = diagnostics
        .into_iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        let first_error = &errors[0];
        Err(first_error.message.clone())
    }
}

/// Apply patches from patches directory to package.
fn apply_patches(patches_dir: &Path, package_dir: &Path) -> Result<(), String> {
    // copy all files from patches dir to package dir
    copy_dir_recursive(patches_dir, package_dir)
}

/// Recursively copy directory contents.
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
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Fetch a package into the cache.
pub fn fetch_package(manifest: &EcosystemManifest, cache_dir: &Path) -> Result<PathBuf, String> {
    let package_dir = cache_dir.join(&manifest.package.name);

    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("failed to create cache dir: {e}"))?;

    // remove existing and re-clone (simpler than updating)
    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir)
            .map_err(|e| format!("failed to remove existing dir: {e}"))?;
    }

    let git_ref = manifest.package.git_ref.as_deref();

    // clone (full clone if we need a specific ref, otherwise shallow)
    if git_ref.is_some() {
        // need full clone to checkout tags/commits
        let status = Command::new("git")
            .args([
                "clone",
                "--no-checkout",
                &manifest.package.repo,
                &manifest.package.name,
            ])
            .current_dir(cache_dir)
            .status()
            .map_err(|e| format!("git clone failed: {e}"))?;

        if !status.success() {
            return Err("git clone failed".to_string());
        }

        // checkout the specific ref
        let ref_name = git_ref.unwrap();
        let status = Command::new("git")
            .args(["checkout", ref_name])
            .current_dir(&package_dir)
            .status()
            .map_err(|e| format!("git checkout failed: {e}"))?;

        if !status.success() {
            return Err(format!("git checkout {ref_name} failed"));
        }
    } else {
        // shallow clone default branch
        let status = Command::new("git")
            .args([
                "clone",
                "--depth", "1",
                &manifest.package.repo,
                &manifest.package.name,
            ])
            .current_dir(cache_dir)
            .status()
            .map_err(|e| format!("git clone failed: {e}"))?;

        if !status.success() {
            return Err("git clone failed".to_string());
        }
    }

    Ok(package_dir)
}

/// Fetch all packages.
pub fn fetch_all_packages() -> ExitCode {
    let ecosystem_dir = fixtures_dir().join("ecosystem");
    let packages_dir = ecosystem_dir.join("packages");
    let cache_dir = ecosystem_dir.join("cache");

    let manifest_paths = EcosystemManifest::discover_all(&packages_dir);

    println!("fetching {} packages...", manifest_paths.len());

    let mut any_failed = false;
    for path in manifest_paths {
        match EcosystemManifest::load(&path) {
            Ok(manifest) => {
                print!("  {} ... ", manifest.package.name);
                match fetch_package(&manifest, &cache_dir) {
                    Ok(_) => println!("ok"),
                    Err(e) => {
                        println!("FAILED: {e}");
                        any_failed = true;
                    }
                }
            }
            Err(e) => {
                eprintln!("  error loading {}: {e}", path.display());
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
