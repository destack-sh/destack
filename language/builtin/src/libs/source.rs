use std::collections::HashSet;
use std::fmt;

/// The kind of builtin library, corresponding to the three layers:
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinLibKind {
    /// Language primitives (operator traits, Result, etc.)
    Core,
    /// Universal Destack extensions (array, async, collections, io, string, time).
    Std,
    /// Target-specific type definitions (ES, DOM, Node, etc.).
    Lib,
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
    /// macOS.
    MacOS,
    /// Linux.
    Linux,
    /// iOS.
    IOS,
    /// Android.
    Android,
    /// WASI.
    Wasi,
    /// Bare metal.
    BareMetal,
    /// Unknown or portable (no platform-specific APIs).
    Universal,
}

/// A builtin library source file.
#[derive(Clone, Copy)]
pub struct BuiltinLibSource {
    /// Root directory under builtin ("std" or "lib").
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

impl fmt::Debug for BuiltinLibSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltinLibSource")
            .field("root", &self.root)
            .field("path", &self.path)
            .field("name", &self.name)
            .finish()
    }
}

impl BuiltinLibSource {
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

    /// Return the full virtual path (e.g., "builtin://lib/es/es2024/arraybuffer.d.ts").
    pub fn virtual_path(&self) -> String {
        if self.path.is_empty() {
            format!("builtin://{}/{}", self.root, self.name)
        } else {
            format!("builtin://{}/{}/{}", self.root, self.path, self.name)
        }
    }

    /// Return the relative module path (e.g., "lib/es/es2024/arraybuffer.d.ts").
    pub fn module_path(&self) -> String {
        if self.path.is_empty() {
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
            && (self.platforms.is_empty() || self.platforms.contains(&platform))
    }
}

/// Define a builtin library source file embedded from builtin/.
#[macro_export]
macro_rules! builtin_lib_source {
    ($vis:vis $name:ident, $root:literal, $path:literal, $file:literal) => {
        $vis const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new(
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
        const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new(
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
        $vis const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new(
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
        const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new(
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
    ([$(($name:ident, $root:literal, $path:literal, $file:literal)),+ $(,)?]) => {
        $(
            $crate::builtin_lib_source!($name, $root, $path, $file);
        )+
    };
}

#[macro_export]
macro_rules! builtin_lib_source_targeted {
    ($vis:vis $name:ident, $root:literal, $path:literal, $file:literal, $runtimes:expr, $outputs:expr, $platforms:expr) => {
        $vis const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new_with_targets(
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
        $vis const $name: $crate::libs::source::BuiltinLibSource =
            $crate::libs::source::BuiltinLibSource::new_with_targets(
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
pub struct BuiltinLib {
    /// The kind of builtin library (Core, Std, or Lib).
    pub kind: BuiltinLibKind,
    /// Library name (e.g., "es2024", "dom").
    pub name: &'static str,
    /// Source files for this library.
    pub sources: &'static [BuiltinLibSource],
    /// Library dependencies by name.
    pub dependencies: &'static [&'static str],
    /// Import specifier aliases for this library.
    pub specifier_aliases: &'static [(&'static str, &'static str)],
    /// Symbols declared by this library used by the compiler.
    pub declared_symbols: &'static [&'static str],
    /// Whether symbols are ambient without explicit imports.
    pub is_ambient: bool,
}

#[allow(dead_code)]
impl BuiltinLib {
    /// Create a new ambient core builtin library.
    pub(crate) const fn ambient_core(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind: BuiltinLibKind::Core,
            name,
            sources,
            dependencies,
            is_ambient: true,
            specifier_aliases: &[],
            declared_symbols: &[],
        }
    }

    /// Create a new ambient std builtin library.
    pub(crate) const fn ambient_std(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind: BuiltinLibKind::Std,
            name,
            sources,
            dependencies,
            is_ambient: true,
            specifier_aliases: &[],
            declared_symbols: &[],
        }
    }

    /// Create a new ambient lib builtin library.
    pub(crate) const fn ambient_lib(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind: BuiltinLibKind::Lib,
            name,
            sources,
            dependencies,
            is_ambient: true,
            specifier_aliases: &[],
            declared_symbols: &[],
        }
    }

    /// Create a new explicit std builtin library.
    pub(crate) const fn explicit_std(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind: BuiltinLibKind::Std,
            name,
            sources,
            dependencies,
            is_ambient: false,
            specifier_aliases: &[],
            declared_symbols: &[],
        }
    }

    /// Create a new explicit lib builtin library.
    pub(crate) const fn explicit_lib(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            kind: BuiltinLibKind::Lib,
            name,
            sources,
            dependencies,
            is_ambient: false,
            specifier_aliases: &[],
            declared_symbols: &[],
        }
    }

    /// Attach declared symbols to the builtin library definition.
    pub(crate) const fn with_declared_symbols(
        mut self,
        declared_symbols: &'static [&'static str],
    ) -> Self {
        self.declared_symbols = declared_symbols;
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

    /// Return reference lib dependencies declared in the builtin sources.
    pub fn reference_libs(&self) -> Vec<&'static str> {
        // track reference libs in source order
        let mut references = Vec::new();
        let mut seen = HashSet::new();

        // collect references from each source file
        for source in self.sources {
            for reference in reference_libs_from_source(source.content) {
                if seen.insert(reference) {
                    references.push(reference);
                }
            }
        }

        references
    }
}

fn reference_libs_from_source(content: &str) -> Vec<&str> {
    // collect reference directives
    let mut references = Vec::new();

    // scan directive lines for reference lib declarations
    for line in content.lines() {
        // skip non directive lines
        let line = line.trim_start();
        if !line.starts_with("///") {
            continue;
        }

        // skip non reference directives
        let line = line.trim_start_matches("///").trim_start();
        if !line.starts_with("<reference") {
            continue;
        }

        // parse the lib name and record it
        if let Some(name) = parse_reference_lib(line) {
            references.push(name);
        }
    }

    references
}

fn parse_reference_lib(line: &str) -> Option<&str> {
    // locate the lib attribute
    let mut parts = line.split_whitespace();
    let head = parts.next()?;
    if !head.starts_with("<reference") {
        return None;
    }

    // scan attributes for a lib entry
    for part in parts {
        let Some(rest) = part.strip_prefix("lib=") else {
            continue;
        };

        // slice out the lib name between quotes
        let rest = rest.trim_start();
        let (value, quote) = if let Some(value) = rest.strip_prefix('"') {
            (value, '"')
        } else if let Some(value) = rest.strip_prefix('\'') {
            (value, '\'')
        } else {
            continue;
        };
        let end = value.find(quote)?;
        return Some(&value[..end]);
    }

    None
}
