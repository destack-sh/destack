use std::path::PathBuf;

use destack_source::{PackageId, TargetId};

/// Select the source of entry points for entry discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    /// Use only target entry paths.
    Target,
    /// Use only package manifest entry targets.
    Manifest,
    /// Use target entries first, then manifest entry targets.
    Auto,
}

/// Select how entry paths are resolved against package and repository paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryResolutionMode {
    /// Resolve entries relative to package path only.
    Strict,
    /// Resolve entries relative to package path, then as repository paths.
    RepositoryRelative,
}

/// Options for entry discovery behavior.
#[derive(Debug, Clone)]
pub struct TargetDiscoveryOptions<'a> {
    /// Source policy for entry paths.
    pub entry_source: EntrySource,
    /// Resolution policy for selected entry paths.
    pub entry_resolution: EntryResolutionMode,
    /// Manifest entry targets from package.json fields.
    pub manifest_entry_targets: &'a [String],
}

impl<'a> Default for TargetDiscoveryOptions<'a> {
    /// Return the default target discovery options.
    fn default() -> Self {
        Self {
            entry_source: EntrySource::Target,
            entry_resolution: EntryResolutionMode::Strict,
            manifest_entry_targets: &[],
        }
    }
}

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub enum TargetDiscoveryIssue {
    /// Repository state lookup failed during discovery.
    Repository {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// The repository error message.
        message: String,
    },
    /// Missing package path for entry based discovery.
    MissingPackagePath {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
    },
    /// Missing entry module for entry based discovery.
    MissingEntry {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// Entry path that could not be resolved.
        path: PathBuf,
    },
}
