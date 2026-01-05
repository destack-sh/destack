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
    /// Canonical exports used by the compiler for fast builtin lookups.
    pub canonical_exports: &'static [&'static str],
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
            canonical_exports: &[],
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
            canonical_exports: &[],
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
            canonical_exports: &[],
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
            canonical_exports: &[],
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
            canonical_exports: &[],
        }
    }

    /// Attach canonical exports to the builtin library definition.
    pub(crate) const fn with_canonical_exports(
        mut self,
        canonical_exports: &'static [&'static str],
    ) -> Self {
        self.canonical_exports = canonical_exports;
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
}
