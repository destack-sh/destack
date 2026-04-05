use std::collections::HashSet;
use std::io;
use std::path::Path;

use destack_source::{File, FileContent, FileId, FileType, ModuleId};
use destack_workspace::{Module, Repository, Revision};

use super::workspace::{WorkspaceSession, warning_message};
use super::{FileUpdate, LanguageService, UpdateImpact, UpdateImpactKind, WorkspaceMessage};

impl LanguageService {
    /// Discover workspace module files for one session before rescanning tracked state.
    pub(super) fn discover_workspace_modules(
        &self,
        session: &WorkspaceSession,
    ) -> Vec<WorkspaceMessage> {
        let mut messages = Vec::new();
        let mut pending_directories = vec![session.root.clone()];
        let mut visited_directories = HashSet::new();
        let compiler = session.compiler().clone();
        let repository = session.repository().clone();
        let revision = session.revision();

        while let Some(directory) = pending_directories.pop() {
            if !visited_directories.insert(directory.clone()) {
                continue;
            }

            let entries = match repository.file_system().read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    messages.push(warning_message(
                        "workspace_discovery_read_dir_failed",
                        &format!(
                            "workspace: failed to read directory {}: {error}",
                            directory.display(),
                        ),
                    ));
                    continue;
                }
            };

            for entry in entries {
                let metadata = match repository.file_system().metadata(&entry) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        messages.push(warning_message(
                            "workspace_discovery_metadata_failed",
                            &format!(
                                "workspace: failed to read metadata {}: {error}",
                                entry.display(),
                            ),
                        ));
                        continue;
                    }
                };

                if metadata.is_directory {
                    if self.should_descend_workspace_directory(&entry) {
                        pending_directories.push(entry);
                    }
                    continue;
                }

                if !metadata.is_file {
                    continue;
                }

                if !self.should_discover_workspace_module_path(&entry) {
                    continue;
                }

                if repository
                    .module_id_for_path(revision, &entry)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    continue;
                }

                let result = compiler.resolve_path_to_module(revision, &entry.to_path_buf());
                if let Err(error) = result {
                    messages.push(warning_message(
                        "workspace_discovery_module_failed",
                        &format!(
                            "workspace: failed to discover module {}: {error}",
                            entry.display(),
                        ),
                    ));
                }
            }
        }

        messages
    }

    /// Build a file update by reading the latest content from disk.
    pub(super) fn rescan_file_update(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<FileUpdate>, WorkspaceMessage> {
        // resolve the tracked file
        let Some(file) = repository.file(revision, file_id).ok().flatten() else {
            return Ok(None);
        };

        // resolve a readable path for this file
        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        // read binary files as bytes
        if file.ty.is_binary() {
            let bytes = match repository.file_system().read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.handle_rescan_read_error(&file, &path, error);
                }
            };

            // emit an update only when bytes changed
            if self.should_update_bytes(&file, &bytes) {
                return Ok(Some(FileUpdate::Bytes { content: bytes }));
            }

            return Ok(None);
        }

        // read non-binary files as text
        let content = match repository.file_system().read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                return self.handle_rescan_read_error(&file, &path, error);
            }
        };

        // emit an update only when text changed
        if self.should_update_text(&file, &content) {
            return Ok(Some(FileUpdate::Text { content }));
        }

        Ok(None)
    }

    /// Decide whether a text update should be applied.
    fn should_update_text(&self, file: &File, content: &str) -> bool {
        match &file.content {
            FileContent::Text { content: current } => current != content,
            FileContent::Json {
                content: current, ..
            } => current != content,
            FileContent::Missing | FileContent::Unloaded => true,
            FileContent::Binary { .. } => true,
        }
    }

    /// Decide whether a byte update should be applied.
    fn should_update_bytes(&self, file: &File, bytes: &[u8]) -> bool {
        match &file.content {
            FileContent::Binary { content } => content.as_slice() != bytes,
            FileContent::Missing | FileContent::Unloaded => true,
            FileContent::Text { .. } | FileContent::Json { .. } => true,
        }
    }

    /// Handle file read errors during rescan.
    fn handle_rescan_read_error(
        &self,
        file: &File,
        path: &Path,
        error: io::Error,
    ) -> Result<Option<FileUpdate>, WorkspaceMessage> {
        // convert missing files to removed updates
        if error.kind() == io::ErrorKind::NotFound {
            if matches!(file.content, FileContent::Missing) {
                return Ok(None);
            }

            return Ok(Some(FileUpdate::Removed));
        }

        // surface all other read errors
        Err(warning_message(
            "rescan_read_failed",
            &format!("watch: failed to read {}: {error}", path.display()),
        ))
    }

    /// Refresh package and workspace configuration for a repository.
    pub(super) fn refresh_repository_configs(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Vec<WorkspaceMessage> {
        let mut messages = Vec::new();
        if let Err(error) = repository.workspace_module_ids(revision) {
            messages.push(warning_message(
                "config_reload_workspace_modules_failed",
                &format!("config: failed to collect workspace modules: {error}"),
            ));

            return messages;
        }

        messages
    }

    /// Check if a path should be handled by watch mode.
    pub(super) fn is_watchable_path(&self, path: &Path) -> bool {
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        file_type.is_code()
            || file_type.is_data()
            || file_type.is_text()
            || file_type.is_binary()
            || self.is_config_filename(path)
    }

    /// Return true when workspace discovery should descend into one directory.
    fn should_descend_workspace_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !matches!(name, ".git" | "node_modules" | "target")
    }

    /// Return true when workspace discovery should admit one file as a module.
    fn should_discover_workspace_module_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if matches!(name, "package.json" | "destack.json") {
            return false;
        }

        if name.starts_with("tsconfig") && name.ends_with(".json") {
            return false;
        }

        self.is_watchable_path(path)
    }

    /// Return true when a module is part of the user workspace surface.
    pub(super) fn is_workspace_module(&self, module: &Module) -> bool {
        module.is_user() && module.path.is_some()
    }

    /// Return true when the module id maps to a workspace module.
    pub(super) fn is_workspace_module_id(
        &self,
        repository: &Repository,
        revision: Revision,
        module_id: ModuleId,
    ) -> bool {
        let Some(module) = repository.module(revision, module_id).ok().flatten() else {
            return false;
        };

        self.is_workspace_module(&module)
    }

    /// Check if a path is a config filename.
    pub(super) fn is_config_filename(&self, path: &Path) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if file_name == "destack.json" {
            return true;
        }

        file_name.starts_with("tsconfig") && file_name.ends_with(".json")
    }

    /// Check whether a config refresh is required for an impact.
    pub(super) fn should_refresh_configs(
        &self,
        _repository: &Repository,
        _revision: Revision,
        impact: &UpdateImpact,
        path: &Path,
    ) -> bool {
        // refresh when impact already reports config kinds
        if impact
            .kinds
            .iter()
            .any(|kind| matches!(kind, UpdateImpactKind::Destack | UpdateImpactKind::TsConfig))
        {
            return true;
        }

        // ordinary source updates cannot affect config state
        if !self.is_config_filename(path) {
            return false;
        }

        true
    }
}
