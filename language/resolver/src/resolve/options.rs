use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The options controlling Destack module resolution.
#[derive(Debug, Clone)]
pub struct ResolverOptions {
    /// The working directory used for local resolution commands.
    pub cwd: Option<PathBuf>,
    /// The explicit alias table.
    pub alias: Alias,
    /// The mounted package source roots.
    pub mounts: Vec<Mount>,
    /// The extension probe order.
    pub extensions: Vec<String>,
    /// The project roots used for slash-prefixed lookups.
    pub roots: Vec<PathBuf>,
    /// The path restrictions applied to resolved candidates.
    pub restrictions: Vec<Restriction>,
    /// Whether resolved paths should be canonicalized through symlinks.
    pub canonicalize_symlinks: bool,
}

impl Default for ResolverOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            alias: Vec::new(),
            mounts: Vec::new(),
            extensions: vec![
                ".ds".into(),
                ".d.ds".into(),
                ".tsx".into(),
                ".ts".into(),
                ".json".into(),
            ],
            roots: Vec::new(),
            restrictions: Vec::new(),
            canonicalize_symlinks: true,
        }
    }
}

impl ResolverOptions {
    /// Create default options for one workspace root.
    pub fn workspace_defaults(cwd: PathBuf) -> Self {
        Self::cwd_defaults(cwd)
    }

    /// Create default options for one working directory.
    pub fn cwd_defaults(cwd: PathBuf) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::default()
        }
    }
}

impl fmt::Display for ResolverOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.alias.is_empty() {
            write!(f, "alias:{:?},", self.alias)?;
        }
        if !self.mounts.is_empty() {
            write!(f, "mounts:{:?},", self.mounts)?;
        }
        if !self.extensions.is_empty() {
            write!(f, "extensions:{:?},", self.extensions)?;
        }
        if !self.roots.is_empty() {
            write!(f, "roots:{:?},", self.roots)?;
        }
        if !self.restrictions.is_empty() {
            write!(f, "restrictions:{:?},", self.restrictions)?;
        }
        if self.canonicalize_symlinks {
            write!(f, "symlinks:{:?},", self.canonicalize_symlinks)?;
        }

        Ok(())
    }
}

/// One mounted package source root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// The package specifier prefix.
    pub specifier: String,
    /// The mounted source root.
    pub root: PathBuf,
    /// The optional entry path for exact package imports.
    pub entry: Option<PathBuf>,
}

impl Mount {
    /// Create one package source mount.
    pub fn new(specifier: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            specifier: specifier.into(),
            root: root.into(),
            entry: None,
        }
    }

    /// Create one package source mount with an exact import entry.
    pub fn with_entry(
        specifier: impl Into<String>,
        root: impl Into<PathBuf>,
        entry: impl Into<PathBuf>,
    ) -> Self {
        Self {
            specifier: specifier.into(),
            root: root.into(),
            entry: Some(entry.into()),
        }
    }
}

/// The alias table used for explicit rewrites.
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

#[cfg(test)]
mod tests {
    use super::ResolverOptions;

    /// Default options should use the Destack source probe set.
    #[test]
    fn test_default_extensions_are_destack_first() {
        let options = ResolverOptions::default();

        assert_eq!(
            options.extensions,
            vec![".ds", ".d.ds", ".tsx", ".ts", ".json"]
        );
    }
}
