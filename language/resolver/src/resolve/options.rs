use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Resolution options (like oxc-resolver / enhanced-resolve).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Current working directory.
    pub cwd: Option<PathBuf>,

    /// How to discover tsconfig.
    pub tsconfig: Option<TsconfigDiscovery>,

    /// Aliases to import or require certain modules more easily.
    pub alias: Alias,

    /// Alias fields in description files (e.g., `["path", "to", "exports"]`)
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub alias_fields: Vec<Vec<String>>,

    /// Condition names for exports field which defines entry points of a package.
    /// The key order in the exports field is significant.
    /// During condition matching, earlier entries have higher priority and take precedence over later entries.
    pub condition_names: Vec<String>,

    /// Whether and how to enforce file extensions.
    /// <https://github.com/webpack/enhanced-resolve/pull/285>.
    pub enforce_extension: EnforceExtension,

    /// Exports fields in description files (like `["exports"]`).
    pub exports_fields: Vec<Vec<String>>,

    /// Imports fields in description files (like `["imports"]`).
    pub imports_fields: Vec<Vec<String>>,

    /// Extension aliases (e.g., `(".js", [".ts", ".tsx"])`).
    pub extension_alias: Vec<(String, Vec<String>)>,

    /// Attempt to resolve these extensions in order (e.g., `[".js", ".json", ".node"]`).
    pub extensions: Vec<String>,

    /// Redirect module requests when normal resolving fails.
    pub fallback: Alias,

    /// Request passed to resolve is already fully specified.
    /// Extensions or main files are not resolved for it (they are still resolved for internal requests).
    pub fully_specified: bool,

    /// Main fields in description files (e.g., `["main"]`).
    pub main_fields: Vec<String>,

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
    /// NOTE that this may cause module resolution to fail when using tools that symlink packages (like `npm link`).
    pub symlinks: bool,

    /// Whether to parse "builtin" Node modules or not.
    pub builtin_modules: bool,

    /// Allow `exports` field in `require('../directory')`. This is not part of the spec but some vite projects rely on this behavior.
    pub allow_package_exports_in_directory_resolve: bool,
}

impl ResolveOptions {
    /// Set condition names.
    /// ```
    pub fn with_condition_names(mut self, names: &[&str]) -> Self {
        self.condition_names = names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        self
    }

    /// Set whether to parse "builtin" modules or not.
    pub const fn with_builtin_modules(mut self, flag: bool) -> Self {
        self.builtin_modules = flag;
        self
    }

    /// Add a single root to the options.
    pub fn with_root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.roots.push(root.as_ref().to_path_buf());
        self
    }

    /// Add a single extension to the list of extensions. Extension must start with a `.`
    pub fn with_extension<S: Into<String>>(mut self, extension: S) -> Self {
        self.extensions.push(extension.into());
        self
    }

    /// Add a single main field to the list of fields
    pub fn with_main_field<S: Into<String>>(mut self, field: S) -> Self {
        self.main_fields.push(field.into());
        self
    }

    /// Set how the extension should be treated
    ///
    pub const fn with_force_extension(mut self, enforce_extension: EnforceExtension) -> Self {
        self.enforce_extension = enforce_extension;
        self
    }

    /// Set whether to resolve fully specified modules or not.
    pub const fn with_fully_specified(mut self, fully_specified: bool) -> Self {
        self.fully_specified = fully_specified;
        self
    }

    /// Set whether to prefer relative requests or not.
    pub const fn with_prefer_relative(mut self, flag: bool) -> Self {
        self.prefer_relative = flag;
        self
    }

    /// Set whether to prefer absolute requests or not.
    pub const fn with_prefer_absolute(mut self, flag: bool) -> Self {
        self.prefer_absolute = flag;
        self
    }

    /// Set whether to resolve symlinks or not.
    pub const fn with_symbolic_link(mut self, flag: bool) -> Self {
        self.symlinks = flag;
        self
    }

    /// Add a module to the list of modules.
    pub fn with_module<M: Into<String>>(mut self, module: M) -> Self {
        self.modules.push(module.into());
        self
    }

    /// Add a main file to the list of main files.
    ///
    pub fn with_main_file<M: Into<String>>(mut self, module: M) -> Self {
        self.main_files.push(module.into());
        self
    }

    /// Sanitize the options.
    pub fn sanitize(mut self) -> Self {
        debug_assert!(
            self.extensions
                .iter()
                .filter(|e| !e.is_empty())
                .all(|e| e.starts_with('.')),
            "All extensions must start with a leading dot"
        );
        // Set `enforceExtension` to `true` when [ResolveOptions::extensions] contains an empty string.
        // See <https://github.com/webpack/enhanced-resolve/pull/285>
        if self.enforce_extension == EnforceExtension::Auto {
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
    Auto,
    /// Enforce file extensions.
    Enabled,
    /// Do not enforce file extensions.
    Disabled,
}

impl Default for EnforceExtension {
    fn default() -> Self {
        Self::Auto
    }
}

impl EnforceExtension {
    /// Check if the enforce extension is automatic.
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
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

/// Alias Value for [ResolveOptions::alias] and [ResolveOptions::fallback]
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

/// Value for [ResolveOptions::restrictions]
#[derive(Clone)]
pub enum Restriction {
    Path(PathBuf),
    Fn(Arc<dyn Fn(&Path) -> bool + Sync + Send>),
}

impl std::fmt::Debug for Restriction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "Path(\"{}\")", path.display()),
            Self::Fn(_) => write!(f, "Fn(<function>)"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TsconfigDiscovery {
    Auto,
    Manual(TsconfigOptions),
}

/// Tsconfig Options for [ResolveOptions::tsconfig]
///
/// Derived from [tsconfig-paths-webpack-plugin](https://github.com/dividab/tsconfig-paths-webpack-plugin#options)
#[derive(Debug, Clone)]
pub struct TsconfigOptions {
    /// Allows you to specify where to find the TypeScript configuration file.
    /// You may provide
    /// * a relative path to the configuration file. It will be resolved relative to cwd.
    /// * an absolute path to the configuration file.
    pub config_file: PathBuf,

    /// Support for Typescript Project References.
    pub references: TsconfigReferences,
}

/// How to handle references in `tsconfig.json`.
#[derive(Debug, Clone)]
pub enum TsconfigReferences {
    /// Disable references.
    Disabled,
    /// Use the `references` field from `tsconfig.json` of `config_file`.
    Auto,
    /// Manually provided relative or absolute paths.
    Paths(Vec<PathBuf>),
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            alias_fields: vec![],
            condition_names: vec![],
            enforce_extension: EnforceExtension::Auto,
            extension_alias: vec![],
            exports_fields: vec![vec!["exports".into()]],
            imports_fields: vec![vec!["imports".into()]],
            extensions: vec![
                ".js".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".wasm".into(),
                ".jsx".into(),
                ".tsx".into(),
                ".json".into(),
                ".node".into(),
            ],
            fallback: vec![],
            fully_specified: false,
            main_fields: vec!["main".into()],
            main_files: vec!["index".into()],
            modules: vec!["node_modules".into()],
            resolve_to_context: false,
            prefer_relative: false,
            prefer_absolute: false,
            restrictions: vec![],
            roots: vec![],
            symlinks: true,
            builtin_modules: false,
            allow_package_exports_in_directory_resolve: false,
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
        if !self.alias_fields.is_empty() {
            write!(f, "alias_fields:{:?},", self.alias_fields)?;
        }
        if !self.condition_names.is_empty() {
            write!(f, "condition_names:{:?},", self.condition_names)?;
        }
        if self.enforce_extension.is_enabled() {
            write!(f, "enforce_extension:{:?},", self.enforce_extension)?;
        }
        if !self.exports_fields.is_empty() {
            write!(f, "exports_fields:{:?},", self.exports_fields)?;
        }
        if !self.imports_fields.is_empty() {
            write!(f, "imports_fields:{:?},", self.imports_fields)?;
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
        if self.fully_specified {
            write!(f, "fully_specified:{:?},", self.fully_specified)?;
        }
        if !self.main_fields.is_empty() {
            write!(f, "main_fields:{:?},", self.main_fields)?;
        }
        if !self.main_files.is_empty() {
            write!(f, "main_files:{:?},", self.main_files)?;
        }
        if !self.modules.is_empty() {
            write!(f, "modules:{:?},", self.modules)?;
        }
        if self.resolve_to_context {
            write!(f, "resolve_to_context:{:?},", self.resolve_to_context)?;
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
        if self.symlinks {
            write!(f, "symlinks:{:?},", self.symlinks)?;
        }
        if self.builtin_modules {
            write!(f, "builtin_modules:{:?},", self.builtin_modules)?;
        }
        if self.allow_package_exports_in_directory_resolve {
            write!(
                f,
                "allow_package_exports_in_directory_resolve:{:?},",
                self.allow_package_exports_in_directory_resolve
            )?;
        }
        Ok(())
    }
}
