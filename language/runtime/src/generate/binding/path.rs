use std::fs;
use std::path::{Path, PathBuf};

/// Resolve the language workspace root for generator outputs.
fn language_root() -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"))
        .to_path_buf()
}

/// Resolve the generated binding path for a runtime domain.
pub(crate) fn runtime_domain_bindings_path(domain: &str) -> PathBuf {
    language_root().join(format!(
        "runtime/src/platform/{domain}/bindings.generated.rs"
    ))
}

/// Resolve the module path for a runtime domain.
pub(crate) fn runtime_domain_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/mod.rs"))
}

/// Resolve the host router path for a runtime domain.
pub(crate) fn runtime_domain_host_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/host.rs"))
}

/// Resolve the native binding path for a runtime domain.
pub(crate) fn runtime_domain_native_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/native.rs"))
}

/// Resolve the unsupported host fallback path for a runtime domain.
pub(crate) fn runtime_domain_unsupported_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/unsupported.rs"))
}

/// Resolve the VM binding path for a runtime domain.
pub(crate) fn runtime_domain_vm_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/vm.rs"))
}

/// Resolve the simulated native binding path for a runtime domain.
pub(crate) fn runtime_domain_simulated_native_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/simulated/native.rs"))
}

/// Resolve the simulated VM binding path for a runtime domain.
pub(crate) fn runtime_domain_simulated_vm_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/simulated/vm.rs"))
}

/// Resolve the simulated module path for a runtime domain.
pub(crate) fn runtime_domain_simulated_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/simulated/mod.rs"))
}

/// Resolve the unix host backend shim path for a runtime domain.
pub(crate) fn runtime_domain_unix_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/unix/mod.rs"))
}

/// Resolve the windows host backend shim path for a runtime domain.
pub(crate) fn runtime_domain_windows_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/windows/mod.rs"))
}

/// Resolve the runtime native binding path for a runtime domain.
pub(crate) fn runtime_domain_runtime_native_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/runtime/native.rs"))
}

/// Resolve the runtime VM binding path for a runtime domain.
pub(crate) fn runtime_domain_runtime_vm_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/runtime/vm.rs"))
}

/// Resolve the runtime module path for a runtime domain.
pub(crate) fn runtime_domain_runtime_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/runtime/mod.rs"))
}

/// Resolve the VM types binding path for a runtime domain.
pub(crate) fn runtime_domain_abi_types_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/abi.generated.rs"))
}

/// Resolve the handwritten tests harness path for a runtime domain.
pub(crate) fn runtime_domain_tests_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/tests/tests.rs"))
}

/// Resolve the tests directory path for a runtime domain.
pub(crate) fn runtime_domain_tests_dir_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/tests"))
}

/// Resolve the tests module path for a runtime domain.
pub(crate) fn runtime_domain_tests_mod_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/tests/mod.rs"))
}

/// Resolve the basic tests module path for a runtime domain.
pub(crate) fn runtime_domain_tests_basic_path(domain: &str) -> PathBuf {
    language_root().join(format!("runtime/src/platform/{domain}/tests/basic.rs"))
}

/// Resolve the generated tests harness adapter path for a runtime domain.
pub(crate) fn runtime_domain_test_harness_generated_path(domain: &str) -> PathBuf {
    language_root().join(format!(
        "runtime/src/platform/{domain}/tests/harness.generated.rs"
    ))
}

/// Resolve the generated platform binding list path.
pub(crate) fn runtime_platform_generated_path() -> PathBuf {
    language_root().join("runtime/src/platform/generated.rs")
}

/// Write generated bindings to the given path.
pub(crate) fn write_domain_bindings(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create output directory");
    }

    fs::write(path, contents).expect("failed to write binding catalog");
}
