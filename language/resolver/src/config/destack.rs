use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PackageId, PathExt, Uri};
use destack_workspace::{Destack, PackageManifest};

use crate::{CachePolicy, ResolveError, ResolveFrame, Resolver};

/// The resolve frame for one Destack config extension chain.
#[derive(Default)]
pub(crate) struct DestackResolveFrame {
    extended_configs: Vec<PathBuf>,
}

impl DestackResolveFrame {
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
    /// Read the destack.json for a package.
    pub fn read_package_destack_config(
        &self,
        package_config: &PackageManifest,
        cache_policy: CachePolicy,
    ) -> Result<Destack, ResolveError> {
        let destack_config_path = package_config.directory.join("destack.json");

        // read the package local Destack config
        let config = self.read_destack_config(&destack_config_path, cache_policy)?;

        Ok(config)
    }

    /// Load the Destack config for one package into the package registry when present.
    pub(crate) fn ensure_package_destack_config(
        &self,
        package_id: PackageId,
        cache_policy: CachePolicy,
    ) -> Result<(), ResolveError> {
        let package = self.packages.get(package_id);
        let package_path = package.read().path.clone();

        let Some(package_path) = package_path else {
            return Ok(());
        };

        // read the package Destack config if it exists
        let next_destack_config = match self.read_destack_config(&package_path, cache_policy) {
            Ok(config) => Some(config),
            Err(ResolveError::DestackNotFound { .. }) => None,
            Err(error) => return Err(error),
        };

        let package = self.packages.get(package_id);
        let mut package = package.write();
        package.config = next_destack_config;
        let config = package.config.clone();

        let next_name_version = config.as_ref().map(|config| {
            let name = config.options.name.clone();
            let version = config.options.version.clone();
            (name, version)
        });

        if let Some(manifest) = package.manifest.as_mut() {
            manifest.refresh_from_destack(config.as_ref());
            let name = manifest.content.name.clone();
            let version = manifest.content.version.clone();
            package.name = name;
            package.version = version;
        } else if let Some((name, version)) = next_name_version {
            package.name = name;
            package.version = version;
        }

        Ok(())
    }

    /// Find one package Destack config by walking to the nearest package scope first.
    pub fn find_package_destack_config(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<Option<Destack>, ResolveError> {
        let mut ctx = ResolveFrame::default();
        let Some(package_id) = self.find_nearest_package_scope(path, &mut ctx)? else {
            return Ok(None);
        };

        self.ensure_package_destack_config(package_id, cache_policy)?;

        let package = self.packages.get(package_id);
        Ok(package.read().config.clone())
    }

    /// Read and parse a `destack.json` file recursively, handling extends.
    pub fn read_destack_config(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<Destack, ResolveError> {
        self.read_destack_config_with_context(
            path,
            &mut DestackResolveFrame::default(),
            cache_policy,
        )
    }

    /// Read and parse a `destack.json` file recursively, handling extends.
    pub(crate) fn read_destack_config_with_context(
        &self,
        path: &Path,
        ctx: &mut DestackResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<Destack, ResolveError> {
        // parse the local config first
        let mut config = self.parse_destack_config(path, cache_policy)?;

        // reject circular extends chains
        if ctx.is_already_extended(&config.path) {
            return Err(ResolveError::DestackCircular {
                paths: ctx.get_extended_configs_with(config.path.to_path_buf()),
            });
        }

        // resolve every extended config path up front
        let extended_config_paths: Vec<PathBuf> = config
            .content
            .extends
            .as_ref()
            .map(|extends| match extends {
                destack_workspace::ExtendsFieldJson::Single(s) => vec![s.clone()],
                destack_workspace::ExtendsFieldJson::Multiple(m) => m.clone(),
            })
            .unwrap_or_default()
            .into_iter()
            .map(|specifier| self.get_extended_destack_config_path(&config.directory, &specifier))
            .collect::<Result<Vec<_>, _>>()?;

        // merge parent configs in order
        if !extended_config_paths.is_empty() {
            let config_path = config.path.clone();
            ctx.with_extended_file(config_path, |ctx| {
                for extended_config_path in extended_config_paths {
                    let extended = self.read_destack_config_with_context(
                        &extended_config_path,
                        ctx,
                        cache_policy,
                    )?;
                    config.extend_from(&extended);
                }
                Ok(())
            })?;
        }

        // rebuild the derived options after merging
        config.build();

        Ok(config)
    }

    /// Parse one destack.json file without processing extends.
    fn parse_destack_config(
        &self,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<Destack, ResolveError> {
        // normalize the input into a concrete config path
        let meta = self.metadata(path).ok();
        let destack_config_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("destack.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // reuse cached file content when allowed
        let existing_id = self.files.get_id_by_path(destack_config_path.as_ref());
        if cache_policy.use_cache()
            && let Some(file_id) = existing_id
            && let Some(file) = self.files.get_maybe(file_id)
        {
            let config = Destack::parse(&file).map_err(|_| ResolveError::DestackInvalid {
                path: destack_config_path.to_path_buf(),
            })?;
            return Ok(config);
        }

        // read the config file from disk
        let content = self
            .read_path_to_string(&destack_config_path)
            .map_err(|_| ResolveError::DestackNotFound {
                path: path.to_path_buf(),
            })?;

        // reuse file ids to keep incremental mappings stable
        let file_id = existing_id.unwrap_or_else(|| self.files.next_id());
        let (name, uri) = Uri::from_path_with_name(&*destack_config_path);
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(destack_config_path.to_path_buf()),
            FileType::Json,
            content,
        )
        .map_err(|_| ResolveError::DestackInvalid {
            path: destack_config_path.to_path_buf(),
        })?;

        if self.files.get_maybe(file_id).is_some() {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }
        let file = self.files.get(file_id);

        // parse the Destack config from the tracked file
        let config = Destack::parse(&file).map_err(|_| ResolveError::DestackInvalid {
            path: destack_config_path.to_path_buf(),
        })?;

        Ok(config)
    }

    /// Resolve the path of one extended Destack config file.
    fn get_extended_destack_config_path(
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
