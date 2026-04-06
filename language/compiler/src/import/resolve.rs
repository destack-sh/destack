use std::path::{Path, PathBuf};

use destack_artifact::{ImportEdgeKind, Loader, ProfileKey};
use destack_builtin::{builtin_library, resolve_profile_builtin_library_name};
use destack_core::StringId;
use destack_dir::{DependencyKind, ModuleResolution, ModuleTarget};
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{FileType, LanguageType, ModuleId, PackageId, Uri};
use destack_workspace::{BUILTIN_PACKAGE_ID, Edit, NodeLinker, ProfileId, TsCompilerOptions};

use crate::import::{
    ImportResolveContext, apply_node_linker_resolve_policy, apply_typescript_import_resolve_policy,
    declaration_companion_path_for_module_path, materialize_import_resolve_options,
};
use crate::{
    Compiler, DiagnosticAnchor, FileRequirement, ImportError, ImportResult, RequirementSet,
};

/// Extensions to try for builtin modules.
const BUILTIN_EXTENSIONS: &[&str] = &[
    ".d.ts", ".d.ds", ".ds", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".node",
];

/// Protocol namespace roots mapped to builtin module roots.
const BUILTIN_NAMESPACE_ROOTS: &[(&str, &str)] = &[
    ("destack", "library/destack"),
    ("platform", "library/platform"),
];

/// Builtin package roots that are valid as builtin-absolute import paths.
const BUILTIN_ABSOLUTE_ROOTS: &[&str] = &["intrinsic/", "language/", "library/"];

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
    /// Convert one repository failure into an internal import error.
    fn import_repository_error(error: destack_workspace::RepositoryError) -> ImportError {
        ImportError::Internal {
            message: error.to_string(),
        }
    }

    /// Return the current revision module id for one workspace path when present.
    fn current_module_id_for_path(
        &self,
        revision: destack_workspace::Revision,
        path: &Path,
    ) -> ImportResult<Option<ModuleId>> {
        self.repository
            .module_id_for_path(revision, path)
            .map_err(Self::import_repository_error)
    }

    /// Return the current revision module id for one uri when present.
    fn current_module_id_for_uri(
        &self,
        revision: destack_workspace::Revision,
        uri: &Uri,
    ) -> ImportResult<Option<ModuleId>> {
        self.repository
            .module_id_for_uri(revision, uri)
            .map_err(Self::import_repository_error)
    }

    /// Resolve a specifier to a ModuleId, materializing source into the current revision if needed.
    ///
    /// If `loader_override` is provided and the module doesn't exist yet, the materialized
    /// module will use that loader instead of the default for its file type.
    /// If the module already exists, the override is ignored.
    pub fn resolve_specifier_to_module(
        &self,
        revision: destack_workspace::Revision,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        kind: DependencyKind,
    ) -> ImportResult<ModuleId> {
        self.resolve_specifier_to_module_with_loader(
            revision,
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
    /// If `loader_override` is provided and the module doesn't exist yet, the materialized
    /// module will use that loader instead of the default for its file type.
    /// If the module already exists, the override is ignored (Option B from plan).
    pub fn resolve_specifier_to_module_with_loader(
        &self,
        revision: destack_workspace::Revision,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        kind: DependencyKind,
        edge_kind: ImportEdgeKind,
        loader_override: Option<Loader>,
    ) -> ImportResult<ModuleId> {
        let specifier_str = self.repository.strings.get(specifier).to_string();
        let profile_key = self.profile(profile_id).key.clone();
        let source_language_type =
            self.source_language_type_for_resolution(revision, source_module);

        // resolve protocol specifiers (destack:, platform:)
        if let Some(module_id) =
            self.resolve_protocol_specifier(revision, &specifier_str, &profile_key)
        {
            return Ok(module_id);
        }

        // resolve builtin module imports (builtin:// URIs)
        if let Some(source_id) = source_module
            && let Some(module_id) =
                self.resolve_builtin_specifier(revision, &specifier_str, source_id, &profile_key)
        {
            return Ok(module_id);
        }

        // resolve non-builtin imports
        let source_path = self.get_resolve_origin_path(revision, source_module);
        let directory = self.get_resolve_directory(revision, source_module);
        let (path, resolver) = self.resolve_specifier_to_path(
            revision,
            source_path.as_deref(),
            &directory,
            specifier,
            &specifier_str,
            kind,
            source_module,
            source_language_type,
            edge_kind,
        )?;
        let module_id =
            self.resolve_specifier_materialization(revision, &path, loader_override, &resolver)?;

        Ok(module_id)
    }

    /// Resolve one triple slash `reference lib` target to a builtin module.
    pub(crate) fn resolve_reference_lib_to_module(
        &self,
        _revision: destack_workspace::Revision,
        profile_id: ProfileId,
        target: &str,
    ) -> ImportResult<ModuleId> {
        // normalize one reference lib target into a builtin lib name
        let Some(lib_name) = Self::reference_lib_name(target) else {
            let target = self.repository.strings.intern(target);
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        };

        // load the builtin lib modules for this profile
        let profile_key = self.profile(profile_id).key.clone();
        let Some(module_ids) = self
            .repository
            .load_builtin_library(&lib_name, &profile_key)
        else {
            let target = self.repository.strings.intern(target);
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        };

        // return the library entry module
        Ok(Self::entry_module_id(self.repository.as_ref(), &module_ids))
    }

    /// Resolve a specifier to a path using dependency-aware rules.
    fn resolve_specifier_to_path(
        &self,
        revision: destack_workspace::Revision,
        source_path: Option<&Path>,
        directory: &Path,
        specifier_id: StringId,
        specifier_str: &str,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> ImportResult<(PathBuf, Resolver)> {
        let resolver = self.resolver_for_kind(
            revision,
            kind,
            source_module,
            source_language_type,
            edge_kind,
        );
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
        revision: destack_workspace::Revision,
        profile_id: ProfileId,
        specifier: StringId,
        source_module: Option<ModuleId>,
        edge_kind: ImportEdgeKind,
        loader_override: Option<Loader>,
    ) -> ImportResult<ModuleResolution> {
        // detect declaration import sites: they should keep type targets in declaration space
        let source_is_declaration = source_module.is_some_and(|source_module_id| {
            self.repository
                .module(revision, source_module_id)
                .ok()
                .flatten()
                .is_some_and(|module| module.language_type.is_declaration())
        });

        // resolve value and type targets through standard resolver options
        let value_target = self
            .resolve_specifier_to_module_with_loader(
                revision,
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
                revision,
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
            type_target =
                self.preferred_declaration_type_target(revision, type_target, value_target);
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

    /// Materialize or reuse a module for a resolved path.
    fn resolve_specifier_materialization(
        &self,
        revision: destack_workspace::Revision,
        path: &PathBuf,
        loader_override: Option<Loader>,
        resolver: &Resolver,
    ) -> ImportResult<ModuleId> {
        // check if module already exists for this path
        // only use this fast path for default loader resolution
        if loader_override.is_none()
            && let Some(module_id) = self.current_module_id_for_path(revision, path)?
        {
            return Ok(module_id);
        }

        // materialize the module with the requested loader semantics
        self.materialize_module_for_path(revision, path, None, loader_override, resolver)
    }

    /// Resolve a specifier from a builtin module to a builtin module (see LanguageBuiltins).
    fn resolve_builtin_specifier(
        &self,
        revision: destack_workspace::Revision,
        specifier: &str,
        source_module: ModuleId,
        profile_key: &ProfileKey,
    ) -> Option<ModuleId> {
        // get source module URI
        let source = self
            .repository
            .module(revision, source_module)
            .ok()
            .flatten()?;
        let source_uri = source.uri.clone();
        let source_str: &str = source_uri.as_ref();

        // check if source is a builtin module
        if !source_str.starts_with("builtin://") {
            return None;
        }

        // resolve builtin-absolute package paths directly
        if let Some(module_id) = self.resolve_builtin_absolute_specifier(specifier) {
            return Some(module_id);
        }

        // resolve builtin libs by name for builtin modules
        if !Self::specifier_is_relative(specifier) {
            // map specifier to builtin lib name
            let lib_name =
                // prefer source lib aliases when available
                if let Some(source_lib_name) =
                    self.repository.builtins().library_name_for_module(source_module)
                {
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
            let module_ids = self
                .repository
                .load_builtin_library(&lib_name, profile_key)?;
            let module_id = Self::entry_module_id(self.repository.as_ref(), &module_ids);
            return Some(module_id);
        }

        // resolve relative path against source URI
        //  - source: builtin://primitive/index.ds
        //  - specifier: ./vector.ds
        //  - target: builtin://primitive/vector.ds
        let source_dir = source_str.rsplit_once('/').map(|(dir, _)| dir)?;
        let target_uri_str = Self::resolve_relative_uri(source_dir, specifier);
        let target_uri = Uri::from_string(&target_uri_str);

        // look up target module by exact URI
        if let Ok(Some(module_id)) = self.current_module_id_for_uri(revision, &target_uri) {
            return Some(module_id);
        }

        // try extensions
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}{extension}"));
            if let Ok(Some(module_id)) = self.current_module_id_for_uri(revision, &candidate_uri) {
                return Some(module_id);
            }
        }

        // try index (with extensions)
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}/index{extension}"));
            if let Ok(Some(module_id)) = self.current_module_id_for_uri(revision, &candidate_uri) {
                return Some(module_id);
            }
        }

        None
    }

    /// Resolve one builtin-absolute import path.
    fn resolve_builtin_absolute_specifier(&self, specifier: &str) -> Option<ModuleId> {
        let is_builtin_absolute = BUILTIN_ABSOLUTE_ROOTS
            .iter()
            .any(|root| specifier.starts_with(root));
        if !is_builtin_absolute {
            return None;
        }

        self.resolve_builtin_module_path(specifier)
    }

    /// Resolve one builtin module path inside the builtin package.
    fn resolve_builtin_module_path(&self, module_path: &str) -> Option<ModuleId> {
        let builtins = self.repository.builtins();
        let module_path = module_path
            .strip_prefix("intrinsic/")
            .unwrap_or(module_path);

        // try exact path
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(module_path));
        if builtins.module_source(module_id).is_some() {
            return Some(module_id);
        }

        // try extensions
        for extension in BUILTIN_EXTENSIONS {
            let candidate_path = format!("{module_path}{extension}");
            let module_id =
                ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&candidate_path));
            if builtins.module_source(module_id).is_some() {
                return Some(module_id);
            }
        }

        // try index (with extensions)
        for extension in BUILTIN_EXTENSIONS {
            let candidate_path = format!("{module_path}/index{extension}");
            let module_id =
                ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&candidate_path));
            if builtins.module_source(module_id).is_some() {
                return Some(module_id);
            }
        }

        None
    }

    /// Resolve a protocol specifier (destack:, platform:) to a builtin module.
    fn resolve_protocol_specifier(
        &self,
        _revision: destack_workspace::Revision,
        specifier: &str,
        profile_key: &ProfileKey,
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
        let lib_name = protocol;
        self.repository
            .load_builtin_library(lib_name, profile_key)?;

        // resolve namespace modules under their builtin root
        let base_path = format!("{root}/{path}");

        self.resolve_builtin_module_path(&base_path)
    }

    /// Resolve a path to a module id inside the active execution scope.
    fn resolve_path_to_module_in_revision(
        &self,
        revision: destack_workspace::Revision,
        path: &PathBuf,
    ) -> ImportResult<ModuleId> {
        let resolver = self.resolver_for_kind(
            revision,
            DependencyKind::Value,
            None,
            None,
            ImportEdgeKind::Import,
        );

        // check if module already exists for this path
        if let Some(module_id) = self.current_module_id_for_path(revision, path)? {
            return Ok(module_id);
        }

        // materialize the module into the active revision
        self.materialize_module_for_path(revision, path, None, None, &resolver)
    }

    /// Resolve a path to a module id at one explicit revision.
    pub fn resolve_path_to_module(
        &self,
        revision: destack_workspace::Revision,
        path: &PathBuf,
    ) -> ImportResult<ModuleId> {
        self.resolve_path_to_module_in_revision(revision, path)
    }

    /// Resolve a URI to a module id inside the active execution scope.
    fn resolve_uri_to_module_in_revision(
        &self,
        revision: destack_workspace::Revision,
        uri: &Uri,
    ) -> ImportResult<ModuleId> {
        // check if module already exists for this URI
        if let Some(module_id) = self.current_module_id_for_uri(revision, uri)? {
            return Ok(module_id);
        }

        let path = uri.to_path().ok_or_else(|| ImportError::ModuleNotFound {
            target: self.repository.strings.intern(uri.as_ref()),
            error: None,
        })?;

        self.resolve_path_to_module_in_revision(revision, &path.to_path_buf())
    }

    /// Resolve a URI to a module id at one explicit revision.
    pub fn resolve_uri_to_module(
        &self,
        revision: destack_workspace::Revision,
        uri: &Uri,
    ) -> ImportResult<ModuleId> {
        self.resolve_uri_to_module_in_revision(revision, uri)
    }

    /// Materialize one module for a path in the active revision.
    ///
    /// If `loader_override` is provided, it will be used instead of the default
    /// loader for the file type. Different loaders produce different ModuleIds
    /// (via hash salting), allowing the same file to be imported multiple ways.
    fn materialize_module_for_path(
        &self,
        revision: destack_workspace::Revision,
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

        // check if another thread already materialized this module
        if let Some(module_id) = *import_guard {
            return Ok(module_id);
        }

        // for default loaders, check URI index (fast path)
        // (for non-default loaders, we skip this since ModuleId is salted)
        if loader_key.is_none()
            && let Some(module_id) = self.current_module_id_for_uri(revision, &uri)?
        {
            *import_guard = Some(module_id);
            return Ok(module_id);
        }

        // find or create package (needed to compute ModuleId)
        let (package_id, package_root) =
            self.resolve_or_create_package_for_path(revision, path, resolver)?;

        // compute ModuleId with loader key
        let module_id =
            ModuleId::from_path_with_loader(package_id, path, package_root.as_deref(), loader_key);

        // check if this exact module already exists
        if self
            .repository
            .module(revision, module_id)
            .map_err(Self::import_repository_error)?
            .is_some()
        {
            *import_guard = Some(module_id);
            return Ok(module_id);
        }

        let file_id = self.repository.file_id_for_workspace_path(path);

        // request source expansion when this file is not in the current snapshot
        if self
            .repository
            .file(revision, file_id)
            .map_err(Self::import_repository_error)?
            .is_none()
        {
            let logical_path = self.repository.normalize_workspace_path(path);
            let content = self
                .repository
                .load_workspace_file_content(path)
                .map_err(Self::import_repository_error)?;
            let requirement = FileRequirement::new(
                DiagnosticAnchor::from(module_id),
                Edit::SetFile {
                    logical_path,
                    content,
                },
            );

            return Err(ImportError::Yield {
                requirement: RequirementSet::one(requirement),
            });
        }

        // find tsconfig (if any)
        let _tsconfig_file_id = resolver
            .find_tsconfig_for_file(path)
            .map_err(|error| ImportError::ModuleNotFound {
                target: self.repository.strings.intern(uri.as_ref()),
                error: Some(error),
            })?
            .map(|tsconfig| self.repository.file_id_for_workspace_path(&tsconfig.path));

        // publish the resolved module id under the import lock
        *import_guard = Some(module_id);
        drop(import_guard);

        tracing::trace!(
            ?module_id,
            ?path,
            ?loader,
            ?loader_key,
            "import.resolve.materialize"
        );
        Ok(module_id)
    }

    /// Build resolve options for a dependency kind.
    fn resolver_options_for_kind(
        &self,
        revision: destack_workspace::Revision,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> ResolveOptions {
        let mut base_options = self.options.import_resolve.clone();
        let source_policy = self.source_import_resolve_policy(revision, source_module);
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
            self.repository.cwd.as_path(),
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
        revision: destack_workspace::Revision,
        source_module: Option<ModuleId>,
    ) -> SourceImportResolvePolicy {
        let Some(source_module_id) = source_module else {
            return SourceImportResolvePolicy::None;
        };

        // read source module identity
        let Some(source_module) = self
            .repository
            .module(revision, source_module_id)
            .ok()
            .flatten()
        else {
            return SourceImportResolvePolicy::None;
        };
        let source_module = source_module.as_ref();
        let package_id = source_module.package_id;
        let tsconfig_file_id = source_module.tsconfig_file_id;

        // prefer package config ownership over tsconfig fallbacks
        let Some(package) = self.repository.package(revision, package_id).ok().flatten() else {
            return SourceImportResolvePolicy::None;
        };
        let package_options = self
            .repository
            .package_options(revision, package.id)
            .ok()
            .flatten();
        if let Some(config) = package_options {
            return SourceImportResolvePolicy::Destack {
                node_linker: config.compiler.node_linker,
            };
        }

        // use source tsconfig policy when present
        let Some(tsconfig_file_id) = tsconfig_file_id else {
            return SourceImportResolvePolicy::None;
        };

        let Some(tsconfig) = self
            .repository
            .tsconfig_declaration(revision, tsconfig_file_id)
            .ok()
            .flatten()
        else {
            return SourceImportResolvePolicy::None;
        };

        SourceImportResolvePolicy::TsConfig {
            config_file: tsconfig.path.clone(),
            compiler_options: tsconfig.options().compiler.clone(),
        }
    }

    /// Create a resolver configured for a dependency kind.
    fn resolver_for_kind(
        &self,
        revision: destack_workspace::Revision,
        kind: DependencyKind,
        source_module: Option<ModuleId>,
        source_language_type: Option<LanguageType>,
        edge_kind: ImportEdgeKind,
    ) -> Resolver {
        let resolver_options = self.resolver_options_for_kind(
            revision,
            kind,
            source_module,
            source_language_type,
            edge_kind,
        );

        self.resolver_with_options(resolver_options)
    }

    /// Return the source language used when resolving one import.
    fn source_language_type_for_resolution(
        &self,
        revision: destack_workspace::Revision,
        source_module: Option<ModuleId>,
    ) -> Option<LanguageType> {
        let module_id = source_module?;
        let module = self.repository.module(revision, module_id).ok().flatten()?;

        Some(module.language_type)
    }

    /// Resolve a declaration companion target for one value module.
    fn declaration_companion_target_for_module(
        &self,
        revision: destack_workspace::Revision,
        value_module_id: ModuleId,
    ) -> Option<ModuleTarget> {
        let value_module = self
            .repository
            .module(revision, value_module_id)
            .ok()
            .flatten()?;
        let value_module = value_module.as_ref();
        let value_path = value_module.path.as_ref()?;

        let companion_path = declaration_companion_path_for_module_path(value_path)?;
        let companion_exists = self
            .repository
            .file_system()
            .metadata(&companion_path)
            .is_ok_and(|meta| meta.is_file);
        if !companion_exists {
            return None;
        }

        if let Ok(Some(companion_module_id)) =
            self.current_module_id_for_path(revision, &companion_path)
        {
            return Some(ModuleTarget::Module(companion_module_id));
        }

        let companion_module_id = self
            .resolve_path_to_module_in_revision(revision, &companion_path)
            .ok()?;

        Some(ModuleTarget::Module(companion_module_id))
    }

    /// Select one declaration target for imports from declaration modules.
    fn preferred_declaration_type_target(
        &self,
        revision: destack_workspace::Revision,
        type_target: Option<ModuleTarget>,
        value_target: Option<ModuleTarget>,
    ) -> Option<ModuleTarget> {
        // keep declaration targets as-is
        if let Some(ModuleTarget::Module(type_module_id)) = type_target {
            let type_module = self
                .repository
                .module(revision, type_module_id)
                .ok()
                .flatten()?;
            if type_module.language_type.is_declaration() {
                return Some(ModuleTarget::Module(type_module_id));
            }

            // upgrade non-declaration type targets through declaration companions
            if let Some(companion_target) =
                self.declaration_companion_target_for_module(revision, type_module_id)
            {
                return Some(companion_target);
            }
        }

        // fall back to declaration companions for value targets
        if let Some(ModuleTarget::Module(value_module_id)) = value_target
            && let Some(companion_target) =
                self.declaration_companion_target_for_module(revision, value_module_id)
        {
            return Some(companion_target);
        }

        // keep resolver result when no declaration companion exists
        type_target
    }

    /// Get the directory to resolve from for a source module.
    pub(super) fn get_resolve_directory(
        &self,
        revision: destack_workspace::Revision,
        source_module: Option<ModuleId>,
    ) -> PathBuf {
        if let Some(module_id) = source_module {
            let Some(module) = self.repository.module(revision, module_id).ok().flatten() else {
                return self.repository.cwd.clone();
            };
            let Some(module_file) = self
                .repository
                .file(revision, module.file_id)
                .ok()
                .flatten()
            else {
                return self.repository.cwd.clone();
            };
            module_file
                .uri
                .to_path_buf()
                .and_then(|path| path.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| self.repository.cwd.clone())
        } else {
            self.repository.cwd.clone()
        }
    }

    /// Get the origin file path to resolve from for a source module.
    pub(super) fn get_resolve_origin_path(
        &self,
        revision: destack_workspace::Revision,
        source_module: Option<ModuleId>,
    ) -> Option<PathBuf> {
        let module_id = source_module?;
        let module = self.repository.module(revision, module_id).ok().flatten()?;
        let module_file = self
            .repository
            .file(revision, module.file_id)
            .ok()
            .flatten()?;
        module_file
            .path
            .clone()
            .or_else(|| module_file.uri.to_path_buf())
    }

    /// Resolve or create a package for a file path.
    pub(super) fn resolve_or_create_package_for_path(
        &self,
        revision: destack_workspace::Revision,
        path: &Path,
        resolver: &Resolver,
    ) -> ImportResult<(PackageId, Option<PathBuf>)> {
        // try to find a physical package (package.json)
        if let Some(package_id) =
            resolver
                .find_package(path)
                .map_err(|error| ImportError::ModuleNotFound {
                    target: self
                        .repository
                        .strings
                        .intern(path.to_string_lossy().as_ref()),
                    error: Some(error),
                })?
        {
            let package_root = self
                .repository
                .package(revision, package_id)
                .ok()
                .flatten()
                .and_then(|package| package.path.clone())
                .or_else(|| {
                    resolver
                        .package_maybe(package_id)
                        .and_then(|package| package.path)
                });
            return Ok((package_id, package_root));
        }

        // create synthetic package for file's directory
        let directory = path.parent().unwrap_or(path);
        let package_id = PackageId::from_synthetic_path(directory);
        let package_path = if directory.is_absolute() {
            directory.to_path_buf()
        } else {
            self.repository.workspace_root().join(directory)
        };

        Ok((package_id, Some(package_path)))
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
        repository: &destack_workspace::Repository,
        module_ids: &[ModuleId],
    ) -> ModuleId {
        let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
        let revision = repository
            .current(&reference)
            .expect("workspace revision should be tracked");
        let mut fallback = None;
        for module_id in module_ids {
            let module = repository
                .module(revision, *module_id)
                .ok()
                .flatten()
                .expect("builtin module snapshot should exist");
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
