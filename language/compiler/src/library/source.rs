use std::fmt;

use destack_artifact::{EmitFormat, Platform, Runtime};

/// The kind of library package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryKind {
    /// Language-level library packages.
    Language,
    /// Runtime library packages.
    Library,
}

/// Runtime targets for library sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryRuntime {
    /// Destack semantic runtime.
    Destack,
    /// JavaScript host runtime.
    Js,
}

/// Output formats for library sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryOutput {
    /// JavaScript (.js).
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

/// Platform targets for library sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryPlatform {
    /// Unknown operating system.
    Unknown,
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
    /// No operating system.
    None,
}

impl LibraryPlatform {
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
        )
    }

    /// Whether this platform matches a target platform filter.
    pub fn matches_target(self, target: LibraryPlatform) -> bool {
        if self == LibraryPlatform::Unix {
            return target.is_unix();
        }

        self == target
    }
}

impl From<Runtime> for LibraryRuntime {
    fn from(value: Runtime) -> Self {
        match value {
            Runtime::Destack => Self::Destack,
            Runtime::Js => Self::Js,
        }
    }
}

impl From<EmitFormat> for LibraryOutput {
    fn from(value: EmitFormat) -> Self {
        match value {
            EmitFormat::Js | EmitFormat::Html => Self::Js,
            EmitFormat::Ts => Self::Ts,
            EmitFormat::Wasm => Self::Wasm,
            EmitFormat::Native => Self::Native,
        }
    }
}

impl From<Platform> for LibraryPlatform {
    fn from(value: Platform) -> Self {
        match value {
            Platform::Unknown => Self::Unknown,
            Platform::Windows => Self::Windows,
            Platform::MacOS => Self::MacOS,
            Platform::Linux => Self::Linux,
            Platform::FreeBsd => Self::FreeBsd,
            Platform::OpenBsd => Self::OpenBsd,
            Platform::NetBsd => Self::NetBsd,
            Platform::DragonFly => Self::DragonFly,
            Platform::Solaris => Self::Solaris,
            Platform::Illumos => Self::Illumos,
            Platform::Haiku => Self::Haiku,
            Platform::Fuchsia => Self::Fuchsia,
            Platform::Redox => Self::Redox,
            Platform::Hermit => Self::Hermit,
            Platform::None => Self::None,
        }
    }
}

/// A library package source file.
#[derive(Clone, Copy)]
pub struct LibrarySource {
    /// Root directory under library.
    pub root: &'static str,
    /// Module path under the root.
    pub path: &'static str,
    /// File name.
    pub name: &'static str,
    /// Source content.
    pub content: &'static str,
    /// Allowed runtime targets (empty means all).
    pub runtimes: &'static [LibraryRuntime],
    /// Allowed output formats (empty means all).
    pub outputs: &'static [LibraryOutput],
    /// Allowed platform targets (empty means all).
    pub platforms: &'static [LibraryPlatform],
}

impl fmt::Debug for LibrarySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LibrarySource")
            .field("root", &self.root)
            .field("path", &self.path)
            .field("name", &self.name)
            .finish()
    }
}

impl LibrarySource {
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

    /// Create a new library source restricted to selected targets.
    #[allow(dead_code)]
    pub(crate) const fn new_with_targets(
        root: &'static str,
        path: &'static str,
        name: &'static str,
        content: &'static str,
        runtimes: &'static [LibraryRuntime],
        outputs: &'static [LibraryOutput],
        platforms: &'static [LibraryPlatform],
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
            format!("library://{}", self.name)
        } else if self.path.is_empty() {
            format!("library://{}/{}", self.root, self.name)
        } else {
            format!("library://{}/{}/{}", self.root, self.path, self.name)
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
        runtime: LibraryRuntime,
        output: LibraryOutput,
        platform: LibraryPlatform,
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

/// Definition for a library package.
#[derive(Clone, Copy, Debug)]
pub struct LibraryPackage {
    /// The kind of library package.
    pub kind: LibraryKind,
    /// Library name.
    pub name: &'static str,
    /// Source files for this library.
    pub sources: &'static [LibrarySource],
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
impl LibraryPackage {
    /// Create a new library package definition.
    pub(crate) const fn new(
        kind: LibraryKind,
        name: &'static str,
        sources: &'static [LibrarySource],
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

    /// Create a new language library package definition.
    pub(crate) const fn language(
        name: &'static str,
        sources: &'static [LibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self::new(LibraryKind::Language, name, sources, dependencies)
    }

    /// Create a new library package definition.
    pub(crate) const fn library(
        name: &'static str,
        sources: &'static [LibrarySource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self::new(LibraryKind::Library, name, sources, dependencies)
    }

    /// Mark the library package as ambient.
    pub(crate) const fn ambient(mut self) -> Self {
        self.is_ambient = true;
        self
    }

    /// Mark the library package as explicit.
    pub(crate) const fn explicit(mut self) -> Self {
        self.is_ambient = false;
        self
    }

    /// Attach declared symbols to the library package definition.
    pub(crate) const fn with_declared_symbols(
        mut self,
        declared_symbols: &'static [&'static str],
    ) -> Self {
        self.declared_symbols = declared_symbols;
        self
    }

    /// Attach reference lib dependencies to the library package definition.
    pub(crate) const fn with_reference_libs(
        mut self,
        reference_libs: &'static [&'static str],
    ) -> Self {
        self.reference_libs = reference_libs;
        self
    }

    /// Attach import specifier aliases to the library package definition.
    pub(crate) const fn with_specifier_aliases(
        mut self,
        specifier_aliases: &'static [(&'static str, &'static str)],
    ) -> Self {
        self.specifier_aliases = specifier_aliases;
        self
    }

    /// Attach tsconfig types package names to the library package definition.
    pub(crate) const fn with_types_package_names(
        mut self,
        types_package_names: &'static [&'static str],
    ) -> Self {
        self.types_package_names = types_package_names;
        self
    }
}
