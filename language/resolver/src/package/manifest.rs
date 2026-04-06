use std::path::Path;

use destack_source::{File, FileId, FileType, PackageId, Uri};
use destack_workspace::{Package, PackageDeclaration, PackageKind};

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
            let package = package.read().unwrap();
            if let Some(ref declaration) = package.package_declaration {
                ctx.track_found_dependency(&declaration.path);
            }
            if let Some(ref declaration) = package.destack_declaration {
                ctx.track_found_dependency(&declaration.path);
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

        // read package.json when present
        let package_declaration = match self.read_path(&package_json_path) {
            Ok(bytes) => {
                // materialize the declaration as a tracked file entry
                let file_id = self
                    .files
                    .get_id_by_path(&package_json_path)
                    .unwrap_or_else(|| FileId::from_logical_path(&package_json_path));
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

                // align declaration realpaths with path canonicalization
                let package_json_realpath = if self.options.canonicalize_symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };

                ctx.track_found_dependency(&package_json_path);
                PackageDeclaration::parse(&file, package_json_realpath).map_err(|_| {
                    ResolveError::InvalidPackageJson {
                        path: package_json_path.clone(),
                    }
                })?
            }
            Err(_) => {
                ctx.track_missing_dependency(&package_json_path);

                if destack_config.is_none() {
                    return Ok(None);
                }

                // no package declaration
                // destack only packages still exist semantically, but package.json features stay absent
                return self.insert_package_entry(path, None, destack_config);
            }
        };

        self.insert_package_entry(path, Some(package_declaration), destack_config)
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

impl Resolver {
    /// Insert or refresh one cached package entry.
    fn insert_package_entry(
        &self,
        path: &Path,
        package_declaration: Option<PackageDeclaration>,
        destack_declaration: Option<destack_workspace::DestackDeclaration>,
    ) -> Result<Option<PackageId>, ResolveError> {
        let package_directory = package_declaration
            .as_ref()
            .map(|declaration| declaration.directory.clone())
            .or_else(|| {
                destack_declaration
                    .as_ref()
                    .map(|declaration| declaration.directory.clone())
            });
        let Some(package_directory) = package_directory else {
            return Ok(None);
        };

        let package_path = Some(package_directory.clone());
        let package_id = PackageId::from_path(&package_directory);
        let package_uri = package_declaration
            .as_ref()
            .map(|declaration| declaration.uri.clone())
            .unwrap_or_else(|| Uri::from_path(&package_directory));
        let package_options = destack_declaration
            .as_ref()
            .map(destack_workspace::DestackDeclaration::package_options);
        let package_name = package_options
            .as_ref()
            .and_then(|options| options.name.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.name().map(ToOwned::to_owned))
            });
        let package_version = package_options
            .as_ref()
            .and_then(|options| options.version.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.version().map(ToOwned::to_owned))
            });

        let entry = crate::ResolverPackageEntry {
            package: Package {
                id: package_id,
                kind: PackageKind::Physical,
                uri: package_uri,
                path: package_path,
                name: package_name,
                version: package_version,
                package_file_id: package_declaration
                    .as_ref()
                    .map(|declaration| declaration.file_id),
                destack_file_id: destack_declaration
                    .as_ref()
                    .map(|declaration| declaration.file_id),
                tsconfig_file_id: None,
                targets: Default::default(),
            },
            package_declaration,
            destack_declaration,
            package_options,
        };

        let _ = path;
        self.packages.insert(entry);

        Ok(Some(package_id))
    }
}
