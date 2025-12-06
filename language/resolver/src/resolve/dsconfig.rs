use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::{File, FileType, PackageId, PathExt, Uri};
use destack_workspace::DsConfig;

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
    /// Load the dsconfig.json for a package, storing it in the package.
    #[tracing::instrument(name = "resolver.load_package_dsconfig", level = "trace", skip(self))]
    pub fn load_package_dsconfig(&self, package_id: PackageId) -> Result<DsConfig, ResolveError> {
        let package = self.program.packages.get(package_id);
        let package_guard = package.read();

        // check if dsconfig is already loaded
        if let Some(ref dsconfig) = package_guard.dsconfig {
            return Ok(dsconfig.clone());
        }

        // need package path to find dsconfig.json
        let Some(ref package_path) = package_guard.path else {
            return Err(ResolveError::DsConfigNotFound {
                path: PathBuf::from("<ephemeral>"),
            });
        };

        let dsconfig_path = package_path.join("dsconfig.json");
        drop(package_guard); // release lock before loading

        // load and parse the dsconfig
        let dsconfig =
            self.load_dsconfig(&dsconfig_path, &mut DsConfigResolveContext::default())?;

        // store in package
        {
            let mut package_guard = package.write();
            package_guard.dsconfig = Some(dsconfig.clone());
        }

        Ok(dsconfig)
    }

    /// Load and parse a `dsconfig.json` file recursively, handling extends.
    #[tracing::instrument(name = "resolver.load_dsconfig", level = "trace", skip(self, ctx))]
    pub(crate) fn load_dsconfig(
        &self,
        path: &Path,
        ctx: &mut DsConfigResolveContext,
    ) -> Result<DsConfig, ResolveError> {
        tracing::trace!(?path, "resolver.load_dsconfig");

        // parse the `dsconfig.json` file
        let mut dsconfig = self.read_dsconfig(path)?;

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
                destack_workspace::DsConfigExtendsField::Single(s) => vec![s.clone()],
                destack_workspace::DsConfigExtendsField::Multiple(m) => m.clone(),
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
                    let extended = self.load_dsconfig(&extended_dsconfig_path, ctx)?;
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
    fn read_dsconfig(&self, path: &Path) -> Result<DsConfig, ResolveError> {
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

    /// Find the dsconfig.json for a path by looking for the containing package.
    pub fn find_dsconfig(&self, path: &Path) -> Option<PathBuf> {
        // walk up directories looking for dsconfig.json
        let mut current = path.to_path_buf();
        loop {
            let dsconfig_path = current.join("dsconfig.json");
            if self.fs().metadata(&dsconfig_path).is_ok_and(|m| m.is_file) {
                return Some(dsconfig_path);
            }

            // also check for package.json (package boundary)
            let package_json_path = current.join("package.json");
            if self
                .fs()
                .metadata(&package_json_path)
                .is_ok_and(|m| m.is_file)
            {
                // found package boundary, dsconfig.json would be here if it exists
                return None;
            }

            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        None
    }
}
