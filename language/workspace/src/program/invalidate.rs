use std::collections::HashSet;

use destack_source::{
    File, FileContent, FileId, FileType, FileVersion, ModuleId, PackageId, ProfileId,
};

use crate::{ModuleContent, Program, TsConfigId};

/// Content update payload for an invalidated file.
#[derive(Debug, Clone)]
pub enum FileUpdate {
    /// Replace file content with text.
    Text { content: String },
    /// Replace file content with raw bytes.
    Bytes { content: Vec<u8> },
    /// Bump the file version without changing content.
    Touch,
    /// Mark file content as missing.
    Removed,
}

/// Classification of invalidation work derived from a file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvalidationKind {
    /// Source module content changed.
    ModuleSource,
    /// Package dsconfig changed.
    DsConfig,
    /// Tsconfig changed.
    TsConfig,
    /// Package manifest changed.
    PackageManifest,
    /// No known mapping for the file.
    Unknown,
}

/// Summary of invalidation work derived from a file change.
#[derive(Debug, Clone)]
pub struct InvalidationPlan {
    /// The file id that triggered invalidation.
    pub file_id: FileId,
    /// The updated file version.
    pub file_version: FileVersion,
    /// The invalidation categories derived from the file.
    pub kinds: Vec<InvalidationKind>,
    /// Modules that were invalidated.
    pub modules: Vec<ModuleId>,
    /// Packages affected by configuration invalidation.
    pub packages: Vec<PackageId>,
    /// Profiles whose version was bumped.
    pub profiles: Vec<ProfileId>,
    /// Profile graphs dropped due to invalidation.
    pub graphs_dropped: Vec<ProfileId>,
}

/// Errors produced while invalidating a file.
#[derive(Debug)]
pub enum InvalidationError {
    /// File id is missing from the registry.
    FileNotFound { file_id: FileId },
    /// JSON content failed to parse.
    InvalidJson { file_id: FileId, message: String },
    /// Text content failed to decode.
    InvalidText { file_id: FileId, message: String },
}

impl std::fmt::Display for InvalidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format invalidation errors
        match self {
            InvalidationError::FileNotFound { file_id } => {
                write!(f, "file not found for id {file_id:?}")
            }
            InvalidationError::InvalidJson { file_id, message } => {
                write!(f, "invalid json for file {file_id:?}: {message}")
            }
            InvalidationError::InvalidText { file_id, message } => {
                write!(f, "invalid text for file {file_id:?}: {message}")
            }
        }
    }
}

impl std::error::Error for InvalidationError {}

impl Program {
    /// Invalidate program state based on a file update.
    pub fn invalidate_file(
        &self,
        file_id: FileId,
        update: FileUpdate,
    ) -> Result<InvalidationPlan, InvalidationError> {
        // update file content and version
        let is_removed = matches!(update, FileUpdate::Removed);
        let file_version = self.update_file_content(file_id, update)?;

        // initialize invalidation sets
        let mut kinds = HashSet::new();
        let mut modules = HashSet::new();
        let mut packages = HashSet::new();
        let mut profiles = HashSet::new();
        let mut graphs_dropped = HashSet::new();

        // detect config file names for fallback invalidation
        let file = self.files.get(file_id);
        let is_dsconfig = file.name == "dsconfig.json"
            || file
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "dsconfig.json");
        let is_package_manifest = file.name == "package.json"
            || file
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "package.json");

        // map file id to module invalidation
        if let Some(module_id) = self.modules.get_id_by_file_id(file_id) {
            kinds.insert(InvalidationKind::ModuleSource);
            modules.insert(module_id);

            // invalidate module caches and collect profiles
            let module_profiles =
                self.invalidate_module_source(module_id, file_version, is_removed);
            profiles.extend(module_profiles);

            // track packages for module source invalidation
            let module = self.modules.get(module_id);
            packages.insert(module.read().package_id);
        }

        // map file id to dsconfig invalidation
        let dsconfig_packages = self.packages_for_dsconfig_file(file_id);
        if !dsconfig_packages.is_empty() {
            kinds.insert(InvalidationKind::DsConfig);
            packages.extend(dsconfig_packages.iter().copied());

            // collect modules for config invalidation
            let config_modules = self.modules_for_packages(&dsconfig_packages);
            modules.extend(config_modules.iter().copied());
            self.refresh_module_semantics_for_modules(&config_modules);

            // invalidate profile data for config modules
            let config_profiles = self.invalidate_profile_data_for_modules(&config_modules);
            profiles.extend(config_profiles.iter().copied());
            graphs_dropped.extend(config_profiles);
        } else if is_dsconfig {
            // fall back to invalidating all modules for workspace configs
            kinds.insert(InvalidationKind::DsConfig);
            let mut config_modules = Vec::new();
            for module in self.modules.iter() {
                config_modules.push(module.read().id);
            }
            modules.extend(config_modules.iter().copied());
            self.refresh_module_semantics_for_modules(&config_modules);

            // invalidate profile data for config modules
            let config_profiles = self.invalidate_profile_data_for_modules(&config_modules);
            profiles.extend(config_profiles.iter().copied());
            graphs_dropped.extend(config_profiles);
        }

        // map file id to package manifest invalidation
        let package_ids = self.packages_for_manifest_file(file_id);
        if !package_ids.is_empty() {
            kinds.insert(InvalidationKind::PackageManifest);
            packages.extend(package_ids.iter().copied());
            self.refresh_package_manifest_for_file(file_id, &package_ids);

            // collect modules for package invalidation
            let config_modules = self.modules_for_packages(&package_ids);
            modules.extend(config_modules.iter().copied());
            self.refresh_module_semantics_for_modules(&config_modules);

            // invalidate profile data for config modules
            let config_profiles = self.invalidate_profile_data_for_modules(&config_modules);
            profiles.extend(config_profiles.iter().copied());
            graphs_dropped.extend(config_profiles);
        } else if is_package_manifest {
            kinds.insert(InvalidationKind::PackageManifest);
        }

        // map file id to tsconfig invalidation
        let tsconfig_ids = self.tsconfig_ids_for_file(file_id);
        if !tsconfig_ids.is_empty() {
            kinds.insert(InvalidationKind::TsConfig);

            // collect modules for config invalidation
            let config_modules = self.modules_for_tsconfigs(&tsconfig_ids);
            modules.extend(config_modules.iter().copied());
            self.refresh_module_semantics_for_modules(&config_modules);

            // invalidate profile data for config modules
            let config_profiles = self.invalidate_profile_data_for_modules(&config_modules);
            profiles.extend(config_profiles.iter().copied());
            graphs_dropped.extend(config_profiles);
        }

        // ensure package invalidation tracks modules discovered by config updates
        for module_id in &modules {
            let module = self.modules.get(*module_id);
            packages.insert(module.read().package_id);
        }

        // fall back to unknown when no mappings matched
        if kinds.is_empty() {
            kinds.insert(InvalidationKind::Unknown);
        }

        // bump package versions for invalidated packages
        for package_id in &packages {
            let _ = self.packages.bump_version(*package_id);
        }

        // build the invalidation plan
        Ok(InvalidationPlan {
            file_id,
            file_version,
            kinds: kinds.into_iter().collect(),
            modules: modules.into_iter().collect(),
            packages: packages.into_iter().collect(),
            profiles: profiles.into_iter().collect(),
            graphs_dropped: graphs_dropped.into_iter().collect(),
        })
    }

    /// Update file content and return the new file version.
    fn update_file_content(
        &self,
        file_id: FileId,
        update: FileUpdate,
    ) -> Result<FileVersion, InvalidationError> {
        // load the existing file metadata
        let file = self
            .files
            .get_maybe(file_id)
            .ok_or(InvalidationError::FileNotFound { file_id })?;
        let file_name = file.name.clone();
        let file_uri = file.uri.clone();
        let file_path = file.path.clone();
        let file_ty = file.ty;

        // compute the next file version
        let next_version = file.version.next();

        // build the updated file content
        let updated_file = match update {
            FileUpdate::Text { content } => {
                let file = if file_ty == FileType::Json {
                    File::from_text_as_jsonc(
                        file_id, file_name, file_uri, file_path, file_ty, content,
                    )
                    .map_err(|error| InvalidationError::InvalidJson {
                        file_id,
                        message: error.to_string(),
                    })?
                } else {
                    File::from_text(file_id, file_name, file_uri, file_path, file_ty, content)
                };
                file.with_version(next_version)
            }
            FileUpdate::Bytes { content } => {
                if file_ty == FileType::Json {
                    let text = String::from_utf8(content).map_err(|error| {
                        InvalidationError::InvalidJson {
                            file_id,
                            message: error.to_string(),
                        }
                    })?;
                    File::from_text_as_jsonc(file_id, file_name, file_uri, file_path, file_ty, text)
                        .map_err(|error| InvalidationError::InvalidJson {
                            file_id,
                            message: error.to_string(),
                        })?
                        .with_version(next_version)
                } else if file_ty.is_binary() {
                    let len = content.len() as u32;
                    File {
                        id: file_id,
                        version: next_version,
                        name: file_name,
                        uri: file_uri,
                        path: file_path,
                        ty: file_ty,
                        len,
                        content: FileContent::Binary { content },
                        line_start_offsets: None,
                    }
                } else {
                    let text = String::from_utf8(content).map_err(|error| {
                        InvalidationError::InvalidText {
                            file_id,
                            message: error.to_string(),
                        }
                    })?;
                    File::from_text(file_id, file_name, file_uri, file_path, file_ty, text)
                        .with_version(next_version)
                }
            }
            FileUpdate::Touch => file.as_ref().clone().with_version(next_version),
            FileUpdate::Removed => File::missing(file_id, file_name, file_uri, file_path, file_ty)
                .with_version(next_version),
        };

        // write the updated file back to the registry
        self.files.replace(updated_file);

        Ok(next_version)
    }

    /// Invalidate module data for a source change.
    fn invalidate_module_source(
        &self,
        module_id: ModuleId,
        file_version: FileVersion,
        is_removed: bool,
    ) -> HashSet<ProfileId> {
        // collect profiles touched by the module
        let module_profiles = self.collect_module_profiles(module_id);

        // update module versions and clear cached data
        {
            let module = self.modules.get(module_id);
            let mut module = module.write();

            // bump module and source versions
            let next_version = module.version.next();
            module.version = next_version;
            module.source_version = file_version;

            // determine whether to clear non code content
            let mut clear_non_code = false;
            match &mut module.content {
                ModuleContent::Code(code) => {
                    // clear code scoped caches
                    code.ast = None;
                    code.dir_base = None;
                    code.dirs.clear();
                    code.comptimes.clear();
                    code.mirs.clear();
                }
                ModuleContent::Data { .. }
                | ModuleContent::Text { .. }
                | ModuleContent::Binary { .. } => {
                    clear_non_code = true;
                }
                ModuleContent::Unloaded => {}
            }

            // drop non code content when required
            if clear_non_code {
                module.content = ModuleContent::Unloaded;
            }
        }

        // drop shared indexes that lack staleness checks
        self.index.global_symbol_tables.clear();
        self.index.module_binding_registry.clear();
        self.index.module_binding_tables.clear();

        // drop cached signatures for this module
        let signature_keys: Vec<_> = self
            .index
            .module_signatures
            .iter()
            .filter(|entry| entry.key().module_id == module_id)
            .map(|entry| *entry.key())
            .collect();
        for key in signature_keys {
            self.index.module_signatures.remove(&key);
        }

        // drop cached signature digests for removed modules
        if is_removed {
            let digest_keys: Vec<_> = self
                .index
                .module_signature_digests
                .iter()
                .filter(|entry| entry.key().module_id == module_id)
                .map(|entry| *entry.key())
                .collect();
            for key in digest_keys {
                self.index.module_signature_digests.remove(&key);
            }
        }

        module_profiles
    }

    /// Invalidate profile data for the provided modules.
    fn invalidate_profile_data_for_modules(&self, modules: &[ModuleId]) -> HashSet<ProfileId> {
        // collect profiles touched by the modules
        let mut profile_ids = HashSet::new();
        for module_id in modules {
            profile_ids.extend(self.collect_module_profiles(*module_id));
        }

        // bump profile versions to invalidate caches
        for profile_id in &profile_ids {
            let _ = self.profiles.bump_version(*profile_id);
        }

        // drop module graphs for affected profiles
        for profile_id in &profile_ids {
            self.drop_module_graph(*profile_id);
        }

        // drop cached signatures for affected profiles
        let signature_keys: Vec<_> = self
            .index
            .module_signatures
            .iter()
            .filter(|entry| profile_ids.contains(&entry.key().profile_id))
            .map(|entry| *entry.key())
            .collect();
        for key in signature_keys {
            self.index.module_signatures.remove(&key);
        }

        // collect modules that share the profile ids
        let mut modules_to_clear = self.modules_for_profiles(&profile_ids);
        for module_id in modules {
            modules_to_clear.insert(*module_id);
        }

        // clear profile scoped data for each module
        for module_id in modules_to_clear {
            self.clear_module_profiles(module_id, &profile_ids);
        }

        // drop shared indexes that lack staleness checks
        self.index.global_symbol_tables.clear();
        self.index.module_binding_registry.clear();
        self.index.module_binding_tables.clear();

        profile_ids
    }

    /// Clear profile scoped data for a module.
    fn clear_module_profiles(&self, module_id: ModuleId, profiles: &HashSet<ProfileId>) {
        // skip when no profiles are specified
        if profiles.is_empty() {
            return;
        }

        // clear matching profile data
        let module = self.modules.get(module_id);
        let mut module = module.write();
        match &mut module.content {
            ModuleContent::Code(code) => {
                // clear profile dirs and comptime caches
                code.dirs
                    .retain(|dir| dir.profile_id.is_none_or(|id| !profiles.contains(&id)));
                code.comptimes
                    .retain(|entry| !profiles.contains(&entry.profile_id));

                // clear mir caches for affected profiles
                code.mirs.clear();
            }
            ModuleContent::Data { dirs, .. }
            | ModuleContent::Text { dirs, .. }
            | ModuleContent::Binary { dirs, .. } => {
                // clear profile dirs for non code modules
                dirs.retain(|dir| dir.profile_id.is_none_or(|id| !profiles.contains(&id)));
            }
            ModuleContent::Unloaded => {}
        }
    }

    /// Collect profile ids referenced by a module.
    fn collect_module_profiles(&self, module_id: ModuleId) -> HashSet<ProfileId> {
        // collect profiles from module data
        let module = self.modules.get(module_id);
        let module = module.read();
        let mut profiles = HashSet::new();
        match &module.content {
            ModuleContent::Code(code) => {
                // collect profile ids from dirs
                for dir in &code.dirs {
                    if let Some(profile_id) = dir.profile_id {
                        profiles.insert(profile_id);
                    }
                }

                // collect profile ids from comptime entries
                for comptime in &code.comptimes {
                    profiles.insert(comptime.profile_id);
                }
            }
            ModuleContent::Data { dirs, .. }
            | ModuleContent::Text { dirs, .. }
            | ModuleContent::Binary { dirs, .. } => {
                // collect profile ids from non code dirs
                for dir in dirs {
                    if let Some(profile_id) = dir.profile_id {
                        profiles.insert(profile_id);
                    }
                }
            }
            ModuleContent::Unloaded => {}
        }

        // release the module lock before reading signatures
        drop(module);

        // collect profiles from cached signatures
        for entry in self.index.module_signatures.iter() {
            let key = entry.key();
            if key.module_id == module_id {
                profiles.insert(key.profile_id);
            }
        }

        // collect profiles from signature digests
        for entry in self.index.module_signature_digests.iter() {
            let key = entry.key();
            if key.module_id == module_id {
                profiles.insert(key.profile_id);
            }
        }

        profiles
    }

    /// Collect modules that have data for any of the provided profiles.
    fn modules_for_profiles(&self, profiles: &HashSet<ProfileId>) -> HashSet<ModuleId> {
        // skip empty profile sets
        if profiles.is_empty() {
            return HashSet::new();
        }

        // collect modules with profile scoped data
        let mut modules = HashSet::new();
        for module in self.modules.iter() {
            let module = module.read();
            let mut matches_profile = false;
            match &module.content {
                ModuleContent::Code(code) => {
                    // check dirs for matching profiles
                    if code
                        .dirs
                        .iter()
                        .any(|dir| dir.profile_id.is_some_and(|id| profiles.contains(&id)))
                    {
                        matches_profile = true;
                    }

                    // check comptime entries for matching profiles
                    if !matches_profile
                        && code
                            .comptimes
                            .iter()
                            .any(|entry| profiles.contains(&entry.profile_id))
                    {
                        matches_profile = true;
                    }
                }
                ModuleContent::Data { dirs, .. }
                | ModuleContent::Text { dirs, .. }
                | ModuleContent::Binary { dirs, .. } => {
                    // check dirs for matching profiles
                    if dirs
                        .iter()
                        .any(|dir| dir.profile_id.is_some_and(|id| profiles.contains(&id)))
                    {
                        matches_profile = true;
                    }
                }
                ModuleContent::Unloaded => {}
            }

            if matches_profile {
                modules.insert(module.id);
            }
        }

        modules
    }

    /// Find packages that own the given dsconfig file id.
    fn packages_for_dsconfig_file(&self, file_id: FileId) -> Vec<PackageId> {
        // collect packages that reference the dsconfig file id
        let mut packages = Vec::new();
        for package in self.packages.iter() {
            let package = package.read();
            let Some(dsconfig) = package.dsconfig.as_ref() else {
                continue;
            };
            if dsconfig.file_id == file_id {
                packages.push(package.id);
            }
        }
        packages
    }

    /// Find tsconfig ids that match the given file id.
    fn tsconfig_ids_for_file(&self, file_id: FileId) -> Vec<TsConfigId> {
        // collect tsconfigs that reference the file id
        let mut tsconfigs = Vec::new();
        for tsconfig in self.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            if tsconfig.file_id == file_id {
                tsconfigs.push(tsconfig.id);
            }
        }
        tsconfigs
    }

    /// Collect modules that belong to a package.
    fn modules_for_package(&self, package_id: PackageId) -> Vec<ModuleId> {
        // collect modules for the package
        let mut modules = Vec::new();
        for module in self.modules.iter() {
            let module = module.read();
            if module.package_id == package_id {
                modules.push(module.id);
            }
        }
        modules
    }

    /// Collect modules that belong to any of the specified packages.
    fn modules_for_packages(&self, package_ids: &[PackageId]) -> Vec<ModuleId> {
        // collect modules across packages
        let mut modules = Vec::new();
        for package_id in package_ids {
            modules.extend(self.modules_for_package(*package_id));
        }
        modules
    }

    /// Collect modules that reference the given tsconfigs.
    fn modules_for_tsconfigs(&self, tsconfigs: &[TsConfigId]) -> Vec<ModuleId> {
        // collect tsconfig ids for lookup
        let mut tsconfig_ids = HashSet::new();
        for tsconfig in tsconfigs {
            tsconfig_ids.insert(*tsconfig);
        }

        // collect modules mapped to those tsconfigs
        let mut modules = Vec::new();
        for module in self.modules.iter() {
            let module = module.read();
            if module
                .tsconfig_id
                .is_some_and(|id| tsconfig_ids.contains(&id))
            {
                modules.push(module.id);
            }
        }
        modules
    }

    /// Find packages that own the given package.json file id.
    fn packages_for_manifest_file(&self, file_id: FileId) -> Vec<PackageId> {
        // collect packages that reference the package manifest file id
        let mut packages = Vec::new();
        for package in self.packages.iter() {
            let package = package.read();
            let Some(manifest) = package.manifest.as_ref() else {
                continue;
            };
            if manifest.file_id == file_id {
                packages.push(package.id);
            }
        }

        packages
    }

    /// Refresh package manifest data for one invalidated file.
    fn refresh_package_manifest_for_file(&self, file_id: FileId, package_ids: &[PackageId]) {
        // load the current package manifest file state
        let file = self.files.get(file_id);

        for package_id in package_ids {
            let package = self.packages.get(*package_id);
            let mut package = package.write();
            let Some(existing_manifest) = package.manifest.as_ref() else {
                continue;
            };
            if existing_manifest.file_id != file_id {
                continue;
            }

            // drop package manifest when file is missing
            if file.is_missing() {
                package.manifest = None;
                continue;
            }

            // parse and apply the updated package manifest
            let Ok(next_manifest) =
                crate::PackageManifest::parse(&file, existing_manifest.realpath.clone())
            else {
                continue;
            };
            package.name = Some(next_manifest.name.clone());
            package.version = Some(next_manifest.version.clone());
            package.manifest = Some(next_manifest);
        }
    }

    /// Refresh module source and format semantics for modules.
    fn refresh_module_semantics_for_modules(&self, modules: &[ModuleId]) {
        // deduplicate modules before refreshing
        let mut visited = HashSet::new();
        for module_id in modules {
            if !visited.insert(*module_id) {
                continue;
            }

            self.refresh_module_semantics(*module_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use indexmap::IndexMap;

    use destack_source::{
        File, FileContent, FileRegistry, FileType, LanguageType, MemoryFileSystem, ModuleId,
        PackageId, PackageVersion, Uri,
    };

    use crate::{
        DsConfig, EnvSnapshot, Loader, Module, ModuleAst, ModuleDetection, ModuleDir, ModuleFormat,
        ModuleSource, ModuleTarget, OutputFormat, Package, PackageKind, PackageManifest, Platform,
        ProfileFlags, ProfileId, ProfileKey, Program, Runtime, SourceType, TsConfig,
    };

    use super::{FileUpdate, InvalidationError};

    /// Reject invalid utf8 byte updates for text files.
    #[test]
    fn test_invalidate_file_rejects_invalid_utf8_bytes() {
        // set up a program with a single text file
        let fs = Arc::new(MemoryFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Program::from_fs(PathBuf::from("/workspace"), fs, files.clone());

        let file_id = files.next_id();
        let path = PathBuf::from("/workspace/src/main.ts");
        let (name, uri) = Uri::from_path_with_name(&path);
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(path),
            FileType::TypeScript,
            "export const value = 1;".to_string(),
        );
        let initial_version = file.version;
        files.insert(file);

        // attempt to write invalid utf8 bytes
        let result = program.invalidate_file(
            file_id,
            FileUpdate::Bytes {
                content: vec![0xff, 0xfe],
            },
        );

        // check that the file is not invalidated
        assert!(matches!(result, Err(InvalidationError::InvalidText { .. })));
        let stored = program.files.get(file_id);
        assert_eq!(stored.version, initial_version);
        assert_eq!(stored.text(), "export const value = 1;");
    }

    /// Mark removed files as missing in the registry.
    #[test]
    fn test_invalidate_file_marks_missing() {
        let fs = Arc::new(MemoryFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Program::from_fs(PathBuf::from("/workspace"), fs, files.clone());

        let file_id = files.next_id();
        let path = PathBuf::from("/workspace/src/main.ts");
        let (name, uri) = Uri::from_path_with_name(&path);
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(path),
            FileType::TypeScript,
            "export const value = 1;".to_string(),
        );
        let initial_version = file.version;
        files.insert(file);

        program
            .invalidate_file(file_id, FileUpdate::Removed)
            .expect("failed to remove file");

        // check that the file is marked as missing
        let stored = program.files.get(file_id);
        assert!(stored.is_missing());
        assert_eq!(stored.version, initial_version.next());
        assert!(matches!(stored.content, FileContent::Missing));
    }

    /// Clear profile scoped data for every module that shares the invalidated profile.
    #[test]
    fn test_invalidate_profile_clears_shared_profile_modules() {
        // set up a program with two packages and shared profile data
        let fs = Arc::new(MemoryFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Program::from_fs(PathBuf::from("/workspace"), fs, files.clone());

        let package_a_id = PackageId::new(1);
        let package_b_id = PackageId::new(2);
        let package_a_path = PathBuf::from("/workspace/pkg-a");
        let package_b_path = PathBuf::from("/workspace/pkg-b");

        let dsconfig_file_id = files.next_id();
        let dsconfig_path = package_a_path.join("dsconfig.json");
        let (ds_name, ds_uri) = Uri::from_path_with_name(&dsconfig_path);
        let dsconfig_file = File::from_text_as_jsonc(
            dsconfig_file_id,
            ds_name,
            ds_uri,
            Some(dsconfig_path),
            FileType::Json,
            "{}".to_string(),
        )
        .unwrap_or_else(|error| panic!("failed to build dsconfig: {error}"));
        files.insert(dsconfig_file);
        let dsconfig_file = files.get(dsconfig_file_id);
        let dsconfig = DsConfig::parse(&dsconfig_file)
            .unwrap_or_else(|error| panic!("failed to parse dsconfig: {error}"));

        let package_a = Package {
            id: package_a_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Physical,
            uri: Uri::from_path(&package_a_path),
            path: Some(package_a_path.clone()),
            name: Some("pkg-a".to_string()),
            version: Some("0.1.0".to_string()),
            manifest: None,
            dsconfig: Some(dsconfig),
            tsconfig: None,
            targets: IndexMap::new(),
        };
        let package_b = Package {
            id: package_b_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Physical,
            uri: Uri::from_path(&package_b_path),
            path: Some(package_b_path.clone()),
            name: Some("pkg-b".to_string()),
            version: Some("0.1.0".to_string()),
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: IndexMap::new(),
        };
        program.packages.insert(package_a);
        program.packages.insert(package_b);

        let module_a_id = insert_module(
            &program,
            &files,
            package_a_id,
            &package_a_path,
            Path::new("src/a.ts"),
            "export const value = 1;",
        );
        let module_b_id = insert_module(
            &program,
            &files,
            package_b_id,
            &package_b_path,
            Path::new("src/b.ts"),
            "export const value = 2;",
        );

        let profile_key = ProfileKey::new(
            OutputFormat::Js,
            Runtime::Node,
            Platform::Web,
            None,
            None,
            None,
            Vec::new(),
            false,
            false,
            false,
            EnvSnapshot::from_env_all(),
            ProfileFlags::default(),
        );
        let profile_id = program.profiles.get_or_create(profile_key);

        attach_profile_dir(&program, module_a_id, profile_id);
        attach_profile_dir(&program, module_b_id, profile_id);

        // ensure profile data exists before invalidation
        let module_a = program.modules.get(module_a_id);
        let module_b = program.modules.get(module_b_id);
        assert!(module_a.read().dir_maybe(profile_id).is_some());
        assert!(module_b.read().dir_maybe(profile_id).is_some());

        // invalidate the dsconfig file and expect shared profile data to drop
        program
            .invalidate_file(dsconfig_file_id, FileUpdate::Touch)
            .unwrap_or_else(|error| panic!("failed to invalidate dsconfig: {error}"));

        // check that the profile data is cleared
        let module_a = program.modules.get(module_a_id);
        let module_b = program.modules.get(module_b_id);
        assert!(module_a.read().dir_maybe(profile_id).is_none());
        assert!(module_b.read().dir_maybe(profile_id).is_none());
    }

    /// Refresh module semantics when package manifest module type changes.
    #[test]
    fn test_invalidate_package_manifest_refreshes_module_semantics() {
        let fs = Arc::new(MemoryFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Program::from_fs(PathBuf::from("/workspace"), fs, files.clone());

        // insert package manifest file with commonjs module type
        let package_json_file_id = files.next_id();
        let package_json_path = PathBuf::from("/workspace/pkg/package.json");
        let (package_json_name, package_json_uri) = Uri::from_path_with_name(&package_json_path);
        let package_json_file = File::from_text_as_jsonc(
            package_json_file_id,
            package_json_name,
            package_json_uri.clone(),
            Some(package_json_path.clone()),
            FileType::Json,
            r#"{ "name": "pkg", "version": "0.1.0", "type": "commonjs" }"#.to_string(),
        )
        .unwrap_or_else(|error| panic!("failed to create package manifest file: {error}"));
        files.insert(package_json_file);
        let package_json_file = files.get(package_json_file_id);
        let package_manifest =
            PackageManifest::parse(&package_json_file, package_json_path.clone())
                .unwrap_or_else(|error| panic!("failed to parse package manifest: {error}"));

        // insert package with manifest
        let package_id = PackageId::from_path(Path::new("/workspace/pkg"));
        let package = Package {
            id: package_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Physical,
            uri: Uri::from_path(PathBuf::from("/workspace/pkg")),
            path: Some(PathBuf::from("/workspace/pkg")),
            name: Some("pkg".to_string()),
            version: Some("0.1.0".to_string()),
            manifest: Some(package_manifest),
            dsconfig: None,
            tsconfig: None,
            targets: IndexMap::new(),
        };
        program.packages.insert(package);

        // insert a script-like typescript module under the package
        let module_file_id = files.next_id();
        let module_path = PathBuf::from("/workspace/pkg/src/main.ts");
        let (module_name, module_uri) = Uri::from_path_with_name(&module_path);
        let module_file = File::from_text(
            module_file_id,
            module_name,
            module_uri.clone(),
            Some(module_path.clone()),
            FileType::TypeScript,
            "const value = 1;".to_string(),
        );
        files.insert(module_file);
        let module_file = files.get(module_file_id);
        let module_id = ModuleId::from_relative_path(package_id, Path::new("src/main.ts"));
        let module = Module::blank(
            module_id,
            module_file_id,
            module_file.version,
            module_uri,
            Some(module_path),
            package_id,
            None,
            SourceType::Script,
            ModuleFormat::CommonJs,
            LanguageType::TypeScript,
            Loader::TypeScript,
            ModuleSource::User,
        );
        program.modules.insert(module);

        // update package manifest module type to module and invalidate
        let result = program
            .invalidate_file(
                package_json_file_id,
                FileUpdate::Text {
                    content: r#"{ "name": "pkg", "version": "0.2.0", "type": "module" }"#
                        .to_string(),
                },
            )
            .unwrap_or_else(|error| panic!("failed to invalidate package manifest: {error}"));

        // assert invalidation kind and module semantics were updated
        assert!(
            result
                .kinds
                .contains(&super::InvalidationKind::PackageManifest)
        );
        let module = program.modules.get(module_id);
        let module = module.read();
        assert!(module.source_type.is_module());
        assert!(module.module_format.is_esm());
    }

    /// Refresh module semantics when tsconfig mapping is invalidated.
    #[test]
    fn test_invalidate_tsconfig_refreshes_module_semantics() {
        let fs = Arc::new(MemoryFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Program::from_fs(PathBuf::from("/workspace"), fs, files.clone());

        // insert package
        let package_id = PackageId::from_path(Path::new("/workspace/pkg"));
        let package = Package {
            id: package_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Physical,
            uri: Uri::from_path(PathBuf::from("/workspace/pkg")),
            path: Some(PathBuf::from("/workspace/pkg")),
            name: Some("pkg".to_string()),
            version: Some("0.1.0".to_string()),
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: IndexMap::new(),
        };
        program.packages.insert(package);

        // insert tsconfig file with force+commonjs semantics
        let tsconfig_file_id = files.next_id();
        let tsconfig_path = PathBuf::from("/workspace/pkg/tsconfig.json");
        let (tsconfig_name, tsconfig_uri) = Uri::from_path_with_name(&tsconfig_path);
        let tsconfig_file = File::from_text_as_jsonc(
            tsconfig_file_id,
            tsconfig_name,
            tsconfig_uri.clone(),
            Some(tsconfig_path.clone()),
            FileType::Json,
            r#"{ "compilerOptions": { "moduleDetection": "force", "module": "commonjs" } }"#
                .to_string(),
        )
        .unwrap_or_else(|error| panic!("failed to create tsconfig file: {error}"));
        files.insert(tsconfig_file);
        let tsconfig_file = files.get(tsconfig_file_id);
        let tsconfig_id = program.tsconfigs.next_id();
        let tsconfig = TsConfig::parse(tsconfig_id, true, &tsconfig_file)
            .unwrap_or_else(|error| panic!("failed to parse tsconfig: {error}"));
        program.tsconfigs.insert(tsconfig);

        // insert a typescript module that intentionally starts with stale semantics
        let module_file_id = files.next_id();
        let module_path = PathBuf::from("/workspace/pkg/src/main.ts");
        let (module_name, module_uri) = Uri::from_path_with_name(&module_path);
        let module_file = File::from_text(
            module_file_id,
            module_name,
            module_uri.clone(),
            Some(module_path.clone()),
            FileType::TypeScript,
            "const value = 1;".to_string(),
        );
        files.insert(module_file);
        let module_file = files.get(module_file_id);
        let module_id = ModuleId::from_relative_path(package_id, Path::new("src/main.ts"));
        let module = Module::blank(
            module_id,
            module_file_id,
            module_file.version,
            module_uri,
            Some(module_path),
            package_id,
            Some(tsconfig_id),
            SourceType::Script,
            ModuleFormat::Esm,
            LanguageType::TypeScript,
            Loader::TypeScript,
            ModuleSource::User,
        );
        program.modules.insert(module);

        // invalidate the tsconfig file and assert semantics are refreshed
        let result = program
            .invalidate_file(tsconfig_file_id, FileUpdate::Touch)
            .unwrap_or_else(|error| panic!("failed to invalidate tsconfig: {error}"));
        assert!(result.kinds.contains(&super::InvalidationKind::TsConfig));

        let module = program.modules.get(module_id);
        let module = module.read();
        assert!(module.source_type.is_module());
        assert!(module.module_format.is_commonjs());

        // assert compiler options are still intact for this test setup
        let tsconfig = program.tsconfigs.get(tsconfig_id);
        let tsconfig = tsconfig.read();
        assert_eq!(
            tsconfig.options.compiler.module_detection,
            ModuleDetection::Force
        );
        assert_eq!(tsconfig.options.compiler.module, ModuleTarget::CommonJs);
    }

    /// Register a new code module for testing.
    fn insert_module(
        program: &Program,
        files: &Arc<FileRegistry>,
        package_id: PackageId,
        package_path: &Path,
        relative_path: &Path,
        source: &str,
    ) -> ModuleId {
        // build file and module paths
        let path = package_path.join(relative_path);
        let (name, uri) = Uri::from_path_with_name(&path);

        // insert the source file
        let file_id = files.next_id();
        let file = File::from_text(
            file_id,
            name,
            uri.clone(),
            Some(path.clone()),
            FileType::TypeScript,
            source.to_string(),
        );
        let file_version = file.version;
        files.insert(file);

        // register the module
        let module_id = ModuleId::from_relative_path(package_id, relative_path);
        let module = Module::blank(
            module_id,
            file_id,
            file_version,
            uri,
            Some(path),
            package_id,
            None,
            SourceType::Script,
            ModuleFormat::CommonJs,
            LanguageType::TypeScript,
            Loader::from_file_type(FileType::TypeScript),
            ModuleSource::User,
        );
        program.modules.insert(module);

        module_id
    }

    /// Attach a profile scoped dir to a module.
    fn attach_profile_dir(program: &Program, module_id: ModuleId, profile_id: ProfileId) {
        // add a minimal dir entry for the profile
        let module = program.modules.get(module_id);
        let mut module = module.write();

        // file id
        let file_id = module.file_id;

        // anchor expression
        let anchor_id = match module.code_mut().ast.as_mut() {
            Some(ast) => ast.ensure_anchor_expression(file_id),
            None => {
                // synthesize a minimal AST for diagnostics
                let mut ast = ModuleAst::new(module_id, module.version);
                let anchor_id = ast.ensure_anchor_expression(file_id);
                module.code_mut().ast = Some(ast);
                anchor_id
            }
        };
        let mut dir = ModuleDir::new_base(module_id, module.version, anchor_id.id);
        dir.profile_id = Some(profile_id);
        module.code_mut().dirs.push(dir);
    }
}
