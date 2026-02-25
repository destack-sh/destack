use std::path::Path;

use destack_source::{File, FileType, PackageId, PackageVersion, Uri};
use destack_workspace::{Package, PackageKind, PackageManifest};

use crate::{CachePolicy, ResolveContext, ResolveError, Resolver};

#[allow(clippy::too_many_arguments)]
impl Resolver {
    pub(crate) fn load_package(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
        cache_policy: CachePolicy,
    ) -> Result<Option<PackageId>, ResolveError> {
        tracing::trace!(?path, "resolver.load.package");
        let package_json_path = path.join("package.json");

        // check if already in registry (indexed by directory path, not package.json path)
        if let Some(package_id) = self.packages.get_id_by_path(path)
            && cache_policy.use_cache()
        {
            let package = self.packages.get(package_id);
            let package = package.read();
            if let Some(ref config) = package.manifest {
                ctx.track_found_dependency(&config.path);
            }
            return Ok(Some(package_id));
        }

        // read file
        let bytes = match self.read_path(&package_json_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                ctx.track_missing_dependency(&package_json_path);
                return Ok(None);
            }
        };

        // create package file
        let file_id = self
            .files
            .get_id_by_path(&package_json_path)
            .unwrap_or_else(|| self.files.next_id());
        let (name, uri) = Uri::from_path_with_name(&package_json_path);
        let file = File::from_bytes_as_json(
            file_id,
            name,
            uri,
            Some(package_json_path.clone()),
            FileType::Json,
            bytes,
        )
        .map_err(|_| ResolveError::InvalidPackageJson {
            path: package_json_path.clone(),
        })?;
        if self.files.get_maybe(file_id).is_some() {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }
        let file = self.files.get(file_id);

        // parse `package.json` from file
        let package_config =
            PackageManifest::parse(&file, package_json_path.clone()).map_err(|_| {
                ResolveError::InvalidPackageJson {
                    path: package_json_path.clone(),
                }
            })?;

        // load dsconfig.json if it exists (optional), but fail loudly for invalid content
        let dsconfig = match self.load_package_dsconfig(&package_config, cache_policy) {
            Ok(dsconfig) => Some(dsconfig),
            Err(ResolveError::DsConfigNotFound { .. }) => None,
            Err(error) => return Err(error),
        };

        // insert or update package
        let package_id = PackageId::from_path(&package_config.directory);
        if let Some(package_id) = self.packages.get_id_by_path(path) {
            let package = self.packages.get(package_id);
            let mut package = package.write();
            package.uri = package_config.uri.clone();
            package.path = Some(package_config.directory.clone());
            package.name = package_config.content.name.clone();
            package.version = package_config.content.version.clone();
            package.manifest = Some(package_config);
            package.dsconfig = dsconfig;
        } else {
            let package = Package {
                id: package_id,
                package_version: PackageVersion::INITIAL,
                kind: PackageKind::Physical,
                uri: package_config.uri.clone(),
                path: Some(package_config.directory.clone()),
                name: package_config.content.name.clone(),
                version: package_config.content.version.clone(),
                manifest: Some(package_config),
                dsconfig,
                tsconfig: None,
                targets: Default::default(),
            };
            self.packages.insert(package);
        }

        ctx.track_found_dependency(&package_json_path);
        Ok(Some(package_id))
    }

    /// Find the nearest package.json by traversing parent directories.
    #[tracing::instrument(name = "resolver.package.find", level = "trace", skip(self, ctx))]
    pub(crate) fn find_package_json(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PackageId>, ResolveError> {
        tracing::trace!(?path, "resolver.package.find");
        let mut current = path.to_path_buf();

        // go up directories when the querying path is not a directory
        while !self.is_directory(&current, ctx) {
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        // traverse parents looking for package.json
        let mut current = Some(current);
        while let Some(dir) = current {
            if let Some(package_id) = self.load_package(&dir, ctx, CachePolicy::UseCache)? {
                return Ok(Some(package_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }
}
