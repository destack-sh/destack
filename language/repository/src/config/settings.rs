use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tspp_source::FileSystem;

use super::SETTINGS_FILE_NAME;

/// Machine-local toolchain settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Package directory settings.
    pub packages: PackageSettings,
    /// Cache directory settings.
    pub cache: CacheSettings,
    /// Registry settings keyed by registry name.
    pub registries: IndexMap<String, RegistrySettings>,
    /// Default registry name.
    pub registry: Option<String>,
    /// Network settings for package and update commands.
    pub network: NetworkSettings,
}

impl Settings {
    /// Load settings from one home directory.
    pub fn load_from_home(fs: &dyn FileSystem, home: &Path) -> Result<Self, SettingsError> {
        // read settings text
        let path = home.join(SETTINGS_FILE_NAME);
        let text = match fs.read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(SettingsError::Read {
                    path,
                    message: error.to_string(),
                });
            }
        };

        // parse settings
        serde_json::from_str(&text).map_err(|error| SettingsError::Parse {
            path,
            message: error.to_string(),
        })
    }
}

/// Package directory settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PackageSettings {
    /// Package directory.
    pub path: Option<PathBuf>,
    /// Maximum package directory size in bytes before pruning is requested.
    pub maximum_bytes: Option<u64>,
}

/// Cache directory and retention settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct CacheSettings {
    /// Cache directory.
    pub path: Option<PathBuf>,
    /// Maximum retained cache size in bytes.
    pub maximum_bytes: Option<u64>,
}

impl Default for CacheSettings {
    /// Return the default machine-local cache settings.
    fn default() -> Self {
        Self {
            path: None,
            maximum_bytes: Some(crate::DEFAULT_CACHE_MAXIMUM_BYTES),
        }
    }
}

/// Registry settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RegistrySettings {
    /// Registry URL.
    pub url: String,
    /// Authentication used for this registry.
    #[serde(default)]
    pub authentication: RegistryAuthentication,
}

/// Registry authentication settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum RegistryAuthentication {
    /// No registry authentication.
    #[default]
    None,
    /// Inline token stored in the machine-local settings file.
    Token {
        /// Registry token.
        token: String,
    },
    /// Token read from one environment variable.
    TokenFromEnvironment {
        /// Environment variable name.
        variable: String,
    },
    /// Token or credential material printed by one command.
    Command {
        /// Program to run.
        program: String,
        /// Program arguments.
        args: Vec<String>,
    },
}

/// Network settings for package and update commands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSettings {
    /// Whether network access should be disabled by default.
    pub offline: bool,
    /// Optional HTTP proxy URL.
    pub proxy: Option<String>,
    /// Request timeout in milliseconds.
    pub timeout_milliseconds: Option<u64>,
    /// Number of retries for transient network failures.
    pub retry_count: Option<u32>,
    /// Maximum concurrent network requests.
    pub concurrency: Option<u32>,
}

/// Error raised while loading settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// Settings could not be read.
    Read {
        /// Settings path.
        path: PathBuf,
        /// Error detail.
        message: String,
    },
    /// Settings could not be parsed.
    Parse {
        /// Settings path.
        path: PathBuf,
        /// Error detail.
        message: String,
    },
}

impl Display for SettingsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::Read { path, message } => {
                write!(formatter, "failed to read {}: {message}", path.display())
            }
            SettingsError::Parse { path, message } => {
                write!(formatter, "invalid {}: {message}", path.display())
            }
        }
    }
}

impl Error for SettingsError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tspp_source::MemoryFileSystem;

    use super::*;

    /// Use default settings when no settings file exists.
    #[test]
    fn test_load_settings_defaults_when_missing() {
        let fs = MemoryFileSystem::new();
        let settings = Settings::load_from_home(&fs, Path::new("/home"))
            .expect("missing settings should load");

        assert_eq!(settings, Settings::default());
    }
}
