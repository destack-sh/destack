use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{File, FileId, FileType, PathExt, Uri};
use destack_workspace::{TsConfigDeclaration, TsConfigProjectReferences};

use crate::{
    CachePolicy, Resolution, ResolveError, ResolveFrame, ResolveOptions, ResolveOrigin, Resolver,
    TypeScriptOptionsDiscovery, TypeScriptOptionsReferences,
};

/// The resolve frame for one tsconfig extension chain.
#[derive(Default)]
pub(crate) struct TypeScriptOptionsResolveFrame {
    extended_configs: Vec<PathBuf>,
}

impl TypeScriptOptionsResolveFrame {
    /// Execute a closure with one extended file pushed on the stack.
    pub(crate) fn with_extended_file<F, T>(
        &mut self,
        path: PathBuf,
        f: F,
    ) -> Result<T, ResolveError>
    where
        F: FnOnce(&mut Self) -> Result<T, ResolveError>,
    {
        self.extended_configs.push(path);
        let result = f(self);
        self.extended_configs.pop();
        result
    }

    /// Return true when this chain already visited the given config.
    fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs
            .iter()
            .any(|extended| extended == path)
    }

    /// Return the current chain plus the given config path.
    fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = self.extended_configs.clone();
        configs.push(path);
        configs
    }
}

impl Resolver {
    /// Resolve a `tsconfig.json` file at the given path.
    pub fn resolve_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<TsConfigDeclaration, ResolveError> {
        let tsconfig_file_id = self.read_tsconfig(
            true,
            path.as_ref(),
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveFrame::default(),
            CachePolicy::UseCache,
        )?;

        self.get_tsconfig(tsconfig_file_id)
    }

    /// Reload a `tsconfig.json` file at the given path.
    pub fn reload_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<TsConfigDeclaration, ResolveError> {
        let tsconfig_file_id = self.read_tsconfig(
            true,
            path.as_ref(),
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveFrame::default(),
            CachePolicy::Reload,
        )?;

        self.get_tsconfig(tsconfig_file_id)
    }

    /// Return the cached tsconfig for one file id.
    pub(crate) fn get_tsconfig(
        &self,
        tsconfig_file_id: FileId,
    ) -> Result<TsConfigDeclaration, ResolveError> {
        self.cached_tsconfig(tsconfig_file_id)
            .ok_or_else(|| ResolveError::TsConfigNotFound {
                path: PathBuf::from(format!("<missing-tsconfig:{tsconfig_file_id:?}>")),
            })
    }

    /// Read and parse a `tsconfig.json` file recursively.
    pub(crate) fn read_tsconfig(
        &self,
        is_root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut TypeScriptOptionsResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<FileId, ResolveError> {
        let tsconfig_path = self.materialize_tsconfig_path(path);

        // reuse an existing parsed config when allowed
        let existing_file_id = self.cached_tsconfig_file_id_by_path(&tsconfig_path);
        if cache_policy.use_cache()
            && let Some(tsconfig_file_id) = existing_file_id
        {
            return Ok(tsconfig_file_id);
        }

        // parse the root config entry
        let mut tsconfig = self.read_tsconfig_into(is_root, &tsconfig_path)?;
        let tsconfig_file_id = tsconfig.file_id;

        // reject circular extends chains
        if ctx.is_already_extended(&tsconfig.path) {
            return Err(ResolveError::TsConfigCircular {
                paths: ctx.get_extended_configs_with(tsconfig.path.to_path_buf()),
            });
        }

        // merge parent configs in order
        let extended_tsconfig_paths = tsconfig
            .json
            .extends()
            .map(|specifier| {
                self.get_extended_tsconfig_path(&tsconfig.directory, &tsconfig, specifier)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !extended_tsconfig_paths.is_empty() {
            let current_tsconfig_path = tsconfig.path.to_owned();
            ctx.with_extended_file(current_tsconfig_path, |ctx| {
                for extended_tsconfig_path in extended_tsconfig_paths {
                    let extended_tsconfig_file_id = self.read_tsconfig(
                        false,
                        &extended_tsconfig_path,
                        &TypeScriptOptionsReferences::Disabled,
                        ctx,
                        cache_policy,
                    )?;
                    let extended_tsconfig = self.get_tsconfig(extended_tsconfig_file_id)?;
                    tsconfig.extend_from(&extended_tsconfig);
                }

                Result::Ok::<(), ResolveError>(())
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
            let reference_tsconfig_file_id = self.read_tsconfig(
                true,
                &reference_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
                cache_policy,
            )?;
            let referenced_tsconfig = self.get_tsconfig(reference_tsconfig_file_id)?;

            // reject self references
            if referenced_tsconfig.path == current_path {
                return Err(ResolveError::TsConfigSelfReference {
                    path: referenced_tsconfig.path.to_path_buf(),
                });
            }
        }

        // build the final config after inheritance and reference shaping
        tsconfig.build();
        self.cache_tsconfig(tsconfig);

        Ok(tsconfig_file_id)
    }

    /// Read and parse one tsconfig file into one declaration.
    fn read_tsconfig_into(
        &self,
        is_root: bool,
        path: &Path,
    ) -> Result<TsConfigDeclaration, ResolveError> {
        // read the config file from disk
        let content =
            self.read_path_to_string(path)
                .map_err(|_| ResolveError::TsConfigNotFound {
                    path: path.to_path_buf(),
                })?;

        // reuse file ids to keep incremental mappings stable
        let file_id = self
            .files
            .get_id_by_path(path)
            .unwrap_or_else(|| FileId::from_logical_path(path));
        let (name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        )
        .map_err(|_| ResolveError::TsConfigInvalid {
            path: path.to_path_buf(),
        })?;

        // track the parsed source file in the resolver file registry
        if self.files.get_maybe(file_id).is_some() {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }
        let file = self.files.get(file_id);

        // parse the tracked file as tsconfig
        TsConfigDeclaration::parse(is_root, &file).map_err(|_| ResolveError::TsConfigInvalid {
            path: path.to_path_buf(),
        })
    }

    /// Resolve one specifier through tsconfig `paths`.
    pub(crate) fn rewrite_tsconfig_paths(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // skip tsconfig path rewrites inside module directories
        if Self::is_inside_modules(path) {
            return Ok(None);
        }

        // choose the root tsconfig from explicit options or discovery
        let root_tsconfig_file_id = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => self.read_tsconfig(
                true,
                &tsconfig_options.config_file,
                &tsconfig_options.references,
                &mut TypeScriptOptionsResolveFrame::default(),
                CachePolicy::UseCache,
            )?,
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                let Some(tsconfig_file_id) = self.find_applicable_tsconfig(origin, path, ctx)?
                else {
                    return Ok(None);
                };

                tsconfig_file_id
            }
        };

        // resolve against the effective project and probe each path candidate
        let tsconfig_file_id = self.select_effective_tsconfig(root_tsconfig_file_id, path);
        let tsconfig = self.get_tsconfig(tsconfig_file_id)?;
        let paths = tsconfig.resolve(path, specifier, |reference_path| {
            let reference_tsconfig_file_id =
                self.cached_tsconfig_file_id_by_path(reference_path)?;
            self.cached_tsconfig(reference_tsconfig_file_id)
        });
        for resolved in paths {
            if let Some(resolution) = self.probe_path(&resolved, ".", ctx)? {
                return Ok(Some(resolution));
            }
        }

        Ok(None)
    }

    /// Find the effective tsconfig for one path.
    pub(crate) fn find_applicable_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<FileId>, ResolveError> {
        // skip ineligible paths before touching caches
        if Self::is_inside_modules(path) {
            return Ok(None);
        }
        if !path.is_absolute() {
            return Ok(None);
        }

        // prefer cached result if available
        if let Some(tsconfig_file_id) = self.cached_effective_tsconfig(origin, path) {
            return Ok(tsconfig_file_id);
        }

        // find the nearest config first, then select the effective project
        let Some(nearest_tsconfig_file_id) = self.find_nearest_tsconfig(origin, path, ctx)? else {
            self.cache_effective_tsconfig(origin, path, None);
            return Ok(None);
        };

        let tsconfig_file_id = self.select_effective_tsconfig(nearest_tsconfig_file_id, path);
        self.cache_effective_tsconfig(origin, path, Some(tsconfig_file_id));

        Ok(Some(tsconfig_file_id))
    }

    /// Find the nearest tsconfig by traversing parent directories.
    fn find_nearest_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<FileId>, ResolveError> {
        let search_start = self.tsconfig_search_start(origin, path);
        if let Some(tsconfig_file_id) = self.cached_nearest_tsconfig(&search_start) {
            return Ok(tsconfig_file_id);
        }

        // walk parents until one tsconfig is found
        let mut visited_paths = vec![search_start.clone()];
        let mut current = Some(search_start);
        while let Some(directory) = current {
            let tsconfig_path = directory.join("tsconfig.json");
            if self.is_file(&tsconfig_path, ctx) {
                let tsconfig_file_id = self.read_tsconfig(
                    true,
                    &tsconfig_path,
                    &TypeScriptOptionsReferences::Automatic,
                    &mut TypeScriptOptionsResolveFrame::default(),
                    CachePolicy::UseCache,
                )?;
                for visited_path in visited_paths {
                    self.cache_nearest_tsconfig(&visited_path, Some(tsconfig_file_id));
                }

                return Ok(Some(tsconfig_file_id));
            }

            let Some(parent) = directory.parent() else {
                break;
            };

            let parent = parent.to_path_buf();
            visited_paths.push(parent.clone());
            current = Some(parent);
        }

        // cache the miss for every visited path
        for visited_path in visited_paths {
            self.cache_nearest_tsconfig(&visited_path, None);
        }

        Ok(None)
    }

    /// Select the effective tsconfig for one path from a root config.
    fn select_effective_tsconfig(&self, tsconfig_file_id: FileId, path: &Path) -> FileId {
        let mut visited = HashSet::new();
        self.select_effective_tsconfig_recursive(tsconfig_file_id, path, &mut visited)
            .unwrap_or(tsconfig_file_id)
    }

    /// Select the first referenced tsconfig that actually applies to the path.
    fn select_effective_tsconfig_recursive(
        &self,
        tsconfig_file_id: FileId,
        path: &Path,
        visited: &mut HashSet<FileId>,
    ) -> Option<FileId> {
        // stop cycles in the project reference graph
        if !visited.insert(tsconfig_file_id) {
            return None;
        }

        // accept the current config when it already applies
        let Ok(tsconfig) = self.get_tsconfig(tsconfig_file_id) else {
            return None;
        };
        if tsconfig.applies_to_path(path) {
            return Some(tsconfig_file_id);
        }

        // otherwise search referenced projects depth first
        for reference in &tsconfig.json.references {
            let reference_path = tsconfig.directory.normalize_with(&reference.path);
            let Some(reference_tsconfig_file_id) =
                self.cached_tsconfig_file_id_by_path(&reference_path)
            else {
                continue;
            };

            if let Some(reference_tsconfig_file_id) =
                self.select_effective_tsconfig_recursive(reference_tsconfig_file_id, path, visited)
            {
                return Some(reference_tsconfig_file_id);
            }
        }

        None
    }

    /// Choose the starting directory for tsconfig ancestor lookup.
    fn tsconfig_search_start(&self, origin: ResolveOrigin, path: &Path) -> PathBuf {
        match origin {
            ResolveOrigin::File => path
                .parent()
                .map_or_else(|| path.to_path_buf(), Path::to_path_buf),
            ResolveOrigin::Directory => path.to_path_buf(),
        }
    }

    /// Resolve the path of one extended tsconfig file.
    fn get_extended_tsconfig_path(
        &self,
        directory: &Path,
        tsconfig: &TsConfigDeclaration,
        specifier: &str,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            // empty specifier
            None => Err(ResolveError::InvalidSpecifier {
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
                self.with_options(ResolveOptions {
                    cwd: self.options.cwd.clone(),
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    yarn_pnp: self.options.yarn_pnp,
                    ..ResolveOptions::default()
                })
                .resolve_package_or_modules(directory, specifier, &mut ResolveFrame::default())
                .map(|resolution| resolution.path)
                .map_err(|error| match error {
                    ResolveError::NotFound { .. } => ResolveError::TsConfigNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => error,
                })
            }
        }
    }

    /// Normalize one tsconfig input into a concrete file path.
    fn materialize_tsconfig_path(&self, path: &Path) -> PathBuf {
        let meta = self.metadata(path).ok();

        // keep explicit files as-is
        if meta.is_some_and(|metadata| metadata.is_file) {
            return path.to_path_buf();
        }

        // directory inputs resolve to tsconfig.json
        if meta.is_some_and(|metadata| metadata.is_directory) {
            return path.join("tsconfig.json");
        }

        // plain specifiers default to .json
        let mut os_string = path.to_path_buf().into_os_string();
        os_string.push(".json");
        PathBuf::from(os_string)
    }
}
