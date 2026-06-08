use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{io, thread};

use destack_artifact::{EmitFormat, MemoryCacheStore, Platform, Runtime};
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_session::Session;
use destack_source::{FileSystem, MemoryFileSystem, ModuleId, TargetId};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Mode, Profile, Ref, Repository, Revision,
    Settings,
};
use indexmap::IndexSet;

use crate::core::{CaseResult, discover_file_cases, load_expected_failures};

use super::parser::MdTestCase;

/// Per test timeout in seconds.
pub const TEST_TIMEOUT_SECONDS: u64 = 5;

/// Library overrides for mdtest cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdTestLibs {
    /// Use the profile's default library set.
    Default,
    /// Disable loading profile libraries.
    None,
    /// Use an explicit library list.
    Explicit(Vec<String>),
}

/// Profile overrides for mdtest cases.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MdTestProfileOverrides {
    /// Emit format override for profile identity.
    pub emit: Option<EmitFormat>,
    /// Runtime override for import.meta.
    pub runtime: Option<Runtime>,
    /// Runtime version override for versioned globals.
    pub runtime_version: Option<String>,
    /// Platform override for import.meta.
    pub platform: Option<Platform>,
    /// Debug mode override for import.meta.
    pub debug: Option<bool>,
}

impl MdTestProfileOverrides {
    /// Return true if versioned globals were requested.
    fn has_versioned_globals(&self) -> bool {
        self.runtime_version.is_some()
    }
}

/// Parse library options from a mdtest case.
pub fn parse_mdtest_libs(test: &MdTestCase) -> Option<MdTestLibs> {
    // extract the raw lib option
    let raw = option_value(&test.options, &["libs", "lib"])?;
    let trimmed = raw.trim();

    // handle none and default options
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return Some(MdTestLibs::None);
    }
    if trimmed.eq_ignore_ascii_case("default") {
        return Some(MdTestLibs::Default);
    }

    // parse explicit library names
    let mut libs: Vec<String> = trimmed
        .split(',')
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();
    libs.sort();
    libs.dedup();

    // normalize empty lists as none
    if libs.is_empty() {
        Some(MdTestLibs::None)
    } else {
        Some(MdTestLibs::Explicit(libs))
    }
}

/// Parse profile overrides from a mdtest case.
fn parse_mdtest_profile_overrides(test: &MdTestCase) -> MdTestProfileOverrides {
    // parse emit format override
    let emit = option_value(&test.options, &["emit"]).map(parse_emit_format);

    // parse runtime override
    let runtime = option_value(&test.options, &["runtime"]).map(|value| {
        Runtime::parse(value).unwrap_or_else(|| panic!("invalid mdtest runtime '{value}'"))
    });

    // parse runtime version override
    let runtime_version = option_value(
        &test.options,
        &["runtime_version", "runtime-version", "runtimeVersion"],
    )
    .map(|value| value.to_string());

    // parse platform override
    let platform = option_value(&test.options, &["platform"]).map(|value| {
        Platform::parse(value).unwrap_or_else(|| panic!("invalid mdtest platform '{value}'"))
    });

    // parse debug override
    let debug = option_value(&test.options, &["debug"]).map(|value| parse_bool(value, "debug"));

    MdTestProfileOverrides {
        emit,
        runtime,
        runtime_version,
        platform,
        debug,
    }
}

/// Select a profile and lib loading mode for a mdtest case.
pub fn select_profile_for_mdtest(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    test: &MdTestCase,
    default_load_libraries: bool,
) -> (Profile, bool) {
    // load explicit base profile state
    let module = repository
        .module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to resolve mdtest module: {error}"))
        .unwrap_or_else(|| panic!("missing mdtest module {module_id:?}"));
    let target_id = TargetId::new(module.package_id, "default");
    let base_profile = repository
        .profile_for_target(revision, target_id)
        .unwrap_or_else(|error| panic!("failed to resolve mdtest profile: {error}"));
    let overrides = parse_mdtest_profile_overrides(test);
    let lib_override = parse_mdtest_libs(test);

    // seed override state
    let mut load_libraries = default_load_libraries;
    let mut key = base_profile.key.clone();
    // apply runtime overrides
    if let Some(emit) = overrides.emit {
        key.emit = emit;
    }
    if let Some(runtime) = overrides.runtime {
        key.conditions.runtime = Some(runtime);
    }
    if let Some(platform) = overrides.platform {
        key.conditions.platform = Some(platform);
    }
    if let Some(debug) = overrides.debug {
        set_mode(&mut key.conditions.modes, Mode::DEBUG.name, debug);
    }

    // validate runtime version usage
    if overrides.runtime_version.is_some() && lib_override.is_none() {
        panic!("mdtest runtime_version requires libs=default or libs=...");
    }

    // apply library overrides
    if let Some(lib_override) = lib_override {
        match lib_override {
            MdTestLibs::None => {
                load_libraries = false;
                key.globals.clear();
            }
            MdTestLibs::Default => {
                load_libraries = true;
                if overrides.has_versioned_globals() {
                    panic!("mdtest runtime_version no longer derives profile globals");
                }
            }
            MdTestLibs::Explicit(libs) => {
                key.globals = libs;
                load_libraries = true;
            }
        }
    }

    // return the resolved profile
    if key != base_profile.key {
        (Profile::from_key(key), load_libraries)
    } else {
        ((*base_profile).clone(), load_libraries)
    }
}

/// Add or remove one profile mode.
fn set_mode(modes: &mut IndexSet<String>, mode: &str, enabled: bool) {
    let has_mode = modes.contains(mode);

    if enabled && !has_mode {
        modes.insert(mode.to_string());
    } else if !enabled {
        modes.shift_remove(mode);
    }
}

/// Load expected failures for mdtest suites.
pub fn load_mdtest_expected_failures(base_dir: &Path) -> std::collections::HashSet<String> {
    let path = base_dir.join("known-failures.txt");
    load_expected_failures(&path)
}

/// Lookup the first matching option value from a set of keys.
fn option_value<'a>(options: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    // scan for the first matching key
    for key in keys {
        if let Some(value) = options.get(*key) {
            return Some(value.as_str());
        }
    }
    None
}

/// Parse a boolean mdtest option.
fn parse_bool(value: &str, key: &str) -> bool {
    // normalize boolean option values
    match value.trim().to_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => panic!("invalid mdtest {key} value '{value}'"),
    }
}

/// Parse an emit format mdtest option.
fn parse_emit_format(value: &str) -> EmitFormat {
    match value.trim().to_lowercase().as_str() {
        "js" | "javascript" => EmitFormat::Js,
        "ts" | "typescript" => EmitFormat::Ts,
        "wasm" | "webassembly" => EmitFormat::Wasm,
        "native" => EmitFormat::Native,
        _ => panic!("invalid mdtest emit value '{value}'"),
    }
}

/// Set up an in memory test environment from a markdown test case.
pub fn setup_test_environment_with_repository(
    test: &MdTestCase,
    repository: Arc<Repository>,
    memory_fs: Arc<MemoryFileSystem>,
    root: PathBuf,
) -> (Arc<Repository>, PathBuf, PathBuf) {
    // populate filesystem with test files
    let mut main_path: Option<PathBuf> = None;
    for file in &test.files {
        let file_path = root.join(&file.path);
        memory_fs
            .add_file(&file_path, file.content.as_bytes())
            .expect("failed to add test file");

        if matches!(file.path.as_str(), "main.ds" | "main.ts" | "main.js") || main_path.is_none() {
            main_path = Some(file_path);
        }
    }

    // choose the main file and create the repository root
    let main_path = main_path.expect("test should have at least one file");
    let head = Ref::for_root(repository.path());
    let compiler = Arc::new(Compiler::new(repository.clone()));
    let linter = Arc::new(Linter::new(repository.clone()));
    let query = Arc::new(Query::new(repository.clone()));
    let session = Session::new(
        repository.path().to_path_buf(),
        root.clone(),
        repository.clone(),
        head,
        compiler,
        linter,
        query,
        1,
        None,
    )
    .expect("failed to create mdtest session");
    session
        .reload_from_fs(session.head())
        .expect("failed to materialize mdtest workspace");
    (repository, root, main_path)
}

/// Set up an in memory test environment from a markdown test case.
pub fn setup_test_environment(test: &MdTestCase) -> (Arc<Repository>, PathBuf, PathBuf) {
    // setup memory filesystem and repository
    let memory_fs = Arc::new(MemoryFileSystem::new());
    let cwd = PathBuf::from("/test");
    let fs: Arc<dyn FileSystem> = memory_fs.clone();
    let environment = Environment::capture_process();
    let layout = DestackLayout::resolve(
        &cwd,
        &cwd,
        &environment,
        &Settings::default(),
        &DestackLayoutOverride::default(),
        None,
    );
    let repository = Arc::new(Repository::new(
        cwd.clone(),
        Arc::new(MemoryCacheStore::new()),
        fs,
        environment,
        Settings::default(),
        layout,
    ));

    // delegate to repository based setup
    setup_test_environment_with_repository(test, repository, memory_fs, cwd)
}

/// Run a test function with a timeout.
/// Returns a failed result if the test times out or panics.
pub fn run_with_timeout<F>(test: MdTestCase, timeout: Duration, f: F) -> CaseResult
where
    F: FnOnce(&MdTestCase) -> CaseResult + Send + 'static,
{
    // allocate the communication channel
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // catch panics to prevent thread cleanup hangs
        let result = catch_unwind(AssertUnwindSafe(|| f(&test)));

        // convert panics into test failures
        let test_result = match result {
            Ok(result) => result,
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");

                // skip tests marked with #Incomplete
                if msg.contains("#Incomplete") {
                    CaseResult::Skipped {
                        reason: msg.to_string(),
                    }
                } else {
                    CaseResult::Failed {
                        message: format!("panic: {msg}"),
                    }
                }
            }
        };

        // send the test result back to the runner
        let _ = tx.send(test_result);
    });

    // wait for the test result or timeout
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => CaseResult::Failed {
            message: format!(
                "test timed out after {}s (likely deadlock or infinite loop)",
                timeout.as_secs()
            ),
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => CaseResult::Failed {
            message: "test thread disconnected unexpectedly".to_string(),
        },
    }
}

/// Discover markdown files recursively in a directory.
/// Skips README files, hidden directories, node_modules, and staging.
pub fn discover_md_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    // collect markdown files recursively
    let mut files = Vec::new();

    // return early when the directory does not exist
    if !dir.exists() {
        return Ok(files);
    }

    // collect direct md files excluding readme
    let direct_files = discover_file_cases(dir, &["md"], "mdtest")?;
    for test in direct_files {
        if test.name.to_lowercase() == "readme" {
            continue;
        }
        files.push(test.path);
    }

    // recurse into subdirectories
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "node_modules" && name != "staging" {
                files.extend(discover_md_files(&path)?);
            }
        }
    }

    Ok(files)
}

/// Convert a test name to a URL safe slug.
pub fn slug(name: &str) -> String {
    // normalize to url safe slugs
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
