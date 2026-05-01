use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::sync::Arc;

use destack_source::{File, FileId, FileType, PackageId, PathExt, Uri};
use destack_workspace::{DestackDeclaration, Package, PackageDeclaration, PackageKind};

use crate::{
    CachePolicy, PackageScope, Resolution, Resolver, ResolverContext, ResolverError,
    ResolverResult, ResolverSearch,
};

impl Resolver {
    /// Read one package manifest from a directory.
    pub(crate) fn read_package(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
        cache_policy: CachePolicy,
    ) -> ResolverResult<Option<PackageId>> {
        // prefer revision backed package truth inside the workspace
        if let Some(package_id) = self.find_repository_package_scope(path, ctx)? {
            return Ok(Some(package_id));
        }

        let package_json_path = path.join("package.json");
        let destack_config_path = path.join("destack.json");

        // reuse the cached package entry when possible
        if cache_policy.use_cache()
            && let Some(package_id) = ctx.package_id_by_path(path)
            && let Some(package) = ctx.package(package_id)
        {
            if let Some(ref declaration) = package.package_declaration {
                self.track_file_dependency(&declaration.path, ctx)?;
            }
            if package.package.destack_file_id.is_some() {
                self.track_file_dependency(&destack_config_path, ctx)?;
            }
            return Ok(Some(package_id));
        }

        // read destack config when present so destack only packages still form package scopes
        let destack_config = match self.read_destack(path, ctx, cache_policy) {
            Ok(config) => Some(config),
            Err(ResolverError::DestackNotFound { .. }) => {
                let _metadata = self.path_metadata(&destack_config_path, ctx)?;
                None
            }
            Err(error) => return Err(error),
        };

        // read package.json when present
        let package_declaration = match self.read_path(&package_json_path, ctx) {
            Ok(bytes) => {
                // materialize the declaration with a stable file id
                let file_id = FileId::from_logical_path(&package_json_path);
                let (name, uri) = Uri::from_path_with_name(&package_json_path);
                let content =
                    String::from_utf8(bytes).map_err(|_| ResolverError::InvalidPackageJson {
                        path: package_json_path.clone(),
                    })?;
                let file = File::from_text(
                    file_id,
                    name,
                    uri,
                    Some(package_json_path.clone()),
                    FileType::Json,
                    content,
                );
                let file = Arc::new(file);

                PackageDeclaration::parse(&file).map_err(|_| ResolverError::InvalidPackageJson {
                    path: package_json_path.clone(),
                })?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let _metadata = self.path_metadata(&package_json_path, ctx)?;

                if destack_config.is_none() {
                    return Ok(None);
                }

                // no package declaration
                // destack only packages still exist semantically, but package.json features stay absent
                return self.insert_package_scope(ctx, None, destack_config);
            }
            Err(error) => {
                return Err(ResolverError::IoError {
                    path: package_json_path.clone(),
                    kind: error.kind(),
                });
            }
        };

        self.insert_package_scope(ctx, Some(package_declaration), destack_config)
    }

    /// Find the nearest package.json by traversing parent directories.
    pub(crate) fn find_package_scope(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<PackageId>> {
        if let Some(package_id) = ctx.package_scope(path) {
            return Ok(package_id);
        }

        // prefer repository backed package discovery within the active workspace
        if let Some(package_id) = self.find_repository_package_scope(path, ctx)? {
            ctx.cache_package_scope(path, Some(package_id));
            return Ok(Some(package_id));
        }

        // start from the querying path, but lift to a directory if needed
        let mut visited_paths = vec![path.to_path_buf()];
        let mut current_directory = if self.is_directory(path, ctx)? {
            path.to_path_buf()
        } else {
            path.parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| ResolverError::ExpectedFilePath {
                    path: path.to_path_buf(),
                })?
        };
        if visited_paths.last() != Some(&current_directory) {
            visited_paths.push(current_directory.clone());
        }

        // walk parents until a package manifest is found
        loop {
            if let Some(package_id) =
                self.read_package(&current_directory, ctx, CachePolicy::UseCache)?
            {
                for visited_path in visited_paths {
                    ctx.cache_package_scope(&visited_path, Some(package_id));
                }
                return Ok(Some(package_id));
            }

            let Some(parent) = current_directory.parent() else {
                break;
            };

            current_directory = parent.to_path_buf();
            visited_paths.push(current_directory.clone());
        }

        // cache the miss for every visited path
        for visited_path in visited_paths {
            ctx.cache_package_scope(&visited_path, None);
        }
        Ok(None)
    }

    /// Resolve one directory through package entry fields before falling back to index probing.
    pub(crate) fn resolve_package_directory(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // load the package manifest for this directory
        let Some(package_id) = self.read_package(path, ctx, CachePolicy::UseCache)? else {
            return Ok(None);
        };
        let Some(package) = ctx.package(package_id) else {
            return Ok(None);
        };
        let Some(declaration) = package.package_declaration.as_ref() else {
            return Ok(None);
        };

        // try the legacy main field first
        if let Some(main_field) = declaration.manifest.main.as_deref()
            && let Some(resolved) =
                self.resolve_package_main_field(path, main_field, search.clone(), ctx)?
        {
            return Ok(Some(resolved));
        }

        // then fall back to the root exports target
        if let Some(exports) = declaration.manifest.exports.as_ref()
            && let Some(target) =
                self.resolve_package_exports_field(path, ".", exports, search.clone(), ctx)?
            && let Some(resolved) = self.finalize_package_target(".", target, search, ctx)?
        {
            return Ok(Some(resolved));
        }

        Ok(None)
    }

    /// Resolve one package `main` field from a package directory.
    fn resolve_package_main_field(
        &self,
        package_path: &Path,
        main_field: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // normalize bare main entries into relative package paths
        let main_field: Cow<'_, str> =
            if main_field.starts_with("./") || main_field.starts_with("../") {
                Cow::Borrowed(main_field)
            } else {
                Cow::Owned(format!("./{main_field}"))
            };

        // build the concrete main path once
        let main_path = package_path.normalize_with(main_field.as_ref());

        // prefer a direct file target before index probing
        if let Some(resolved) = self.probe_file(&main_path, search.clone(), ctx)? {
            return Ok(Some(resolved));
        }

        // explicit file targets should fall back to the package directory
        if Path::new(main_field.as_ref()).extension().is_some() {
            return Ok(None);
        }

        self.probe_directory_index(&main_path, search, ctx)
    }
}

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Return one repository backed package scope when the current query is revision aware.
    fn find_repository_package_scope(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<PackageId>> {
        let (repository, revision) = self.repository_revision(ctx);

        if !path.starts_with(repository.workspace_root()) {
            return Ok(None);
        }

        let package = repository
            .nearest_package(revision, path)
            .map_err(|error| ResolverError::RepositoryError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let Some(package) = package else {
            return Ok(None);
        };
        let package_declaration = repository
            .package_declaration_for_package(revision, package.as_ref())
            .map_err(|error| ResolverError::RepositoryError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?
            .map(|declaration| declaration.as_ref().clone());
        let destack_declaration = repository
            .destack_declaration_for_package(revision, package.as_ref())
            .map_err(|error| ResolverError::RepositoryError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?
            .map(|declaration| declaration.as_ref().clone());

        if let Some(declaration) = package_declaration.as_ref() {
            self.track_file_dependency(&declaration.path, ctx)?;
        }
        if let Some(declaration) = destack_declaration.as_ref() {
            self.track_file_dependency(&declaration.path, ctx)?;
        }

        let package_id = self.insert_package(ctx, package.as_ref().clone(), package_declaration)?;

        if package_id.is_some()
            && destack_declaration.is_none()
            && let Some(package_path) = package.path.as_ref()
        {
            let destack_path = package_path.join("destack.json");
            let _metadata = self.path_metadata(&destack_path, ctx)?;
        }

        Ok(package_id)
    }

    /// Insert or refresh one request local package entry.
    fn insert_package_scope(
        &self,
        ctx: &mut ResolverContext,
        package_declaration: Option<PackageDeclaration>,
        destack_declaration: Option<DestackDeclaration>,
    ) -> ResolverResult<Option<PackageId>> {
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

        let package = Package {
            id: PackageId::from_path(&package_directory),
            kind: PackageKind::Declared,
            uri: package_declaration
                .as_ref()
                .map(|declaration| declaration.uri.clone())
                .unwrap_or_else(|| Uri::from_path(&package_directory)),
            path: Some(package_directory.clone()),
            name: package_declaration
                .as_ref()
                .and_then(|declaration| declaration.name().map(ToOwned::to_owned)),
            version: package_declaration
                .as_ref()
                .and_then(|declaration| declaration.version().map(ToOwned::to_owned)),
            package_file_id: package_declaration
                .as_ref()
                .map(|declaration| declaration.file_id),
            destack_file_id: destack_declaration
                .as_ref()
                .map(|declaration| declaration.file_id),
            tsconfig_file_id: None,
            targets: Default::default(),
        };

        self.insert_package(ctx, package, package_declaration)
    }

    /// Insert one request local package entry.
    fn insert_package(
        &self,
        ctx: &mut ResolverContext,
        package: Package,
        package_declaration: Option<PackageDeclaration>,
    ) -> ResolverResult<Option<PackageId>> {
        let package_id = package.id;
        let entry = PackageScope {
            package,
            package_declaration,
        };

        ctx.cache_package(entry);

        Ok(Some(package_id))
    }
}
