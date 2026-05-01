use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_workspace::{NodeLinker, WorkspaceOptions};
use indexmap::IndexMap;

/// The options controlling resolver behavior.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct ResolverOptions {
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

impl Default for ResolverOptions {
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

impl ResolverOptions {
    /// Create default options using workspace linker policy.
    pub fn workspace_defaults(cwd: PathBuf, workspace_options: Option<&WorkspaceOptions>) -> Self {
        let node_linker = workspace_options
            .map(|workspace_options| workspace_options.package.compiler.node_linker)
            .unwrap_or_default();

        Self::cwd_defaults_with_node_linker(cwd, node_linker)
    }

    /// Create default options with automatic linker detection.
    /// Use this when workspace config is not available yet.
    pub fn cwd_defaults(cwd: PathBuf) -> Self {
        Self::cwd_defaults_with_node_linker(cwd, NodeLinker::Auto)
    }

    /// Create default options with explicit linker policy.
    pub fn cwd_defaults_with_node_linker(cwd: PathBuf, node_linker: NodeLinker) -> Self {
        let yarn_pnp = Self::yarn_pnp_enabled(node_linker, cwd.as_path());

        Self {
            cwd: Some(cwd),
            yarn_pnp,
            ..Self::default()
        }
    }

    /// Set one node linker policy on these resolve options.
    pub fn set_node_linker(&mut self, node_linker: NodeLinker, cwd: &Path) {
        if self.cwd.is_none() {
            self.cwd = Some(cwd.to_path_buf());
        }

        self.yarn_pnp = Self::yarn_pnp_enabled(node_linker, cwd);
    }

    /// Return whether one node linker policy enables Yarn PnP.
    pub fn yarn_pnp_enabled(node_linker: NodeLinker, cwd: &Path) -> bool {
        match node_linker {
            NodeLinker::Auto => Self::detect_yarn_pnp(cwd),
            NodeLinker::NodeModules => false,
            NodeLinker::Pnp => true,
        }
    }

    /// Detect whether Yarn Plug'n'Play should be enabled for one working directory.
    pub fn detect_yarn_pnp(cwd: &Path) -> bool {
        // find a pnp manifest from cwd up to the root
        #[cfg(not(target_arch = "wasm32"))]
        {
            match pnp::find_pnp_manifest(cwd) {
                Ok(Some(_)) => true,
                Ok(None) => false,
                Err(_) => false,
            }
        }

        // wasm targets do not support pnp filesystem access
        #[cfg(target_arch = "wasm32")]
        {
            let _ = cwd;
            false
        }
    }
}

impl fmt::Display for ResolverOptions {
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
    /// Discover the configuration automatically from the base path.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    use super::ResolverOptions;
    use destack_workspace::NodeLinker;

    /// Set node modules policy and disable yarn pnp.
    #[test]
    fn test_set_node_linker_sets_node_modules_policy() {
        let mut options = ResolverOptions {
            yarn_pnp: true,
            ..ResolverOptions::default()
        };
        let cwd = Path::new("/workspace");

        options.set_node_linker(NodeLinker::NodeModules, cwd);

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(!options.yarn_pnp);
    }

    /// Set pnp policy and keep yarn pnp enabled.
    #[test]
    fn test_set_node_linker_sets_pnp_policy() {
        let mut options = ResolverOptions::default();
        let cwd = Path::new("/workspace");

        options.set_node_linker(NodeLinker::Pnp, cwd);

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(options.yarn_pnp);
    }
}
