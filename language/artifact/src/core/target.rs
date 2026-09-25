use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Artifact produced by one build target.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// Linked JavaScript files and resources.
    #[default]
    Bundle,
    /// One TS++ Program.
    Program,
}

impl Output {
    /// Return the lowercase configuration name.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Bundle => "bundle",
            Self::Program => "program",
        }
    }
}

/// Executable representation included in one Program.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Code {
    /// TS++ bytecode.
    Bytecode,
    /// Native machine code.
    Native,
    /// WebAssembly.
    Wasm,
}

impl Code {
    /// Return the lowercase configuration name.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Bytecode => "bytecode",
            Self::Native => "native",
            Self::Wasm => "wasm",
        }
    }
}

/// Semantic runtime contract for compiled code.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    /// TS++ native runtime.
    #[default]
    Tspp,
    /// JavaScript host runtime.
    Js,
}

impl std::str::FromStr for Runtime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tspp" => Ok(Self::Tspp),
            "js" | "javascript" => Ok(Self::Js),
            _ => Err(()),
        }
    }
}

impl Runtime {
    /// Return the canonical lowercase tag for this runtime.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Tspp => "tspp",
            Self::Js => "js",
        }
    }

    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this runtime is a JS engine.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }
}

/// Build distribution profile.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BuildProfile {
    /// Full build.
    #[default]
    Full,
    /// Smaller build with optional services omitted when possible.
    Minimal,
    /// Freestanding output without the normal TS++ runtime contract.
    Freestanding,
}

impl std::str::FromStr for BuildProfile {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "minimal" => Ok(Self::Minimal),
            "freestanding" => Ok(Self::Freestanding),
            _ => Err(()),
        }
    }
}

impl BuildProfile {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Build payload linkage.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BuildLinkage {
    /// Ship a portable TS++ payload consumed by a runtime.
    #[default]
    Portable,
    /// Link the build payload into the produced platform binary.
    Static,
    /// Ship the build payload as a dynamic library.
    Dynamic,
}

impl std::str::FromStr for BuildLinkage {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "portable" => Ok(Self::Portable),
            "static" => Ok(Self::Static),
            "dynamic" => Ok(Self::Dynamic),
            _ => Err(()),
        }
    }
}

impl BuildLinkage {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Operating system component of a target.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Unknown operating system.
    #[default]
    Unknown,
    /// Windows.
    Windows,
    /// macOS.
    MacOS,
    /// Linux.
    Linux,
    /// FreeBSD.
    FreeBsd,
    /// OpenBSD.
    OpenBsd,
    /// NetBSD.
    NetBsd,
    /// DragonFly BSD.
    DragonFly,
    /// Solaris.
    Solaris,
    /// Illumos.
    Illumos,
    /// Haiku.
    Haiku,
    /// Fuchsia.
    Fuchsia,
    /// Redox.
    Redox,
    /// Hermit.
    Hermit,
    /// No operating system.
    None,
}

impl std::str::FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unknown" => Ok(Self::Unknown),
            "windows" | "win32" | "win" => Ok(Self::Windows),
            "macos" | "darwin" | "mac" => Ok(Self::MacOS),
            "linux" => Ok(Self::Linux),
            "freebsd" => Ok(Self::FreeBsd),
            "openbsd" => Ok(Self::OpenBsd),
            "netbsd" => Ok(Self::NetBsd),
            "dragonfly" | "dragonflybsd" => Ok(Self::DragonFly),
            "solaris" => Ok(Self::Solaris),
            "illumos" => Ok(Self::Illumos),
            "haiku" => Ok(Self::Haiku),
            "fuchsia" => Ok(Self::Fuchsia),
            "redox" => Ok(Self::Redox),
            "hermit" | "hermitos" => Ok(Self::Hermit),
            "none" => Ok(Self::None),
            _ => Err(()),
        }
    }
}

impl Platform {
    /// Return the canonical lowercase tag for this platform.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Windows => "windows",
            Self::MacOS => "macos",
            Self::Linux => "linux",
            Self::FreeBsd => "freebsd",
            Self::OpenBsd => "openbsd",
            Self::NetBsd => "netbsd",
            Self::DragonFly => "dragonfly",
            Self::Solaris => "solaris",
            Self::Illumos => "illumos",
            Self::Haiku => "haiku",
            Self::Fuchsia => "fuchsia",
            Self::Redox => "redox",
            Self::Hermit => "hermit",
            Self::None => "none",
        }
    }

    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this is a Unix-style platform.
    pub fn is_unix(&self) -> bool {
        matches!(
            self,
            Self::MacOS
                | Self::Linux
                | Self::FreeBsd
                | Self::OpenBsd
                | Self::NetBsd
                | Self::DragonFly
                | Self::Solaris
                | Self::Illumos
                | Self::Haiku
        )
    }

    /// Return the target family tag used by `import.meta.target.family`.
    pub fn family_tag(&self) -> &'static str {
        if matches!(self, Self::Windows) {
            return "windows";
        }

        if self.is_unix() {
            return "unix";
        }

        if matches!(self, Self::None) {
            return "bare-metal";
        }

        if matches!(self, Self::Unknown) {
            return "unknown";
        }

        "other"
    }

    /// Resolve the target triple OS component for this platform.
    pub fn triple_os_component(&self) -> Option<&'static str> {
        match self {
            Self::Windows => Some("windows"),
            Self::MacOS => Some("darwin"),
            Self::Linux => Some("linux"),
            Self::FreeBsd => Some("freebsd"),
            Self::OpenBsd => Some("openbsd"),
            Self::NetBsd => Some("netbsd"),
            Self::DragonFly => Some("dragonfly"),
            Self::Solaris => Some("solaris"),
            Self::Illumos => Some("illumos"),
            Self::Haiku => Some("haiku"),
            Self::Fuchsia => Some("fuchsia"),
            Self::Redox => Some("redox"),
            Self::Hermit => Some("hermit"),
            Self::None => Some("none"),
            Self::Unknown => None,
        }
    }
}

/// Host environment that provides target imports and ambient effects.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Host {
    /// Native host environment.
    Native,
    /// Browser host environment.
    Browser,
    /// WASI host environment.
    Wasi,
    /// Emscripten host environment.
    Emscripten,
    /// Freestanding target without host imports.
    Freestanding,
}

impl std::str::FromStr for Host {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "native" => Ok(Self::Native),
            "browser" | "web" => Ok(Self::Browser),
            "wasi" => Ok(Self::Wasi),
            "emscripten" | "emscripten-wasm" => Ok(Self::Emscripten),
            "freestanding" | "bare-metal" | "bare_metal" | "baremetal" | "none" => {
                Ok(Self::Freestanding)
            }
            _ => Err(()),
        }
    }
}

impl Host {
    /// Return the canonical lowercase tag for this host.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Browser => "browser",
            Self::Wasi => "wasi",
            Self::Emscripten => "emscripten",
            Self::Freestanding => "freestanding",
        }
    }

    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this host is a WebAssembly host environment.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Browser | Self::Wasi | Self::Emscripten)
    }

    /// Resolve the target triple system component for host-defined targets.
    pub fn triple_system_component(&self) -> Option<&'static str> {
        match self {
            Self::Wasi => Some("wasi"),
            Self::Emscripten => Some("emscripten"),
            Self::Freestanding => Some("none"),
            Self::Native | Self::Browser => None,
        }
    }
}

/// CPU architecture for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TargetArch {
    /// x86_64.
    X86_64,
    /// x86.
    X86,
    /// AArch64.
    Aarch64,
    /// ARMv7.
    Armv7,
    /// ARMv6.
    Armv6,
    /// RISC-V 64-bit.
    Riscv64,
    /// RISC-V 32-bit.
    Riscv32,
    /// PowerPC 64-bit.
    PowerPc64,
    /// PowerPC 64-bit little-endian.
    PowerPc64le,
    /// s390x.
    S390x,
    /// MIPS64.
    Mips64,
    /// MIPS64 little-endian.
    Mips64el,
    /// LoongArch64.
    LoongArch64,
    /// WebAssembly 32-bit.
    Wasm32,
    /// WebAssembly 64-bit.
    Wasm64,
    /// Other architecture.
    Other(String),
}

impl std::str::FromStr for TargetArch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "x86_64" | "amd64" => Self::X86_64,
            "x86" | "i686" | "i586" | "i386" => Self::X86,
            "aarch64" | "arm64" => Self::Aarch64,
            "armv7" | "armv7l" => Self::Armv7,
            "armv6" | "armv6l" => Self::Armv6,
            "riscv64" => Self::Riscv64,
            "riscv32" => Self::Riscv32,
            "powerpc64" | "ppc64" => Self::PowerPc64,
            "powerpc64le" | "ppc64le" => Self::PowerPc64le,
            "s390x" => Self::S390x,
            "mips64" => Self::Mips64,
            "mips64el" | "mips64le" => Self::Mips64el,
            "loongarch64" | "loong64" => Self::LoongArch64,
            "wasm32" => Self::Wasm32,
            "wasm64" => Self::Wasm64,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetArch {
    /// Parse a target architecture from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Return this architecture's target triple component.
    pub fn triple_component(&self) -> &str {
        match self {
            Self::X86_64 => "x86_64",
            Self::X86 => "i686",
            Self::Aarch64 => "aarch64",
            Self::Armv7 => "armv7",
            Self::Armv6 => "armv6",
            Self::Riscv64 => "riscv64",
            Self::Riscv32 => "riscv32",
            Self::PowerPc64 => "powerpc64",
            Self::PowerPc64le => "powerpc64le",
            Self::S390x => "s390x",
            Self::Mips64 => "mips64",
            Self::Mips64el => "mips64el",
            Self::LoongArch64 => "loongarch64",
            Self::Wasm32 => "wasm32",
            Self::Wasm64 => "wasm64",
            Self::Other(value) => value,
        }
    }

    /// Return the architecture pointer width in bytes when known.
    pub fn pointer_bytes(&self) -> Option<u8> {
        match self {
            Self::X86_64
            | Self::Aarch64
            | Self::Riscv64
            | Self::PowerPc64
            | Self::PowerPc64le
            | Self::S390x
            | Self::Mips64
            | Self::Mips64el
            | Self::LoongArch64
            | Self::Wasm64 => Some(8),
            Self::X86 | Self::Armv7 | Self::Armv6 | Self::Riscv32 | Self::Wasm32 => Some(4),
            Self::Other(_) => None,
        }
    }
}

/// Target vendor for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TargetVendor {
    /// Unknown vendor.
    Unknown,
    /// Apple.
    Apple,
    /// PC.
    Pc,
    /// IBM.
    Ibm,
    /// Nintendo.
    Nintendo,
    /// Other vendor.
    Other(String),
}

impl std::str::FromStr for TargetVendor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "unknown" => Self::Unknown,
            "apple" => Self::Apple,
            "pc" => Self::Pc,
            "ibm" => Self::Ibm,
            "nintendo" => Self::Nintendo,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetVendor {
    /// Parse a target vendor from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Resolve the default vendor for a platform when none is specified.
    pub fn default_for_platform(platform: Platform) -> Self {
        match platform {
            Platform::Windows => Self::Pc,
            Platform::MacOS => Self::Apple,
            _ => Self::Unknown,
        }
    }

    /// Return this vendor's target triple component.
    pub fn triple_component(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::Apple => "apple",
            Self::Pc => "pc",
            Self::Ibm => "ibm",
            Self::Nintendo => "nintendo",
            Self::Other(value) => value,
        }
    }
}

/// Target ABI flavor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TargetAbi {
    /// GNU environment.
    Gnu,
    /// Musl environment.
    Musl,
    /// MSVC environment.
    Msvc,
    /// GNU + LLVM environment.
    GnuLlvm,
    /// EABI.
    Eabi,
    /// EABI with hard-float.
    Eabihf,
    /// Musl + EABI.
    MuslEabi,
    /// Musl + EABI hard-float.
    MuslEabihf,
    /// Other environment.
    Other(String),
}

impl std::str::FromStr for TargetAbi {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "gnu" => Self::Gnu,
            "musl" => Self::Musl,
            "msvc" => Self::Msvc,
            "gnullvm" | "gnu_llvm" => Self::GnuLlvm,
            "eabi" => Self::Eabi,
            "eabihf" => Self::Eabihf,
            "musleabi" => Self::MuslEabi,
            "musleabihf" => Self::MuslEabihf,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetAbi {
    /// Parse a target ABI from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Resolve the default ABI for a platform when none is specified.
    pub fn default_for_platform(platform: Platform) -> Option<Self> {
        match platform {
            Platform::Linux => Some(Self::Gnu),
            Platform::Windows => Some(Self::Msvc),
            _ => None,
        }
    }

    /// Return this ABI's target triple component.
    pub fn triple_component(&self) -> &str {
        match self {
            Self::Gnu => "gnu",
            Self::Musl => "musl",
            Self::Msvc => "msvc",
            Self::GnuLlvm => "gnullvm",
            Self::Eabi => "eabi",
            Self::Eabihf => "eabihf",
            Self::MuslEabi => "musleabi",
            Self::MuslEabihf => "musleabihf",
            Self::Other(value) => value,
        }
    }
}
