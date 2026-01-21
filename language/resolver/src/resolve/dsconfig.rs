use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PathExt, Uri};
use destack_workspace::{DsConfig, PackageManifest};

use crate::{CachePolicy, ResolveError, Resolver};

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
    /// Load the dsconfig.json for a package, storing it in the package.
    #[tracing::instrument(name = "resolver.load_package_dsconfig", level = "trace", skip(self))]
    pub fn load_package_dsconfig(
        &self,
        package_config: &PackageManifest,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        let dsconfig_path = package_config.directory.join("dsconfig.json");

        // load and parse the dsconfig
        let dsconfig = self.load_dsconfig(&dsconfig_path, cache_policy)?;

        Ok(dsconfig)
    }

    /// Load and parse a `dsconfig.json` file recursively, handling extends.
    #[tracing::instrument(name = "resolver.load_dsconfig", level = "trace", skip(self))]
    pub fn load_dsconfig(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        self.load_dsconfig_with_context(path, &mut DsConfigResolveContext::default(), cache_policy)
    }

    /// Load and parse a `dsconfig.json` file recursively, handling extends.
    #[tracing::instrument(name = "resolver.load_dsconfig", level = "trace", skip(self, ctx))]
    pub(crate) fn load_dsconfig_with_context(
        &self,
        path: &Path,
        ctx: &mut DsConfigResolveContext,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        tracing::trace!(?path, "resolver.load_dsconfig");

        // parse the `dsconfig.json` file
        let mut dsconfig = self.read_dsconfig(path, cache_policy)?;

        // check for circular extends
        if ctx.is_already_extended(&dsconfig.path) {
            return Err(ResolveError::DsConfigCircular {
                paths: ctx.get_extended_configs_with(dsconfig.path.to_path_buf()),
            });
        }

        // collect extended config paths
        let extended_dsconfig_paths: Vec<PathBuf> = dsconfig
            .content
            .extends
            .as_ref()
            .map(|extends| match extends {
                destack_workspace::ExtendsFieldJson::Single(s) => vec![s.clone()],
                destack_workspace::ExtendsFieldJson::Multiple(m) => m.clone(),
            })
            .unwrap_or_default()
            .into_iter()
            .map(|specifier| self.get_extended_dsconfig_path(&dsconfig.directory, &specifier))
            .collect::<Result<Vec<_>, _>>()?;

        // extend from parent configs
        if !extended_dsconfig_paths.is_empty() {
            let dsconfig_path = dsconfig.path.clone();
            ctx.with_extended_file(dsconfig_path, |ctx| {
                for extended_dsconfig_path in extended_dsconfig_paths {
                    let extended = self.load_dsconfig_with_context(
                        &extended_dsconfig_path,
                        ctx,
                        cache_policy,
                    )?;
                    dsconfig.extend_from(&extended);
                }
                Ok(())
            })?;
        }

        // finalize options
        dsconfig.build();

        Ok(dsconfig)
    }

    /// Read and parse a dsconfig.json file.
    fn read_dsconfig(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
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

        // reuse cached file contents when allowed
        let existing_id = self.files.get_id_by_path(dsconfig_path.as_ref());
        if cache_policy.use_cache()
            && let Some(file_id) = existing_id
            && let Some(file) = self.files.get_maybe(file_id)
        {
            let dsconfig = DsConfig::parse(&file).map_err(|_| ResolveError::DsConfigInvalid {
                path: dsconfig_path.to_path_buf(),
            })?;
            return Ok(dsconfig);
        }

        // read `dsconfig.json` file
        let content = self.fs().read_to_string(&dsconfig_path).map_err(|_| {
            ResolveError::DsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // reuse file ids when possible to keep incremental mappings stable
        let file_id = existing_id.unwrap_or_else(|| self.files.next_id());
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

        if self.files.get_maybe(file_id).is_some() {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }
        let file = self.files.get(file_id);

        // parse dsconfig from file
        let dsconfig = DsConfig::parse(&file).map_err(|_| ResolveError::DsConfigInvalid {
            path: dsconfig_path.to_path_buf(),
        })?;

        Ok(dsconfig)
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
