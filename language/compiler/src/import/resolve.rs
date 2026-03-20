use std::path::{Path, PathBuf};

use destack_builtin::{builtin_library, resolve_profile_builtin_library_name};
use destack_core::StringId;
use destack_dir::{DependencyKind, ModuleResolution, ModuleTarget};
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{File, FileType, LanguageType, ModuleId, PackageId, PackageVersion, Uri};
use destack_workspace::{
    ImportEdgeKind, Loader, Module, ModuleSource, NodeLinker, Package, PackageKind, ProfileId,
    ProfileKey, TsCompilerOptions,
};

use crate::import::{
    ImportResolveContext, apply_node_linker_resolve_policy, apply_typescript_import_resolve_policy,
    declaration_companion_path_for_module_path, materialize_import_resolve_options,
};
use crate::{Compiler, ImportError, ImportResult};

/// Extensions to try for builtin modules.
const BUILTIN_EXTENSIONS: &[&str] = &[
    ".d.ts", ".d.ds", ".ds", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".node",
];

/// Protocol namespace roots mapped to builtin module roots.
const BUILTIN_NAMESPACE_ROOTS: &[(&str, &str)] = &[
    ("destack", "library/destack"),
    ("platform", "library/platform"),
];

/// Source module resolve policy derived from package and tsconfig ownership.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum SourceImportResolvePolicy {
    /// Package owns Destack config, so tsconfig path mapping is disabled.
    Destack {
        /// The linker mode from source package config.
        node_linker: NodeLinker,
    },
    /// Source module uses tsconfig with compiler settings.
    TsConfig {
        /// Path to the source tsconfig file.
        config_file: PathBuf,
        /// Compiler options from the source tsconfig.
        compiler_options: TsCompilerOptions,
    },
    /// Source module has no config-specific resolver policy.
    None,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Refresh package Destack config state from the filesystem.
    fn refresh_package_destack_config(
        &self,
        package_id: PackageId,
        directory: &Path,
        resolver: &Resolver,
    ) {
        // stop when the package already has config state
        if self
            .program
            .packages
            .get(package_id)
            .read()
            .config
            .is_some()
        {
            return;
        }

        // load and attach the discovered config
        let Some(config_path) = self.find_destack_config_path(directory, resolver) else {
            return;
        };
        let Some(config) = self.session.load_destack_for_path(&config_path) else {
            return;
        };

        let package = self.program.packages.get(package_id);
        package.write().config = Some(config);
    }

    /// Find the nearest `destack.json` for one directory before crossing a package boundary.
    fn find_destack_config_path(&self, directory: &Path, resolver: &Resolver) -> Option<PathBuf> {
        let mut current = directory.to_path_buf();
        loop {
            let candidate = current.join("destack.json");
            if resolver
                .fs()
                .metadata(&candidate)
                .is_ok_and(|meta| meta.is_file)
            {
                return Some(candidate);
            }

            let package_json_path = current.join("package.json");
            if resolver
                .fs()
                .metadata(&package_json_path)
                .is_ok_and(|meta| meta.is_file)
            {
                return None;
            }

            let parent = current.parent()?;
            current = parent.to_path_buf();
        }
    }

    /// Resolve a specifier to a ModuleId, registering a blank module if needed.
    ///
    /// If `loader_override` is provided and the module doesn't exist yet, the module
    /// will be registered with the specified loader instead of the default for its file type.
    /// If the module already exists, the override is ignored.
    pub fn resolve_specifier_to_module(
        &self,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        kind: DependencyKind,
    ) -> ImportResult<ModuleId> {
        self.resolve_specifier_to_module_with_loader(
            profile_id,
            specifier,
            source_module,
            kind,
            ImportEdgeKind::Import,
            None,
        )
    }

    /// Resolve a specifier to a ModuleId with explicit edge semantics.
    ///
    /// If `loader_override` is provided and the module doesn't exist yet, the module
    /// will be registered with the specified loader instead of the default for its file type.
    /// If the module already exists, the override is ignored (Option B from plan).
    pub fn resolve_specifier_to_module_with_loader(
        &self,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        kind: DependencyKind,
        edge_kind: ImportEdgeKind,
        loader_override: Option<Loader>,
    ) -> ImportResult<ModuleId> {
        let specifier_str = self.program.strings.get(specifier).to_string();
        let profile_key = self.program.profile(profile_id).key.clone();
        let source_language_type = self.source_language_type_for_resolution(source_module);

        // resolve protocol specifiers (destack:, platform:)
        if let Some(module_id) = self.resolve_protocol_specifier(&specifier_str, &profile_key) {
            return Ok(module_id);
        }

        // resolve builtin module imports (builtin:// URIs)
        if let Some(source_id) = source_module
            && let Some(module_id) =
                self.resolve_builtin_specifier(&specifier_str, source_id, &profile_key)
        {
            return Ok(module_id);
        }

        // resolve non-builtin imports
        let source_path = self.get_resolve_origin_path(source_module);
        let directory = self.get_resolve_directory(source_module);
        let (path, resolver) = self.resolve_specifier_to_path(
            source_path.as_deref(),
            &directory,
            specifier,
            &specifier_str,
            kind,
            source_module,
            source_language_type,
            edge_kind,
        )?;
        let module_id = self.resolve_specifier_registration(&path, loader_override, &resolver)?;

        Ok(module_id)
    }

    /// Resolve one triple slash `reference lib` target to a builtin module.
    pub(crate) fn resolve_reference_lib_to_module(
        &self,
        profile_id: ProfileId,
        target: &str,
    ) -> ImportResult<ModuleId> {
        // normalize one reference lib target into a builtin lib name
        let Some(lib_name) = Self::reference_lib_name(target) else {
            let target = self.program.strings.intern(target);
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        };

        // load the builtin lib modules for this profile
        let Some(builtins) = self.program.builtins.as_ref() else {
            let target = self.program.strings.intern(target);
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        };
        let profile_key = self.program.profile(profile_id).key.clone();
        let Some(module_ids) = builtins.load_library(
            &lib_name,
            self.program.files.clone(),
            self.program.modules.clone(),
            &profile_key,
        ) else {
            let target = self.program.strings.intern(target);
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        };

        // return the library entry module
        Ok(Self::entry_module_id(&self.program.modules, &module_ids))
    }

    /// Resolve a specifier to a path using dependency-aware rules.
    fn resolve_specifier_to_path(
        &self,
        source_path: Option<&Path>,
        directory: &Path,
        specifier_id: StringId,
        specifier_str: &str,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> ImportResult<(PathBuf, Resolver)> {
        let resolver = self.resolver_for_kind(kind, source_module, source_language_type, edge_kind);
        let resolution = match source_path {
            Some(source_path) => resolver.resolve_from_file(source_path, specifier_str),
            None => resolver.resolve_from_directory(directory, specifier_str),
        };
        if let Ok(resolution) = resolution {
            return Ok((resolution.path, resolver));
        }
        let error = resolution.err();

        Err(ImportError::ModuleNotFound {
            target: specifier_id,
            error,
        })
    }

    /// Resolve a specifier to value and type targets with explicit edge semantics.
    pub(crate) fn resolve_specifier_to_module_resolution(
        &self,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        edge_kind: ImportEdgeKind,
        loader_override: Option<Loader>,
    ) -> ImportResult<ModuleResolution> {
        // detect declaration import sites: they should keep type targets in declaration space
        let source_is_declaration = source_module.is_some_and(|source_module_id| {
            let source_module = self.program.modules.get(source_module_id);
            source_module.language_type.is_declaration()
        });

        // resolve value and type targets through standard resolver options
        let value_target = self
            .resolve_specifier_to_module_with_loader(
                profile_id,
                specifier,
                source_module,
                DependencyKind::Value,
                edge_kind,
                loader_override,
            )
            .ok()
            .map(ModuleTarget::Module);
        let mut type_target = self
            .resolve_specifier_to_module_with_loader(
                profile_id,
                specifier,
                source_module,
                DependencyKind::Type,
                edge_kind,
                loader_override,
            )
            .ok()
            .map(ModuleTarget::Module);

        // for declaration sources: prefer declaration companions for type targets
        if source_is_declaration {
            type_target = self.preferred_declaration_type_target(type_target, value_target);
        }

        if value_target.is_none() && type_target.is_none() {
            return Err(ImportError::ModuleNotFound {
                target: specifier,
                error: None,
            });
        }

        let resolution = ModuleResolution {
            value: value_target,
            ty: type_target,
        };

        Ok(resolution)
    }

    /// Register or reuse a module for a resolved path.
    fn resolve_specifier_registration(
        &self,
        path: &PathBuf,
        loader_override: Option<Loader>,
        resolver: &Resolver,
    ) -> ImportResult<ModuleId> {
        // check if module already exists for this path
        // only use this fast path for default loader resolution
        if loader_override.is_none()
            && let Some(module_id) = self.program.modules.get_id_by_path(path)
        {
            return Ok(module_id);
        }

        // register blank module with optional loader override
        self.register_blank_module(path, None, loader_override, resolver)
    }

    /// Resolve a specifier from a builtin module to a builtin module (see LanguageBuiltins).
    fn resolve_builtin_specifier(
        &self,
        specifier: &str,
        source_module: ModuleId,
        profile_key: &ProfileKey,
    ) -> Option<ModuleId> {
        // get source module URI
        let source = self.program.modules.get(source_module);
        let source_uri = source.uri.clone();
        let source_str: &str = source_uri.as_ref();

        // check if source is a builtin module
        if !source_str.starts_with("builtin://") {
            return None;
        }

        // resolve builtin libs by name for builtin modules
        if !Self::specifier_is_relative(specifier) {
            let builtins = self.program.builtins.as_ref()?;

            // map specifier to builtin lib name
            let lib_name =
                // prefer source lib aliases when available
                if let Some(source_lib_name) = builtins.library_name_for_module(source_module) {
                    let source_lib = builtin_library(source_lib_name)?;

                    // alias mapping for bare specifiers
                    if let Some((_, target)) = source_lib
                        .specifier_aliases
                        .iter()
                        .find(|(alias, _)| *alias == specifier)
                    {
                        (*target).to_string()
                    }
                    // fallback to builtin lib name when no alias matches
                    else if let Some(lib_name) =
                        resolve_profile_builtin_library_name(specifier, &profile_key.lib)
                    {
                        lib_name
                    }
                    // no alias mapping exists
                    else {
                        return None;
                    }
                }
                // direct builtin lib lookup without source context
                else if let Some(lib_name) =
                    resolve_profile_builtin_library_name(specifier, &profile_key.lib)
                {
                    lib_name
                }
                // specifier is not a builtin lib
                else {
                    return None;
                };

            // resolve entry module and ensure lib is loaded
            let module_ids = builtins.load_library(
                &lib_name,
                self.program.files.clone(),
                self.program.modules.clone(),
                profile_key,
            )?;
            let module_id = Self::entry_module_id(&self.program.modules, &module_ids);
            return Some(module_id);
        }

        // resolve relative path against source URI
        //  - source: builtin://intrinsic/prelude.ds
        //  - specifier: ./reflection/type.ds
        //  - target: builtin://intrinsic/reflect/type.ds
        let source_dir = source_str.rsplit_once('/').map(|(dir, _)| dir)?;
        let target_uri_str = Self::resolve_relative_uri(source_dir, specifier);
        let target_uri = Uri::from_string(&target_uri_str);

        // look up target module by exact URI
        if let Some(module_id) = self.program.modules.get_id_by_uri(&target_uri) {
            return Some(module_id);
        }

        // try extensions
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        // try index (with extensions)
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}/index{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        None
    }

    /// Resolve a protocol specifier (destack:, platform:) to a builtin module.
    fn resolve_protocol_specifier(
        &self,
        specifier: &str,
        profile_key: &destack_workspace::ProfileKey,
    ) -> Option<ModuleId> {
        let (protocol, raw_path) = specifier.split_once(':')?;
        let raw_path = raw_path.trim_start_matches('/');
        let (_, root) = BUILTIN_NAMESPACE_ROOTS
            .iter()
            .find(|(namespace, _)| *namespace == protocol)?;

        // normalize empty protocol imports to the namespace entrypoint
        let path = if raw_path.is_empty() {
            "index".to_string()
        } else {
            raw_path.to_string()
        };

        // load the namespace builtin lib first
        let builtins = self.program.builtins.as_ref()?;
        let lib_name = protocol;
        builtins.load_library(
            lib_name,
            self.program.files.clone(),
            self.program.modules.clone(),
            profile_key,
        )?;

        // resolve namespace modules under their builtin root
        let base_path = format!("{root}/{path}");
        let base_uri = Uri::from_string(format!("builtin://{base_path}"));

        // try exact path
        if let Some(module_id) = self.program.modules.get_id_by_uri(&base_uri) {
            return Some(module_id);
        }

        // try extensions
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("builtin://{base_path}{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        // try index (with extensions)
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("builtin://{base_path}/index{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        None
    }

    /// Resolve a path to a ModuleId, registering a blank module if needed.
    pub fn resolve_path_to_module(&self, path: &PathBuf) -> ImportResult<ModuleId> {
        let resolver =
            self.resolver_for_kind(DependencyKind::Value, None, None, ImportEdgeKind::Import);

        // check if module already exists for this path
        if let Some(module_id) = self.program.modules.get_id_by_path(path) {
            return Ok(module_id);
        }

        // register blank module
        self.register_blank_module(path, None, None, &resolver)
    }

    /// Resolve a URI to a ModuleId, registering a blank module if needed.
    pub fn resolve_uri_to_module(&self, uri: &Uri) -> ImportResult<ModuleId> {
        // check if module already exists for this URI
        if let Some(module_id) = self.program.modules.get_id_by_uri(uri) {
            return Ok(module_id);
        }

        let path = uri.to_path().ok_or_else(|| ImportError::ModuleNotFound {
            target: self.program.strings.intern(uri.as_ref()),
            error: None,
        })?;

        self.resolve_path_to_module(&path.to_path_buf())
    }

    /// Register a blank module for a path.
    ///
    /// Creates a blank File, determines the package, computes ModuleId,
    /// and registers a blank Module (without AST).
    ///
    /// If `loader_override` is provided, it will be used instead of the default
    /// loader for the file type. Different loaders produce different ModuleIds
    /// (via hash salting), allowing the same file to be imported multiple ways.
    fn register_blank_module(
        &self,
        path: &PathBuf,
        ty: Option<FileType>,
        loader_override: Option<Loader>,
        resolver: &Resolver,
    ) -> ImportResult<ModuleId> {
        let ty = ty.unwrap_or_else(|| FileType::from_path_or_unknown(path));
        let loader = loader_override.unwrap_or_else(|| Loader::from_file_type(ty));

        // compute loader key (only for non-default loaders)
        let loader_key = loader.key_for_file_type(ty);

        // lock to prevent race conditions (keyed by URI + loader)
        let uri = Uri::from_path(path);
        let import_lock = self.get_import_lock(&uri, loader_key);
        let mut import_guard = import_lock.lock();

        // check if another thread already registered this module
        if let Some(module_id) = *import_guard {
            return Ok(module_id);
        }

        // for default loaders, check URI index (fast path)
        // (for non-default loaders, we skip this since ModuleId is salted)
        if loader_key.is_none()
            && let Some(module_id) = self.program.modules.get_id_by_uri(&uri)
        {
            *import_guard = Some(module_id);
            return Ok(module_id);
        }

        // find or create package (needed to compute ModuleId)
        let (package_id, package_root) = self.resolve_or_create_package_for_path(path, resolver)?;

        // compute ModuleId with loader key
        let module_id =
            ModuleId::from_path_with_loader(package_id, path, package_root.as_deref(), loader_key);

        // check if this exact module already exists
        if self.program.modules.contains(module_id) {
            *import_guard = Some(module_id);
            return Ok(module_id);
        }

        // create blank file entry (content loaded later during import)
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let file_id = self.program.files.next_id();
        let file_version = self.session.workspace_file_version_for_path(path);
        let file = File::unloaded(file_id, name, uri.clone(), Some(path.clone()), ty)
            .with_version(file_version);
        self.program.files.insert(file);

        // find tsconfig (if any)
        let tsconfig_id =
            resolver
                .find_tsconfig_for_file(path)
                .map_err(|error| ImportError::ModuleNotFound {
                    target: self.program.strings.intern(uri.as_ref()),
                    error: Some(error),
                })?;

        // create and register blank module
        let language_type = LanguageType::from(ty);
        let source_type = self.program.detect_module_source_type(
            Some(path.as_path()),
            package_id,
            tsconfig_id,
            false,
        );
        let module_format = self.program.detect_module_format(
            Some(path.as_path()),
            language_type,
            source_type,
            package_id,
            tsconfig_id,
        );
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            Some(path.clone()),
            package_id,
            language_type,
            loader,
            ModuleSource::User,
        );
        let module_version = self
            .session
            .workspace_module_version_for_id(module_id, file_version);
        self.program.modules.insert(
            module,
            module_version,
            file_version,
            tsconfig_id,
            source_type,
            module_format,
        );

        // mark as registered
        *import_guard = Some(module_id);
        drop(import_guard);

        tracing::trace!(
            ?module_id,
            ?path,
            ?loader,
            ?loader_key,
            "import.resolve.register"
        );
        Ok(module_id)
    }

    /// Build resolve options for a dependency kind.
    fn resolver_options_for_kind(
        &self,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> ResolveOptions {
        let mut base_options = self.options.import_resolve.clone();
        let source_policy = self.source_import_resolve_policy(source_module);
        let node_linker = match &source_policy {
            SourceImportResolvePolicy::Destack { node_linker } => *node_linker,
            SourceImportResolvePolicy::TsConfig { .. } | SourceImportResolvePolicy::None => {
                NodeLinker::Auto
            }
        };

        // apply package manager linker behavior before source specific overlays
        apply_node_linker_resolve_policy(
            &mut base_options,
            node_linker,
            self.program.cwd.as_path(),
        );

        // config-owned packages suppress tsconfig overlays
        if matches!(source_policy, SourceImportResolvePolicy::Destack { .. }) {
            base_options.tsconfig = None;
        }

        // tsconfig packages apply source tsconfig resolver policy
        if let SourceImportResolvePolicy::TsConfig {
            config_file,
            compiler_options,
        } = source_policy
        {
            apply_typescript_import_resolve_policy(
                &mut base_options,
                &compiler_options,
                source_language_type,
                config_file,
            );
        }

        let context = ImportResolveContext {
            dependency_kind: kind,
            source_language_type,
            edge_kind,
        };

        materialize_import_resolve_options(&base_options, context)
    }

    /// Read source module resolver ownership from package and tsconfig data.
    fn source_import_resolve_policy(
        &self,
        source_module: Option<ModuleId>,
    ) -> SourceImportResolvePolicy {
        let Some(source_module_id) = source_module else {
            return SourceImportResolvePolicy::None;
        };

        // read source module identity
        let source_module = self.program.modules.get(source_module_id);
        let source_module = source_module.as_ref();
        let package_id = source_module.package_id;
        let tsconfig_id = self.program.modules.tsconfig_id(source_module.id);

        // prefer package config ownership over tsconfig fallbacks
        let package = self.program.packages.get(package_id);
        let package = package.read();
        if let Some(config) = package.config.as_ref() {
            return SourceImportResolvePolicy::Destack {
                node_linker: config.options.compiler.node_linker,
            };
        }
        drop(package);

        // use source tsconfig policy when present
        let Some(tsconfig_id) = tsconfig_id else {
            return SourceImportResolvePolicy::None;
        };

        let tsconfig = self.program.tsconfigs.get(tsconfig_id);
        let tsconfig = tsconfig.read();

        SourceImportResolvePolicy::TsConfig {
            config_file: tsconfig.path.clone(),
            compiler_options: tsconfig.options.compiler.clone(),
        }
    }

    /// Create a resolver configured for a dependency kind.
    fn resolver_for_kind(
        &self,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> Resolver {
        let resolver_options =
            self.resolver_options_for_kind(kind, source_module, source_language_type, edge_kind);

        self.resolver_with_options(resolver_options)
    }

    /// Return the source language used when resolving one import.
    fn source_language_type_for_resolution(
        &self,
        source_module: Option<ModuleId>,
    ) -> Option<LanguageType> {
        source_module.map(|module_id| {
            let module = self.program.modules.get(module_id);
            module.language_type
        })
    }

    /// Resolve a declaration companion target for one value module.
    fn declaration_companion_target_for_module(
        &self,
        value_module_id: ModuleId,
    ) -> Option<ModuleTarget> {
        let value_module = self.program.modules.get(value_module_id);
        let value_module = value_module.as_ref();
        let value_path = value_module.path.as_ref()?;

        let companion_path = declaration_companion_path_for_module_path(value_path)?;
        let companion_exists = self
            .program
            .fs
            .metadata(&companion_path)
            .is_ok_and(|meta| meta.is_file);
        if !companion_exists {
            return None;
        }

        if let Some(companion_module_id) = self.program.modules.get_id_by_path(&companion_path) {
            return Some(ModuleTarget::Module(companion_module_id));
        }

        let companion_module_id = self.resolve_path_to_module(&companion_path).ok()?;

        Some(ModuleTarget::Module(companion_module_id))
    }

    /// Select one declaration target for imports from declaration modules.
    fn preferred_declaration_type_target(
        &self,
        type_target: Option<ModuleTarget>,
        value_target: Option<ModuleTarget>,
    ) -> Option<ModuleTarget> {
        // keep declaration targets as-is
        if let Some(ModuleTarget::Module(type_module_id)) = type_target {
            let type_module = self.program.modules.get(type_module_id);
            if type_module.language_type.is_declaration() {
                return Some(ModuleTarget::Module(type_module_id));
            }

            // upgrade non-declaration type targets through declaration companions
            if let Some(companion_target) =
                self.declaration_companion_target_for_module(type_module_id)
            {
                return Some(companion_target);
            }
        }

        // fall back to declaration companions for value targets
        if let Some(ModuleTarget::Module(value_module_id)) = value_target
            && let Some(companion_target) =
                self.declaration_companion_target_for_module(value_module_id)
        {
            return Some(companion_target);
        }

        // keep resolver result when no declaration companion exists
        type_target
    }

    /// Get the directory to resolve from for a source module.
    pub(super) fn get_resolve_directory(&self, source_module: Option<ModuleId>) -> PathBuf {
        if let Some(module_id) = source_module {
            let module = self.program.modules.get(module_id);
            let module_file = self.program.files.get(module.file_id);
            module_file
                .uri
                .to_path_buf()
                .and_then(|path| path.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| self.program.cwd.clone())
        } else {
            self.program.cwd.clone()
        }
    }

    /// Get the origin file path to resolve from for a source module.
    pub(super) fn get_resolve_origin_path(
        &self,
        source_module: Option<ModuleId>,
    ) -> Option<PathBuf> {
        let module_id = source_module?;
        let module = self.program.modules.get(module_id);
        let module_file = self.program.files.get(module.file_id);
        module_file
            .path
            .clone()
            .or_else(|| module_file.uri.to_path_buf())
    }

    /// Resolve or create a package for a file path.
    pub(super) fn resolve_or_create_package_for_path(
        &self,
        path: &Path,
        resolver: &Resolver,
    ) -> ImportResult<(PackageId, Option<PathBuf>)> {
        // try to find a physical package (package.json)
        if let Some(package_id) =
            resolver
                .find_package(path)
                .map_err(|error| ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path.to_string_lossy().as_ref()),
                    error: Some(error),
                })?
        {
            let package = self.program.packages.get(package_id);
            let package_root = package.read().path.clone();
            if let Some(package_root) = package_root.as_deref() {
                self.refresh_package_destack_config(package_id, package_root, resolver);
            }
            return Ok((package_id, package_root));
        }

        // create synthetic package for file's directory
        let directory = path.parent().unwrap_or(path);
        let package_id = PackageId::from_synthetic_path(directory);

        // check if synthetic package already exists
        if self.program.packages.contains(package_id) {
            self.refresh_package_destack_config(package_id, directory, resolver);
            return Ok((package_id, Some(directory.to_path_buf())));
        }

        // load destack.json for synthetic packages when present
        let config = self
            .find_destack_config_path(directory, resolver)
            .and_then(|path| self.session.load_destack_for_path(&path));
        // create and insert synthetic package
        let package_name = self.synthetic_package_name(directory);
        let package = Package {
            id: package_id,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Synthetic,
            uri: Uri::from_path(directory),
            path: Some(directory.to_path_buf()),
            name: Some(package_name),
            version: None,
            manifest: None,
            config,
            tsconfig: None,
            targets: Default::default(),
        };
        self.program.packages.insert(package);
        self.refresh_package_destack_config(package_id, directory, resolver);

        Ok((package_id, Some(directory.to_path_buf())))
    }

    /// Build a synthetic package name from a directory.
    fn synthetic_package_name(&self, directory: &Path) -> String {
        let name = directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("synthetic");
        format!("<{name}>")
    }

    /// Normalize one triple slash `reference lib` target to a builtin lib name.
    fn reference_lib_name(target: &str) -> Option<String> {
        let target = target.trim();
        if target.is_empty() {
            return None;
        }

        // keep direct builtin lib names first
        if let Some(lib) = builtin_library(target) {
            return Some(lib.name.to_string());
        }

        // normalize common `lib.*.d.ts` spellings used in directives
        let mut normalized = target.to_ascii_lowercase();
        if let Some(stripped) = normalized.strip_prefix("lib.") {
            normalized = stripped.to_string();
        }
        if let Some(stripped) = normalized.strip_suffix(".d.ts") {
            normalized = stripped.to_string();
        }

        builtin_library(&normalized).map(|lib| lib.name.to_string())
    }

    /// Return true when a specifier is a relative path import.
    fn specifier_is_relative(specifier: &str) -> bool {
        specifier == "."
            || specifier == ".."
            || specifier.starts_with("./")
            || specifier.starts_with("../")
    }

    /// Return the entry module id for a loaded builtin lib module set.
    fn entry_module_id(
        modules: &destack_workspace::ModuleRegistry,
        module_ids: &[ModuleId],
    ) -> ModuleId {
        let mut fallback = None;
        for module_id in module_ids {
            let module = modules.get(*module_id);
            let module = module.as_ref();
            let uri = module.uri.as_ref();
            if uri.ends_with("/index.d.ts")
                || uri.ends_with("/index.d.ds")
                || uri.ends_with("/index.ds")
            {
                return *module_id;
            }
            fallback.get_or_insert(*module_id);
        }

        fallback.unwrap_or_else(|| {
            panic!("builtin lib returned no modules for entry resolution");
        })
    }

    /// Resolve a relative specifier against a base URI directory.
    /// e.g., resolve_relative_uri("builtin://intrinsic", "./reflect/type.ds")
    ///       -> "builtin://intrinsic/reflect/type.ds"
    fn resolve_relative_uri(base_dir: &str, specifier: &str) -> String {
        let mut parts: Vec<&str> = base_dir.split('/').collect();

        for segment in specifier.split('/') {
            match segment {
                "." | "" => continue,
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }

        parts.join("/")
    }
}
