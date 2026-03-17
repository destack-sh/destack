use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PathExt, Uri};
use destack_workspace::{TsConfig, TsConfigId, TsConfigProjectReferences};

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
    pub fn resolve_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<TsConfigId, ResolveError> {
        self.read_tsconfig(
            true,
            path.as_ref(),
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveFrame::default(),
            CachePolicy::UseCache,
        )
    }

    /// Reload a `tsconfig.json` file at the given path.
    pub fn reload_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<TsConfigId, ResolveError> {
        self.read_tsconfig(
            true,
            path.as_ref(),
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveFrame::default(),
            CachePolicy::Reload,
        )
    }

    /// Return the tsconfig for one id.
    pub fn get_tsconfig(&self, tsconfig_id: TsConfigId) -> TsConfig {
        let tsconfig = self.tsconfigs.get(tsconfig_id);
        let tsconfig_guard = tsconfig.read();
        (*tsconfig_guard).clone()
    }

    /// Read and parse a `tsconfig.json` file recursively.
    pub(crate) fn read_tsconfig(
        &self,
        is_root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut TypeScriptOptionsResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<TsConfigId, ResolveError> {
        // reuse an existing parsed config when allowed
        let existing_id = self.tsconfigs.get_id_by_path(path);
        if cache_policy.use_cache()
            && let Some(tsconfig_id) = existing_id
        {
            return Ok(tsconfig_id);
        }

        // parse and register the root config entry
        let tsconfig_id = existing_id.unwrap_or_else(|| self.tsconfigs.next_id());
        let tsconfig = self.read_tsconfig_into(tsconfig_id, is_root, path)?;
        if let Some(tsconfig_id) = existing_id {
            let entry = self.tsconfigs.get(tsconfig_id);
            *entry.write() = tsconfig;
        } else {
            self.tsconfigs.insert(tsconfig);
        }
        let tsconfig = self.tsconfigs.get(tsconfig_id);

        // reject circular extends chains
        {
            let tsconfig = tsconfig.read();
            if ctx.is_already_extended(&tsconfig.path) {
                return Err(ResolveError::TsConfigCircular {
                    paths: ctx.get_extended_configs_with(tsconfig.path.to_path_buf()),
                });
            }
        }

        // resolve every parent config path
        let extended_tsconfig_paths = {
            let tsconfig = tsconfig.read();
            tsconfig
                .content
                .extends()
                .map(|specifier| {
                    self.get_extended_tsconfig_path(&tsconfig.directory, &tsconfig, specifier)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        // merge parent configs in order
        if !extended_tsconfig_paths.is_empty() {
            let tsconfig_path = tsconfig.read().path.to_owned();
            ctx.with_extended_file(tsconfig_path, |ctx| {
                for extended_tsconfig_path in extended_tsconfig_paths {
                    let extended_tsconfig_id = self.read_tsconfig(
                        false,
                        &extended_tsconfig_path,
                        &TypeScriptOptionsReferences::Disabled,
                        ctx,
                        cache_policy,
                    )?;
                    let extended = self.tsconfigs.get(extended_tsconfig_id);
                    let extended_guard = extended.read();
                    let mut tsconfig = tsconfig.write();
                    tsconfig.extend_from(&extended_guard);
                }
                Result::Ok::<(), ResolveError>(())
            })?;
        }

        // replace references when the caller requested it
        {
            let mut tsconfig = tsconfig.write();
            match references {
                TypeScriptOptionsReferences::Disabled => {
                    tsconfig.content.references.drain(..);
                }
                TypeScriptOptionsReferences::Automatic => {}
                TypeScriptOptionsReferences::Paths(paths) => {
                    tsconfig.content.references = paths
                        .iter()
                        .map(|path| TsConfigProjectReferences { path: path.clone() })
                        .collect();
                }
            }
        }

        // load and validate referenced projects
        let references_to_load: Vec<_> = {
            let tsconfig = tsconfig.read();
            tsconfig
                .content
                .references
                .iter()
                .map(|r| tsconfig.directory.normalize_with(&r.path))
                .collect()
        };
        let current_path = tsconfig.read().path.to_path_buf();
        for reference_tsconfig_path in references_to_load {
            let reference_tsconfig_id = self.read_tsconfig(
                true,
                &reference_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
                cache_policy,
            )?;

            // reject self references
            {
                let referenced = self.tsconfigs.get(reference_tsconfig_id);
                let referenced_tsconfig = referenced.read();
                if referenced_tsconfig.path == current_path {
                    return Err(ResolveError::TsConfigSelfReference {
                        path: referenced_tsconfig.path.to_path_buf(),
                    });
                }
            }

            // extend each referenced config before building it
            {
                let referenced_tsconfig = self.tsconfigs.get(reference_tsconfig_id);
                let directory = referenced_tsconfig.read().directory.to_path_buf();
                self.extend_tsconfig(reference_tsconfig_id, &directory, ctx, cache_policy)?;
            }

            // build each referenced config after extension
            {
                let referenced_tsconfig = self.tsconfigs.get(reference_tsconfig_id);
                let mut referenced_tsconfig = referenced_tsconfig.write();
                referenced_tsconfig.build();
                referenced_tsconfig.refresh_options();
            }
        }

        // build the main config last
        {
            let mut tsconfig = tsconfig.write();
            tsconfig.build();
            tsconfig.refresh_options();
        }

        Ok(tsconfig_id)
    }

    /// Read and parse one tsconfig file into a `TsConfig`.
    fn read_tsconfig_into(
        &self,
        tsconfig_id: TsConfigId,
        is_root: bool,
        path: &Path,
    ) -> Result<TsConfig, ResolveError> {
        // normalize the input into a concrete config path
        let meta = self.metadata(path).ok();
        let tsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("tsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // read the config file from disk
        let content = self.read_path_to_string(&tsconfig_path).map_err(|_| {
            ResolveError::TsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // reuse file ids to keep incremental mappings stable
        let file_id = self
            .files
            .get_id_by_path(&tsconfig_path)
            .unwrap_or_else(|| self.files.next_id());
        let (name, uri) = Uri::from_path_with_name(&*tsconfig_path);
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(tsconfig_path.to_path_buf()),
            FileType::Json,
            content,
        )
        .map_err(|_| ResolveError::TsConfigInvalid {
            path: tsconfig_path.to_path_buf(),
        })?;
        if self.files.get_maybe(file_id).is_some() {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }
        let file = self.files.get(file_id);

        // parse the tsconfig from the tracked file
        let tsconfig = TsConfig::parse(tsconfig_id, is_root, &file).map_err(|_| {
            ResolveError::TsConfigInvalid {
                path: tsconfig_path.to_path_buf(),
            }
        })?;

        Ok(tsconfig)
    }

    /// Extend one tsconfig entry with its inherited configurations.
    fn extend_tsconfig(
        &self,
        tsconfig_id: TsConfigId,
        directory: &Path,
        ctx: &mut TypeScriptOptionsResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<(), ResolveError> {
        // collect the parent config paths first
        let extended_tsconfig_paths = {
            let tsconfig_lock = self.tsconfigs.get(tsconfig_id);
            let tsconfig = tsconfig_lock.read();
            tsconfig
                .content
                .extends()
                .map(|specifier| self.get_extended_tsconfig_path(directory, &tsconfig, specifier))
                .collect::<Result<Vec<_>, _>>()?
        };

        // merge each parent config in order
        for extended_tsconfig_path in extended_tsconfig_paths {
            let extended_tsconfig_id = self.read_tsconfig(
                false,
                &extended_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
                cache_policy,
            )?;
            let extended = self.tsconfigs.get(extended_tsconfig_id);
            let extended_guard = extended.read();
            let tsconfig_lock = self.tsconfigs.get(tsconfig_id);
            let mut tsconfig = tsconfig_lock.write();
            tsconfig.extend_from(&extended_guard);
        }
        Ok(())
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
        let root_tsconfig_id = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => self.read_tsconfig(
                true,
                &tsconfig_options.config_file,
                &tsconfig_options.references,
                &mut TypeScriptOptionsResolveFrame::default(),
                CachePolicy::UseCache,
            )?,
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                let Some(tsconfig_id) = self.find_applicable_tsconfig(origin, path, ctx)? else {
                    return Ok(None);
                };
                tsconfig_id
            }
        };

        // resolve against the effective project and probe each path candidate
        let tsconfig_id = self.select_effective_tsconfig(root_tsconfig_id, path);
        let tsconfig = self.get_tsconfig(tsconfig_id);
        let paths = tsconfig.resolve(path, specifier, &self.tsconfigs);
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
    ) -> Result<Option<TsConfigId>, ResolveError> {
        // skip ineligible paths before touching caches
        if Self::is_inside_modules(path) {
            return Ok(None);
        }
        if !path.is_absolute() {
            return Ok(None);
        }

        // prefer cached result if available
        if let Some(tsconfig_id) = self.cached_effective_tsconfig(origin, path) {
            return Ok(tsconfig_id);
        }

        // find the nearest config first, then select the effective project
        let Some(nearest_tsconfig_id) = self.find_nearest_tsconfig(origin, path, ctx)? else {
            self.cache_effective_tsconfig(origin, path, None);
            return Ok(None);
        };

        let tsconfig_id = self.select_effective_tsconfig(nearest_tsconfig_id, path);
        self.cache_effective_tsconfig(origin, path, Some(tsconfig_id));
        Ok(Some(tsconfig_id))
    }

    /// Find the nearest tsconfig by traversing parent directories.
    fn find_nearest_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<TsConfigId>, ResolveError> {
        let search_start = self.tsconfig_search_start(origin, path);
        if let Some(tsconfig_id) = self.cached_nearest_tsconfig(&search_start) {
            return Ok(tsconfig_id);
        }

        // walk parents until one tsconfig is found
        let mut visited_paths = vec![search_start.clone()];
        let mut current = Some(search_start);
        while let Some(directory) = current {
            let tsconfig_path = directory.join("tsconfig.json");
            if self.is_file(&tsconfig_path, ctx) {
                let tsconfig_id = self.resolve_tsconfig(&tsconfig_path)?;
                for visited_path in visited_paths {
                    self.cache_nearest_tsconfig(&visited_path, Some(tsconfig_id));
                }
                return Ok(Some(tsconfig_id));
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
    fn select_effective_tsconfig(&self, tsconfig_id: TsConfigId, path: &Path) -> TsConfigId {
        let mut visited = HashSet::new();
        self.select_effective_tsconfig_recursive(tsconfig_id, path, &mut visited)
            .unwrap_or(tsconfig_id)
    }

    /// Select the first referenced tsconfig that actually applies to the path.
    fn select_effective_tsconfig_recursive(
        &self,
        tsconfig_id: TsConfigId,
        path: &Path,
        visited: &mut HashSet<TsConfigId>,
    ) -> Option<TsConfigId> {
        // stop cycles in the project reference graph
        if !visited.insert(tsconfig_id) {
            return None;
        }

        // accept the current config when it already applies
        let tsconfig = self.get_tsconfig(tsconfig_id);
        if tsconfig.applies_to_path(path) {
            return Some(tsconfig_id);
        }

        // otherwise search referenced projects depth first
        for reference in &tsconfig.content.references {
            let reference_path = tsconfig.directory.normalize_with(&reference.path);
            let Some(reference_tsconfig_id) = self.tsconfigs.get_id_by_path(&reference_path) else {
                continue;
            };

            if let Some(reference_tsconfig_id) =
                self.select_effective_tsconfig_recursive(reference_tsconfig_id, path, visited)
            {
                return Some(reference_tsconfig_id);
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
        tsconfig: &TsConfig,
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
                .map_err(|err| match err {
                    ResolveError::NotFound { .. } => ResolveError::TsConfigNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => err,
                })
            }
        }
    }
}
