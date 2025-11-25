use std::borrow::Cow;
use std::path::{Path, PathBuf};

use dyst_dir::{TsConfig, TsConfigId, TsConfigProjectReferences};
use dyst_source::PathExt;

use super::file::is_inside_modules;
use crate::{
    ResolutionContext, ResolveError, ResolveOptions, Resolver, TypeScriptOptionsDiscovery,
    TypeScriptOptionsReferences,
};

/// Context for tracking tsconfig extension chains.
#[derive(Default)]
pub(crate) struct TypeScriptOptionsResolveContext {
    extended_configs: Vec<PathBuf>,
}

impl TypeScriptOptionsResolveContext {
    /// Execute a closure with a new extended file added to the context.
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

    /// Check if a tsconfig path has already been extended in this chain.
    fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs
            .iter()
            .any(|extended| extended == path)
    }

    /// Get the list of extended configurations including the given path.
    fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = self.extended_configs.clone();
        configs.push(path);
        configs
    }
}

impl Resolver {
    /// Resolve a tsconfig.json file at the given path.
    pub fn resolve_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<TsConfigId, ResolveError> {
        self.load_tsconfig(
            true,
            path.as_ref(),
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveContext::default(),
        )
    }

    /// Get the TsConfig for a given TsConfigId.
    pub fn get_tsconfig(&self, tsconfig_id: TsConfigId) -> TsConfig {
        let tsconfig = self.program.tsconfigs.get(tsconfig_id);
        let tsconfig_guard = tsconfig.read();
        (*tsconfig_guard).clone()
    }

    /// Load and parse a tsconfig.json file recursively, using the registry for caching.
    pub(crate) fn load_tsconfig(
        &self,
        is_root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<TsConfigId, ResolveError> {
        // check if already in registry
        if let Some(tsconfig_id) = self.program.tsconfigs.get_id_by_path(path) {
            return Ok(tsconfig_id);
        }

        // parse the `tsconfig.json` file
        let tsconfig_id = self.program.tsconfigs.next_id();
        // nocheckin TODO @Incomplete: should properly register the file in FileRegistry with content
        let file_id = self.program.files.next_id();
        let mut tsconfig = self.read_tsconfig(tsconfig_id, file_id, is_root, path)?;

        // check for circular extends
        if ctx.is_already_extended(&tsconfig.path) {
            return Err(ResolveError::TsConfigCircular {
                paths: ctx.get_extended_configs_with(tsconfig.path.to_path_buf()),
            });
        }

        // extend tsconfig from parent configs
        let extended_tsconfig_paths = tsconfig
            .content
            .extends()
            .map(|specifier| {
                self.get_extended_tsconfig_path(&tsconfig.directory, &tsconfig, specifier)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !extended_tsconfig_paths.is_empty() {
            ctx.with_extended_file(tsconfig.path.to_owned(), |ctx| {
                for extended_tsconfig_path in extended_tsconfig_paths {
                    let extended_tsconfig_id = self.load_tsconfig(
                        false,
                        &extended_tsconfig_path,
                        &TypeScriptOptionsReferences::Disabled,
                        ctx,
                    )?;
                    let extended = self.program.tsconfigs.get(extended_tsconfig_id);
                    let extended_guard = extended.read();
                    tsconfig.extend_from(&extended_guard);
                }
                Result::Ok::<(), ResolveError>(())
            })?;
        }

        // load the given references into this tsconfig
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

        // load reference tsconfigs
        if !tsconfig.content.references.is_empty() {
            let current_path = tsconfig.path.to_path_buf();
            for reference in &tsconfig.content.references {
                let reference_tsconfig_path = tsconfig.directory.normalize_with(&reference.path);
                let reference_tsconfig_id = self.program.tsconfigs.next_id();
                let reference_file_id = self.program.files.next_id();
                // nocheckin TODO @Incomplete: should properly register the file in FileRegistry with content
                let mut referenced_tsconfig = self.read_tsconfig(
                    reference_tsconfig_id,
                    reference_file_id,
                    true,
                    &reference_tsconfig_path,
                )?;

                // error if reference tsconfig points to itself
                if referenced_tsconfig.path == current_path {
                    return Err(ResolveError::TsConfigSelfReference {
                        path: referenced_tsconfig.path.to_path_buf(),
                    });
                }

                // extend the reference tsconfig
                self.extend_tsconfig(
                    &referenced_tsconfig.directory.to_path_buf(),
                    &mut referenced_tsconfig,
                    ctx,
                )?;

                // insert the tsconfig into registry (looked up by path later)
                let referenced_tsconfig = referenced_tsconfig.build();
                self.program.tsconfigs.insert(referenced_tsconfig);
            }
        }

        // store in registry
        let tsconfig = tsconfig.build();
        self.program.tsconfigs.insert(tsconfig);

        Ok(tsconfig_id)
    }

    /// Read and parse a tsconfig.json file.
    fn read_tsconfig(
        &self,
        tsconfig_id: TsConfigId,
        file_id: dyst_source::FileId,
        root: bool,
        path: &Path,
    ) -> Result<TsConfig, ResolveError> {
        // resolve path to actual tsconfig file
        let meta = self.fs().metadata(path).ok();
        let tsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("tsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // read file
        let mut tsconfig_string = self.fs().read_to_string(&tsconfig_path).map_err(|_| {
            ResolveError::TsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // parse
        TsConfig::parse(
            tsconfig_id,
            file_id,
            root,
            &tsconfig_path,
            &mut tsconfig_string,
        )
        .map_err(|_| ResolveError::TsConfigInvalid {
            path: tsconfig_path.to_path_buf(),
        })
    }

    /// Extend a tsconfig with inherited configurations.
    fn extend_tsconfig(
        &self,
        directory: &Path,
        tsconfig: &mut TsConfig,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<(), ResolveError> {
        let extended_tsconfig_paths = tsconfig
            .content
            .extends()
            .map(|specifier| self.get_extended_tsconfig_path(directory, tsconfig, specifier))
            .collect::<Result<Vec<_>, _>>()?;
        for extended_tsconfig_path in extended_tsconfig_paths {
            let extended_tsconfig_id = self.load_tsconfig(
                false,
                &extended_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
            )?;
            let extended = self.program.tsconfigs.get(extended_tsconfig_id);
            let extended_guard = extended.read();
            tsconfig.extend_from(&extended_guard);
        }
        Ok(())
    }

    /// Resolve the specifier using tsconfig `paths` configuration.
    pub(crate) fn load_tsconfig_paths(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if is_inside_modules(path) {
            return Ok(None);
        }

        let tsconfig_id = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => self.load_tsconfig(
                true,
                &tsconfig_options.config_file,
                &tsconfig_options.references,
                &mut TypeScriptOptionsResolveContext::default(),
            )?,
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                let Some(tsconfig_id) = self.find_tsconfig(path, ctx)? else {
                    return Ok(None);
                };
                tsconfig_id
            }
        };

        let tsconfig = self.get_tsconfig(tsconfig_id);
        let paths = tsconfig.resolve(path, specifier, &self.program.tsconfigs);
        for resolved in paths {
            if let Some(resolution) = self.load_as_file_or_directory(&resolved, ".", ctx)? {
                return Ok(Some(resolution));
            }
        }
        Ok(None)
    }

    /// Find tsconfig.json by traversing parent directories.
    pub(crate) fn find_tsconfig(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<TsConfigId>, ResolveError> {
        // don't discover tsconfig for paths inside node_modules
        if is_inside_modules(path) {
            return Ok(None);
        }
        // skip non-absolute paths (e.g. virtual modules)
        if !path.is_absolute() {
            return Ok(None);
        }

        // walk up parent directories looking for tsconfig.json
        let mut current = Some(path.to_path_buf());
        while let Some(dir) = current {
            let tsconfig_path = dir.join("tsconfig.json");
            if self.is_file(&tsconfig_path, ctx) {
                let tsconfig_id = self.resolve_tsconfig(&tsconfig_path)?;
                return Ok(Some(tsconfig_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
        Ok(None)
    }

    /// Resolve the path of an extended tsconfig file.
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
            _ => self
                .with_options(ResolveOptions {
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    ..ResolveOptions::default()
                })
                .load_package_self_or_modules(
                    directory,
                    specifier,
                    &mut ResolutionContext::default(),
                )
                .map(|p| p.to_path_buf())
                .map_err(|err| match err {
                    ResolveError::NotFound { .. } => ResolveError::TsConfigNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => err,
                }),
        }
    }
}
