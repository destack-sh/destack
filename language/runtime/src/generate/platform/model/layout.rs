use std::path::{Path, PathBuf};

/// The generated and handwritten paths for one binding module.
#[derive(Debug, Clone)]
pub(crate) struct ModuleLayout {
    /// The module root directory.
    pub dir: PathBuf,
    /// The handwritten module file.
    pub mod_path: PathBuf,
    /// The generated ABI file.
    pub abi_types_path: PathBuf,
    /// The generated binding dispatch file.
    pub bindings_path: PathBuf,
    /// The handwritten host router file.
    pub host_path: PathBuf,
    /// The handwritten native binding file.
    pub native_path: PathBuf,
    /// The handwritten VM binding file.
    pub vm_path: PathBuf,
    /// The handwritten unsupported host fallback file.
    pub unsupported_path: PathBuf,
    /// The simulation module file.
    pub simulation_mod_path: PathBuf,
    /// The simulation native binding file.
    pub simulation_native_path: PathBuf,
    /// The simulation VM binding file.
    pub simulation_vm_path: PathBuf,
    /// The unix backend shim file.
    pub unix_mod_path: PathBuf,
    /// The windows backend shim file.
    pub windows_mod_path: PathBuf,
}

/// The generator filesystem layout rooted at the language workspace.
#[derive(Debug, Clone)]
pub(crate) struct WorkspaceLayout {
    /// The language workspace root.
    pub language_root: PathBuf,
}

impl WorkspaceLayout {
    /// Build one generator layout from the runtime crate root.
    pub(crate) fn from_runtime_crate() -> Self {
        let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let language_root = runtime_root
            .parent()
            .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"))
            .to_path_buf();

        Self { language_root }
    }

    /// Build the file layout for one platform module.
    pub(crate) fn module_layout(&self, module: &str) -> ModuleLayout {
        let platform_dir = self
            .language_root
            .join(format!("runtime/src/platform/{module}"));
        let simulation_dir = platform_dir.join("simulation");
        let unix_dir = platform_dir.join("unix");
        let windows_dir = platform_dir.join("windows");

        ModuleLayout {
            dir: platform_dir.clone(),
            mod_path: platform_dir.join("mod.rs"),
            abi_types_path: platform_dir.join("abi.generated.rs"),
            bindings_path: platform_dir.join("bindings.generated.rs"),
            host_path: platform_dir.join("host.rs"),
            native_path: platform_dir.join("native.rs"),
            vm_path: platform_dir.join("vm.rs"),
            unsupported_path: platform_dir.join("unsupported.rs"),
            simulation_mod_path: simulation_dir.join("mod.rs"),
            simulation_native_path: simulation_dir.join("native.rs"),
            simulation_vm_path: simulation_dir.join("vm.rs"),
            unix_mod_path: unix_dir.join("mod.rs"),
            windows_mod_path: windows_dir.join("mod.rs"),
        }
    }

    /// Return the generated platform index path.
    pub(crate) fn platform_generated_path(&self) -> PathBuf {
        self.language_root.join("runtime/src/platform/generated.rs")
    }
}
