use std::fmt;

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

    /// Return the full virtual path (e.g., "builtin://lib/es/es2024/arraybuffer.d.ds").
    pub fn virtual_path(&self) -> String {
        if self.path.is_empty() {
            format!("builtin://{}/{}", self.root, self.name)
        } else {
            format!("builtin://{}/{}/{}", self.root, self.path, self.name)
        }
    }

    /// Return the relative module path (e.g., "lib/es/es2024/arraybuffer.d.ds").
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
    /// Library name (e.g., "es2024", "dom").
    pub name: &'static str,
    /// Source files for this library.
    pub sources: &'static [BuiltinLibSource],
    /// Whether symbols are ambient without explicit imports.
    pub is_ambient: bool,
    /// Library dependencies by name.
    pub dependencies: &'static [&'static str],
}

impl BuiltinLib {
    /// Create a new ambient builtin library.
    pub(crate) const fn ambient(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            sources,
            is_ambient: true,
            dependencies,
        }
    }

    /// Create a new explicit builtin library.
    pub(crate) const fn explicit(
        name: &'static str,
        sources: &'static [BuiltinLibSource],
        dependencies: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            sources,
            is_ambient: false,
            dependencies,
        }
    }
}
