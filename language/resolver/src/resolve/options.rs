use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_workspace::{Destack, NodeLinker};
use indexmap::IndexMap;

/// The options controlling resolver behavior.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// The working directory used for environment dependent lookups.
    pub cwd: Option<PathBuf>,

    /// The TypeScript configuration discovery policy.
    pub tsconfig: Option<TypeScriptOptionsDiscovery>,

    /// The primary alias table.
    pub alias: Alias,

    /// The active package export conditions in priority order.
    pub conditions: Vec<String>,

    /// Whether package `exports` mappings are enabled.
    pub resolve_package_json_exports: bool,

    /// Whether package `imports` mappings are enabled.
    pub resolve_package_json_imports: bool,

    /// The file extension enforcement policy.
    pub enforce_extension: EnforceExtension,

    /// The extension alias table.
    pub extension_alias: IndexMap<String, Vec<String>>,

    /// The extension probe order.
    pub extensions: Vec<String>,

    /// Whether incoming requests are already fully specified.
    pub is_fully_specified: bool,

    /// The fallback alias table.
    pub fallback: Alias,

    /// The main files probed inside directories.
    pub main_files: Vec<String>,

    /// The module directory names to search while walking parents.
    pub modules: Vec<String>,

    /// Whether directory requests should resolve to the directory itself.
    pub resolve_to_context: bool,

    /// Whether bare specifiers should prefer relative resolution first.
    pub prefer_relative: bool,

    /// Whether server relative specifiers should prefer absolute resolution first.
    pub prefer_absolute: bool,

    /// The path restrictions applied to resolved candidates.
    pub restrictions: Vec<Restriction>,

    /// The project roots used for slash prefixed lookups.
    pub roots: Vec<PathBuf>,

    /// Whether resolved paths should be canonicalized through symlinks.
    pub canonicalize_symlinks: bool,

    /// Whether Yarn Plug'n'Play resolution is enabled.
    pub yarn_pnp: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            conditions: vec![],
            resolve_package_json_exports: true,
            resolve_package_json_imports: true,
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
            yarn_pnp: false,
        }
    }
}

impl ResolveOptions {
    /// Create default options for one working directory with workspace linker policy.
    pub fn default_for_workspace(cwd: PathBuf, workspace_config: Option<&Destack>) -> Self {
        let node_linker = workspace_config
            .map(|workspace_config| workspace_config.options.compiler.node_linker)
            .unwrap_or_default();

        Self::default_for_cwd_with_node_linker(cwd, node_linker)
    }

    /// Create default options for one working directory with automatic linker detection.
    /// Use this when workspace config is not available yet.
    pub fn default_for_cwd(cwd: PathBuf) -> Self {
        Self::default_for_cwd_with_node_linker(cwd, NodeLinker::Auto)
    }

    /// Create default options for one working directory with explicit linker policy.
    pub fn default_for_cwd_with_node_linker(cwd: PathBuf, node_linker: NodeLinker) -> Self {
        let yarn_pnp = Self::yarn_pnp_for_node_linker(node_linker, cwd.as_path());

        Self {
            cwd: Some(cwd),
            yarn_pnp,
            ..Self::default()
        }
    }

    /// Apply one node linker policy to these resolve options.
    pub fn apply_node_linker_for_cwd(&mut self, node_linker: NodeLinker, cwd: &Path) {
        if self.cwd.is_none() {
            self.cwd = Some(cwd.to_path_buf());
        }

        self.yarn_pnp = Self::yarn_pnp_for_node_linker(node_linker, cwd);
    }

    /// Resolve yarn pnp state from one node linker policy.
    pub fn yarn_pnp_for_node_linker(node_linker: NodeLinker, cwd: &Path) -> bool {
        match node_linker {
            NodeLinker::Auto => Self::detect_yarn_pnp_for_cwd(cwd),
            NodeLinker::NodeModules => false,
            NodeLinker::Pnp => true,
        }
    }

    /// Detect whether Yarn Plug'n'Play should be enabled for one working directory.
    pub fn detect_yarn_pnp_for_cwd(cwd: &Path) -> bool {
        // find a pnp manifest from cwd up to the root
        #[cfg(not(target_arch = "wasm32"))]
        {
            pnp::find_pnp_manifest(cwd).ok().flatten().is_some()
        }

        // wasm targets do not support pnp filesystem access
        #[cfg(target_arch = "wasm32")]
        {
            let _ = cwd;
            false
        }
    }

    /// Create blank resolve options.
    pub fn blank() -> Self {
        Self {
            cwd: None,
            tsconfig: None,
            alias: vec![],
            conditions: vec![],
            resolve_package_json_exports: true,
            resolve_package_json_imports: true,
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
            yarn_pnp: false,
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

    /// Set package json exports resolution.
    pub fn with_resolve_package_json_exports(mut self, value: bool) -> Self {
        self.resolve_package_json_exports = value;
        self
    }

    /// Set package json imports resolution.
    pub fn with_resolve_package_json_imports(mut self, value: bool) -> Self {
        self.resolve_package_json_imports = value;
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

    /// Set whether incoming requests are fully specified.
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

    /// Set yarn pnp resolution.
    pub fn with_yarn_pnp(mut self, yarn_pnp: bool) -> Self {
        self.yarn_pnp = yarn_pnp;
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
        if !self.resolve_package_json_exports {
            write!(
                f,
                "resolve_package_json_exports:{:?},",
                self.resolve_package_json_exports
            )?;
        }
        if !self.resolve_package_json_imports {
            write!(
                f,
                "resolve_package_json_imports:{:?},",
                self.resolve_package_json_imports
            )?;
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
        if self.yarn_pnp {
            write!(f, "yarn_pnp:{:?},", self.yarn_pnp)?;
        }
        Ok(())
    }
}

/// The file extension enforcement policy.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum EnforceExtension {
    /// Require requests to include their extension.
    Enabled,
    /// Probe configured extensions when the request omits them.
    #[default]
    Disabled,
}

/// The alias table used for primary and fallback rewrites.
pub type Alias = Vec<(String, Vec<AliasValue>)>;

/// Alias value.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AliasValue {
    /// The replacement path value.
    Path(String),
    /// The explicit ignore value.
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

/// The restriction applied to resolved paths.
#[derive(Clone)]
pub enum Restriction {
    /// The allowed path prefix.
    Path(PathBuf),
    /// The custom predicate restriction.
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

/// Cache policy for resolver data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    /// Reuse cached data when available.
    UseCache,
    /// Reload data from the filesystem.
    Reload,
}

impl CachePolicy {
    /// Return true when cached data can be reused.
    pub fn use_cache(self) -> bool {
        matches!(self, CachePolicy::UseCache)
    }
}

/// The TypeScript configuration discovery policy.
#[derive(Debug, Clone)]
pub enum TypeScriptOptionsDiscovery {
    /// Discover the configuration automatically from the issuer path.
    Automatic,
    /// Use one explicit configuration location.
    Manual(TypeScriptOptionsLocation),
}

/// The explicit TypeScript configuration location.
#[derive(Debug, Clone)]
pub struct TypeScriptOptionsLocation {
    /// The path to the `tsconfig.json` file.
    pub config_file: PathBuf,
    /// The project reference handling policy.
    pub references: TypeScriptOptionsReferences,
}

/// The TypeScript project reference handling policy.
#[derive(Debug, Clone)]
pub enum TypeScriptOptionsReferences {
    /// Disable project references.
    Disabled,
    /// Discover project references from the root config.
    Automatic,
    /// Use the given referenced config paths.
    Paths(Vec<PathBuf>),
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::ResolveOptions;
    use destack_workspace::NodeLinker;

    /// Apply node modules policy and disable yarn pnp.
    #[test]
    fn test_apply_node_linker_for_cwd_sets_node_modules_policy() {
        let mut options = ResolveOptions::default().with_yarn_pnp(true);
        let cwd = Path::new("/workspace");

        options.apply_node_linker_for_cwd(NodeLinker::NodeModules, cwd);

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(!options.yarn_pnp);
    }

    /// Apply pnp policy and keep yarn pnp enabled.
    #[test]
    fn test_apply_node_linker_for_cwd_sets_pnp_policy() {
        let mut options = ResolveOptions::default();
        let cwd = Path::new("/workspace");

        options.apply_node_linker_for_cwd(NodeLinker::Pnp, cwd);

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(options.yarn_pnp);
    }
}
