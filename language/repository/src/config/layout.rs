use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    DEFAULT_PACKAGE_DIRECTORY, DEFAULT_VENDOR_DIRECTORY, Environment, HOME, LOCAL_APPDATA,
    Settings, TSPP_CACHE_DIR, TSPP_HOME, TSPP_PACKAGE_DIR, UNIX_HOME_DIRECTORY, USERPROFILE,
    WINDOWS_HOME_DIRECTORY, XDG_CACHE_HOME,
};

/// Resolved TS++ storage layout for one invocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StorageLayout {
    /// Machine-local toolchain home.
    pub home: PathBuf,
    /// Machine-local package directory.
    pub packages: PathBuf,
    /// Machine-local cache directory.
    pub cache: PathBuf,
    /// Workspace-owned vendor directory.
    pub vendor: PathBuf,
}

impl StorageLayout {
    /// Resolve the layout for one workspace invocation.
    pub fn resolve(
        workspace_root: &Path,
        cwd: &Path,
        environment: &Environment,
        settings: &Settings,
        overrides: &StorageLayoutOverride,
        vendor_path: Option<&Path>,
    ) -> Self {
        // resolve machine-owned roots
        let home = Self::resolve_home(cwd, environment, overrides.home.as_deref());
        let packages = Self::resolve_packages(
            cwd,
            environment,
            settings,
            overrides.packages.as_deref(),
            &home,
        );

        let cache = Self::resolve_cache(
            cwd,
            &home,
            environment,
            settings,
            overrides.cache.as_deref(),
        );

        // resolve workspace-owned roots
        let vendor = Self::resolve_vendor(workspace_root, vendor_path);

        Self {
            home,
            packages,
            cache,
            vendor,
        }
    }

    /// Resolve the home directory without loading settings.
    pub fn resolve_home(
        cwd: &Path,
        environment: &Environment,
        override_path: Option<&Path>,
    ) -> PathBuf {
        // cli override
        if let Some(path) = override_path {
            resolve_path(path, cwd)
        }
        // environment override
        else if let Some(path) = environment_path(environment, TSPP_HOME) {
            resolve_path(&path, cwd)
        }
        // unix home
        else if let Some(home) = environment_path(environment, HOME) {
            home.join(UNIX_HOME_DIRECTORY)
        }
        // windows local app data
        else if let Some(local) = environment_path(environment, LOCAL_APPDATA) {
            local.join(WINDOWS_HOME_DIRECTORY)
        }
        // windows profile fallback
        else if let Some(profile) = environment_path(environment, USERPROFILE) {
            profile.join(UNIX_HOME_DIRECTORY)
        }
        // invocation fallback
        else {
            cwd.join(UNIX_HOME_DIRECTORY)
        }
    }

    /// Resolve the machine-local cache without loading repository state.
    pub fn resolve_cache(
        cwd: &Path,
        home: &Path,
        environment: &Environment,
        settings: &Settings,
        override_path: Option<&Path>,
    ) -> PathBuf {
        // cli override
        if let Some(path) = override_path {
            resolve_path(path, cwd)
        }
        // environment override
        else if let Some(path) = environment_path(environment, TSPP_CACHE_DIR) {
            resolve_path(&path, cwd)
        }
        // settings override
        else if let Some(path) = settings.cache.path.as_deref() {
            resolve_path(path, home)
        }
        // windows local app data
        else if cfg!(windows)
            && let Some(path) = environment_path(environment, LOCAL_APPDATA)
        {
            path.join(WINDOWS_HOME_DIRECTORY)
        }
        // macOS user caches
        else if cfg!(target_os = "macos")
            && let Some(path) = environment_path(environment, HOME)
        {
            path.join("Library/Caches/tspp")
        }
        // freedesktop user cache
        else if let Some(path) = environment_path(environment, XDG_CACHE_HOME) {
            path.join("tspp")
        }
        // unix home fallback
        else if let Some(path) = environment_path(environment, HOME) {
            path.join(".cache/tspp")
        }
        // invocation fallback
        else {
            cwd.join(".tspp")
        }
    }

    /// Resolve the machine-local package directory.
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
        else if let Some(path) = environment_path(environment, TSPP_PACKAGE_DIR) {
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

    /// Resolve the workspace-owned vendor directory.
    fn resolve_vendor(workspace_root: &Path, vendor_path: Option<&Path>) -> PathBuf {
        let path = vendor_path.unwrap_or_else(|| Path::new(DEFAULT_VENDOR_DIRECTORY));

        resolve_path(path, workspace_root)
    }
}

/// Invocation-level layout overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct StorageLayoutOverride {
    /// Home directory override.
    pub home: Option<PathBuf>,
    /// Package directory override.
    pub packages: Option<PathBuf>,
    /// Cache directory override.
    pub cache: Option<PathBuf>,
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
        let overrides = StorageLayoutOverride {
            home: Some(PathBuf::from("home")),
            packages: Some(PathBuf::from("packages")),
            cache: Some(PathBuf::from("cache")),
        };

        let layout = StorageLayout::resolve(
            workspace_root,
            cwd,
            &environment,
            &settings,
            &overrides,
            Some(Path::new("third_party")),
        );

        assert_eq!(layout.home, cwd.join("home"));
        assert_eq!(layout.packages, cwd.join("packages"));
        assert_eq!(layout.cache, cwd.join("cache"));
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

        let layout = StorageLayout::resolve(
            workspace_root,
            cwd,
            &environment,
            &Settings::default(),
            &StorageLayoutOverride::default(),
            None,
        );

        assert_eq!(layout.home, PathBuf::from("/Users/tester/.tspp"));
        assert_eq!(
            layout.packages,
            PathBuf::from("/Users/tester/.tspp/packages")
        );
        let cache = if cfg!(target_os = "macos") {
            PathBuf::from("/Users/tester/Library/Caches/tspp")
        } else {
            PathBuf::from("/Users/tester/.cache/tspp")
        };
        assert_eq!(layout.cache, cache);
        assert_eq!(layout.vendor, workspace_root.join("vendor"));
    }
}
