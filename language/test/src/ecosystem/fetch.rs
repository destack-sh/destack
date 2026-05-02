use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use destack_core::StableHasher;

use crate::core::fixtures_dir;

use crate::ecosystem::manifest::{EcosystemManifest, EcosystemPhase};
use crate::ecosystem::runner::FetchOptions;

const PATCH_STAMP_FILE: &str = ".destack_patch_stamp";
const GIT_DIRECTORY_NAME: &str = ".git";

/// Ensure patch overlays are applied exactly once per patch snapshot.
pub(super) fn ensure_patches_applied(patches_dir: &Path, package_dir: &Path) -> Result<(), String> {
    if !patches_dir.exists() {
        return Ok(());
    }

    let patch_stamp = compute_patch_stamp(patches_dir)?;
    let stamp_path = package_dir.join(PATCH_STAMP_FILE);

    if let Ok(existing_stamp) = fs::read_to_string(&stamp_path)
        && existing_stamp.trim() == patch_stamp
    {
        return Ok(());
    }

    copy_dir_recursive(patches_dir, package_dir)?;
    fs::write(&stamp_path, format!("{patch_stamp}\n"))
        .map_err(|error| format!("failed writing {}: {error}", stamp_path.display()))?;
    Ok(())
}

/// Compute a stable content stamp for a patch directory.
fn compute_patch_stamp(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files_recursive(root, &mut files)?;
    files.sort();

    let mut hasher = StableHasher::new();
    for file in files {
        let relative = file.strip_prefix(root).unwrap_or(file.as_path());
        relative.to_string_lossy().hash(&mut hasher);

        let content = fs::read(file.as_path())
            .map_err(|error| format!("failed reading {}: {error}", file.display()))?;
        content.hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish_u64()))
}

/// Collect file paths recursively from a directory.
fn collect_files_recursive(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(root).map_err(|error| format!("read_dir {}: {error}", root.display()))?
    {
        let entry = entry.map_err(|error| format!("read_dir entry {}: {error}", root.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(path.as_path(), output)?;
            continue;
        }

        output.push(path);
    }

    Ok(())
}

/// Copy directory contents recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(src).map_err(|error| format!("read_dir {}: {error}", src.display()))?
    {
        let entry = entry.map_err(|error| format!("read_dir entry {}: {error}", src.display()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .map_err(|error| format!("create_dir_all {}: {error}", dst_path.display()))?;
            copy_dir_recursive(src_path.as_path(), dst_path.as_path())?;
            continue;
        }

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create_dir_all {}: {error}", parent.display()))?;
        }

        if dst_path.exists() {
            let src_bytes = fs::read(src_path.as_path())
                .map_err(|error| format!("read {}: {error}", src_path.display()))?;
            let dst_bytes = fs::read(dst_path.as_path())
                .map_err(|error| format!("read {}: {error}", dst_path.display()))?;
            if src_bytes == dst_bytes {
                continue;
            }
        }

        fs::copy(src_path.as_path(), dst_path.as_path()).map_err(|error| {
            format!(
                "copy {} -> {}: {error}",
                src_path.display(),
                dst_path.display()
            )
        })?;
    }

    Ok(())
}

/// Run a git command in a directory and return a descriptive error on failure.
fn run_git(args: &[&str], current_dir: &Path, context: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(args)
        .current_dir(current_dir)
        .status()
        .map_err(|error| {
            format!(
                "git {context} failed to start in {}: {error}",
                current_dir.display()
            )
        })?;

    if status.success() {
        return Ok(());
    }

    Err(format!(
        "git {context} failed in {} with args: {:?}",
        current_dir.display(),
        args,
    ))
}

/// Package manager used for fixture dependency installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EcosystemPackageManager {
    /// Use pnpm for dependency installation.
    Pnpm,
    /// Use yarn for dependency installation.
    Yarn,
    /// Use npm for dependency installation.
    Npm,
    /// Use bun for dependency installation.
    Bun,
}

/// Command line used for fixture dependency installation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EcosystemInstallCommand {
    /// The executable to run.
    command: String,
    /// The arguments passed to the executable.
    args: Vec<String>,
}

/// Yarn mode used for fixture dependency installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EcosystemYarnFlavor {
    /// Use yarn classic arguments.
    Classic,
    /// Use yarn modern arguments.
    Berry,
}

/// Install dependencies for one fetched ecosystem package checkout.
fn ensure_package_dependencies_installed(package_dir: &Path) -> Result<(), String> {
    // skip non-node packages
    if !package_dir.join("package.json").is_file() {
        return Ok(());
    }

    // skip already installed dependencies
    if package_dependencies_are_installed(package_dir) {
        return Ok(());
    }

    // choose package manager from lockfiles and workspace metadata
    let package_manager = detect_package_manager(package_dir);
    let install = install_command_for_package_manager(package_dir, package_manager);

    // run install with deterministic ci environment
    let status = Command::new(&install.command)
        .args(install.args.iter().map(String::as_str))
        .current_dir(package_dir)
        .env("CI", "1")
        .status()
        .map_err(|error| {
            format!(
                "{} install failed to start in {}: {error}",
                install.command,
                package_dir.display()
            )
        })?;

    if status.success() {
        return Ok(());
    }

    Err(format!(
        "{} install failed in {} with args: {:?}",
        install.command,
        package_dir.display(),
        install.args,
    ))
}

/// Return whether one package checkout already has installed dependencies.
fn package_dependencies_are_installed(package_dir: &Path) -> bool {
    package_dir.join("node_modules").is_dir()
        || package_dir.join(".pnp.cjs").is_file()
        || package_dir.join(".pnp.js").is_file()
        || package_dir.join(".pnp.loader.mjs").is_file()
}

/// Detect package manager from lockfiles and workspace metadata.
fn detect_package_manager(package_dir: &Path) -> EcosystemPackageManager {
    // choose pnpm for pnpm lockfiles
    if package_dir.join("pnpm-lock.yaml").is_file()
        || package_dir.join("pnpm-workspace.yaml").is_file()
    {
        return EcosystemPackageManager::Pnpm;
    }

    // choose yarn for yarn lockfiles and config
    if package_dir.join("yarn.lock").is_file() || package_dir.join(".yarnrc.yml").is_file() {
        return EcosystemPackageManager::Yarn;
    }

    // choose bun for bun lockfiles
    if package_dir.join("bun.lock").is_file() || package_dir.join("bun.lockb").is_file() {
        return EcosystemPackageManager::Bun;
    }

    // default to npm
    EcosystemPackageManager::Npm
}

/// Select the install command for one package manager.
fn install_command_for_package_manager(
    package_dir: &Path,
    package_manager: EcosystemPackageManager,
) -> EcosystemInstallCommand {
    // use frozen lockfile installs for pnpm packages
    if package_manager == EcosystemPackageManager::Pnpm {
        return EcosystemInstallCommand {
            command: "corepack".to_string(),
            args: vec![
                "pnpm".to_string(),
                "install".to_string(),
                "--ignore-scripts".to_string(),
                "--frozen-lockfile".to_string(),
            ],
        };
    }

    // use yarn classic or berry compatible script-suppression flags
    if package_manager == EcosystemPackageManager::Yarn {
        if detect_yarn_flavor(package_dir) == EcosystemYarnFlavor::Berry {
            return EcosystemInstallCommand {
                command: "corepack".to_string(),
                args: vec![
                    "yarn".to_string(),
                    "install".to_string(),
                    "--mode".to_string(),
                    "skip-build".to_string(),
                ],
            };
        }

        return EcosystemInstallCommand {
            command: "corepack".to_string(),
            args: vec![
                "yarn".to_string(),
                "install".to_string(),
                "--ignore-scripts".to_string(),
            ],
        };
    }

    // prefer npm ci when lockfiles exist
    if package_manager == EcosystemPackageManager::Npm {
        if package_dir.join("package-lock.json").is_file()
            || package_dir.join("npm-shrinkwrap.json").is_file()
        {
            return EcosystemInstallCommand {
                command: "npm".to_string(),
                args: vec!["ci".to_string(), "--ignore-scripts".to_string()],
            };
        }

        return EcosystemInstallCommand {
            command: "npm".to_string(),
            args: vec!["install".to_string(), "--ignore-scripts".to_string()],
        };
    }

    // default bun install command
    EcosystemInstallCommand {
        command: "bun".to_string(),
        args: vec!["install".to_string(), "--ignore-scripts".to_string()],
    }
}

/// Detect yarn flavor from checkout metadata.
fn detect_yarn_flavor(package_dir: &Path) -> EcosystemYarnFlavor {
    // berry repositories define modern yarn config or release directory
    if package_dir.join(".yarnrc.yml").is_file() || package_dir.join(".yarn").is_dir() {
        return EcosystemYarnFlavor::Berry;
    }

    EcosystemYarnFlavor::Classic
}
/// Auto fetch missing package checkouts before running tests.
pub(super) fn auto_fetch_missing_checkouts(
    manifests: &[EcosystemManifest],
    phases: &[EcosystemPhase],
    filter: Option<&str>,
    checkouts_dir: &Path,
    patches_dir: &Path,
    is_list_mode: bool,
) -> HashMap<String, String> {
    if is_list_mode {
        return HashMap::new();
    }

    // install dependencies when selected phases need module resolution
    let should_install_dependencies = phases.iter().any(|phase| *phase != EcosystemPhase::Parse);

    // collect selected manifests and missing checkout paths
    let mut selected = Vec::new();
    let mut missing_or_broken = Vec::new();
    for manifest in manifests {
        if !manifest_matches_filter(manifest, phases, filter) {
            continue;
        }

        selected.push(manifest);

        let package_dir = checkouts_dir.join(&manifest.package.name);
        if checkout_requires_refetch(&package_dir) {
            missing_or_broken.push(manifest);
        }
    }

    let mut failures = HashMap::new();

    // fetch missing checkouts first
    if !missing_or_broken.is_empty() {
        println!();
        println!(
            "auto-fetching {} missing or broken ecosystem checkouts",
            missing_or_broken.len()
        );

        for manifest in missing_or_broken {
            print!(
                "  {}@{} ... ",
                manifest.package.name, manifest.package.git_ref
            );
            let fetch_result = fetch_package(
                manifest,
                checkouts_dir,
                FetchOptions {
                    refresh: false,
                    install: false,
                },
            );
            match fetch_result {
                Ok(package_dir) => {
                    let package_patches_dir = patches_dir.join(&manifest.package.name);
                    match ensure_patches_applied(&package_patches_dir, &package_dir) {
                        Ok(()) => {
                            println!("ok");
                        }
                        Err(error) => {
                            println!("FAILED: patch apply failed: {error}");
                            failures.insert(manifest.package.name.clone(), error);
                        }
                    }
                }
                Err(error) => {
                    println!("FAILED: {error}");
                    failures.insert(manifest.package.name.clone(), error);
                }
            }
        }

        println!();
    }

    // install dependencies for selected packages in compiler phases
    if should_install_dependencies {
        for manifest in selected {
            if failures.contains_key(&manifest.package.name) {
                continue;
            }

            let package_dir = checkouts_dir.join(&manifest.package.name);
            if !package_dir.exists() {
                continue;
            }

            if let Err(error) = ensure_package_dependencies_installed(&package_dir) {
                failures.insert(manifest.package.name.clone(), error);
            }
        }
    }

    failures
}
/// Build a stable case id from package and phase.
fn case_id_for(package_name: &str, phase: EcosystemPhase) -> String {
    format!("{package_name}-{}", phase.name())
}

/// Return whether one manifest can produce a selected case for the current filter.
fn manifest_matches_filter(
    manifest: &EcosystemManifest,
    phases: &[EcosystemPhase],
    filter: Option<&str>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    if manifest.package.name.contains(filter) {
        return true;
    }

    phases
        .iter()
        .copied()
        .any(|phase| case_id_for(manifest.package.name.as_str(), phase).contains(filter))
}

/// Return whether one checkout directory should be re-fetched.
fn checkout_requires_refetch(package_dir: &Path) -> bool {
    // missing directories always need fetch
    if !package_dir.exists() {
        return true;
    }

    // unreadable directories are treated as broken checkouts
    let Ok(entries) = fs::read_dir(package_dir) else {
        return true;
    };

    // detect any worktree content beyond git metadata
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy() == GIT_DIRECTORY_NAME {
            continue;
        }

        return false;
    }

    // git metadata only: fetch a fresh working tree
    true
}

/// Fetch one package checkout into checkouts directory.
pub(super) fn fetch_package(
    manifest: &EcosystemManifest,
    checkouts_dir: &Path,
    options: FetchOptions,
) -> Result<PathBuf, String> {
    let package_dir = checkouts_dir.join(&manifest.package.name);

    fs::create_dir_all(checkouts_dir).map_err(|error| {
        format!(
            "failed to create checkouts dir {}: {error}",
            checkouts_dir.display()
        )
    })?;

    if package_dir.exists() && !options.refresh {
        // update existing clone to requested ref and clean local mutations
        run_git(
            &["fetch", "--depth=1", "origin", &manifest.package.git_ref],
            &package_dir,
            "fetch",
        )?;
        run_git(
            &["checkout", "--force", &manifest.package.git_ref],
            &package_dir,
            "checkout",
        )?;
        run_git(&["reset", "--hard"], &package_dir, "reset")?;
        run_git(&["clean", "-fd"], &package_dir, "clean")?;
        return Ok(package_dir);
    }

    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|error| {
            format!(
                "failed to remove existing package dir {}: {error}",
                package_dir.display()
            )
        })?;
    }

    run_git(
        &[
            "clone",
            "--no-checkout",
            "--filter=blob:none",
            &manifest.package.repo,
            &manifest.package.name,
        ],
        checkouts_dir,
        "clone",
    )?;

    run_git(
        &["checkout", "--force", &manifest.package.git_ref],
        &package_dir,
        "checkout",
    )?;

    Ok(package_dir)
}

/// Remove checkout directories without one matching manifest.
fn prune_stale_checkouts(
    manifests: &[EcosystemManifest],
    checkouts_dir: &Path,
) -> Result<Vec<String>, String> {
    if !checkouts_dir.exists() {
        return Ok(Vec::new());
    }

    // build the expected checkout names from loaded manifests
    let expected_names: HashSet<&str> = manifests
        .iter()
        .map(|manifest| manifest.package.name.as_str())
        .collect();
    let entries = fs::read_dir(checkouts_dir).map_err(|error| {
        format!(
            "failed to read checkouts dir {}: {error}",
            checkouts_dir.display()
        )
    })?;
    let mut removed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read checkouts dir entry in {}: {error}",
                checkouts_dir.display()
            )
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(directory_name) = path.file_name() else {
            continue;
        };
        let directory_name = directory_name.to_string_lossy().to_string();
        if expected_names.contains(directory_name.as_str()) {
            continue;
        }

        fs::remove_dir_all(&path).map_err(|error| {
            format!(
                "failed to remove stale checkout {}: {error}",
                path.display()
            )
        })?;
        removed.push(directory_name);
    }

    removed.sort();

    Ok(removed)
}

/// Fetch all package checkouts declared by ecosystem manifests.
pub(super) fn fetch_all_packages(options: FetchOptions) -> ExitCode {
    let ecosystem_dir = fixtures_dir().join("ecosystem");
    let packages_dir = ecosystem_dir.join("packages");
    let checkouts_dir = ecosystem_dir.join("checkouts");
    let patches_dir = ecosystem_dir.join("patches");

    let manifest_paths = match EcosystemManifest::discover_all(&packages_dir) {
        Ok(manifest_paths) => manifest_paths,
        Err(error) => {
            eprintln!("  error: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("fetching {} packages...", manifest_paths.len());

    // load all manifests first: this allows stale checkout pruning
    let mut manifests = Vec::new();
    let mut any_failed = false;
    for path in manifest_paths {
        match EcosystemManifest::load(&path) {
            Ok(manifest) => {
                manifests.push(manifest);
            }
            Err(error) => {
                eprintln!("  error loading {}: {error}", path.display());
                any_failed = true;
            }
        }
    }

    // prune stale checkouts so deleted manifests do not accumulate old clones
    match prune_stale_checkouts(&manifests, &checkouts_dir) {
        Ok(removed) => {
            if !removed.is_empty() {
                println!("pruned {} stale checkouts", removed.len());
            }
        }
        Err(error) => {
            eprintln!("FAILED: stale checkout prune failed: {error}");
            any_failed = true;
        }
    }

    for manifest in manifests {
        print!(
            "  {}@{} ... ",
            manifest.package.name, manifest.package.git_ref
        );
        match fetch_package(&manifest, &checkouts_dir, options) {
            Ok(package_dir) => {
                let package_patches_dir = patches_dir.join(&manifest.package.name);
                match ensure_patches_applied(&package_patches_dir, &package_dir) {
                    Ok(()) => {
                        if options.install {
                            match ensure_package_dependencies_installed(&package_dir) {
                                Ok(()) => println!("ok"),
                                Err(error) => {
                                    println!("FAILED: install failed: {error}");
                                    any_failed = true;
                                }
                            }
                        } else {
                            println!("ok");
                        }
                    }
                    Err(error) => {
                        println!("FAILED: patch apply failed: {error}");
                        any_failed = true;
                    }
                }
            }
            Err(error) => {
                println!("FAILED: {error}");
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::checkout_requires_refetch;

    /// Build a unique temporary path for ecosystem fetch tests.
    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "destack-ecosystem-fetch-{label}-{nanos}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn test_checkout_requires_refetch_for_missing_directory() {
        let directory = unique_temp_dir("missing");

        assert!(checkout_requires_refetch(&directory));
    }

    #[test]
    fn test_checkout_requires_refetch_for_git_only_checkout() {
        let directory = unique_temp_dir("git-only");
        fs::create_dir_all(directory.join(".git")).expect("failed to create git directory");

        assert!(checkout_requires_refetch(&directory));

        fs::remove_dir_all(&directory).expect("failed to remove temp directory");
    }

    #[test]
    fn test_checkout_requires_refetch_for_checkout_with_worktree_content() {
        let directory = unique_temp_dir("worktree");
        fs::create_dir_all(directory.join(".git")).expect("failed to create git directory");
        fs::write(directory.join("README.md"), "content").expect("failed to write worktree file");

        assert!(!checkout_requires_refetch(&directory));

        fs::remove_dir_all(&directory).expect("failed to remove temp directory");
    }
}
