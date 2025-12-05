use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PathExt, Uri};
use destack_workspace::{DsConfig, DsConfigId};

use crate::{ResolveError, Resolver};

/// Context for tracking dsconfig extension chains.
#[derive(Default)]
pub(crate) struct DsConfigResolveContext {
    extended_configs: Vec<PathBuf>,
}

impl DsConfigResolveContext {
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

    /// Check if a dsconfig path has already been extended in this chain.
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
    /// Resolve a `dsconfig.json` file at the given path.
    #[tracing::instrument(name = "resolver.resolve_dsconfig", level = "trace", skip(self, path))]
    pub fn resolve_dsconfig<P: AsRef<Path>>(&self, path: P) -> Result<DsConfigId, ResolveError> {
        tracing::trace!(path = ?path.as_ref(), "resolver.resolve_dsconfig");
        self.load_dsconfig(true, path.as_ref(), &mut DsConfigResolveContext::default())
    }

    /// Get the DsConfig for a given DsConfigId.
    pub fn get_dsconfig(&self, dsconfig_id: DsConfigId) -> DsConfig {
        let dsconfig = self.program.dsconfigs.get(dsconfig_id);
        let dsconfig_guard = dsconfig.read();
        (*dsconfig_guard).clone()
    }

    /// Load and parse a `dsconfig.json` file recursively.
    #[tracing::instrument(name = "resolver.load_dsconfig", level = "trace", skip(self, ctx))]
    pub(crate) fn load_dsconfig(
        &self,
        is_root: bool,
        path: &Path,
        ctx: &mut DsConfigResolveContext,
    ) -> Result<DsConfigId, ResolveError> {
        tracing::trace!(?path, is_root, "resolver.load_dsconfig");

        // check if already in registry
        if let Some(dsconfig_id) = self.program.dsconfigs.get_id_by_path(path) {
            return Ok(dsconfig_id);
        }

        // parse the `dsconfig.json` file
        let dsconfig_id = self.read_dsconfig(is_root, path)?;
        let dsconfig = self.program.dsconfigs.get(dsconfig_id);

        // check for circular extends
        {
            let dsconfig = dsconfig.read();
            if ctx.is_already_extended(&dsconfig.path) {
                return Err(ResolveError::DsConfigCircular {
                    paths: ctx.get_extended_configs_with(dsconfig.path.to_path_buf()),
                });
            }
        }

        // extend dsconfig from parent configs
        let extended_dsconfig_paths = {
            let dsconfig = dsconfig.read();
            let directory = dsconfig.directory.clone();
            let specifiers: Vec<String> = dsconfig
                .content
                .extends
                .as_ref()
                .map(|extends| match extends {
                    destack_workspace::DsConfigExtendsField::Single(s) => vec![s.clone()],
                    destack_workspace::DsConfigExtendsField::Multiple(m) => m.clone(),
                })
                .unwrap_or_default();
            drop(dsconfig); // release the read lock before resolving paths
            specifiers
                .into_iter()
                .map(|specifier| self.get_extended_dsconfig_path(&directory, &specifier))
                .collect::<Result<Vec<_>, _>>()?
        };

        if !extended_dsconfig_paths.is_empty() {
            let dsconfig_path = dsconfig.read().path.to_owned();
            ctx.with_extended_file(dsconfig_path, |ctx| {
                for extended_dsconfig_path in extended_dsconfig_paths {
                    let extended_dsconfig_id =
                        self.load_dsconfig(false, &extended_dsconfig_path, ctx)?;
                    let extended = self.program.dsconfigs.get(extended_dsconfig_id);
                    let extended_guard = extended.read();
                    let mut dsconfig = dsconfig.write();
                    dsconfig.extend_from(&extended_guard);
                }
                Result::Ok::<(), ResolveError>(())
            })?;
        }

        // build the main dsconfig (finalize options)
        {
            let mut dsconfig = dsconfig.write();
            dsconfig.build();
        }

        Ok(dsconfig_id)
    }

    /// Read and parse a dsconfig.json file, inserting into registries.
    fn read_dsconfig(&self, is_root: bool, path: &Path) -> Result<DsConfigId, ResolveError> {
        // resolve path to actual dsconfig file
        let meta = self.fs().metadata(path).ok();
        let dsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("dsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // read `dsconfig.json` file
        let content = self.fs().read_to_string(&dsconfig_path).map_err(|_| {
            ResolveError::DsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;
        let file_id = self.program.files.next_id();
        let (name, uri) = Uri::from_path_with_name(&*dsconfig_path);
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(dsconfig_path.to_path_buf()),
            FileType::Json,
            content,
        )
        .map_err(|_| ResolveError::DsConfigInvalid {
            path: dsconfig_path.to_path_buf(),
        })?;
        self.program.files.insert(file);
        let file = self.program.files.get(file_id);

        // parse dsconfig from file
        let dsconfig_id = self.program.dsconfigs.next_id();
        let dsconfig = DsConfig::parse(dsconfig_id, is_root, &file).map_err(|_| {
            ResolveError::DsConfigInvalid {
                path: dsconfig_path.to_path_buf(),
            }
        })?;
        self.program.dsconfigs.insert(dsconfig);

        Ok(dsconfig_id)
    }

    /// Resolve the path of an extended dsconfig file.
    fn get_extended_dsconfig_path(
        &self,
        directory: &Path,
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
            Some(b'.') => Ok(directory.normalize_with(specifier)),
            // package specifier - for now just treat as relative
            _ => Ok(directory.normalize_with(specifier)),
        }
    }
}
