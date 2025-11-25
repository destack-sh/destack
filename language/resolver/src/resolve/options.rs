use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;

/// Resolution options (derived from `oxc-resolver` / `enhanced-resolve`).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Current working directory to start from.
    pub cwd: Option<PathBuf>,

    /// How to discover the TypeScript configuration file.
    pub tsconfig: Option<TypeScriptOptionsDiscovery>,

    /// Aliases to import or require certain modules more easily.
    pub alias: Alias,

    /// Condition names for exports field which defines entry points of a package.
    /// The key order in the exports key is significant.
    /// During condition matching, earlier entries have higher priority and take precedence over later entries.
    pub conditions: Vec<String>,

    /// Whether and how to enforce file extensions.
    /// <https://github.com/webpack/enhanced-resolve/pull/285>.
    pub enforce_extension: EnforceExtension,

    /// Extension aliases (e.g., `(".js", [".ts", ".tsx"])`).
    pub extension_alias: IndexMap<String, Vec<String>>,

    /// Attempt to resolve these extensions in order (e.g., `[".js", ".json", ".node"]`).
    pub extensions: Vec<String>,

    /// Request passed to resolve is already fully specified (should ignore extensions).
    pub is_fully_specified: bool,

    /// Redirect module requests when normal resolving fails.
    pub fallback: Alias,

    /// Main files in description files (e.g., `["index"]`).
    pub main_files: Vec<String>,

    /// Directories to resolve modules from (e.g., `["node_modules"]`).
    pub modules: Vec<String>,

    /// Resolve to a context instead of a file.
    pub resolve_to_directory: bool,

    /// Prefer to resolve module requests as relative requests instead of using modules from node_modules directories.
    pub prefer_relative: bool,

    /// Prefer to resolve server-relative urls as absolute paths before falling back to resolve in ResolveOptions::roots.
    pub prefer_absolute: bool,

    /// A list of resolve restrictions to restrict the paths that a request can be resolved on.
    pub restrictions: Vec<Restriction>,

    /// A list of directories where requests of server-relative URLs (starting with '/') are resolved.
    /// On non-Windows systems these requests are resolved as an absolute path first.
    pub roots: Vec<PathBuf>,

    /// Whether to resolve symlinks to their symlinked location, if possible.
    /// (May cause module resolution to fail when using tools that symlink packages like `npm link`).
    pub canonicalize_symlinks: bool,
}

impl ResolveOptions {
    /// Sanitize the options.
    pub fn sanitize(mut self) -> Self {
        // set `enforceExtension` to `true` when [ResolveOptions::extensions] contains an empty string
        // See <https://github.com/webpack/enhanced-resolve/pull/285>
        if self.enforce_extension == EnforceExtension::Automatic {
            if !self.extensions.is_empty() && self.extensions.iter().any(String::is_empty) {
                self.enforce_extension = EnforceExtension::Enabled;
            } else {
                self.enforce_extension = EnforceExtension::Disabled;
            }
        }
        self
    }
}

/// How to enforce file extensions.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EnforceExtension {
    /// Automatically determine whether to enforce file extensions based on the list of extensions.
    Automatic,
    /// Enforce file extensions.
    Enabled,
    /// Do not enforce file extensions.
    Disabled,
}

impl Default for EnforceExtension {
    fn default() -> Self {
        Self::Automatic
    }
}

impl EnforceExtension {
    /// Check if the enforce extension is automatic.
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Automatic)
    }

    /// Check if the enforce extension is enabled.
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Check if the enforce extension is disabled.
    pub const fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }
}

/// Alias for [ResolveOptions::alias] and [ResolveOptions::fallback]
pub type Alias = Vec<(String, Vec<AliasValue>)>;

/// Alias value.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AliasValue {
    /// The path value
    Path(String),
    /// The `false` value
    Ignore,
}

impl<S> From<S> for AliasValue
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self::Path(value.into())
    }
}

/// Restriction for resolution.
#[derive(Clone)]
pub enum Restriction {
    /// Prefix path restriction.
    Path(PathBuf),
    /// Function restriction.
    Function(Arc<dyn Fn(&Path) -> bool + Sync + Send>),
}

impl std::fmt::Debug for Restriction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "Path(\"{}\")", path.display()),
            Self::Function(_) => write!(f, "Function(<function>)"),
        }
    }
}

/// How to discover the TypeScript configuration file.
#[derive(Debug, Clone)]
pub enum TypeScriptOptionsDiscovery {
    /// Auto-discover the TypeScript configuration file.
    Automatic,
    /// Manual discovery of the TypeScript configuration file.
    Manual(TypeScriptOptionsLocation),
}

/// Location of the TypeScript configuration file.
#[derive(Debug, Clone)]
pub struct TypeScriptOptionsLocation {
    /// Path to the TypeScript configuration file.
    pub config_file: PathBuf,
    /// How to handle references in the TypeScript configuration file.
    pub references: TypeScriptOptionsReferences,
}

/// How to handle references in the TypeScript configuration file.
#[derive(Debug, Clone)]
pub enum TypeScriptOptionsReferences {
    /// Disable references.
    Disabled,
    /// Auto-discover the TypeScript configuration file references.
    Automatic,
    /// Manually provided paths to the TypeScript configuration files.
    Paths(Vec<PathBuf>),
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            conditions: vec![],
            enforce_extension: EnforceExtension::Automatic,
            extension_alias: IndexMap::new(),
            extensions: vec![
                ".tsx".into(),
                ".ts".into(),
                ".jsx".into(),
                ".js".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
                ".node".into(),
            ],
            fallback: vec![],
            is_fully_specified: false,
            main_files: vec!["index".into()],
            modules: vec!["node_modules".into()],
            resolve_to_directory: false,
            prefer_relative: false,
            prefer_absolute: false,
            restrictions: vec![],
            roots: vec![],
            canonicalize_symlinks: true,
        }
    }
}

impl fmt::Display for ResolveOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tsconfig) = &self.tsconfig {
            write!(f, "tsconfig:{tsconfig:?},")?;
        }
        if !self.alias.is_empty() {
            write!(f, "alias:{:?},", self.alias)?;
        }
        if !self.conditions.is_empty() {
            write!(f, "condition_names:{:?},", self.conditions)?;
        }
        if self.enforce_extension.is_enabled() {
            write!(f, "enforce_extension:{:?},", self.enforce_extension)?;
        }
        if !self.extension_alias.is_empty() {
            write!(f, "extension_alias:{:?},", self.extension_alias)?;
        }
        if !self.extensions.is_empty() {
            write!(f, "extensions:{:?},", self.extensions)?;
        }
        if !self.fallback.is_empty() {
            write!(f, "fallback:{:?},", self.fallback)?;
        }
        if self.is_fully_specified {
            write!(f, "fully_specified:{:?},", self.is_fully_specified)?;
        }
        if !self.main_files.is_empty() {
            write!(f, "main_files:{:?},", self.main_files)?;
        }
        if !self.modules.is_empty() {
            write!(f, "modules:{:?},", self.modules)?;
        }
        if self.resolve_to_directory {
            write!(f, "resolve_directory:{:?},", self.resolve_to_directory)?;
        }
        if self.prefer_relative {
            write!(f, "prefer_relative:{:?},", self.prefer_relative)?;
        }
        if self.prefer_absolute {
            write!(f, "prefer_absolute:{:?},", self.prefer_absolute)?;
        }
        if !self.restrictions.is_empty() {
            write!(f, "restrictions:{:?},", self.restrictions)?;
        }
        if !self.roots.is_empty() {
            write!(f, "roots:{:?},", self.roots)?;
        }
        if self.canonicalize_symlinks {
            write!(f, "symlinks:{:?},", self.canonicalize_symlinks)?;
        }
        Ok(())
    }
}
