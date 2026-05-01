use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{File, FileId, FileType, PathExt, Uri};
use destack_workspace::{TsConfigDeclaration, TsConfigProjectReferences};

use crate::{
    CachePolicy, Resolution, Resolver, ResolverBase, ResolverContext, ResolverError,
    ResolverOptions, ResolverResult, ResolverSearch, TypeScriptOptionsDiscovery,
    TypeScriptOptionsReferences,
};

/// The query local cache key for one tsconfig declaration shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TsConfigKey {
    /// The concrete tsconfig path.
    pub path: PathBuf,
    /// Whether the config is being loaded as a root.
    pub is_root: bool,
    /// The project reference shaping mode.
    pub references: TypeScriptOptionsReferences,
}

impl TsConfigKey {
    /// Build one tsconfig cache key.
    pub(crate) fn new(
        path: &Path,
        is_root: bool,
        references: &TypeScriptOptionsReferences,
    ) -> Self {
        Self {
            path: path.to_path_buf(),
            is_root,
            references: references.clone(),
        }
    }
}

impl Resolver {
    /// Return one already loaded referenced tsconfig when present.
    pub(crate) fn find_loaded_reference_tsconfig(
        &self,
        reference_path: &Path,
        ctx: &ResolverContext,
    ) -> Option<TsConfigDeclaration> {
        if let Some(tsconfig) =
            ctx.tsconfig_for_path(reference_path, true, &TypeScriptOptionsReferences::Disabled)
        {
            return Some(tsconfig);
        }

        let directory_path = reference_path.join("tsconfig.json");
        if let Some(tsconfig) = ctx.tsconfig_for_path(
            &directory_path,
            true,
            &TypeScriptOptionsReferences::Disabled,
        ) {
            return Some(tsconfig);
        }

        let mut json_path = reference_path.to_path_buf().into_os_string();
        json_path.push(".json");
        let json_path = PathBuf::from(json_path);

        ctx.tsconfig_for_path(&json_path, true, &TypeScriptOptionsReferences::Disabled)
    }

    /// Read and parse a `tsconfig.json` file recursively.
    pub(crate) fn read_tsconfig(
        &self,
        is_root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut ResolverContext,
        cache_policy: CachePolicy,
    ) -> ResolverResult<TsConfigDeclaration> {
        let tsconfig_path = self.materialize_tsconfig_path(path, ctx)?;
        let tsconfig_key = TsConfigKey::new(&tsconfig_path, is_root, references);

        // reuse an existing config shape when allowed
        if cache_policy.use_cache()
            && let Some(tsconfig) = ctx.tsconfig_for_path(&tsconfig_path, is_root, references)
        {
            return Ok(tsconfig);
        }

        // parse the root config entry
        let (repository, revision) = self.repository_revision(ctx);
        let mut tsconfig = if tsconfig_path.starts_with(repository.workspace_root())
            && let Some(tsconfig) = repository
                .tsconfig_declaration_for_file(revision, repository.file_id(&tsconfig_path))
                .map_err(|error| ResolverError::RepositoryError {
                    path: tsconfig_path.to_path_buf(),
                    message: error.to_string(),
                })? {
            let tsconfig = tsconfig.as_ref().clone();
            self.track_file_dependency(&tsconfig.path, ctx)?;
            tsconfig
        } else {
            self.read_tsconfig_into(is_root, &tsconfig_path, ctx)?
        };
        // reject circular extends chains
        if ctx.is_extended_tsconfig(&tsconfig.path) {
            return Err(ResolverError::TsConfigCircular {
                paths: ctx.extended_tsconfig_paths_with(tsconfig.path.to_path_buf()),
            });
        }

        // merge parent configs in order
        let extended_tsconfig_paths = tsconfig
            .json
            .extends()
            .map(|specifier| {
                self.extended_tsconfig_path(&tsconfig.directory, &tsconfig, specifier, ctx)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !extended_tsconfig_paths.is_empty() {
            let current_tsconfig_path = tsconfig.path.to_owned();
            ctx.with_extended_tsconfig(current_tsconfig_path, |ctx| {
                for extended_tsconfig_path in extended_tsconfig_paths {
                    let extended_tsconfig = self.read_tsconfig(
                        false,
                        &extended_tsconfig_path,
                        &TypeScriptOptionsReferences::Disabled,
                        ctx,
                        cache_policy,
                    )?;
                    tsconfig.extend_from(&extended_tsconfig);
                }

                Ok(())
            })?;
        }

        // replace references when requested
        match references {
            TypeScriptOptionsReferences::Disabled => {
                tsconfig.json.references.drain(..);
            }
            TypeScriptOptionsReferences::Automatic => {}
            TypeScriptOptionsReferences::Paths(paths) => {
                tsconfig.json.references = paths
                    .iter()
                    .map(|path| TsConfigProjectReferences { path: path.clone() })
                    .collect();
            }
        }

        // validate referenced projects
        let references_to_load: Vec<_> = tsconfig
            .json
            .references
            .iter()
            .map(|reference| tsconfig.directory.normalize_with(&reference.path))
            .collect();
        let current_path = tsconfig.path.to_path_buf();
        for reference_tsconfig_path in references_to_load {
            let referenced_tsconfig = self.read_tsconfig(
                true,
                &reference_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
                cache_policy,
            )?;

            // reject self references
            if referenced_tsconfig.path == current_path {
                return Err(ResolverError::TsConfigSelfReference {
                    path: referenced_tsconfig.path.to_path_buf(),
                });
            }
        }

        // build the final config after inheritance and reference shaping
        tsconfig.build();
        ctx.cache_tsconfig(tsconfig_key, tsconfig.clone());

        Ok(tsconfig)
    }

    /// Read and parse one tsconfig file into one declaration.
    fn read_tsconfig_into(
        &self,
        is_root: bool,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<TsConfigDeclaration> {
        // read the config file from disk
        let content = match self.read_path_to_string(path, ctx) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(ResolverError::TsConfigNotFound {
                    path: path.to_path_buf(),
                });
            }
            Err(error) => {
                return Err(ResolverError::IoError {
                    path: path.to_path_buf(),
                    kind: error.kind(),
                });
            }
        };

        // keep file ids stable across reloads
        let file_id = FileId::from_logical_path(path);
        let (name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        );
        let file = Arc::new(file);

        // parse the local file as tsconfig
        TsConfigDeclaration::parse(is_root, &file).map_err(|_| ResolverError::TsConfigInvalid {
            path: path.to_path_buf(),
        })
    }
    /// Resolve through tsconfig `paths` substitutions.
    pub(crate) fn resolve_tsconfig_paths(
        &self,
        base: ResolverBase<'_>,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // skip tsconfig path rewrites inside module directories
        if Self::is_inside_modules(path) {
            return Ok(None);
        }

        // choose the root tsconfig from explicit options or discovery
        let tsconfig = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => {
                let root_tsconfig = self.read_tsconfig(
                    true,
                    &tsconfig_options.config_file,
                    &tsconfig_options.references,
                    ctx,
                    CachePolicy::UseCache,
                )?;

                self.find_effective_tsconfig(ctx, &root_tsconfig, path)
            }
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                let Some(tsconfig) = self.find_applicable_tsconfig(base, path, ctx)? else {
                    return Ok(None);
                };

                tsconfig
            }
        };

        // resolve against the selected project and probe each path candidate
        let paths = tsconfig.resolve(path, specifier, |reference_path| {
            self.find_loaded_reference_tsconfig(reference_path, ctx)
        });
        for resolved in paths {
            if let Some(resolution) = self.probe_path(&resolved, ".", search.clone(), ctx)? {
                return Ok(Some(resolution));
            }
        }

        Ok(None)
    }

    /// Resolve the path of one extended tsconfig file.
    fn extended_tsconfig_path(
        &self,
        directory: &Path,
        tsconfig: &TsConfigDeclaration,
        specifier: &str,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<PathBuf> {
        match specifier.as_bytes().first() {
            // empty specifier
            None => Err(ResolverError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: None,
            }),

            // absolute path
            Some(b'/') => Ok(PathBuf::from(specifier)),

            // relative path
            Some(b'.') => Ok(tsconfig.directory.normalize_with(specifier)),

            // package specifier
            _ => {
                // resolve package specifiers with a tsconfig specific resolver shape
                self.with_options(ResolverOptions {
                    cwd: self.options.cwd.clone(),
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    yarn_pnp: self.options.yarn_pnp,
                    ..ResolverOptions::default()
                })
                .resolve_package_or_modules(
                    directory,
                    specifier,
                    ResolverSearch::root(&self.options),
                    ctx,
                )
                .map(Resolution::into_path_buf)
                .map_err(|error| match error {
                    ResolverError::NotFound { .. } => ResolverError::TsConfigNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => error,
                })
            }
        }
    }

    /// Normalize one tsconfig input into a concrete file path.
    pub(crate) fn materialize_tsconfig_path(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<PathBuf> {
        match self.path_metadata(path, ctx)? {
            // keep explicit files as-is
            Some(metadata) if metadata.is_file => Ok(path.to_path_buf()),

            // directory inputs resolve to tsconfig.json
            Some(metadata) if metadata.is_directory => Ok(path.join("tsconfig.json")),

            // plain specifiers default to .json
            Some(_) | None => {
                let mut os_string = path.to_path_buf().into_os_string();
                os_string.push(".json");
                Ok(PathBuf::from(os_string))
            }
        }
    }

    /// Find the effective tsconfig for one path.
    pub(crate) fn find_applicable_tsconfig(
        &self,
        base: ResolverBase<'_>,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<TsConfigDeclaration>> {
        // skip ineligible paths before touching caches
        if Self::is_inside_modules(path) {
            return Ok(None);
        }

        if !path.is_absolute() {
            return Ok(None);
        }

        // prefer cached result if available
        if let Some(tsconfig) = ctx.effective_tsconfig(base, path)
            && let Some(tsconfig) = tsconfig.as_ref().and_then(|key| ctx.tsconfig(key))
        {
            return Ok(Some(tsconfig));
        }

        // prefer repository backed tsconfig discovery within the active workspace
        let (repository, revision) = self.repository_revision(ctx);
        if path.starts_with(repository.workspace_root()) {
            let tsconfig = match repository
                .tsconfig_file_id_for_path(revision, path)
                .map_err(|error| ResolverError::RepositoryError {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })? {
                Some(file_id) => {
                    let declaration = repository
                        .tsconfig_declaration_for_file(revision, file_id)
                        .map_err(|error| ResolverError::RepositoryError {
                            path: path.to_path_buf(),
                            message: error.to_string(),
                        })?
                        .map(|tsconfig| tsconfig.as_ref().clone());

                    declaration
                        .as_ref()
                        .map(|declaration| {
                            self.read_tsconfig(
                                true,
                                &declaration.path,
                                &TypeScriptOptionsReferences::Automatic,
                                ctx,
                                CachePolicy::UseCache,
                            )
                        })
                        .transpose()?
                }
                None => None,
            };

            let key = tsconfig.as_ref().map(|tsconfig| {
                TsConfigKey::new(
                    &tsconfig.path,
                    true,
                    &TypeScriptOptionsReferences::Automatic,
                )
            });

            if let Some(tsconfig) = tsconfig.as_ref()
                && let Some(key) = key.clone()
            {
                ctx.cache_tsconfig(key, tsconfig.clone());
            }

            ctx.cache_effective_tsconfig(base, path, key);
            return Ok(tsconfig);
        }

        // find the nearest config first, then select the effective project
        let Some(nearest_tsconfig) = self.find_nearest_tsconfig(base, path, ctx)? else {
            ctx.cache_effective_tsconfig(base, path, None);
            return Ok(None);
        };

        let tsconfig = self.find_effective_tsconfig(ctx, &nearest_tsconfig, path);
        let key = TsConfigKey::new(
            &tsconfig.path,
            true,
            &TypeScriptOptionsReferences::Automatic,
        );
        ctx.cache_tsconfig(key.clone(), tsconfig.clone());
        ctx.cache_effective_tsconfig(base, path, Some(key));

        Ok(Some(tsconfig))
    }

    /// Find the nearest tsconfig by traversing parent directories.
    fn find_nearest_tsconfig(
        &self,
        base: ResolverBase<'_>,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<TsConfigDeclaration>> {
        let search_start = self.tsconfig_search_start(base, path)?;
        if let Some(tsconfig) = ctx.nearest_tsconfig(&search_start)
            && let Some(tsconfig) = tsconfig.as_ref().and_then(|key| ctx.tsconfig(key))
        {
            return Ok(Some(tsconfig));
        }

        let mut visited_paths = vec![search_start.clone()];
        let mut current = Some(search_start);
        while let Some(directory) = current {
            let tsconfig_path = directory.join("tsconfig.json");
            if self.is_file(&tsconfig_path, ctx)? {
                let tsconfig = self.read_tsconfig(
                    true,
                    &tsconfig_path,
                    &TypeScriptOptionsReferences::Automatic,
                    ctx,
                    CachePolicy::UseCache,
                )?;
                let key = TsConfigKey::new(
                    &tsconfig.path,
                    true,
                    &TypeScriptOptionsReferences::Automatic,
                );

                for visited_path in visited_paths {
                    ctx.cache_nearest_tsconfig(&visited_path, Some(key.clone()));
                }

                return Ok(Some(tsconfig));
            }

            let Some(parent) = directory.parent() else {
                break;
            };

            let parent = parent.to_path_buf();
            visited_paths.push(parent.clone());
            current = Some(parent);
        }

        for visited_path in visited_paths {
            ctx.cache_nearest_tsconfig(&visited_path, None);
        }

        Ok(None)
    }

    /// Find the effective tsconfig for one path from a root config.
    pub(crate) fn find_effective_tsconfig(
        &self,
        ctx: &ResolverContext,
        tsconfig: &TsConfigDeclaration,
        path: &Path,
    ) -> TsConfigDeclaration {
        let mut visited = HashSet::new();
        self.find_effective_tsconfig_recursive(ctx, tsconfig, path, &mut visited)
            .unwrap_or_else(|| tsconfig.clone())
    }

    /// Find the first referenced tsconfig that actually applies to the path.
    fn find_effective_tsconfig_recursive(
        &self,
        ctx: &ResolverContext,
        tsconfig: &TsConfigDeclaration,
        path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Option<TsConfigDeclaration> {
        if !visited.insert(tsconfig.path.clone()) {
            return None;
        }

        if tsconfig.applies_to_path(path) {
            return Some(tsconfig.clone());
        }

        for reference in &tsconfig.json.references {
            let reference_path = tsconfig.directory.normalize_with(&reference.path);
            let referenced_tsconfig = self.find_loaded_reference_tsconfig(&reference_path, ctx);

            let Some(referenced_tsconfig) = referenced_tsconfig else {
                continue;
            };

            if let Some(referenced_tsconfig) =
                self.find_effective_tsconfig_recursive(ctx, &referenced_tsconfig, path, visited)
            {
                return Some(referenced_tsconfig);
            }
        }

        None
    }

    /// Choose the starting directory for tsconfig ancestor lookup.
    fn tsconfig_search_start(
        &self,
        base: ResolverBase<'_>,
        path: &Path,
    ) -> ResolverResult<PathBuf> {
        match base {
            ResolverBase::File(_) => path.parent().map(Path::to_path_buf).ok_or_else(|| {
                ResolverError::ExpectedFilePath {
                    path: path.to_path_buf(),
                }
            }),
            ResolverBase::Directory(_) => Ok(path.to_path_buf()),
        }
    }
}
