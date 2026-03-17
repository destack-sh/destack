use std::io;
use std::path::Path;

use destack_resolver::{CachePolicy, ResolveError, Resolver};
use destack_source::{File, FileContent, FileId, FileType, ModuleId};
use destack_workspace::{
    FileUpdate, InvalidationKind, InvalidationPlan, Module, Program, TargetId,
};

use super::workspace::warning_message;
use super::{LanguageService, WorkspaceMessage};

impl LanguageService {
    /// Build a file update by reading the latest content from disk.
    pub(super) fn rescan_file_update(
        &self,
        program: &Program,
        file_id: FileId,
    ) -> Result<Option<FileUpdate>, WorkspaceMessage> {
        // resolve the tracked file
        let Some(file) = program.files.get_maybe(file_id) else {
            return Ok(None);
        };

        // resolve a readable path for this file
        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        // read binary files as bytes
        if file.ty.is_binary() {
            let bytes = match program.fs.read(&path) {
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
        let content = match program.fs.read_to_string(&path) {
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

    /// Refresh package and workspace configuration for a program.
    pub(super) fn refresh_program_configs(
        &self,
        resolver: &Resolver,
        program: &Program,
    ) -> Vec<WorkspaceMessage> {
        let mut messages = Vec::new();

        // reload the workspace config
        let workspace_root = self.session.workspace_root();
        match resolver.read_destack_config(&workspace_root, CachePolicy::Reload) {
            Ok(config) => {
                self.session.update_workspace_config(Some(config));
            }
            Err(ResolveError::DestackNotFound { .. }) => {
                self.session.update_workspace_config(None);
            }
            Err(error) => {
                messages.push(warning_message(
                    "config_reload_workspace_failed",
                    &format!("config: failed to refresh workspace config: {error}"),
                ));
            }
        }

        // reload package configs and rebuild package targets
        for package in program.packages.iter() {
            let package_guard = package.read();
            let package_id = package_guard.id;
            let package_path = package_guard.path.clone();
            drop(package_guard);

            let Some(package_path) = package_path else {
                continue;
            };

            let next_config = match resolver.read_destack_config(&package_path, CachePolicy::Reload)
            {
                Ok(config) => Some(config),
                Err(ResolveError::DestackNotFound { .. }) => None,
                Err(error) => {
                    messages.push(warning_message(
                        "config_reload_package_failed",
                        &format!(
                            "config: failed to refresh package {}: {error}",
                            package_path.display()
                        ),
                    ));
                    continue;
                }
            };

            let package = program.packages.get(package_id);
            let mut package = package.write();
            package.config = next_config.clone();
            package.targets.clear();
            if let Some(config) = next_config {
                for (name, options) in config.options.targets.iter() {
                    let target = options.to_target(name);
                    let target_id = TargetId::new(package_id, name);
                    package.targets.insert(target_id, target);
                }
            }
        }

        // reload tracked tsconfig files
        let mut tsconfig_paths = Vec::new();
        for tsconfig in program.tsconfigs.iter() {
            tsconfig_paths.push(tsconfig.read().path.clone());
        }
        for path in tsconfig_paths {
            if let Err(error) = resolver.reload_tsconfig(&path) {
                if matches!(error, ResolveError::TsConfigNotFound { .. }) {
                    continue;
                }
                messages.push(warning_message(
                    "config_reload_tsconfig_failed",
                    &format!(
                        "config: failed to refresh tsconfig {}: {error}",
                        path.display()
                    ),
                ));
            }
        }

        // refresh module to tsconfig mapping
        let mut module_updates: Vec<(ModuleId, _)> = Vec::new();
        for module in program.modules.iter() {
            let module = module.read();
            let Some(path) = module.path.as_ref() else {
                continue;
            };
            let next_tsconfig = match resolver.find_tsconfig_for_file(path) {
                Ok(tsconfig_id) => tsconfig_id,
                Err(error) => {
                    messages.push(warning_message(
                        "config_reload_tsconfig_failed",
                        &format!(
                            "config: failed to refresh tsconfig {}: {error}",
                            path.display()
                        ),
                    ));
                    continue;
                }
            };
            if module.tsconfig_id != next_tsconfig {
                module_updates.push((module.id, next_tsconfig));
            }
        }

        for (module_id, tsconfig_id) in module_updates {
            let module = program.modules.get(module_id);
            let mut module = module.write();
            module.tsconfig_id = tsconfig_id;
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

    /// Return true when a module is part of the user workspace surface.
    pub(super) fn is_workspace_module(&self, module: &Module) -> bool {
        module.is_user() && module.path.is_some()
    }

    /// Return true when the module id maps to a workspace module.
    pub(super) fn is_workspace_module_id(&self, program: &Program, module_id: ModuleId) -> bool {
        let module = program.modules.get(module_id);
        let module = module.read();
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

    /// Check if a config file is already tracked by this program.
    fn is_tracked_config_path(&self, program: &Program, path: &Path) -> bool {
        // check workspace level config
        if let Some(workspace_config) = self.session.workspace_config()
            && workspace_config.path.as_path() == path
        {
            return true;
        }

        // check package level config files
        for package in program.packages.iter() {
            let package = package.read();
            let Some(config) = package.config.as_ref() else {
                continue;
            };
            if config.path.as_path() == path {
                return true;
            }
        }

        // check tracked tsconfig files
        for tsconfig in program.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            if tsconfig.path.as_path() == path {
                return true;
            }
        }

        false
    }

    /// Check whether a config refresh is required for an invalidation.
    pub(super) fn should_refresh_configs(
        &self,
        program: &Program,
        invalidation: &InvalidationPlan,
        path: &Path,
    ) -> bool {
        // refresh when invalidation already reports config kinds
        if invalidation
            .kinds
            .iter()
            .any(|kind| matches!(kind, InvalidationKind::Destack | InvalidationKind::TsConfig))
        {
            return true;
        }

        // refresh when the updated path is a tracked config
        self.is_tracked_config_path(program, path)
    }
}
