use std::fmt;

/// The kind of builtin library, corresponding to the three layers:
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinLibraryKind {
    /// Runtime and compiler intrinsics.
    Intrinsic,
    /// Language-level builtin libraries.
    Language,
    /// Host and compatibility libraries.
    Library,
}

/// Runtime targets for builtin sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinRuntime {
    /// Web browser (Chrome, Firefox, Safari, etc.).
    Browser,
    /// Node.js.
    Node,
    /// Deno.
    Deno,
    /// Bun.
    Bun,
    /// Web Worker / Service Worker / Shared Worker.
    Worker,
    /// WASM running in a JS host (browser or Node).
    WasmJs,
    /// WASM with WASI (wasmtime, wasmer, etc.).
    WasmWasi,
    /// Native hosted runtime (OS services available).
    NativeHosted,
    /// Native freestanding runtime (no OS services assumed).
    NativeFreestanding,
    /// Native embedded runtime (freestanding with tight constraints).
    NativeEmbedded,
}

/// Output formats for builtin sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinOutputFormat {
    /// JavaScript (.js).
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

/// Platform targets for builtin sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinPlatform {
    /// Web browser platform.
    Web,
    /// Windows.
    Windows,
    /// Unix family (Linux, macOS, BSDs, etc.).
    Unix,
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
    /// Unknown or portable (no platform-specific APIs).
    Universal,
}

impl BuiltinPlatform {
    /// Whether this platform is part of the Unix family.
    pub fn is_unix(&self) -> bool {
        matches!(
            self,
            Self::Unix
                | Self::MacOS
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

    /// Whether this platform matches a target platform filter.
    pub fn matches_target(self, target: BuiltinPlatform) -> bool {
        if self == BuiltinPlatform::Unix {
            return target.is_unix();
        }

        self == target
    }
}

/// A builtin library source file.
#[derive(Clone, Copy)]
pub struct BuiltinLibrarySource {
    /// Root directory under builtin.
    pub root: &'static str,
    /// Module path under the root.
    pub path: &'static str,
    /// File name.
    pub name: &'static str,
    /// Source content.
    pub content: &'static str,
    /// Allowed runtime targets (empty means all).
    pub runtimes: &'static [BuiltinRuntime],
    /// Allowed output formats (empty means all).
    pub outputs: &'static [BuiltinOutputFormat],
    /// Allowed platform targets (empty means all).
    pub platforms: &'static [BuiltinPlatform],
}

impl fmt::Debug for BuiltinLibrarySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltinLibrarySource")
            .field("root", &self.root)
            .field("path", &self.path)
            .field("name", &self.name)
            .finish()
    }
}

impl BuiltinLibrarySource {
    pub(crate) const fn new(
        root: &'static str,
        path: &'static str,
        name: &'static str,
        content: &'static str,
    ) -> Self {
        Self {
            root,
            path,
            name,
            content,
            runtimes: &[],
            outputs: &[],
            platforms: &[],
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn new_with_targets(
        root: &'static str,
        path: &'static str,
        name: &'static str,
        content: &'static str,
        runtimes: &'static [BuiltinRuntime],
        outputs: &'static [BuiltinOutputFormat],
        platforms: &'static [BuiltinPlatform],
    ) -> Self {
        Self {
            root,
            path,
            name,
            content,
            runtimes,
            outputs,
            platforms,
        }
    }

    /// Return the full virtual path.
    pub fn virtual_path(&self) -> String {
        if self.root.is_empty() && self.path.is_empty() {
            format!("builtin://{}", self.name)
        } else if self.path.is_empty() {
            format!("builtin://{}/{}", self.root, self.name)
        } else {
            format!("builtin://{}/{}/{}", self.root, self.path, self.name)
        }
    }

    /// Return the relative module path.
    pub fn module_path(&self) -> String {
        if self.root.is_empty() && self.path.is_empty() {
            self.name.to_string()
        } else if self.path.is_empty() {
            format!("{}/{}", self.root, self.name)
        } else {
            format!("{}/{}/{}", self.root, self.path, self.name)
        }
    }

    /// Return true if this source matches the requested target.
    pub fn matches_target(
        &self,
        runtime: BuiltinRuntime,
        output: BuiltinOutputFormat,
        platform: BuiltinPlatform,
    ) -> bool {
        (self.runtimes.is_empty() || self.runtimes.contains(&runtime))
            && (self.outputs.is_empty() || self.outputs.contains(&output))
            && (self.platforms.is_empty()
                || self
                    .platforms
                    .iter()
                    .copied()
                    .any(|allowed| allowed.matches_target(platform)))
    }
}

/// Define a builtin library source file embedded from builtin/.
#[macro_export]
macro_rules! builtin_lib_source {
    ($vis:vis $name:ident, $root:literal, $path:literal, $file:literal) => {
        $vis const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new(
                $root,
                $path,
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $path,
                    "/",
                    $file
                )),
            );
    };
    ($name:ident, $root:literal, $path:literal, $file:literal) => {
        const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new(
                $root,
                $path,
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $path,
                    "/",
                    $file
                )),
            );
    };
    ($vis:vis $name:ident, $root:literal, $file:literal) => {
        $vis const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new(
                $root,
                "",
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $file
                )),
            );
    };
    ($name:ident, $root:literal, $file:literal) => {
        const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new(
                $root,
                "",
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $file
                )),
            );
    };
}

#[macro_export]
macro_rules! builtin_lib_sources {
    ([]) => {};
    ([$(($name:ident, $root:literal, $path:literal, $file:literal)),+ $(,)?]) => {
        $(
            $crate::builtin_lib_source!($name, $root, $path, $file);
        )+
    };
}

#[macro_export]
macro_rules! builtin_lib_source_targeted {
    ($vis:vis $name:ident, $root:literal, $path:literal, $file:literal, $runtimes:expr, $outputs:expr, $platforms:expr) => {
        $vis const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new_with_targets(
                $root,
                $path,
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $path,
                    "/",
                    $file
                )),
                $runtimes,
                $outputs,
                $platforms,
            );
    };
    ($vis:vis $name:ident, $root:literal, $file:literal, $runtimes:expr, $outputs:expr, $platforms:expr) => {
        $vis const $name: $crate::libs::source::BuiltinLibrarySource =
            $crate::libs::source::BuiltinLibrarySource::new_with_targets(
                $root,
                "",
                $file,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/",
                    $root,
                    "/",
                    $file
                )),
                $runtimes,
                $outputs,
                $platforms,
            );
    };
}

#[macro_export]
macro_rules! builtin_lib_sources_targeted {
    ($runtimes:expr, $outputs:expr, $platforms:expr, []) => {};
    ($runtimes:expr, $outputs:expr, $platforms:expr, [$(($name:ident, $root:literal, $path:literal, $file:literal)),+ $(,)?]) => {
        $(
            $crate::builtin_lib_source_targeted!(
                $name,
                $root,
                $path,
                $file,
                $runtimes,
                $outputs,
                $platforms
            );
        )+
    };
}

/// Definition for a builtin library.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinLibrary {
    /// The kind of builtin library.
    pub kind: BuiltinLibraryKind,
    /// Library name (e.g., "es2024", "dom").
    pub name: &'static str,
    /// Source files for this library.
    pub sources: &'static [BuiltinLibrarySource],
    /// Library dependencies by name.
    pub dependencies: &'static [&'static str],
    /// Library dependencies declared through reference directives.
    pub reference_libs: &'static [&'static str],
    /// Import specifier aliases for this library.
    pub specifier_aliases: &'static [(&'static str, &'static str)],
    /// Types package names that should map to this library.
    pub types_package_names: &'static [&'static str],
    /// Symbols declared by this library used by the compiler.
    pub declared_symbols: &'static [&'static str],
    /// Whether symbols are ambient without explicit imports.
    pub is_ambient: bool,
}

#[allow(dead_code)]
impl BuiltinLibrary {
    /// Create a new builtin library definition.
    pub(crate) const fn new(
        kind: BuiltinLibraryKind,
        name: &'static str,
        sources: &'static [BuiltinLibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind,
            name,
            sources,
            dependencies,
            reference_libs: &[],
            is_ambient: false,
            specifier_aliases: &[],
            types_package_names: &[],
            declared_symbols: &[],
        }
    }

    /// Create a new intrinsic builtin library definition.
    pub(crate) const fn intrinsic(
        name: &'static str,
        sources: &'static [BuiltinLibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self::new(BuiltinLibraryKind::Intrinsic, name, sources, dependencies)
    }

    /// Create a new language builtin library definition.
    pub(crate) const fn language(
        name: &'static str,
        sources: &'static [BuiltinLibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self::new(BuiltinLibraryKind::Language, name, sources, dependencies)
    }

    /// Create a new library builtin definition.
    pub(crate) const fn library(
        name: &'static str,
        sources: &'static [BuiltinLibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self::new(BuiltinLibraryKind::Library, name, sources, dependencies)
    }

    /// Mark the builtin library as ambient.
    pub(crate) const fn ambient(mut self) -> Self {
        self.is_ambient = true;
        self
    }

    /// Mark the builtin library as explicit.
    pub(crate) const fn explicit(mut self) -> Self {
        self.is_ambient = false;
        self
    }

    /// Attach declared symbols to the builtin library definition.
    pub(crate) const fn with_declared_symbols(
        mut self,
        declared_symbols: &'static [&'static str],
    ) -> Self {
        self.declared_symbols = declared_symbols;
        self
    }

    /// Attach reference lib dependencies to the builtin library definition.
    pub(crate) const fn with_reference_libs(
        mut self,
        reference_libs: &'static [&'static str],
    ) -> Self {
        self.reference_libs = reference_libs;
        self
    }

    /// Attach import specifier aliases to the builtin library definition.
    pub(crate) const fn with_specifier_aliases(
        mut self,
        specifier_aliases: &'static [(&'static str, &'static str)],
    ) -> Self {
        self.specifier_aliases = specifier_aliases;
        self
    }

    /// Attach tsconfig types package names to the builtin library definition.
    pub(crate) const fn with_types_package_names(
        mut self,
        types_package_names: &'static [&'static str],
    ) -> Self {
        self.types_package_names = types_package_names;
        self
    }
}
