use serde::Deserialize;

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
pub enum LinkerFlavor {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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
