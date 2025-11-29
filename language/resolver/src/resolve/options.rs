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
    pub enforce_extension: EnforceExtension,

    /// Extension aliases (e.g., `(".js", [".ts", ".tsx"])`).
    pub extension_alias: IndexMap<String, Vec<String>>,

    /// Attempt to resolve these extensions in order (e.g., `[".js", ".json", ".node"]`).
    pub extensions: Vec<String>,

    /// Request passed to resolve is already fully specified.
    pub is_fully_specified: bool,

    /// Redirect module requests when normal resolving fails.
    pub fallback: Alias,

    /// Main files in description files (e.g., `["index"]`).
    pub main_files: Vec<String>,

    /// Directories to resolve modules from (e.g., `["node_modules"]`).
    pub modules: Vec<String>,

    /// Resolve to a context instead of a file.
    pub resolve_to_context: bool,

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

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            conditions: vec![],
            enforce_extension: EnforceExtension::Disabled,
            extension_alias: IndexMap::new(),
            extensions: vec![
                ".ds".into(),
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
            resolve_to_context: false,
            prefer_relative: false,
            prefer_absolute: false,
            restrictions: vec![],
            roots: vec![],
            canonicalize_symlinks: true,
        }
    }
}

impl ResolveOptions {
    /// Create a new blank resolve options.
    pub fn blank() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            conditions: vec![],
            enforce_extension: EnforceExtension::Disabled,
            extension_alias: IndexMap::new(),
            extensions: vec![],
            fallback: vec![],
            is_fully_specified: false,
            main_files: vec![],
            modules: vec![],
            resolve_to_context: false,
            prefer_relative: false,
            prefer_absolute: false,
            restrictions: vec![],
            roots: vec![],
            canonicalize_symlinks: true,
        }
    }

    /// Set the current working directory.
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Set the TypeScript configuration file.
    pub fn with_tsconfig(mut self, tsconfig: TypeScriptOptionsDiscovery) -> Self {
        self.tsconfig = Some(tsconfig);
        self
    }

    /// Set the aliases.
    pub fn with_alias(mut self, alias: Alias) -> Self {
        self.alias = alias;
        self
    }

    /// Set the conditions.
    pub fn with_conditions(mut self, conditions: Vec<String>) -> Self {
        self.conditions = conditions;
        self
    }

    /// Set the enforce extension.
    pub fn with_enforce_extension(mut self, enforce_extension: EnforceExtension) -> Self {
        self.enforce_extension = enforce_extension;
        self
    }

    /// Set the extension aliases.
    pub fn with_extension_alias(mut self, extension_alias: IndexMap<String, Vec<String>>) -> Self {
        self.extension_alias = extension_alias;
        self
    }

    /// Set the extensions.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set the skip extension.
    pub fn with_is_fully_specified(mut self, is_fully_specified: bool) -> Self {
        self.is_fully_specified = is_fully_specified;
        self
    }

    /// Set the main files.
    pub fn with_main_files(mut self, main_files: Vec<String>) -> Self {
        self.main_files = main_files;
        self
    }

    /// Set the modules.
    pub fn with_modules(mut self, modules: Vec<String>) -> Self {
        self.modules = modules;
        self
    }

    /// Set the resolve to context.
    pub fn with_resolve_to_context(mut self, resolve_to_context: bool) -> Self {
        self.resolve_to_context = resolve_to_context;
        self
    }

    /// Set the prefer relative.
    pub fn with_prefer_relative(mut self, prefer_relative: bool) -> Self {
        self.prefer_relative = prefer_relative;
        self
    }

    /// Set the prefer absolute.
    pub fn with_prefer_absolute(mut self, prefer_absolute: bool) -> Self {
        self.prefer_absolute = prefer_absolute;
        self
    }

    /// Set the restrictions.
    pub fn with_restrictions(mut self, restrictions: Vec<Restriction>) -> Self {
        self.restrictions = restrictions;
        self
    }

    /// Set the roots.
    pub fn with_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.roots = roots;
        self
    }

    /// Set the canonicalize symlinks.
    pub fn with_canonicalize_symlinks(mut self, canonicalize_symlinks: bool) -> Self {
        self.canonicalize_symlinks = canonicalize_symlinks;
        self
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
        if self.enforce_extension == EnforceExtension::Enabled {
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
        if self.resolve_to_context {
            write!(f, "resolve_directory:{:?},", self.resolve_to_context)?;
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

/// How to enforce file extensions.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum EnforceExtension {
    /// Enforce file extensions (path must include extension).
    Enabled,
    /// Do not enforce file extensions (resolve tries appending extensions from the list).
    #[default]
    Disabled,
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
