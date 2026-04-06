use std::borrow::Cow;
use std::path::Path;

use destack_source::PathExt;

use crate::{CachePolicy, Resolution, ResolveError, ResolveFrame, Resolver};

impl Resolver {
    /// Resolve one directory through package entry fields before falling back to index probing.
    pub(crate) fn resolve_package_directory(
        &self,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // load the package manifest for this directory
        let Some(package_id) = self.read_package_manifest(path, ctx, CachePolicy::UseCache)? else {
            return Ok(None);
        };
        let package = self.packages.get(package_id);
        let package = package.read().unwrap();
        let Some(declaration) = package.package_declaration.as_ref() else {
            return Ok(None);
        };

        // try the legacy main field first
        if let Some(main_field) = declaration.json.main.as_deref()
            && let Some(resolved) = self.resolve_package_main_field(path, main_field, ctx)?
        {
            return Ok(Some(resolved));
        }

        // then fall back to the root exports target
        if let Some(exports) = declaration.json.exports.as_ref()
            && let Some(target) = self.package_exports_resolve(path, ".", exports, ctx)?
            && let Some(resolved) = self.finalize_package_target(".", target, ctx)?
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
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
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
        if let Some(resolved) = self.probe_file(&main_path, ctx)? {
            return Ok(Some(resolved));
        }

        self.probe_directory_index(&main_path, ctx)
    }
}
