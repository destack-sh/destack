use std::path::PathBuf;

use destack_artifact::{TargetAbi, TargetArch, TargetVendor};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native target configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetNativeOptions {
    /// Native output shape.
    pub output: NativeOutputKind,
    /// Target architecture for native codegen.
    pub arch: Option<TargetArch>,
    /// Target vendor for native codegen.
    pub vendor: Option<TargetVendor>,
    /// Target ABI for native codegen.
    pub abi: Option<TargetAbi>,
    /// CPU name for native codegen.
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen.
    pub cpu_features: Vec<String>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<PathBuf>,
    /// C runtime linkage policy.
    pub crt: CrtLinkage,
    /// Native linker configuration.
    pub link: TargetLinkOptions,
}

/// Target native linker configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetLinkOptions {
    /// Explicit linker executable.
    pub linker: Option<String>,
    /// Extra linker arguments.
    pub args: Vec<String>,
    /// Additional library search paths.
    pub library_paths: Vec<PathBuf>,
    /// Additional libraries to link.
    pub libraries: Vec<String>,
    /// Additional framework search paths.
    pub framework_paths: Vec<PathBuf>,
    /// Additional frameworks to link.
    pub frameworks: Vec<String>,
    /// Runtime dynamic library search paths.
    pub runtime_library_paths: Vec<String>,
    /// Symbol visibility policy.
    pub symbol_visibility: SymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: PositionIndependentMode,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
}

/// Native output kind for one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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

/// Position independent code policy for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PositionIndependentMode {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum CrtLinkage {
    /// Use the toolchain default runtime linkage.
    #[default]
    Default,
    /// Link against one dynamic C runtime.
    Dynamic,
    /// Link against one static C runtime.
    Static,
}

/// Symbol visibility policy for one native target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SymbolVisibility {
    /// Use the toolchain default visibility policy.
    #[default]
    Default,
    /// Prefer hidden visibility for non-exported symbols.
    Hidden,
    /// Prefer protected visibility for exported symbols.
    Protected,
}
