use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    DESTACK_CACHE_DIR, DESTACK_HOME, DESTACK_PACKAGE_DIR, Environment, HOME, LOCAL_APPDATA,
    Settings, USERPROFILE,
};

const DEFAULT_WORKSPACE_CACHE_DIRECTORY: &str = ".destack";
const DEFAULT_PACKAGE_DIRECTORY: &str = "packages";
const DEFAULT_VENDOR_DIRECTORY: &str = "vendor";
const UNIX_DESTACK_HOME_DIRECTORY: &str = ".destack";
const WINDOWS_DESTACK_HOME_DIRECTORY: &str = "Destack";

/// Resolved Destack storage layout for one invocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DestackLayout {
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Machine-local package directory.
    pub packages: PathBuf,
    /// Workspace-local cache and session directory.
    pub workspace_cache: PathBuf,
    /// Workspace-owned vendor directory.
    pub vendor: PathBuf,
}

impl DestackLayout {
    /// Resolve the layout for one workspace invocation.
    pub fn resolve(
        workspace_root: &Path,
        cwd: &Path,
        environment: &Environment,
        settings: &Settings,
        overrides: &DestackLayoutOverride,
        vendor_path: Option<&Path>,
    ) -> Self {
        // resolve machine-owned roots
        let home = resolve_home(cwd, environment, overrides.home.as_deref());
        let packages = resolve_packages(
            cwd,
            environment,
            settings,
            overrides.packages.as_deref(),
            &home,
        );

        // resolve workspace-owned roots
        let workspace_cache = resolve_workspace_cache(
            cwd,
            workspace_root,
            environment,
            settings,
            overrides.workspace_cache.as_deref(),
        );
        let vendor = resolve_vendor(workspace_root, vendor_path);

        Self {
            home,
            packages,
            workspace_cache,
            vendor,
        }
    }

    /// Resolve the home directory without loading settings.
    pub fn resolve_home(
        cwd: &Path,
        environment: &Environment,
        override_path: Option<&Path>,
    ) -> PathBuf {
        resolve_home(cwd, environment, override_path)
    }
}

/// Invocation-level layout overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct DestackLayoutOverride {
    /// Home directory override.
    pub home: Option<PathBuf>,
    /// Package directory override.
    pub packages: Option<PathBuf>,
    /// Workspace cache override.
    pub workspace_cache: Option<PathBuf>,
}

/// Resolve one cache root from one workspace root and optional directory override.
pub fn resolve_cache_root(
    workspace_root: &Path,
    cache_directory_override: Option<&Path>,
) -> PathBuf {
    // honor explicit cache roots first
    if let Some(cache_directory_override) = cache_directory_override {
        resolve_path(cache_directory_override, workspace_root)
    }
    // use the workspace default
    else {
        workspace_root.join(DEFAULT_WORKSPACE_CACHE_DIRECTORY)
    }
}

/// Resolve one global cache root directory for the given name.
pub fn resolve_global_cache_root(dir_name: &str, environment: &Environment) -> Option<PathBuf> {
    // derive the root from captured invocation state
    let cwd = environment.cwd.as_deref().unwrap_or_else(|| Path::new("."));
    let home = resolve_home(cwd, environment, None);

    Some(home.join(dir_name))
}

/// Resolve the home directory in a given context.
fn resolve_home(cwd: &Path, environment: &Environment, override_path: Option<&Path>) -> PathBuf {
    // cli override
    if let Some(path) = override_path {
        resolve_path(path, cwd)
    }
    // environment override
    else if let Some(path) = environment_path(environment, DESTACK_HOME) {
        resolve_path(&path, cwd)
    }
    // unix home
    else if let Some(home) = environment_path(environment, HOME) {
        home.join(UNIX_DESTACK_HOME_DIRECTORY)
    }
    // windows local app data
    else if let Some(local) = environment_path(environment, LOCAL_APPDATA) {
        local.join(WINDOWS_DESTACK_HOME_DIRECTORY)
    }
    // windows profile fallback
    else if let Some(profile) = environment_path(environment, USERPROFILE) {
        profile.join(UNIX_DESTACK_HOME_DIRECTORY)
    }
    // default to unix fallback
    else {
        cwd.join(UNIX_DESTACK_HOME_DIRECTORY)
    }
}

/// Resolve the package directory in a given context.
fn resolve_packages(
    cwd: &Path,
    environment: &Environment,
    settings: &Settings,
    override_path: Option<&Path>,
    home: &Path,
) -> PathBuf {
    // cli override
    if let Some(path) = override_path {
        resolve_path(path, cwd)
    }
    // environment override
    else if let Some(path) = environment_path(environment, DESTACK_PACKAGE_DIR) {
        resolve_path(&path, cwd)
    }
    // settings override
    else if let Some(path) = settings.packages.path.as_deref() {
        resolve_path(path, home)
    }
    // default to home directory
    else {
        home.join(DEFAULT_PACKAGE_DIRECTORY)
    }
}

/// Resolve the workspace cache directory in a given context.
fn resolve_workspace_cache(
    cwd: &Path,
    workspace_root: &Path,
    environment: &Environment,
    settings: &Settings,
    override_path: Option<&Path>,
) -> PathBuf {
    // cli override
    if let Some(path) = override_path {
        resolve_path(path, cwd)
    }
    // environment override
    else if let Some(path) = environment_path(environment, DESTACK_CACHE_DIR) {
        resolve_path(&path, cwd)
    }
    // settings override
    else if let Some(path) = settings.cache.path.as_deref() {
        resolve_path(path, workspace_root)
    }
    // default to workspace root
    else {
        workspace_root.join(DEFAULT_WORKSPACE_CACHE_DIRECTORY)
    }
}

/// Resolve the vendor directory in a given workspace.
fn resolve_vendor(workspace_root: &Path, vendor_path: Option<&Path>) -> PathBuf {
    // manifest path or default
    let path = vendor_path.unwrap_or_else(|| Path::new(DEFAULT_VENDOR_DIRECTORY));

    resolve_path(path, workspace_root)
}

/// Read one nonempty environment path.
fn environment_path(environment: &Environment, name: &str) -> Option<PathBuf> {
    // read captured environment
    let value = environment.get(name)?;

    if !value.is_empty() {
        Some(PathBuf::from(value))
    } else {
        None
    }
}

/// Resolve one path relative to one root.
fn resolve_path(path: &Path, root: &Path) -> PathBuf {
    // return absolute paths unchanged
    if path.is_absolute() {
        path.to_path_buf()
    }
    // resolve relative paths against the root
    else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolve explicit layout overrides relative to the invocation directories.
    #[test]
    fn test_resolve_layout_uses_explicit_overrides() {
        let workspace_root = Path::new("/workspace");
        let cwd = Path::new("/workspace/app");
        let environment = Environment::default();
        let settings = Settings::default();
        let overrides = DestackLayoutOverride {
            home: Some(PathBuf::from("home")),
            packages: Some(PathBuf::from("packages")),
            workspace_cache: Some(PathBuf::from("cache")),
        };

        let layout = DestackLayout::resolve(
            workspace_root,
            cwd,
            &environment,
            &settings,
            &overrides,
            Some(Path::new("third_party")),
        );

        assert_eq!(layout.home, cwd.join("home"));
        assert_eq!(layout.packages, cwd.join("packages"));
        assert_eq!(layout.workspace_cache, cwd.join("cache"));
        assert_eq!(layout.vendor, workspace_root.join("third_party"));
    }

    /// Resolve defaults from the captured home directory.
    #[test]
    fn test_resolve_layout_uses_home_defaults() {
        let workspace_root = Path::new("/workspace");
        let cwd = Path::new("/workspace/app");
        let mut environment = Environment::default();
        environment
            .env
            .insert(HOME.to_string(), "/Users/tester".to_string());

        let layout = DestackLayout::resolve(
            workspace_root,
            cwd,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );

        assert_eq!(layout.home, PathBuf::from("/Users/tester/.destack"));
        assert_eq!(
            layout.packages,
            PathBuf::from("/Users/tester/.destack/packages")
        );
        assert_eq!(layout.workspace_cache, workspace_root.join(".destack"));
        assert_eq!(layout.vendor, workspace_root.join("vendor"));
    }
}
