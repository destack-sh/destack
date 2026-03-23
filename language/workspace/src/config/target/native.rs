use std::path::PathBuf;

use serde::Deserialize;

use super::optimization::*;

/// Native output kind for one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativeOutputKind {
    /// Emit one executable program.
    #[default]
    Executable,
    /// Emit one static library archive.
    StaticLibrary,
    /// Emit one shared or dynamic library.
    SharedLibrary,
}

/// Linker driver family for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativeLinkerFlavor {
    /// Use the compiler default linker driver.
    #[default]
    Auto,
    /// Use one direct `ld` style linker driver.
    Ld,
    /// Use one C compiler style driver.
    Cc,
    /// Use clang as the linker driver.
    Clang,
    /// Use lld as the linker driver.
    Lld,
    /// Use mold as the linker driver.
    Mold,
    /// Use the MSVC linker driver.
    Msvc,
    /// Use the wasm-ld linker driver.
    WasmLd,
}

/// Position independent code policy for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativePositionIndependentMode {
    /// Use the default policy for the target platform.
    #[default]
    Default,
    /// Disable position independent output.
    Disabled,
    /// Emit a position independent executable.
    Pie,
    /// Emit a static position independent executable.
    StaticPie,
}

/// C runtime linkage policy for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativeCrtLinkage {
    /// Use the toolchain default runtime linkage.
    #[default]
    Default,
    /// Link against one dynamic C runtime.
    Dynamic,
    /// Link against one static C runtime.
    Static,
}

/// Symbol visibility policy for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativeSymbolVisibility {
    /// Use the toolchain default visibility policy.
    #[default]
    Default,
    /// Prefer hidden visibility for non-exported symbols.
    Hidden,
    /// Prefer protected visibility for exported symbols.
    Protected,
}

/// Normalized native code generation and linking options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetNative {
    /// Native output kind for this target.
    pub output: NativeOutputKind,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: NativeLinkerFlavor,
    /// Extra linker arguments for native targets.
    pub link_args: Vec<String>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<PathBuf>,
    /// Additional library search paths.
    pub library_search_paths: Vec<PathBuf>,
    /// Additional libraries to link.
    pub libraries: Vec<String>,
    /// Additional framework search paths.
    pub framework_search_paths: Vec<PathBuf>,
    /// Additional frameworks to link.
    pub frameworks: Vec<String>,
    /// Runtime search paths embedded into the final output.
    pub rpath: Vec<String>,
    /// Runtime search paths emitted as runpath entries.
    pub runpath: Vec<String>,
    /// Explicit entry symbol override.
    pub entry_symbol: Option<String>,
    /// Explicitly exported symbol names.
    pub export_symbols: Vec<String>,
    /// Symbol visibility policy.
    pub symbol_visibility: NativeSymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: NativePositionIndependentMode,
    /// C runtime linkage policy.
    pub crt: NativeCrtLinkage,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
}

impl From<&TargetNativeJson> for TargetNative {
    fn from(json: &TargetNativeJson) -> Self {
        Self {
            output: json.output.unwrap_or_default(),
            link_mode: json.link_mode.map(LinkMode::from).unwrap_or_default(),
            linker: json.linker.clone(),
            linker_flavor: json.linker_flavor.unwrap_or_default(),
            link_args: json.link_args.clone().unwrap_or_default(),
            sysroot: json.sysroot.as_ref().map(PathBuf::from),
            library_search_paths: json
                .library_search_paths
                .as_ref()
                .map(|paths| paths.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            libraries: json.libraries.clone().unwrap_or_default(),
            framework_search_paths: json
                .framework_search_paths
                .as_ref()
                .map(|paths| paths.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            frameworks: json.frameworks.clone().unwrap_or_default(),
            rpath: json.rpath.clone().unwrap_or_default(),
            runpath: json.runpath.clone().unwrap_or_default(),
            entry_symbol: json.entry_symbol.clone(),
            export_symbols: json.export_symbols.clone().unwrap_or_default(),
            symbol_visibility: json.symbol_visibility.unwrap_or_default(),
            version_script: json.version_script.as_ref().map(PathBuf::from),
            linker_script: json.linker_script.as_ref().map(PathBuf::from),
            position_independent: json.position_independent.unwrap_or_default(),
            crt: json.crt.unwrap_or_default(),
            soname: json.soname.clone(),
            install_name: json.install_name.clone(),
        }
    }
}

/// Native code generation and linking options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetNativeJson {
    /// Native output kind for this target.
    pub output: Option<NativeOutputKind>,
    /// Link mode for native targets.
    pub link_mode: Option<LinkModeJson>,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: Option<NativeLinkerFlavor>,
    /// Extra linker arguments for native targets.
    pub link_args: Option<Vec<String>>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<String>,
    /// Additional library search paths.
    pub library_search_paths: Option<Vec<String>>,
    /// Additional libraries to link.
    pub libraries: Option<Vec<String>>,
    /// Additional framework search paths.
    pub framework_search_paths: Option<Vec<String>>,
    /// Additional frameworks to link.
    pub frameworks: Option<Vec<String>>,
    /// Runtime search paths embedded into the final output.
    pub rpath: Option<Vec<String>>,
    /// Runtime search paths emitted as runpath entries.
    pub runpath: Option<Vec<String>>,
    /// Explicit entry symbol override.
    pub entry_symbol: Option<String>,
    /// Explicitly exported symbol names.
    pub export_symbols: Option<Vec<String>>,
    /// Symbol visibility policy.
    pub symbol_visibility: Option<NativeSymbolVisibility>,
    /// Version script for exported symbols.
    pub version_script: Option<String>,
    /// Linker script for the final link.
    pub linker_script: Option<String>,
    /// Position independent code policy.
    pub position_independent: Option<NativePositionIndependentMode>,
    /// C runtime linkage policy.
    pub crt: Option<NativeCrtLinkage>,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
}
