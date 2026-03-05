use serde::{Deserialize, Serialize};

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OptimizeLevel {
    /// No optimization (O0).
    #[default]
    O0,
    /// Basic optimization (O1).
    O1,
    /// Standard optimization (O2).
    O2,
    /// Aggressive optimization (O3).
    O3,
    /// Maximal optimization (O4).
    O4,
}

/// Link time optimization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LtoMode {
    /// Use defaults based on optimization level.
    /// Auto enables Thin LTO at O4 and disables LTO at lower levels.
    #[default]
    Auto,
    /// Disable link time optimization.
    None,
    /// Enable Thin LTO at package scope.
    Thin,
    /// Enable Full LTO at program scope.
    Full,
}

impl std::str::FromStr for LtoMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "none" | "off" | "disabled" => Ok(Self::None),
            "thin" | "thinlto" | "thin_lto" => Ok(Self::Thin),
            "full" | "lto" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl LtoMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            3 => Self::O3,
            _ => Self::O4,
        }
    }
}

impl From<OptimizeLevel> for u8 {
    fn from(level: OptimizeLevel) -> Self {
        match level {
            OptimizeLevel::O0 => 0,
            OptimizeLevel::O1 => 1,
            OptimizeLevel::O2 => 2,
            OptimizeLevel::O3 => 3,
            OptimizeLevel::O4 => 4,
        }
    }
}

/// Shrink level for builds (code size reduction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShrinkLevel {
    /// No shrinking (S0).
    #[default]
    S0,
    /// Basic shrinking (S1).
    S1,
    /// Standard shrinking (S2).
    S2,
    /// Aggressive shrinking (S3).
    S3,
}

/// Relocation model for native codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RelocationModel {
    /// Static relocation model.
    Static,
    /// Position-independent code.
    #[default]
    Pic,
    /// Position-independent executable.
    Pie,
}

impl std::str::FromStr for RelocationModel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "static" => Ok(Self::Static),
            "pic" => Ok(Self::Pic),
            "pie" => Ok(Self::Pie),
            _ => Err(()),
        }
    }
}

impl RelocationModel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Link mode for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LinkMode {
    /// Prefer static linking.
    Static,
    /// Prefer dynamic linking.
    #[default]
    Dynamic,
}

impl std::str::FromStr for LinkMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "static" => Ok(Self::Static),
            "dynamic" | "shared" => Ok(Self::Dynamic),
            _ => Err(()),
        }
    }
}

impl LinkMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Debug info emission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DebugInfoLevel {
    /// No debug info.
    #[default]
    None,
    /// Line tables only.
    Line,
    /// Full debug info.
    Full,
}

impl std::str::FromStr for DebugInfoLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "line" | "lines" => Ok(Self::Line),
            "full" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl DebugInfoLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Debug execution mode for VM/native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DebugMode {
    /// Choose mode based on target debug settings.
    #[default]
    Auto,
    /// Always run the interpreter.
    Vm,
    /// Run native with deopt-first debugging.
    Deopt,
    /// Run native only (no deopt).
    Native,
}

impl std::str::FromStr for DebugMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "vm" | "interpreter" => Ok(Self::Vm),
            "deopt" => Ok(Self::Deopt),
            "native" => Ok(Self::Native),
            _ => Err(()),
        }
    }
}

impl DebugMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// OSR entry mode for native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OsrMode {
    /// OSR disabled.
    Disabled,
    /// OSR at loop headers.
    #[default]
    LoopHeaders,
    /// OSR only at explicitly marked sites.
    Explicit,
}

impl std::str::FromStr for OsrMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "disabled" | "off" => Ok(Self::Disabled),
            "loop_headers" | "loops" => Ok(Self::LoopHeaders),
            "explicit" => Ok(Self::Explicit),
            _ => Err(()),
        }
    }
}

impl OsrMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Safepoint insertion mode for native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SafepointMode {
    /// Call sites, allocation points, and loop back-edges only.
    #[default]
    CallsAllocBackEdges,
    /// Add instruction-budget safepoints for bounded latency.
    Budgeted,
}

impl std::str::FromStr for SafepointMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "calls_alloc_backedges" | "standard" => Ok(Self::CallsAllocBackEdges),
            "budgeted" | "budget" => Ok(Self::Budgeted),
            _ => Err(()),
        }
    }
}

impl SafepointMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Speculation mode for native optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpeculationMode {
    /// Disable speculative optimizations.
    None,
    /// Guarded speculations with explicit deopt metadata.
    #[default]
    Guarded,
    /// Aggressive speculation across more sites.
    Aggressive,
}

impl std::str::FromStr for SpeculationMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" | "off" => Ok(Self::None),
            "guarded" => Ok(Self::Guarded),
            "aggressive" => Ok(Self::Aggressive),
            _ => Err(()),
        }
    }
}

impl SpeculationMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Profiling mode for tiering and optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProfilingMode {
    /// Disable runtime profiling collection.
    None,
    /// Counters only (calls, branches, allocations).
    Counters,
    /// Sampling only (periodic opcode and site sampling).
    Sampling,
    /// Counters + sampling + inline caches.
    #[default]
    Hybrid,
}

impl std::str::FromStr for ProfilingMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" | "off" => Ok(Self::None),
            "counters" => Ok(Self::Counters),
            "sampling" => Ok(Self::Sampling),
            "hybrid" | "full" => Ok(Self::Hybrid),
            _ => Err(()),
        }
    }
}

impl ProfilingMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Symbol stripping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StripLevel {
    /// Keep all symbols.
    #[default]
    None,
    /// Strip local symbols.
    Partial,
    /// Strip all symbols.
    Full,
}

impl std::str::FromStr for StripLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "partial" => Ok(Self::Partial),
            "full" | "all" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl StripLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Global allocator selection for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Allocator {
    /// Use the platform default allocator.
    #[default]
    System,
    /// Use mimalloc.
    MiMalloc,
    /// Use jemalloc.
    JeMalloc,
    /// Use a custom allocator provided by the runtime.
    Custom,
}

impl std::str::FromStr for Allocator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "system" => Ok(Self::System),
            "mimalloc" => Ok(Self::MiMalloc),
            "jemalloc" => Ok(Self::JeMalloc),
            "custom" => Ok(Self::Custom),
            _ => Err(()),
        }
    }
}

impl Allocator {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl From<u8> for ShrinkLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::S0,
            1 => Self::S1,
            2 => Self::S2,
            _ => Self::S3,
        }
    }
}

impl From<ShrinkLevel> for u8 {
    fn from(level: ShrinkLevel) -> Self {
        match level {
            ShrinkLevel::S0 => 0,
            ShrinkLevel::S1 => 1,
            ShrinkLevel::S2 => 2,
            ShrinkLevel::S3 => 3,
        }
    }
}

/// Relocation model for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RelocationModelJson {
    /// Static relocation model.
    #[serde(alias = "static")]
    Static,
    /// Position-independent code.
    #[serde(alias = "position_independent")]
    #[serde(alias = "position_independent_code")]
    #[serde(alias = "position_independent_executable")]
    Pic,
    /// Position-independent executable.
    #[serde(alias = "pie")]
    Pie,
}

impl From<RelocationModelJson> for RelocationModel {
    fn from(value: RelocationModelJson) -> Self {
        match value {
            RelocationModelJson::Static => RelocationModel::Static,
            RelocationModelJson::Pic => RelocationModel::Pic,
            RelocationModelJson::Pie => RelocationModel::Pie,
        }
    }
}

/// Link mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LinkModeJson {
    /// Prefer static linking.
    #[serde(alias = "static")]
    Static,
    /// Prefer dynamic linking.
    #[serde(alias = "shared")]
    Dynamic,
}

impl From<LinkModeJson> for LinkMode {
    fn from(value: LinkModeJson) -> Self {
        match value {
            LinkModeJson::Static => LinkMode::Static,
            LinkModeJson::Dynamic => LinkMode::Dynamic,
        }
    }
}

/// Debug info emission policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DebugInfoLevelJson {
    /// No debug info.
    #[serde(alias = "off")]
    None,
    /// Line tables only.
    #[serde(alias = "lines")]
    #[serde(alias = "line_tables")]
    Line,
    /// Full debug info.
    #[serde(alias = "full")]
    Full,
}

impl From<DebugInfoLevelJson> for DebugInfoLevel {
    fn from(value: DebugInfoLevelJson) -> Self {
        match value {
            DebugInfoLevelJson::None => DebugInfoLevel::None,
            DebugInfoLevelJson::Line => DebugInfoLevel::Line,
            DebugInfoLevelJson::Full => DebugInfoLevel::Full,
        }
    }
}

/// Debug execution mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DebugModeJson {
    /// Choose mode based on target debug settings.
    #[serde(alias = "default")]
    Auto,
    /// Always run the interpreter.
    #[serde(alias = "interpreter")]
    Vm,
    /// Run native with deopt-first debugging.
    Deopt,
    /// Run native only (no deopt).
    Native,
}

impl From<DebugModeJson> for DebugMode {
    fn from(value: DebugModeJson) -> Self {
        match value {
            DebugModeJson::Auto => DebugMode::Auto,
            DebugModeJson::Vm => DebugMode::Vm,
            DebugModeJson::Deopt => DebugMode::Deopt,
            DebugModeJson::Native => DebugMode::Native,
        }
    }
}

/// OSR mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OsrModeJson {
    /// OSR disabled.
    #[serde(alias = "off")]
    Disabled,
    /// OSR at loop headers.
    #[serde(alias = "loops")]
    #[serde(alias = "loop-headers")]
    LoopHeaders,
    /// OSR only at explicit sites.
    Explicit,
}

impl From<OsrModeJson> for OsrMode {
    fn from(value: OsrModeJson) -> Self {
        match value {
            OsrModeJson::Disabled => OsrMode::Disabled,
            OsrModeJson::LoopHeaders => OsrMode::LoopHeaders,
            OsrModeJson::Explicit => OsrMode::Explicit,
        }
    }
}

/// Safepoint mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SafepointModeJson {
    /// Safepoints at calls, allocations, and loop back-edges.
    #[serde(rename = "calls-alloc-backedges")]
    #[serde(alias = "calls_alloc_backedges")]
    #[serde(alias = "standard")]
    CallsAllocBackEdges,
    /// Add instruction-budget safepoints for bounded latency.
    #[serde(alias = "budget")]
    Budgeted,
}

impl From<SafepointModeJson> for SafepointMode {
    fn from(value: SafepointModeJson) -> Self {
        match value {
            SafepointModeJson::CallsAllocBackEdges => SafepointMode::CallsAllocBackEdges,
            SafepointModeJson::Budgeted => SafepointMode::Budgeted,
        }
    }
}

/// Speculation mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SpeculationModeJson {
    /// Disable speculative optimizations.
    #[serde(alias = "off")]
    None,
    /// Guarded speculations with explicit deopt metadata.
    Guarded,
    /// Aggressive speculation across more sites.
    Aggressive,
}

impl From<SpeculationModeJson> for SpeculationMode {
    fn from(value: SpeculationModeJson) -> Self {
        match value {
            SpeculationModeJson::None => SpeculationMode::None,
            SpeculationModeJson::Guarded => SpeculationMode::Guarded,
            SpeculationModeJson::Aggressive => SpeculationMode::Aggressive,
        }
    }
}

/// Profiling mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ProfilingModeJson {
    /// Disable runtime profiling.
    #[serde(alias = "off")]
    None,
    /// Counters only.
    Counters,
    /// Sampling only.
    Sampling,
    /// Counters + sampling + inline caches.
    #[serde(alias = "full")]
    Hybrid,
}

impl From<ProfilingModeJson> for ProfilingMode {
    fn from(value: ProfilingModeJson) -> Self {
        match value {
            ProfilingModeJson::None => ProfilingMode::None,
            ProfilingModeJson::Counters => ProfilingMode::Counters,
            ProfilingModeJson::Sampling => ProfilingMode::Sampling,
            ProfilingModeJson::Hybrid => ProfilingMode::Hybrid,
        }
    }
}

/// Link time optimization mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LtoModeJson {
    /// Choose mode based on optimization level.
    #[serde(alias = "auto")]
    Auto,
    /// Disable link time optimization.
    #[serde(alias = "none")]
    #[serde(alias = "off")]
    #[serde(alias = "disabled")]
    None,
    /// Enable thin link time optimization.
    #[serde(alias = "thin")]
    #[serde(alias = "thinlto")]
    #[serde(alias = "thin_lto")]
    Thin,
    /// Enable full link time optimization.
    #[serde(alias = "full")]
    #[serde(alias = "lto")]
    Full,
}

impl From<LtoModeJson> for LtoMode {
    fn from(value: LtoModeJson) -> Self {
        match value {
            LtoModeJson::Auto => LtoMode::Auto,
            LtoModeJson::None => LtoMode::None,
            LtoModeJson::Thin => LtoMode::Thin,
            LtoModeJson::Full => LtoMode::Full,
        }
    }
}

/// Symbol stripping policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum StripLevelJson {
    /// Keep all symbols.
    #[serde(alias = "none")]
    None,
    /// Strip local symbols.
    #[serde(alias = "locals")]
    Partial,
    /// Strip all symbols.
    #[serde(alias = "all")]
    Full,
}

impl From<StripLevelJson> for StripLevel {
    fn from(value: StripLevelJson) -> Self {
        match value {
            StripLevelJson::None => StripLevel::None,
            StripLevelJson::Partial => StripLevel::Partial,
            StripLevelJson::Full => StripLevel::Full,
        }
    }
}

/// Allocator selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AllocatorJson {
    /// Use the platform default allocator.
    #[serde(alias = "system")]
    System,
    /// Use mimalloc.
    #[serde(alias = "mimalloc")]
    #[serde(alias = "mi_malloc")]
    MiMalloc,
    /// Use jemalloc.
    #[serde(alias = "jemalloc")]
    #[serde(alias = "je_malloc")]
    JeMalloc,
    /// Use a custom allocator provided by the runtime.
    #[serde(alias = "custom")]
    Custom,
}

impl From<AllocatorJson> for Allocator {
    fn from(value: AllocatorJson) -> Self {
        match value {
            AllocatorJson::System => Allocator::System,
            AllocatorJson::MiMalloc => Allocator::MiMalloc,
            AllocatorJson::JeMalloc => Allocator::JeMalloc,
            AllocatorJson::Custom => Allocator::Custom,
        }
    }
}
