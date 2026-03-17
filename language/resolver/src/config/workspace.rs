use std::path::{Path, PathBuf};

use destack_source::{File, FileType, Uri};
use destack_workspace::{Destack, Workspace, WorkspacesField};
use serde::Deserialize;

use crate::{CachePolicy, ResolveError, Resolver};

/// The workspace field extracted from one `package.json` file.
#[derive(Deserialize)]
struct PackageJsonWorkspaces {
    /// The optional workspace declaration.
    workspaces: Option<WorkspacesField>,
}

impl Resolver {
    /// Discover a workspace from one path.
    ///
    /// Walk up directories looking for:
    /// 1. `destack.json` with a `workspace.members` field
    /// 2. `package.json` with a `workspaces` field
    /// 3. `pnpm-workspace.yaml`
    ///
    /// Return a single package workspace if no monorepo root is found.
    pub fn discover_workspace(&self, path: &Path) -> Result<Workspace, ResolveError> {
        // walk up directories looking for a workspace root
        let mut current = path.to_path_buf();
        loop {
            // check for a Destack workspace root
            if let Some(workspace) = self.check_destack_workspace(&current)? {
                return Ok(workspace);
            }

            // check for workspaces in package.json
            if let Some(workspace) = self.check_npm_workspace(&current)? {
                return Ok(workspace);
            }

            // check for pnpm workspace
            if let Some(workspace) = self.check_pnpm_workspace(&current)? {
                return Ok(workspace);
            }

            // check for a package boundary without workspaces
            let package_json_path = current.join("package.json");
            if self
                .fs()
                .metadata(&package_json_path)
                .is_ok_and(|m| m.is_file)
            {
                // treat a plain package as a single package workspace
                let workspace = Workspace::single_package(current);
                return self.attach_workspace_config(workspace);
            }

            // move up to the parent
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        // fall back to a single package workspace rooted at the input path
        let workspace = Workspace::single_package(path.to_path_buf());
        self.attach_workspace_config(workspace)
    }

    /// Check if a directory contains a Destack workspace root.
    fn check_destack_workspace(&self, dir: &Path) -> Result<Option<Workspace>, ResolveError> {
        let Some(config) = self.read_workspace_destack_config(dir)? else {
            return Ok(None);
        };

        if config.options.workspace.members.is_empty() {
            return Ok(None);
        }

        let package_paths =
            self.expand_workspace_patterns(dir, &config.options.workspace.members)?;
        let workspace = Workspace::monorepo(dir.to_path_buf(), package_paths).with_config(config);

        Ok(Some(workspace))
    }

    /// Attach the root Destack config to the workspace when available.
    fn attach_workspace_config(&self, mut workspace: Workspace) -> Result<Workspace, ResolveError> {
        // try to load the root Destack config when present
        if let Some(config) = self.read_workspace_destack_config(&workspace.root)? {
            workspace = workspace.with_config(config);
        }

        Ok(workspace)
    }

    /// Read the workspace root Destack config when present.
    fn read_workspace_destack_config(&self, root: &Path) -> Result<Option<Destack>, ResolveError> {
        let destack_config_path = root.join("destack.json");
        match self.read_destack_config(&destack_config_path, CachePolicy::UseCache) {
            Ok(config) => Ok(Some(config)),
            Err(ResolveError::DestackNotFound { .. }) => Ok(None),
            Err(error) => Err(error),
        }
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
        let file_id = self.files.next_id();
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
        self.files.insert(file);
        let file = self.files.get(file_id);

        // read the workspaces field
        let destack_source::FileContent::Json { value, .. } = &file.content else {
            return Ok(None);
        };

        // parse just the workspaces field
        let pkg: PackageJsonWorkspaces = serde_json::from_value(value.clone()).map_err(|_| {
            ResolveError::InvalidPackageJson {
                path: package_json_path.clone(),
            }
        })?;

        // ignore packages without workspaces
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

        // read pnpm workspace config
        let content = match self.fs().read_to_string(&pnpm_workspace_path) {
            Ok(content) => content,
            Err(_) => return Ok(None),
        };

        // parse the package globs from the yaml file
        let patterns = Self::parse_pnpm_workspace_packages(&content);

        // expand workspace patterns to package paths
        let package_paths = self.expand_workspace_patterns(dir, &patterns)?;

        Ok(Some(Workspace::monorepo(dir.to_path_buf(), package_paths)))
    }

    /// Parse the packages field from `pnpm-workspace.yaml` content.
    fn parse_pnpm_workspace_packages(content: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut in_packages = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // enter the packages section
            if trimmed.starts_with("packages:") {
                in_packages = true;
                // handle inline arrays
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

            // parse list items inside the packages section
            if in_packages {
                // stop at the next top level key
                if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('#') {
                    in_packages = false;
                    continue;
                }

                // parse one list item
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

            // expand the supported glob forms manually
            let expanded = self.expand_glob_pattern(root, pattern);
            for path in expanded {
                // check if it is one project root
                if self.is_workspace_member_root(&path) {
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
        }
        // recursive glob: packages/**
        else if let Some(base) = pattern.strip_suffix("/**") {
            self.collect_directories_recursive(root, base, &mut results);
        }
        // exact path
        else {
            let path = root.join(pattern);
            if self.fs().metadata(&path).is_ok_and(|m| m.is_directory) {
                results.push(path);
            }
        }

        results
    }

    /// Check whether one path looks like a workspace member root.
    fn is_workspace_member_root(&self, path: &Path) -> bool {
        let destack_config_path = path.join("destack.json");
        if self
            .fs()
            .metadata(&destack_config_path)
            .is_ok_and(|metadata| metadata.is_file)
        {
            return true;
        }

        let package_json_path = path.join("package.json");
        self.fs()
            .metadata(&package_json_path)
            .is_ok_and(|metadata| metadata.is_file)
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
            // check for a Destack workspace root
            if let Ok(Some(config)) = self.read_workspace_destack_config(&current)
                && !config.options.workspace.members.is_empty()
            {
                return Some(current);
            }

            // check for npm/yarn workspaces
            let package_json_path = current.join("package.json");
            if self
                .fs()
                .metadata(&package_json_path)
                .is_ok_and(|m| m.is_file)
                && let Ok(content) = self.fs().read_to_string(&package_json_path)
                && content.contains("\"workspaces\"")
            {
                return Some(current);
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
