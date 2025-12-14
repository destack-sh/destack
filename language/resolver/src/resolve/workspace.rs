use std::path::{Path, PathBuf};

use destack_source::{File, FileType, Uri};
use destack_workspace::{Workspace, WorkspacesField};
use serde::Deserialize;

use crate::{ResolveError, Resolver};

impl Resolver {
    /// Discover a workspace from a given path.
    ///
    /// Walks up directories looking for:
    /// 1. `package.json` with a `workspaces` field (npm/yarn)
    /// 2. `pnpm-workspace.yaml` (pnpm)
    ///
    /// Returns a single-package workspace if no monorepo root is found.
    #[tracing::instrument(name = "resolver.discover_workspace", level = "trace", skip(self))]
    pub fn discover_workspace(&self, path: &Path) -> Result<Workspace, ResolveError> {
        tracing::trace!(?path, "resolver.discover_workspace");

        // walk up directories looking for workspace root
        let mut current = path.to_path_buf();
        loop {
            // check for npm/yarn workspaces in package.json
            if let Some(workspace) = self.check_npm_workspace(&current)? {
                return Ok(workspace);
            }

            // check for pnpm workspace
            if let Some(workspace) = self.check_pnpm_workspace(&current)? {
                return Ok(workspace);
            }

            // check for package.json (package boundary without workspaces)
            let package_json_path = current.join("package.json");
            if self.fs().metadata(&package_json_path).is_ok_and(|m| m.is_file) {
                // found a package.json without workspaces, treat as single-package workspace
                return Ok(Workspace::single_package(current));
            }

            // move up to parent
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        // no workspace found, treat the original path as a single-package workspace
        Ok(Workspace::single_package(path.to_path_buf()))
    }

    /// Check if a directory contains an npm/yarn workspace root.
    fn check_npm_workspace(&self, dir: &Path) -> Result<Option<Workspace>, ResolveError> {
        let package_json_path = dir.join("package.json");

        // read package.json
        let bytes = match self.fs().read(&package_json_path) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(None),
        };

        // parse package.json
        let file_id = self.program.files.next_id();
        let (name, uri) = Uri::from_path_with_name(&package_json_path);
        let file = File::from_bytes_as_json(
            file_id,
            name,
            uri,
            Some(package_json_path.clone()),
            FileType::Json,
            bytes,
        )
        .map_err(|_| ResolveError::InvalidPackageJson {
            path: package_json_path.clone(),
        })?;
        self.program.files.insert(file);
        let file = self.program.files.get(file_id);

        // check for workspaces field
        let destack_source::FileContent::Json { value, .. } = &file.content else {
            return Ok(None);
        };

        // parse just the workspaces field
        #[derive(Deserialize)]
        struct PackageJsonWorkspaces {
            workspaces: Option<WorkspacesField>,
        }

        let pkg: PackageJsonWorkspaces = serde_json::from_value(value.clone()).map_err(|_| {
            ResolveError::InvalidPackageJson {
                path: package_json_path.clone(),
            }
        })?;

        // check if this is a workspace root
        let Some(workspaces) = pkg.workspaces else {
            return Ok(None);
        };

        // expand workspace patterns to package paths
        let package_paths = self.expand_workspace_patterns(dir, workspaces.patterns())?;

        Ok(Some(Workspace::monorepo(dir.to_path_buf(), package_paths)))
    }

    /// Check if a directory contains a pnpm workspace root.
    fn check_pnpm_workspace(&self, dir: &Path) -> Result<Option<Workspace>, ResolveError> {
        let pnpm_workspace_path = dir.join("pnpm-workspace.yaml");

        // read pnpm-workspace.yaml
        let content = match self.fs().read_to_string(&pnpm_workspace_path) {
            Ok(content) => content,
            Err(_) => return Ok(None),
        };

        // simple yaml parsing for packages field
        // format: packages:\n  - "packages/*"\n  - "apps/*"
        let patterns = Self::parse_pnpm_workspace_packages(&content);

        // expand workspace patterns to package paths
        let package_paths = self.expand_workspace_patterns(dir, &patterns)?;

        Ok(Some(Workspace::monorepo(dir.to_path_buf(), package_paths)))
    }

    /// Parse the packages field from pnpm-workspace.yaml content.
    /// Simple parser that handles common formats without a full yaml library.
    fn parse_pnpm_workspace_packages(content: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut in_packages = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // check for packages: key
            if trimmed.starts_with("packages:") {
                in_packages = true;
                // handle inline array: packages: ["foo/*", "bar/*"]
                if let Some(rest) = trimmed.strip_prefix("packages:") {
                    let rest = rest.trim();
                    if rest.starts_with('[') && rest.ends_with(']') {
                        let inner = &rest[1..rest.len() - 1];
                        for item in inner.split(',') {
                            let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
                            if !item.is_empty() {
                                patterns.push(item.to_string());
                            }
                        }
                        in_packages = false;
                    }
                }
                continue;
            }

            // check for list items under packages:
            if in_packages {
                // stop if we hit another top-level key
                if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('#') {
                    in_packages = false;
                    continue;
                }

                // parse list item: - "pattern" or - 'pattern' or - pattern
                if let Some(item) = trimmed.strip_prefix('-') {
                    let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
                    if !item.is_empty() {
                        patterns.push(item.to_string());
                    }
                }
            }
        }

        patterns
    }

    /// Expand workspace glob patterns to package directory paths.
    fn expand_workspace_patterns(
        &self,
        root: &Path,
        patterns: &[String],
    ) -> Result<Vec<PathBuf>, ResolveError> {
        let mut package_paths = Vec::new();

        for pattern in patterns {
            // skip negation patterns
            if pattern.starts_with('!') {
                continue;
            }

            // expand simple glob patterns manually
            // supports: packages/*, packages/**, packages/foo
            let expanded = self.expand_glob_pattern(root, pattern);
            for path in expanded {
                // check if it has a package.json
                let package_json = path.join("package.json");
                if self.fs().metadata(&package_json).is_ok_and(|m| m.is_file) {
                    package_paths.push(path);
                }
            }
        }

        Ok(package_paths)
    }

    /// Expand a simple glob pattern to matching directories.
    fn expand_glob_pattern(&self, root: &Path, pattern: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();

        // handle simple patterns: "foo/*", "foo/**", "foo"
        if let Some(base) = pattern.strip_suffix("/*") {
            // single level glob: packages/*
            let base_path = root.join(base);
            if let Ok(entries) = self.fs().read_dir(&base_path) {
                for entry in entries {
                    if self.fs().metadata(&entry).is_ok_and(|m| m.is_directory) {
                        results.push(entry);
                    }
                }
            }
        } else if let Some(base) = pattern.strip_suffix("/**") {
            // recursive glob: packages/**
            self.collect_directories_recursive(root, base, &mut results);
        } else {
            // exact path
            let path = root.join(pattern);
            if self.fs().metadata(&path).is_ok_and(|m| m.is_directory) {
                results.push(path);
            }
        }

        results
    }

    /// Recursively collect directories.
    fn collect_directories_recursive(&self, root: &Path, base: &str, results: &mut Vec<PathBuf>) {
        let base_path = root.join(base);
        if let Ok(entries) = self.fs().read_dir(&base_path) {
            for entry in entries {
                if self.fs().metadata(&entry).is_ok_and(|m| m.is_directory) {
                    results.push(entry.clone());
                    // recurse into subdirectories
                    if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                        let sub_base = format!("{base}/{name}");
                        self.collect_directories_recursive(root, &sub_base, results);
                    }
                }
            }
        }
    }

    /// Find the workspace root for a given path without full discovery.
    /// Returns the workspace root path if found.
    pub fn find_workspace_root(&self, path: &Path) -> Option<PathBuf> {
        let mut current = path.to_path_buf();
        loop {
            // check for npm/yarn workspaces
            let package_json_path = current.join("package.json");
            if self.fs().metadata(&package_json_path).is_ok_and(|m| m.is_file) {
                if let Ok(content) = self.fs().read_to_string(&package_json_path) {
                    if content.contains("\"workspaces\"") {
                        return Some(current);
                    }
                }
            }

            // check for pnpm workspace
            let pnpm_workspace_path = current.join("pnpm-workspace.yaml");
            if self
                .fs()
                .metadata(&pnpm_workspace_path)
                .is_ok_and(|m| m.is_file)
            {
                return Some(current);
            }

            // move up
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        None
    }
}
