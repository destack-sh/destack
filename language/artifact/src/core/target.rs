use serde::{Deserialize, Serialize};

use destack_builtin::{BuiltinPlatform, BuiltinRuntime};

/// Runtime environment that actually executes the compiled code.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum Runtime {
    /// Web browser.
    Browser,
    /// Node.js.
    #[default]
    Node,
    /// Deno.
    Deno,
    /// Bun.
    Bun,
    /// Web Worker style host.
    Worker,
    /// WASM running in a JS host.
    WasmJs,
    /// WASM with WASI.
    WasmWasi,
    /// Native managed runtime.
    NativeManaged,
    /// Native freestanding runtime.
    NativeFreestanding,
    /// Native embedded runtime.
    NativeEmbedded,
}

impl From<Runtime> for BuiltinRuntime {
    fn from(value: Runtime) -> Self {
        match value {
            Runtime::Browser => BuiltinRuntime::Browser,
            Runtime::Node => BuiltinRuntime::Node,
            Runtime::Deno => BuiltinRuntime::Deno,
            Runtime::Bun => BuiltinRuntime::Bun,
            Runtime::Worker => BuiltinRuntime::Worker,
            Runtime::WasmJs => BuiltinRuntime::WasmJs,
            Runtime::WasmWasi => BuiltinRuntime::WasmWasi,
            Runtime::NativeManaged => BuiltinRuntime::NativeManaged,
            Runtime::NativeFreestanding => BuiltinRuntime::NativeFreestanding,
            Runtime::NativeEmbedded => BuiltinRuntime::NativeEmbedded,
        }
    }
}

impl std::str::FromStr for Runtime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "browser" => Ok(Self::Browser),
            "node" => Ok(Self::Node),
            "deno" => Ok(Self::Deno),
            "bun" => Ok(Self::Bun),
            "worker" => Ok(Self::Worker),
            "wasm_js" | "wasm-js" | "wasmjs" => Ok(Self::WasmJs),
            "wasm_wasi" | "wasm-wasi" | "wasmwasi" | "wasi" => Ok(Self::WasmWasi),
            "native" | "native_managed" | "native-managed" => Ok(Self::NativeManaged),
            "native_freestanding" | "native-freestanding" => Ok(Self::NativeFreestanding),
            "native_embedded" | "native-embedded" => Ok(Self::NativeEmbedded),
            _ => Err(()),
        }
    }
}

impl Runtime {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this runtime is a JS engine.
    pub fn is_js(&self) -> bool {
        matches!(
            self,
            Self::Browser | Self::Node | Self::Deno | Self::Bun | Self::Worker
        )
    }

    /// Whether this runtime is a WASM host.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::WasmJs | Self::WasmWasi)
    }

    /// Whether this runtime is a native runtime.
    pub fn is_native(&self) -> bool {
        matches!(
            self,
            Self::NativeManaged | Self::NativeFreestanding | Self::NativeEmbedded
        )
    }

    /// Whether this runtime is browser-like.
    pub fn is_browser_like(&self) -> bool {
        matches!(self, Self::Browser | Self::WasmJs)
    }

    /// Whether this runtime is server-side.
    pub fn is_server(&self) -> bool {
        matches!(
            self,
            Self::Node
                | Self::Deno
                | Self::Bun
                | Self::WasmWasi
                | Self::NativeManaged
                | Self::NativeFreestanding
                | Self::NativeEmbedded
        )
    }
}

/// Operating system or target platform.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum Platform {
    /// Web browser.
    #[default]
    Web,
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
    /// iOS.
    IOS,
    /// Android.
    Android,
    /// WASI.
    Wasi,
    /// Emscripten.
    Emscripten,
    /// Bare metal.
    BareMetal,
    /// Portable or unknown.
    Universal,
}

impl From<Platform> for BuiltinPlatform {
    fn from(value: Platform) -> Self {
        match value {
            Platform::Web => BuiltinPlatform::Web,
            Platform::Windows => BuiltinPlatform::Windows,
            Platform::MacOS => BuiltinPlatform::MacOS,
            Platform::Linux => BuiltinPlatform::Linux,
            Platform::FreeBsd => BuiltinPlatform::FreeBsd,
            Platform::OpenBsd => BuiltinPlatform::OpenBsd,
            Platform::NetBsd => BuiltinPlatform::NetBsd,
            Platform::DragonFly => BuiltinPlatform::DragonFly,
            Platform::Solaris => BuiltinPlatform::Solaris,
            Platform::Illumos => BuiltinPlatform::Illumos,
            Platform::Haiku => BuiltinPlatform::Haiku,
            Platform::Fuchsia => BuiltinPlatform::Fuchsia,
            Platform::Redox => BuiltinPlatform::Redox,
            Platform::Hermit => BuiltinPlatform::Hermit,
            Platform::IOS => BuiltinPlatform::IOS,
            Platform::Android => BuiltinPlatform::Android,
            Platform::Wasi => BuiltinPlatform::Wasi,
            Platform::Emscripten => BuiltinPlatform::Emscripten,
            Platform::BareMetal => BuiltinPlatform::BareMetal,
            Platform::Universal => BuiltinPlatform::Universal,
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" | "browser" => Ok(Self::Web),
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
            "ios" => Ok(Self::IOS),
            "android" => Ok(Self::Android),
            "wasi" => Ok(Self::Wasi),
            "emscripten" | "emscripten-wasm" => Ok(Self::Emscripten),
            "bare_metal" | "bare-metal" | "baremetal" | "none" => Ok(Self::BareMetal),
            "universal" | "portable" | "any" => Ok(Self::Universal),
            _ => Err(()),
        }
    }
}

impl Platform {
    /// Return the canonical lowercase tag for this platform.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Web => "web",
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
            Self::IOS => "ios",
            Self::Android => "android",
            Self::Wasi => "wasi",
            Self::Emscripten => "emscripten",
            Self::BareMetal => "baremetal",
            Self::Universal => "universal",
        }
    }

    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this platform is a web platform.
    pub fn is_web(&self) -> bool {
        matches!(self, Self::Web)
    }

    /// Whether this platform is a WASM target.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasi | Self::Emscripten)
    }

    /// Whether this is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::IOS | Self::Android)
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
                | Self::IOS
                | Self::Android
        )
    }

    /// Whether this is a bare metal platform.
    pub fn is_bare_metal(&self) -> bool {
        matches!(self, Self::BareMetal)
    }

    /// Return the target family tag used by `import.meta.target.family`.
    pub fn family_tag(&self) -> &'static str {
        if self.is_web() {
            return "web";
        }

        if matches!(self, Self::Windows) {
            return "windows";
        }

        if self.is_unix() {
            return "unix";
        }

        if self.is_wasm() {
            return "wasm";
        }

        if self.is_bare_metal() {
            return "bare-metal";
        }

        if matches!(self, Self::Universal) {
            return "universal";
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
            Self::IOS => Some("ios"),
            Self::Android => Some("android"),
            Self::Wasi => Some("wasi"),
            Self::Emscripten => Some("emscripten"),
            Self::BareMetal => Some("none"),
            Self::Web | Self::Universal => None,
        }
    }
}

/// CPU architecture for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    /// Format this architecture as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::X86_64 => "x86_64".to_string(),
            Self::X86 => "i686".to_string(),
            Self::Aarch64 => "aarch64".to_string(),
            Self::Armv7 => "armv7".to_string(),
            Self::Armv6 => "armv6".to_string(),
            Self::Riscv64 => "riscv64".to_string(),
            Self::Riscv32 => "riscv32".to_string(),
            Self::PowerPc64 => "powerpc64".to_string(),
            Self::PowerPc64le => "powerpc64le".to_string(),
            Self::S390x => "s390x".to_string(),
            Self::Mips64 => "mips64".to_string(),
            Self::Mips64el => "mips64el".to_string(),
            Self::LoongArch64 => "loongarch64".to_string(),
            Self::Wasm32 => "wasm32".to_string(),
            Self::Wasm64 => "wasm64".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Target vendor for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
            Platform::MacOS | Platform::IOS => Self::Apple,
            _ => Self::Unknown,
        }
    }

    /// Format this vendor as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Apple => "apple".to_string(),
            Self::Pc => "pc".to_string(),
            Self::Ibm => "ibm".to_string(),
            Self::Nintendo => "nintendo".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Target ABI flavor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// Android.
    Android,
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
            "android" => Self::Android,
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
            Platform::Android => Some(Self::Android),
            _ => None,
        }
    }

    /// Format this ABI as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::Gnu => "gnu".to_string(),
            Self::Musl => "musl".to_string(),
            Self::Msvc => "msvc".to_string(),
            Self::GnuLlvm => "gnullvm".to_string(),
            Self::Eabi => "eabi".to_string(),
            Self::Eabihf => "eabihf".to_string(),
            Self::MuslEabi => "musleabi".to_string(),
            Self::MuslEabihf => "musleabihf".to_string(),
            Self::Android => "android".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}
