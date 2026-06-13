use std::borrow::Cow;
use std::path::PathBuf;

use destack_source::{PackageId, ProfileId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Active package dependency and export index for one profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIndex {
    /// The profile this index belongs to.
    pub profile: ProfileId,
    /// Indexed packages keyed by package id.
    pub packages: IndexMap<PackageId, PackageImportIndex>,
}

impl PackageIndex {
    /// Return one indexed package.
    pub fn package(&self, package: PackageId) -> Option<&PackageImportIndex> {
        self.packages.get(&package)
    }
}

/// Active import index for one package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageImportIndex {
    /// The package this index belongs to.
    pub package: PackageId,
    /// Package root path when filesystem backed.
    pub root: Option<PathBuf>,
    /// Active direct dependencies keyed by package specifier.
    pub dependencies: IndexMap<String, Option<PackageId>>,
    /// Active public exports.
    pub exports: ExportIndex,
}

impl PackageImportIndex {
    /// Return one active direct dependency by package specifier.
    pub fn dependency(&self, package: &str) -> Option<Option<PackageId>> {
        self.dependencies.get(package).copied()
    }
}

/// Active package exports indexed for module import resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportIndex {
    /// The package this export index belongs to.
    pub package: PackageId,
    /// Exact exports keyed by export specifier.
    pub exact: IndexMap<String, ExportTarget>,
    /// Pattern exports sorted from most specific to least specific.
    pub patterns: Vec<ExportPattern>,
}

impl ExportIndex {
    /// Return the active export matching one export key.
    pub fn get<'a>(&'a self, key: &'a str) -> Option<ResolvedExport<'a>> {
        // prefer exact exports
        if let Some(target) = self.exact.get(key) {
            return Some(ResolvedExport {
                target,
                path: Cow::Borrowed(target.path.as_str()),
            });
        }

        // scan active patterns in specificity order
        for pattern in &self.patterns {
            let Some(replacement) = pattern.replacement(key) else {
                continue;
            };
            let path = pattern.target.path.replace('*', replacement);

            return Some(ResolvedExport {
                target: &pattern.target,
                path: Cow::Owned(path),
            });
        }

        None
    }
}

/// Active package export target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExportTarget {
    /// Package relative export path.
    pub path: String,
    /// Whether the export can be imported as a source module.
    pub is_module: bool,
}

/// Active pattern export target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExportPattern {
    /// Export key prefix before `*`.
    pub prefix: String,
    /// Export key suffix after `*`.
    pub suffix: String,
    /// Pattern export target.
    pub target: ExportTarget,
}

impl ExportPattern {
    /// Return the replacement matched by this pattern.
    fn replacement<'a>(&self, key: &'a str) -> Option<&'a str> {
        // require matching edges
        if !key.starts_with(&self.prefix) || !key.ends_with(&self.suffix) {
            return None;
        }

        // slice the wildcard body
        let start = self.prefix.len();
        let end = key.len().checked_sub(self.suffix.len())?;

        // reject overlapping pattern edges
        if start > end {
            return None;
        }

        Some(&key[start..end])
    }
}

/// Resolved active package export.
#[derive(Debug, Clone)]
pub struct ResolvedExport<'a> {
    /// Matched export target.
    pub target: &'a ExportTarget,
    /// Resolved package relative path.
    pub path: Cow<'a, str>,
}
