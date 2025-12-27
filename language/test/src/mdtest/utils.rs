use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{io, thread};

use destack_source::{FileSystem, MemoryFileSystem, ModuleId};
use destack_workspace::{ImportMetaEnv, Platform, ProfileId, Program, Runtime, Session, Target};

use crate::harness::{TestResult, discover_test_files};

use super::parser::MdTestCase;

/// Per-test timeout in seconds.
pub const TEST_TIMEOUT_SECONDS: u64 = 1;

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
    /// Runtime override for import.meta and lib derivation.
    pub runtime: Option<Runtime>,
    /// Runtime version override for versioned libs.
    pub runtime_version: Option<String>,
    /// Platform override for import.meta and lib derivation.
    pub platform: Option<Platform>,
    /// Debug override for import.meta.
    pub debug: Option<bool>,
}

impl MdTestProfileOverrides {
    /// Return true if any override affects lib derivation.
    fn has_lib_overrides(&self) -> bool {
        self.runtime.is_some() || self.runtime_version.is_some() || self.platform.is_some()
    }
}

/// Parse library options from a mdtest case.
pub fn parse_mdtest_libs(test: &MdTestCase) -> Option<MdTestLibs> {
    let raw = option_value(&test.options, &["libs", "lib"])?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return Some(MdTestLibs::None);
    }
    if trimmed.eq_ignore_ascii_case("default") {
        return Some(MdTestLibs::Default);
    }
    let mut libs: Vec<String> = trimmed
        .split(',')
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();
    libs.sort();
    libs.dedup();
    if libs.is_empty() {
        Some(MdTestLibs::None)
    } else {
        Some(MdTestLibs::Explicit(libs))
    }
}

/// Parse profile overrides from a mdtest case.
fn parse_mdtest_profile_overrides(test: &MdTestCase) -> MdTestProfileOverrides {
    let runtime = option_value(&test.options, &["runtime"]).map(|value| {
        Runtime::parse(value).unwrap_or_else(|| panic!("invalid mdtest runtime '{value}'"))
    });
    let runtime_version = option_value(
        &test.options,
        &["runtime_version", "runtime-version", "runtimeVersion"],
    )
    .map(|value| value.to_string());
    let platform = option_value(&test.options, &["platform"]).map(|value| {
        Platform::parse(value).unwrap_or_else(|| panic!("invalid mdtest platform '{value}'"))
    });
    let debug = option_value(&test.options, &["debug"]).map(|value| parse_bool(value, "debug"));

    MdTestProfileOverrides {
        runtime,
        runtime_version,
        platform,
        debug,
    }
}

/// Select a profile and lib-loading mode for a mdtest case.
pub fn select_profile_for_mdtest(
    program: &Program,
    module_id: ModuleId,
    test: &MdTestCase,
    default_load_libs: bool,
) -> (ProfileId, bool) {
    let base_profile_id = program.default_profile_id_for_module(module_id);
    let base_profile = program.profile(base_profile_id);
    let overrides = parse_mdtest_profile_overrides(test);
    let lib_override = parse_mdtest_libs(test);

    let mut load_libs = default_load_libs;
    let mut key = base_profile.key.clone();
    let mut recompute_test = false;

    if let Some(runtime) = overrides.runtime {
        key.runtime = runtime;
    }
    if let Some(platform) = overrides.platform {
        key.platform = platform;
    }
    if let Some(debug) = overrides.debug {
        key.debug = debug;
        recompute_test = true;
    }

    if overrides.runtime_version.is_some() && lib_override.is_none() {
        panic!("mdtest runtime_version requires libs=default or libs=...");
    }

    if let Some(lib_override) = lib_override {
        match lib_override {
            MdTestLibs::None => {
                load_libs = false;
                key.lib.clear();
            }
            MdTestLibs::Default => {
                load_libs = true;
                if overrides.has_lib_overrides() {
                    let target = Target {
                        runtime: key.runtime,
                        platform: key.platform,
                        runtime_version: overrides.runtime_version.clone(),
                        ..Target::default()
                    };
                    key.lib = target.derived_lib();
                }
            }
            MdTestLibs::Explicit(libs) => {
                key.lib = libs;
                load_libs = true;
            }
        }
    }

    if recompute_test {
        let (_, _, _, test_flag) = ImportMetaEnv::mode_from_snapshot(&key.env, key.debug);
        key.test = test_flag;
    }

    if key != base_profile.key {
        let profile_id = program.profiles.get_or_create(key);
        (profile_id, load_libs)
    } else {
        (base_profile_id, load_libs)
    }
}

/// Lookup the first matching option value from a set of keys.
fn option_value<'a>(options: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = options.get(*key) {
            return Some(value.as_str());
        }
    }
    None
}

/// Parse a boolean mdtest option.
fn parse_bool(value: &str, key: &str) -> bool {
    match value.trim().to_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => panic!("invalid mdtest {key} value '{value}'"),
    }
}

/// Set up an in-memory test environment from a markdown test case.
pub fn setup_test_environment_with_session(
    test: &MdTestCase,
    session: Arc<Session>,
    memory_fs: Arc<MemoryFileSystem>,
    root: PathBuf,
) -> (
    Arc<Session>,
    Arc<destack_workspace::program::Program>,
    PathBuf,
) {
    // populate filesystem with test files
    let mut main_path: Option<PathBuf> = None;
    for file in &test.files {
        let file_path = root.join(&file.path);
        memory_fs
            .add_file(&file_path, file.content.as_bytes())
            .expect("failed to add test file");

        if file.path == "main.ds" || main_path.is_none() {
            main_path = Some(file_path);
        }
    }

    let main_path = main_path.expect("test should have at least one file");
    let program = session.add_root(root);

    (session, program, main_path)
}

/// Set up an in-memory test environment from a markdown test case.
pub fn setup_test_environment(
    test: &MdTestCase,
) -> (
    Arc<Session>,
    Arc<destack_workspace::program::Program>,
    PathBuf,
) {
    let memory_fs = Arc::new(MemoryFileSystem::new());
    let cwd = PathBuf::from("/test");
    let fs: Arc<dyn FileSystem> = memory_fs.clone();
    let session = Arc::new(Session::new(cwd.clone()).with_fs(fs));

    setup_test_environment_with_session(test, session, memory_fs, cwd)
}

/// Run a test function with a timeout.
/// Returns a failed result if the test times out or panics.
pub fn run_with_timeout<F>(test: MdTestCase, timeout: Duration, f: F) -> TestResult
where
    F: FnOnce(&MdTestCase) -> TestResult + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // catch panics to prevent thread from hanging during cleanup
        let result = catch_unwind(AssertUnwindSafe(|| f(&test)));

        let test_result = match result {
            Ok(result) => result,
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");

                // if it's a todo!() panic with #Incomplete marker, skip the test
                if msg.contains("#Incomplete") {
                    TestResult::Skipped {
                        reason: msg.to_string(),
                    }
                } else {
                    TestResult::Failed {
                        message: format!("panic: {msg}"),
                    }
                }
            }
        };

        let _ = tx.send(test_result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => TestResult::Failed {
            message: format!(
                "test timed out after {}s (likely deadlock or infinite loop)",
                timeout.as_secs()
            ),
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => TestResult::Failed {
            message: "test thread disconnected unexpectedly".to_string(),
        },
    }
}

/// Discover markdown files recursively in a directory.
/// Skips README files, hidden directories, node_modules, and staging.
pub fn discover_md_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    // direct .md files, excluding README
    let direct_files = discover_test_files(dir, &["md"], "mdtest")?;
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

/// Convert a test name to a URL-safe slug.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
