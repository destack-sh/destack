use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PackageId, PathExt, Uri};
use destack_workspace::{DsConfig, PackageManifest};

use crate::{CachePolicy, ResolveError, ResolveFrame, Resolver};

/// The resolve frame for one dsconfig extension chain.
#[derive(Default)]
pub(crate) struct DsConfigResolveFrame {
    extended_configs: Vec<PathBuf>,
}

impl DsConfigResolveFrame {
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
    /// Read the dsconfig.json for a package.
    pub fn read_package_dsconfig(
        &self,
        package_config: &PackageManifest,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        let dsconfig_path = package_config.directory.join("dsconfig.json");

        // read the package local dsconfig
        let dsconfig = self.read_dsconfig(&dsconfig_path, cache_policy)?;

        Ok(dsconfig)
    }

    /// Load the dsconfig for one package into the package registry when present.
    pub(crate) fn ensure_package_dsconfig(
        &self,
        package_id: PackageId,
        cache_policy: CachePolicy,
    ) -> Result<(), ResolveError> {
        let package = self.packages.get(package_id);
        let package_path = package.read().path.clone();

        let Some(package_path) = package_path else {
            return Ok(());
        };

        // read the package dsconfig if it exists
        let next_dsconfig = match self.read_dsconfig(&package_path, cache_policy) {
            Ok(dsconfig) => Some(dsconfig),
            Err(ResolveError::DsConfigNotFound { .. }) => None,
            Err(error) => return Err(error),
        };

        let package = self.packages.get(package_id);
        package.write().dsconfig = next_dsconfig;
        Ok(())
    }

    /// Find one package dsconfig by walking to the nearest package scope first.
    pub fn find_package_dsconfig(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<Option<DsConfig>, ResolveError> {
        let mut ctx = ResolveFrame::default();
        let Some(package_id) = self.find_nearest_package_scope(path, &mut ctx)? else {
            return Ok(None);
        };

        self.ensure_package_dsconfig(package_id, cache_policy)?;

        let package = self.packages.get(package_id);
        Ok(package.read().dsconfig.clone())
    }

    /// Read and parse a `dsconfig.json` file recursively, handling extends.
    pub fn read_dsconfig(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        self.read_dsconfig_with_context(path, &mut DsConfigResolveFrame::default(), cache_policy)
    }

    /// Read and parse a `dsconfig.json` file recursively, handling extends.
    pub(crate) fn read_dsconfig_with_context(
        &self,
        path: &Path,
        ctx: &mut DsConfigResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        // parse the local config first
        let mut dsconfig = self.parse_dsconfig(path, cache_policy)?;

        // reject circular extends chains
        if ctx.is_already_extended(&dsconfig.path) {
            return Err(ResolveError::DsConfigCircular {
                paths: ctx.get_extended_configs_with(dsconfig.path.to_path_buf()),
            });
        }

        // resolve every extended config path up front
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

        // merge parent configs in order
        if !extended_dsconfig_paths.is_empty() {
            let dsconfig_path = dsconfig.path.clone();
            ctx.with_extended_file(dsconfig_path, |ctx| {
                for extended_dsconfig_path in extended_dsconfig_paths {
                    let extended = self.read_dsconfig_with_context(
                        &extended_dsconfig_path,
                        ctx,
                        cache_policy,
                    )?;
                    dsconfig.extend_from(&extended);
                }
                Ok(())
            })?;
        }

        // rebuild the derived options after merging
        dsconfig.build();

        Ok(dsconfig)
    }

    /// Parse one dsconfig.json file without processing extends.
    fn parse_dsconfig(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<DsConfig, ResolveError> {
        // normalize the input into a concrete config path
        let meta = self.metadata(path).ok();
        let dsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("dsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // reuse cached file content when allowed
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

        // read the config file from disk
        let content = self.read_path_to_string(&dsconfig_path).map_err(|_| {
            ResolveError::DsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // reuse file ids to keep incremental mappings stable
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

        // parse the dsconfig from the tracked file
        let dsconfig = DsConfig::parse(&file).map_err(|_| ResolveError::DsConfigInvalid {
            path: dsconfig_path.to_path_buf(),
        })?;

        Ok(dsconfig)
    }

    /// Resolve the path of one extended dsconfig file.
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
            // package specifier: for now just treat as relative
            _ => Ok(directory.normalize_with(specifier)),
        }
    }
}
