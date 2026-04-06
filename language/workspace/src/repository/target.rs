use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{EnvSnapshot, Platform, ProfileKey, Runtime};
use destack_source::{FileId, ModuleId, PackageId, ProfileId, TargetId, matches as glob_matches};

use crate::repository::{ModuleTsConfigContext, Profile, Repository, RepositoryError, Revision};
use crate::{
    CompilerOptions, DsPathAliases, EntryResolutionMode, EntrySource, Module, ModuleDetection,
    ModuleFormat, Package, PackageOptions, ProfileConfig, ProfileEnv, Target, TargetDiscoveryIssue,
    TargetDiscoveryOptions, TsConfigDeclaration, TsConfigOptions, builtin_libs_for_type_entries,
    discover_typescript_type_entries, normalize_typescript_lib_names,
    normalize_typescript_type_entries, profile_flags_for_compiler_options, typescript_default_libs,
};

impl Repository {
    /// Build compiler options for one target.
    pub fn compiler_options_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
    ) -> CompilerOptions {
        let mut options = compiler_options.clone();
        let is_native_output = target.emit.is_wasm() || target.emit.is_native();

        // native-like outputs force stricter semantics
        if is_native_output {
            options.apply_native_restrictions();
        }

        options
    }

    /// Access tsconfig for a module via closure.
    pub fn read_tsconfig_declaration_for_module<T>(
        &self,
        revision: Revision,
        module: &Module,
        read: impl FnOnce(&TsConfigDeclaration) -> T,
    ) -> Result<Option<T>, RepositoryError> {
        let Some(tsconfig_file_id) = module.tsconfig_file_id else {
            return Ok(None);
        };
        let Some(tsconfig) = self.tsconfig_declaration(revision, tsconfig_file_id)? else {
            return Ok(None);
        };

        Ok(Some(read(&tsconfig)))
    }

    /// Return tsconfig options for one module.
    pub fn tsconfig_options_for_module(
        &self,
        revision: Revision,
        module: &Module,
    ) -> Result<Option<TsConfigOptions>, RepositoryError> {
        self.read_tsconfig_declaration_for_module(revision, module, |tsconfig| tsconfig.options())
    }

    /// Return package options for one module.
    pub fn package_options_for_module(
        &self,
        revision: Revision,
        module: &Module,
    ) -> Result<Option<PackageOptions>, RepositoryError> {
        self.package_options(revision, module.package_id)
    }

    /// Return the package id for one target id when known.
    pub fn package_id_by_target_id(&self, target_id: TargetId) -> Option<PackageId> {
        self.package_id_by_target_id
            .get(&target_id)
            .map(|entry| *entry.value())
    }

    /// Return the target name for one target id when known.
    pub fn target_name_by_target_id(&self, target_id: TargetId) -> Option<Arc<str>> {
        self.target_name_by_target_id
            .get(&target_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Intern one target identity and return its stable id.
    pub fn intern_target_id(&self, package_id: PackageId, name: &str) -> TargetId {
        let target_id = TargetId::new(package_id, name);

        self.package_id_by_target_id
            .entry(target_id)
            .or_insert(package_id);
        self.target_name_by_target_id
            .entry(target_id)
            .or_insert_with(|| Arc::<str>::from(name.to_string()));

        target_id
    }

    /// Get the default profile for one module.
    pub fn default_profile_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Profile, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };

        let (compiler_options, tsconfig_context) =
            self.profile_compiler_options_for_module(revision, &package, &module)?;
        let package_options = self.package_options(revision, package.id)?;

        let (target, profile_config) = if let Some(package_options) = package_options.as_ref() {
            let target = package_options
                .default_target
                .as_ref()
                .and_then(|name| {
                    let target_id = self.intern_target_id(package.id, name);
                    package.targets.get(&target_id)
                })
                .or_else(|| package.targets.values().find(|target| !target.synthetic))
                .cloned()
                .unwrap_or_else(|| self.implicit_target_for_module(&module));
            let profile_config = compiler_options
                .profile
                .as_ref()
                .and_then(|name| package_options.profiles.get(name));
            (target, profile_config)
        } else {
            (self.implicit_target_for_module(&module), None)
        };

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_context.as_ref().map(|context| &context.options),
        );

        Ok(Profile::from_key(key))
    }

    /// Get the default profile id for one module.
    pub fn default_profile_id_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<ProfileId, RepositoryError> {
        Ok(self.default_profile_for_module(revision, module_id)?.id())
    }

    /// Get the profile for a target.
    pub fn profile_for_target(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Result<Option<Profile>, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };
        let Some(target) = package.targets.get(target_id).cloned().or_else(|| {
            self.target_name_by_target_id(*target_id).and_then(|name| {
                let name: &str = name.as_ref();
                Target::implicit_for_name(name)
            })
        }) else {
            return Ok(None);
        };

        let (compiler_options, tsconfig_context) =
            self.profile_compiler_options_for_module(revision, &package, &module)?;
        let package_options = self.package_options(revision, package.id)?;
        let profile_config = package_options.as_ref().and_then(|package_options| {
            target
                .profile
                .as_ref()
                .or(compiler_options.profile.as_ref())
                .and_then(|name| package_options.profiles.get(name))
        });

        let key = Self::profile_key_for_target(
            &target,
            &compiler_options,
            profile_config,
            tsconfig_context.as_ref().map(|context| &context.options),
        );

        Ok(Some(Profile::from_key(key)))
    }

    /// Get the profile id for a target.
    pub fn profile_id_for_target(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Result<Option<ProfileId>, RepositoryError> {
        Ok(self
            .profile_for_target(revision, module_id, target_id)?
            .map(|profile| profile.id()))
    }

    /// Get the profile for a target, falling back to the module default profile.
    pub fn profile_for_target_or_default(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Result<Profile, RepositoryError> {
        let profile = self.profile_for_target(revision, module_id, target_id)?;

        Ok(profile.unwrap_or(self.default_profile_for_module(revision, module_id)?))
    }

    /// Get the profile id for a target, falling back to the module default profile.
    pub fn profile_id_for_target_or_default(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Result<ProfileId, RepositoryError> {
        Ok(self
            .profile_for_target_or_default(revision, module_id, target_id)?
            .id())
    }

    /// Select one diagnostic target id for one module without mutating repository state.
    pub fn diagnostic_target_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<TargetId, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };

        if let Some((target_id, _)) = package.targets.iter().find(|(_, target)| {
            !target.synthetic && (target.emit.is_native() || target.emit.is_wasm())
        }) {
            return Ok(*target_id);
        }

        Ok(self.intern_target_id(package.id, "native"))
    }

    /// Return one tsconfig module detection mode.
    pub(crate) fn tsconfig_module_detection_at(
        &self,
        revision: Revision,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<ModuleDetection, RepositoryError> {
        let Some(tsconfig_file_id) = tsconfig_file_id else {
            return Ok(ModuleDetection::default());
        };

        let Some(tsconfig) = self.tsconfig_declaration(revision, tsconfig_file_id)? else {
            return Ok(ModuleDetection::default());
        };
        let options = tsconfig.options();

        Ok(options.compiler.module_detection)
    }

    /// Return one tsconfig module format override.
    pub(crate) fn tsconfig_module_format_at(
        &self,
        revision: Revision,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<Option<ModuleFormat>, RepositoryError> {
        let Some(tsconfig_file_id) = tsconfig_file_id else {
            return Ok(None);
        };

        let Some(tsconfig) = self.tsconfig_declaration(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        let module_target = tsconfig.options().compiler.module;

        Ok(ModuleFormat::from_tsconfig_target(module_target))
    }

    /// Match one target glob against one path.
    fn matches_target_glob(pattern: &str, path: &Path) -> bool {
        let pattern = pattern.as_bytes();
        let path = path.to_string_lossy();
        let path = path.as_bytes();

        glob_matches(pattern, 0, path, 0)
    }

    /// Select effective entry paths for one target.
    fn select_target_entry_paths(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
        options: &TargetDiscoveryOptions<'_>,
    ) -> Result<Vec<PathBuf>, TargetDiscoveryIssue> {
        // target entries
        if matches!(options.entry_source, EntrySource::Target) {
            return Ok(target.entry.clone());
        }

        // target-first auto mode
        if matches!(options.entry_source, EntrySource::Auto) && !target.entry.is_empty() {
            return Ok(target.entry.clone());
        }

        let package_directory =
            package_path
                .as_ref()
                .ok_or(TargetDiscoveryIssue::MissingPackagePath {
                    package: package_id,
                    target: *target_id,
                })?;

        // no manifest entries
        if options.manifest_entry_targets.is_empty() {
            return Ok(Vec::new());
        }

        let module_ids = self
            .package_module_ids(revision, package_id)
            .map_err(|error| TargetDiscoveryIssue::Repository {
                package: package_id,
                target: *target_id,
                message: error.to_string(),
            })?;
        let mut candidates = Vec::new();

        for module_id in module_ids {
            let module = self.module(revision, module_id).map_err(|error| {
                TargetDiscoveryIssue::Repository {
                    package: package_id,
                    target: *target_id,
                    message: error.to_string(),
                }
            })?;
            let Some(module) = module else {
                continue;
            };
            let Some(module_path) = module.path.clone() else {
                continue;
            };

            candidates.push(module_path);
        }

        Ok(Self::select_manifest_entry_paths(
            package_directory,
            &candidates,
            options.manifest_entry_targets,
        ))
    }

    /// Select package entry paths from manifest target names.
    fn select_manifest_entry_paths(
        package_directory: &Path,
        candidates: &[PathBuf],
        manifest_entry_targets: &[String],
    ) -> Vec<PathBuf> {
        let mut selected_paths = Vec::new();

        for manifest_entry_target in manifest_entry_targets {
            let manifest_entry_path = package_directory.join(manifest_entry_target);

            if candidates.contains(&manifest_entry_path) {
                selected_paths.push(manifest_entry_path);
            }
        }

        selected_paths
    }

    /// Resolve selected entry paths to package-local module ids.
    fn resolve_entry_modules(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target_id: &TargetId,
        entry_paths: &[PathBuf],
        resolution_mode: EntryResolutionMode,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        let mut discovered_modules = Vec::new();

        for entry_path in entry_paths {
            let module_id = match resolution_mode {
                EntryResolutionMode::Strict => self.resolve_entry_module_strict(
                    revision,
                    package_id,
                    package_path,
                    target_id,
                    entry_path,
                )?,
                EntryResolutionMode::RepositoryRelative => self
                    .resolve_entry_module_repository_relative(
                        revision,
                        package_id,
                        package_path,
                        target_id,
                        entry_path,
                    )?,
            };

            discovered_modules.push(module_id);
        }

        Ok(discovered_modules)
    }

    /// Resolve one entry path relative to package path only.
    fn resolve_entry_module_strict(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target_id: &TargetId,
        entry_path: &Path,
    ) -> Result<ModuleId, TargetDiscoveryIssue> {
        let package_directory =
            package_path
                .as_ref()
                .ok_or(TargetDiscoveryIssue::MissingPackagePath {
                    package: package_id,
                    target: *target_id,
                })?;

        let resolved_path = package_directory.join(entry_path);
        self.resolve_entry_module_id(revision, package_id, &resolved_path)
            .ok_or(TargetDiscoveryIssue::MissingEntry {
                package: package_id,
                target: *target_id,
                path: resolved_path,
            })
    }

    /// Resolve one entry path with package-relative and repository-relative checks.
    fn resolve_entry_module_repository_relative(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target_id: &TargetId,
        entry_path: &Path,
    ) -> Result<ModuleId, TargetDiscoveryIssue> {
        let mut candidate_paths = Vec::new();

        if !entry_path.is_absolute() {
            let package_directory =
                package_path
                    .as_ref()
                    .ok_or(TargetDiscoveryIssue::MissingPackagePath {
                        package: package_id,
                        target: *target_id,
                    })?;
            candidate_paths.push(package_directory.join(entry_path));
        }

        candidate_paths.push(entry_path.to_path_buf());

        for candidate_path in &candidate_paths {
            if let Some(module_id) =
                self.resolve_entry_module_id(revision, package_id, candidate_path)
            {
                return Ok(module_id);
            }
        }

        let missing_path = candidate_paths
            .first()
            .cloned()
            .unwrap_or_else(|| entry_path.to_path_buf());

        Err(TargetDiscoveryIssue::MissingEntry {
            package: package_id,
            target: *target_id,
            path: missing_path,
        })
    }

    /// Resolve one entry path to a package-local module id.
    fn resolve_entry_module_id(
        &self,
        revision: Revision,
        package_id: PackageId,
        entry_path: &Path,
    ) -> Option<ModuleId> {
        let module_id = self.module_id_for_path(revision, entry_path).ok()??;
        let module = self.module(revision, module_id).ok()??;

        if module.package_id != package_id {
            return None;
        }

        Some(module_id)
    }

    /// Discover entry module ids for one package target.
    pub fn entry_module_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
        options: &TargetDiscoveryOptions<'_>,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        let entry_paths = self.select_target_entry_paths(
            revision,
            package_id,
            package_path,
            target,
            target_id,
            options,
        )?;

        self.resolve_entry_modules(
            revision,
            package_id,
            package_path,
            target_id,
            &entry_paths,
            options.entry_resolution,
        )
    }

    /// Discover include module ids for one package target.
    pub fn include_module_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        let mut discovered_modules = Vec::new();

        // package-local include scan
        let module_ids = self
            .package_module_ids(revision, package_id)
            .map_err(|error| TargetDiscoveryIssue::Repository {
                package: package_id,
                target: *target_id,
                message: error.to_string(),
            })?;
        for module_id in module_ids {
            let Some(module) = self.module(revision, module_id).map_err(|error| {
                TargetDiscoveryIssue::Repository {
                    package: package_id,
                    target: *target_id,
                    message: error.to_string(),
                }
            })?
            else {
                continue;
            };
            let module = module.as_ref();
            let Some(module_path) = module.path.as_ref() else {
                continue;
            };

            let relative_path = package_path
                .as_ref()
                .and_then(|package_path| module_path.strip_prefix(package_path).ok())
                .unwrap_or(module_path);

            let is_included = target.include.is_empty()
                || target
                    .include
                    .iter()
                    .any(|pattern| Self::matches_target_glob(pattern, relative_path));

            if !is_included {
                continue;
            }

            let is_excluded = target
                .exclude
                .iter()
                .any(|pattern| Self::matches_target_glob(pattern, relative_path));

            if is_excluded {
                continue;
            }

            discovered_modules.push(module.id);
        }

        Ok(discovered_modules)
    }

    /// Build profile compiler options for one module.
    fn profile_compiler_options_for_module(
        &self,
        revision: Revision,
        package: &Package,
        module: &Module,
    ) -> Result<(CompilerOptions, Option<ModuleTsConfigContext>), RepositoryError> {
        let mut compiler_options = self
            .package_options(revision, package.id)?
            .map(|package_options| package_options.compiler)
            .unwrap_or_default();

        let tsconfig_context = if package.destack_file_id.is_none() {
            self.read_tsconfig_declaration_for_module(revision, module, |tsconfig| {
                ModuleTsConfigContext {
                    options: tsconfig.options(),
                    directory: tsconfig.directory.clone(),
                }
            })?
        } else {
            None
        };

        if let Some(tsconfig_context) = tsconfig_context.as_ref() {
            Self::apply_tsconfig_profile_overrides(
                &mut compiler_options,
                &tsconfig_context.options,
            );
            self.apply_tsconfig_implicit_type_overrides(
                module,
                &mut compiler_options,
                tsconfig_context,
            );
        }

        Ok((compiler_options, tsconfig_context))
    }

    /// Apply tsconfig profile overrides to compiler options.
    fn apply_tsconfig_profile_overrides(
        compiler_options: &mut CompilerOptions,
        tsconfig_options: &TsConfigOptions,
    ) {
        let ts_compiler_options = &tsconfig_options.compiler;

        compiler_options.base_url = ts_compiler_options.base_url.clone();
        compiler_options.paths = ts_compiler_options.paths.as_ref().map(|paths| {
            let mut mapped_paths = DsPathAliases::default();
            for (key, values) in paths {
                mapped_paths.insert(key.clone(), values.clone());
            }
            mapped_paths
        });
        compiler_options.module = ts_compiler_options.module;
        compiler_options.es_target = ts_compiler_options.es_target;
        compiler_options.allow_js = ts_compiler_options.allow_js;
        compiler_options.check_js = ts_compiler_options.check_js;
        compiler_options.skip_lib_check = ts_compiler_options.skip_lib_check;

        if !ts_compiler_options.lib.is_empty() {
            compiler_options.lib = normalize_typescript_lib_names(&ts_compiler_options.lib);
        }

        if !ts_compiler_options.types.is_empty() {
            compiler_options.types = normalize_typescript_type_entries(&ts_compiler_options.types);
        }
    }

    /// Apply implicit tsconfig type overrides.
    fn apply_tsconfig_implicit_type_overrides(
        &self,
        module: &Module,
        compiler_options: &mut CompilerOptions,
        tsconfig_context: &ModuleTsConfigContext,
    ) {
        if !tsconfig_context.options.compiler.types.is_empty() {
            return;
        }

        let discovered_types = discover_typescript_type_entries(
            self.fs.as_ref(),
            &tsconfig_context.options,
            tsconfig_context.directory.as_path(),
            module.path.as_deref(),
        );
        if discovered_types.is_empty() {
            return;
        }

        let mut types = normalize_typescript_type_entries(&compiler_options.types);
        let mut seen = HashSet::new();
        for type_name in &types {
            seen.insert(type_name.clone());
        }

        for discovered_type in discovered_types {
            if seen.insert(discovered_type.clone()) {
                types.push(discovered_type);
            }
        }

        compiler_options.types = types;
    }

    /// Return the fallback target for one module.
    fn implicit_target_for_module(&self, module: &Module) -> Target {
        if module.language_type.is_destack() {
            Target::native("default")
        } else {
            Target::js("default")
        }
    }

    /// Collect type libraries for one target profile.
    fn collect_types_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
    ) -> Vec<String> {
        let mut type_entries = Vec::new();

        if !compiler_options.types.is_empty() {
            type_entries.extend(compiler_options.types.clone());
        }

        if let Some(target_types) = &target.types {
            type_entries.extend(target_types.clone());
        }

        if let Some(profile_types) = profile_config.and_then(|profile| profile.types.as_ref()) {
            type_entries.extend(profile_types.clone());
        }

        builtin_libs_for_type_entries(&type_entries)
    }

    /// Return the effective builtin library set for one target profile.
    fn effective_libs_for_target_profile(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
        tsconfig_options: Option<&TsConfigOptions>,
        runtime: Runtime,
        runtime_version: Option<String>,
        platform: Platform,
    ) -> Vec<String> {
        let derived_target = Target {
            runtime,
            runtime_version,
            platform,
            ..Target::default()
        };

        let base_libs =
            if let Some(profile_lib) = profile_config.and_then(|profile| profile.lib.as_ref()) {
                profile_lib.clone()
            } else if let Some(target_lib) = target.lib.as_ref() {
                target_lib.clone()
            } else if !compiler_options.lib.is_empty() {
                compiler_options.lib.clone()
            } else if let Some(tsconfig_options) = tsconfig_options {
                typescript_default_libs(tsconfig_options)
            } else {
                derived_target.derived_lib()
            };

        let mut libs = base_libs;
        let mut seen = HashSet::new();
        for lib_name in &libs {
            seen.insert(lib_name.clone());
        }

        let type_libs = Self::collect_types_for_target(target, compiler_options, profile_config);
        for type_lib in type_libs {
            if seen.insert(type_lib.clone()) {
                libs.push(type_lib);
            }
        }

        if runtime.is_native() {
            libs.retain(|lib| {
                let Some(builtin) = destack_builtin::builtin_library(lib) else {
                    return true;
                };

                matches!(builtin.kind, destack_builtin::BuiltinLibraryKind::Language)
                    || matches!(builtin.name, "native" | "platform" | "destack")
            });
            if !libs.iter().any(|lib| lib == "native") {
                libs.push("native".to_string());
            }
        }

        libs
    }

    /// Build one profile key for one target.
    fn profile_key_for_target(
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileConfig>,
        tsconfig_options: Option<&TsConfigOptions>,
    ) -> ProfileKey {
        let compiler_options = Self::compiler_options_for_target(target, compiler_options);
        let emit = target.emit;
        let runtime = profile_config
            .and_then(|profile| profile.runtime.as_ref().map(|runtime| runtime.host))
            .unwrap_or(target.runtime);
        let runtime_version = profile_config
            .and_then(|profile| {
                profile
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.version.clone())
            })
            .or_else(|| target.runtime_version.clone());
        let platform = profile_config
            .and_then(|profile| profile.platform)
            .unwrap_or(target.platform);
        let debug = profile_config
            .and_then(|profile| profile.debug)
            .unwrap_or(target.debug);

        let libs = Self::effective_libs_for_target_profile(
            target,
            &compiler_options,
            profile_config,
            tsconfig_options,
            runtime,
            runtime_version,
            platform,
        );

        let env = profile_config
            .and_then(|profile| profile.comptime_env.as_ref())
            .or(compiler_options.comptime_env.as_ref())
            .map(|keys| EnvSnapshot::from_env_whitelist(keys))
            .unwrap_or_else(EnvSnapshot::from_env_all);

        let flags = profile_flags_for_compiler_options(&compiler_options);
        let (_, _, _, test) = ProfileEnv::mode_from_snapshot(&env, debug);

        ProfileKey::new(
            emit,
            runtime,
            platform,
            target.target_arch.clone(),
            target.target_vendor.clone(),
            target.target_env.clone(),
            libs,
            debug,
            test,
            compiler_options.skip_lib_check,
            env,
            flags,
        )
    }
}
