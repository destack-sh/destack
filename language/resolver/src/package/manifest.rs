use std::path::Path;

use destack_source::{File, FileType, PackageId, PackageVersion, Uri};
use destack_workspace::{Package, PackageKind, PackageManifest};

use crate::{CachePolicy, ResolveError, ResolveFrame, Resolver};

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Read one package manifest from a directory.
    pub(crate) fn read_package_manifest(
        &self,
        path: &Path,
        ctx: &mut ResolveFrame,
        cache_policy: CachePolicy,
    ) -> Result<Option<PackageId>, ResolveError> {
        let package_json_path = path.join("package.json");
        let destack_config_path = path.join("destack.json");

        // reuse the cached package entry when possible
        if let Some(package_id) = self.packages.get_id_by_path(path)
            && cache_policy.use_cache()
        {
            let package = self.packages.get(package_id);
            let package = package.read();
            if let Some(ref config) = package.manifest {
                ctx.track_found_dependency(&config.path);
            }
            if let Some(ref config) = package.config {
                ctx.track_found_dependency(&config.path);
            }
            return Ok(Some(package_id));
        }

        // read destack config when present so it can override package compatibility fields
        let destack_config = match self.read_destack_config(path, cache_policy) {
            Ok(config) => {
                ctx.track_found_dependency(&config.path);
                Some(config)
            }
            Err(ResolveError::DestackNotFound { .. }) => {
                ctx.track_missing_dependency(&destack_config_path);
                None
            }
            Err(error) => return Err(error),
        };

        // read package.json when present, otherwise synthesize a manifest from destack.json
        let mut package_config = match self.read_path(&package_json_path) {
            Ok(bytes) => {
                // materialize the manifest as a tracked file entry
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

                // align manifest realpaths with path canonicalization
                let package_json_realpath = if self.options.canonicalize_symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };

                ctx.track_found_dependency(&package_json_path);
                PackageManifest::parse(&file, package_json_realpath).map_err(|_| {
                    ResolveError::InvalidPackageJson {
                        path: package_json_path.clone(),
                    }
                })?
            }
            Err(_) => {
                ctx.track_missing_dependency(&package_json_path);

                let Some(config) = destack_config.as_ref() else {
                    return Ok(None);
                };

                let realpath = if self.options.canonicalize_symlinks {
                    self.canonicalize(&config.path)?
                } else {
                    config.path.clone()
                };

                PackageManifest::from_destack(config, realpath)
            }
        };

        // make destack authoritative for overlapping manifest fields
        package_config.refresh_from_destack(destack_config.as_ref());

        // insert or refresh the package registry entry
        let package_id = PackageId::from_path(&package_config.directory);
        if let Some(package_id) = self.packages.get_id_by_path(path) {
            let package = self.packages.get(package_id);
            let mut package = package.write();
            package.uri = package_config.uri.clone();
            package.path = Some(package_config.directory.clone());
            package.name = package_config.content.name.clone();
            package.version = package_config.content.version.clone();
            package.manifest = Some(package_config);
            package.config = destack_config;
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
                config: destack_config,
                tsconfig: None,
                targets: Default::default(),
            };
            self.packages.insert(package);
        }
        Ok(Some(package_id))
    }

    /// Find the nearest package.json by traversing parent directories.
    pub(crate) fn find_nearest_package_scope(
        &self,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<PackageId>, ResolveError> {
        if let Some(package_id) = self.cached_package_scope(path) {
            return Ok(package_id);
        }

        // start from the querying path, but lift to a directory if needed
        let lookup_path = path.to_path_buf();
        let mut visited_paths = vec![lookup_path.clone()];
        let mut current = path.to_path_buf();

        while !self.is_directory(&current, ctx) {
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
                visited_paths.push(current.clone());
            } else {
                break;
            }
        }

        // walk parents until a package manifest is found
        let mut current = Some(current);
        while let Some(dir) = current {
            if !visited_paths.contains(&dir) {
                visited_paths.push(dir.clone());
            }

            if let Some(package_id) =
                self.read_package_manifest(&dir, ctx, CachePolicy::UseCache)?
            {
                for visited_path in visited_paths {
                    self.cache_package_scope(&visited_path, Some(package_id));
                }
                return Ok(Some(package_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        // cache the miss for every visited path
        for visited_path in visited_paths {
            self.cache_package_scope(&visited_path, None);
        }
        Ok(None)
    }
}
