use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::{Component, Path, PathBuf};

use destack_source::{File, FileType, PackageId, PathExt, Uri};
use destack_workspace::{ModuleSpecifier, Package, PackageConfig, PackageKind};

use crate::{ResolveContext, ResolveError, Resolver};

/// Check if a path is an invalid exports target.
fn is_path_invalid_exports_target(path: &Path) -> bool {
    path.components().enumerate().any(|(index, c)| match c {
        Component::ParentDir => true,
        Component::CurDir => index > 0,
        Component::Normal(c) => c.eq_ignore_ascii_case("node_modules"),
        _ => false,
    })
}

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Load a package.json from a directory, registering it in the program.
    #[tracing::instrument(name = "resolver.load.package", level = "trace", skip(self, ctx))]
    pub(crate) fn load_package(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PackageId>, ResolveError> {
        tracing::trace!(?path, "resolver.load.package");
        let package_json_path = path.join("package.json");

        // check if already in registry (indexed by directory path, not package.json path)
        if let Some(package_id) = self.program.packages.get_id_by_path(path) {
            let package = self.program.packages.get(package_id);
            let package = package.read();
            if let Some(ref config) = package.package_config {
                ctx.track_found_dependency(&config.path);
            }
            return Ok(Some(package_id));
        }

        // read file
        let bytes = match self.fs().read(&package_json_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                ctx.track_missing_dependency(&package_json_path);
                return Ok(None);
            }
        };

        // create package file
        let file_id = self.program.files.next_id();
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
        self.program.files.insert(file);
        let file = self.program.files.get(file_id);

        // parse `package.json` from file
        let package_config =
            PackageConfig::parse(&file, package_json_path.clone()).map_err(|_| {
                ResolveError::InvalidPackageJson {
                    path: package_json_path.clone(),
                }
            })?;

        // load dsconfig.json if it exists (optional)
        let dsconfig = self.load_package_dsconfig(&package_config).ok();

        // insert package
        let package_id = PackageId::from_path(&package_config.directory);
        let package = Package {
            id: package_id,
            kind: PackageKind::Physical,
            uri: package_config.uri.clone(),
            path: Some(package_config.directory.clone()),
            name: package_config.content.name.clone(),
            version: package_config.content.version.clone(),
            package_config: Some(package_config),
            dsconfig,
            main_tsconfig_id: None,
            targets: Default::default(),
        };
        self.program.packages.insert(package);

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
            if let Some(package_id) = self.load_package(&dir, ctx)? {
                return Ok(Some(package_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }

    /// Try to resolve from the package itself (self-reference) or node_modules.
    pub(crate) fn load_package_self_or_modules(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.is_fully_specified = false;
        }

        // try to load from the package itself (also checks browser field)
        if let Some(resolved) = self.load_package_self(path, specifier, ctx)? {
            return Ok(resolved);
        }

        // try to load from node_modules
        if let Some(resolved) = self.load_modules(path, specifier, package_name, subpath, ctx)? {
            return Ok(resolved);
        }

        // abnormal relative specifier like `jest-runner-../../..`
        if specifier.contains("/../..") || specifier.contains("../../") {
            let normalized_path = Path::new(specifier).normalize_relative();
            let mut normalized_specifier = normalized_path.to_string_lossy().into_owned();
            if specifier.ends_with('/') {
                normalized_specifier += "/";
            }
            let normalized_specifier = normalized_specifier.as_str();

            // try to load package from modules
            let (package_name, subpath) = Self::parse_package_specifier(normalized_specifier);
            if package_name == ".."
                && let Some(resolved) =
                    self.load_modules(path, normalized_specifier, package_name, subpath, ctx)?
            {
                return Ok(resolved);
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a `#`-prefixed import specifier against package.json imports field.
    #[tracing::instrument(
        name = "resolver.load.package.imports",
        level = "trace",
        skip(self, ctx)
    )]
    pub(crate) fn load_package_imports(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.load.package.imports");
        // find the closest package scope to the directory
        let Some(package_id) = self.find_package_json(path, ctx)? else {
            return Ok(None);
        };
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // check if the package has imports
        if let Some(ref config) = package.package_config
            && let Some(resolved) = self.package_imports_resolve(specifier, config, ctx)?
        {
            return self.resolve_esm_match(specifier, &resolved, ctx);
        }
        Ok(None)
    }

    /// Search node_modules directories walking up from the given path.
    #[tracing::instrument(name = "resolver.load.modules", level = "trace", skip(self, ctx))]
    pub(crate) fn load_modules(
        &self,
        path: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(
            ?path,
            ?specifier,
            ?package_name,
            ?subpath,
            "resolver.load.modules"
        );
        // check each module directory (node_modules)
        for module_name in &self.options.modules {
            // walk up parent directories
            let mut current = Some(path.to_path_buf());
            while let Some(current_path) = current {
                // skip if /path/to/node_modules does not exist
                if !self.is_directory(&current_path, ctx) {
                    current = current_path.parent().map(|p| p.to_path_buf());
                    continue;
                }

                // get the module directory
                let Some(module_dir) = self.get_module_directory(&current_path, module_name, ctx)
                else {
                    current = current_path.parent().map(|p| p.to_path_buf());
                    continue;
                };

                // optimize node_modules lookup by inspecting whether the package exists
                if !package_name.is_empty() {
                    let package_path = module_dir.normalize_with(package_name);

                    // try <foo>/node_modules/package_name
                    if self.is_directory(&package_path, ctx) {
                        if let Some(resolved) =
                            self.load_package_exports(specifier, subpath, &package_path, ctx)?
                        {
                            return Ok(Some(resolved));
                        }
                    }
                    // package_name is not a directory, skip unless we're looking for scope
                    else {
                        if !subpath.is_empty() {
                            current = current_path.parent().map(|p| p.to_path_buf());
                            continue;
                        }
                        // skip if the scope directory doesn't exist
                        if package_name.starts_with('@')
                            && let Some(parent) = package_path.parent()
                            && !self.is_directory(parent, ctx)
                        {
                            current = current_path.parent().map(|p| p.to_path_buf());
                            continue;
                        }
                    }
                }

                // try as file or directory for all other cases
                let resolved_path = module_dir.normalize_with(specifier);

                // prefer directory
                if self.options.resolve_to_context {
                    return Ok(self
                        .is_directory(&resolved_path, ctx)
                        .then(|| resolved_path.to_path_buf()));
                }

                // load directory
                if self.is_directory(&resolved_path, ctx) {
                    if let Some(resolved) = self.load_browser_field_or_alias(&resolved_path, ctx)? {
                        return Ok(Some(resolved));
                    }
                    if let Some(resolved) = self.load_directory(&resolved_path, ctx)? {
                        return Ok(Some(resolved));
                    }
                }
                // load file
                else if let Some(resolved) = self.load_file(&resolved_path, ctx)? {
                    return Ok(Some(resolved));
                }

                current = current_path.parent().map(|p| p.to_path_buf());
            }
        }
        Ok(None)
    }

    /// Get a subdirectory of the given path if it exists.
    fn get_module_directory(
        &self,
        path: &Path,
        module_name: &str,
        ctx: &mut ResolveContext,
    ) -> Option<PathBuf> {
        // check if already in the module directory
        if path
            .components()
            .next_back()
            .is_some_and(|c| c.as_os_str() == module_name)
        {
            return Some(path.to_path_buf());
        }

        // check subdirectory
        let subdir = path.join(module_name);
        if self.is_directory(&subdir, ctx) {
            Some(subdir)
        } else {
            None
        }
    }

    /// Try to resolve a specifier via package.json exports field.
    #[tracing::instrument(
        name = "resolver.load.package.exports",
        level = "trace",
        skip(self, ctx)
    )]
    pub(crate) fn load_package_exports(
        &self,
        specifier: &str,
        subpath: &str,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?specifier, ?subpath, ?path, "resolver.load.package.exports");
        // check if package.json exists
        let Some(package_id) = self.load_package(path, ctx)? else {
            return Ok(None);
        };
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // resolve exports
        if let Some(ref config) = package.package_config
            && let Some(exports) = config.content.exports.as_ref()
            && let Some(resolved) =
                self.package_exports_resolve(path, &format!(".{subpath}"), exports, ctx)?
        {
            return self.resolve_esm_match(specifier, &resolved, ctx);
        }

        Ok(None)
    }

    /// Try to resolve a self-reference (package importing itself).
    #[tracing::instrument(name = "resolver.load.package.self", level = "trace", skip(self, ctx))]
    pub(crate) fn load_package_self(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?specifier, ?path, "resolver.load.package.self");
        // find the closest package scope to the directory
        let Some(package_id) = self.find_package_json(path, ctx)? else {
            return Ok(None);
        };
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // check if the package has config
        let Some(ref config) = package.package_config else {
            return Ok(None);
        };

        // check if the package name matches the specifier (self-reference)
        if let Some(subpath) = config
            .content
            .name
            .as_ref()
            .and_then(|package_name: &String| {
                Self::strip_package_name(specifier, package_name.as_str())
            })
        {
            let package_url = config
                .path
                .parent()
                .unwrap_or_else(|| {
                    panic!(
                        "package.json path is not in a directory: {}",
                        config.path.display()
                    )
                })
                .to_path_buf();

            if let Some(exports) = config.content.exports.as_ref()
                && let Some(resolved) = self.package_exports_resolve(
                    &package_url,
                    &format!(".{subpath}"),
                    exports,
                    ctx,
                )?
            {
                return self.resolve_esm_match(specifier, &resolved, ctx);
            }
        }

        // fallback to browser field
        self.load_browser_field(path, Some(specifier), config, ctx)
    }

    /// Resolve an ESM match by loading as file or directory.
    pub(crate) fn resolve_esm_match(
        &self,
        specifier: &str,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // non-compliant ESM can result in a directory, so directory is tried as well
        if let Some(resolved) = self.load_file_or_directory(path, "", ctx)? {
            Ok(Some(resolved))
        } else {
            Err(ResolveError::NotFound {
                specifier: specifier.to_string(),
            })
        }
    }

    /// Resolve a bare package specifier by searching modules directories.
    #[tracing::instrument(name = "resolver.package.resolve", level = "trace", skip(self, ctx))]
    pub(crate) fn package_resolve(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?specifier, ?path, "resolver.package.resolve");
        let (package_name, subpath) = Self::parse_package_specifier(specifier);

        // iterate over all possible modules directories
        for module_name in &self.options.modules {
            // walk up parent directories
            let mut current = Some(path.to_path_buf());
            while let Some(current_path) = current {
                // check if the module directory exists
                let Some(module_dir) = self.get_module_directory(&current_path, module_name, ctx)
                else {
                    current = current_path.parent().map(|p| p.to_path_buf());
                    continue;
                };

                // check if the package exists in the module directory
                let package_path = module_dir.normalize_with(package_name);
                if self.is_directory(&package_path, ctx) {
                    // load `package.json`
                    if let Some(package_id) = self.load_package(&package_path, ctx)? {
                        let package = self.program.packages.get(package_id);
                        let package = package.read();

                        if let Some(config) = &package.package_config {
                            // resolve exports
                            if let Some(exports) = config.content.exports.as_ref()
                                && let Some(resolved) = self.package_exports_resolve(
                                    &package_path,
                                    &format!(".{subpath}"),
                                    exports,
                                    ctx,
                                )?
                            {
                                return Ok(Some(resolved));
                            }

                            // resolve main field
                            if subpath == "."
                                && let Some(main_field) = config.content.main.as_deref()
                            {
                                let main_path = package_path.normalize_with(main_field);
                                if self.is_file(&main_path, ctx)
                                    && self.check_restrictions(&main_path)
                                {
                                    return Ok(Some(main_path));
                                }
                            }
                        }
                    }

                    // resolve subpath
                    let subpath_spec = format!(".{subpath}");
                    ctx.is_fully_specified = false;
                    return self.require(&package_path, &subpath_spec, ctx).map(Some);
                }
                current = current_path.parent().map(|p| p.to_path_buf());
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a subpath against a package's exports field.
    pub(crate) fn package_exports_resolve(
        &self,
        package_url: &Path,
        subpath: &str,
        exports: &serde_json::Value,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        let conditions = &self.options.conditions;

        // validate exports (cannot mix keys starting with "." and not)
        if let Some(map) = exports.as_object() {
            let mut has_dot = false;
            let mut without_dot = false;
            for key in map.keys() {
                let starts_with_dot_or_hash = key.starts_with(['.', '#']);
                has_dot = has_dot || starts_with_dot_or_hash;
                without_dot = without_dot || !starts_with_dot_or_hash;
                if has_dot && without_dot {
                    return Err(ResolveError::InvalidPackageJson {
                        path: package_url.join("package.json"),
                    });
                }
            }
        }

        // resolve root export
        if subpath == "." {
            let main_export = match exports {
                serde_json::Value::String(_) | serde_json::Value::Array(_) => {
                    Some(Cow::Borrowed(exports))
                }
                serde_json::Value::Object(map) => map.get(".").map_or_else(
                    || {
                        if map
                            .keys()
                            .any(|key| key.starts_with("./") || key.starts_with('#'))
                        {
                            None
                        } else {
                            Some(Cow::Borrowed(exports))
                        }
                    },
                    |entry| Some(Cow::Borrowed(entry)),
                ),
                _ => None,
            };
            if let Some(main_export) = main_export {
                let resolved = self.package_target_resolve(
                    package_url,
                    ".",
                    main_export.as_ref(),
                    None,
                    false,
                    conditions,
                    ctx,
                )?;
                if let Some(path) = resolved {
                    return Ok(Some(path));
                }
            }
        }

        // resolve subpath export
        if let Some(exports) = exports.as_object()
            && let Some(resolved) =
                self.package_match_resolve(subpath, exports, package_url, false, conditions, ctx)?
        {
            return Ok(Some(resolved));
        }

        // package path not exported
        Err(ResolveError::PackagePathNotExported {
            subpath: subpath.to_string(),
            package_path: package_url.to_path_buf(),
            package_json_path: package_url.join("package.json"),
            conditions: self.options.conditions.clone(),
        })
    }

    /// Resolve an imports specifier against package.json imports field.
    fn package_imports_resolve(
        &self,
        specifier: &str,
        package_config: &PackageConfig,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        debug_assert!(specifier.starts_with('#'), "{specifier}");

        // bail if no imports are configured
        let Some(imports) = package_config.content.imports.as_ref() else {
            return Ok(None);
        };

        // error if specifier is invalid
        if specifier == "#" || specifier.starts_with("#/") {
            return Err(ResolveError::InvalidModuleSpecifier {
                specifier: specifier.to_string(),
                package_path: package_config.path.to_path_buf(),
            });
        }

        // resolve imports
        if let Some(resolved) = self.package_match_resolve(
            specifier,
            imports,
            &package_config.directory,
            true,
            &self.options.conditions,
            ctx,
        )? {
            Ok(Some(resolved))
        } else {
            Err(ResolveError::PackageImportNotDefined {
                specifier: specifier.to_string(),
                package_path: package_config.path.to_path_buf(),
            })
        }
    }

    /// Resolve a key against an imports or exports mapping object.
    pub(crate) fn package_match_resolve(
        &self,
        match_key: &str,
        match_obj: &serde_json::Map<String, serde_json::Value>,
        package_url: &Path,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if match_key.ends_with('/') {
            return Ok(None);
        }

        // direct match
        if !match_key.contains('*')
            && let Some(target) = match_obj.get(match_key)
        {
            return self.package_target_resolve(
                package_url,
                match_key,
                target,
                None,
                is_imports,
                conditions,
                ctx,
            );
        }

        // find the best matching pattern key
        let mut best_target = None;
        let mut best_match = "";
        let mut best_key = "";
        for (source_key, target_key) in match_obj.iter() {
            // ignore invalid mappings
            if source_key.ends_with('*') && target_key.as_str().is_some_and(|s| !s.contains('*')) {
                // (can't have asterisk in source key but not in target)
                continue;
            }

            if source_key.starts_with("./") || source_key.starts_with('#') {
                // wildcard pattern match
                if let Some((pattern_base, pattern_trailer)) = source_key.split_once('*') {
                    if match_key.starts_with(pattern_base)
                        && !pattern_trailer.contains('*')
                        && (pattern_trailer.is_empty()
                            || (match_key.len() >= source_key.len()
                                && match_key.ends_with(pattern_trailer)))
                        && Self::pattern_key_compare(best_key, source_key).is_gt()
                    {
                        best_target = Some(target_key);
                        best_match =
                            &match_key[pattern_base.len()..match_key.len() - pattern_trailer.len()];
                        best_key = source_key;
                    }
                }
                // directory pattern match
                else if source_key.ends_with('/')
                    && match_key.starts_with(source_key)
                    && Self::pattern_key_compare(best_key, source_key).is_gt()
                {
                    best_target = Some(target_key);
                    best_match = &match_key[source_key.len()..];
                    best_key = source_key;
                }
            }
        }

        // resolve the best matching key
        if let Some(best_target) = best_target {
            return self.package_target_resolve(
                package_url,
                best_key,
                best_target,
                Some(best_match),
                is_imports,
                conditions,
                ctx,
            );
        }

        Ok(None)
    }

    /// Resolve a package target value (string, object, or array) to a path.
    fn package_target_resolve(
        &self,
        package_url: &Path,
        target_key: &str,
        target: &serde_json::Value,
        pattern_match: Option<&str>,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        /// Normalize a string target by substituting pattern match.
        fn normalize_string_target<'a>(
            target_key: &'a str,
            target: &'a str,
            pattern_match: Option<&'a str>,
            package_url: &Path,
        ) -> Result<Cow<'a, str>, ResolveError> {
            if let Some(pattern_match) = pattern_match {
                if !target_key.contains('*') && !target.contains('*') {
                    // enhanced-resolve behaviour: trailing slash patterns
                    if target_key.ends_with('/') && target.ends_with('/') {
                        Ok(Cow::Owned(format!("{target}{pattern_match}")))
                    } else {
                        Err(ResolveError::InvalidPackageConfigDirectory {
                            path: package_url.join("package.json"),
                        })
                    }
                } else {
                    Ok(Cow::Owned(target.replace('*', pattern_match)))
                }
            } else {
                Ok(Cow::Borrowed(target))
            }
        }

        // resolve string target
        if let Some(target) = target.as_str() {
            // parse target
            let parsed = ModuleSpecifier::parse(target);
            if let Some(query) = &parsed.query {
                ctx.query.replace(query.to_string());
            }
            if let Some(fragment) = &parsed.fragment {
                ctx.fragment.replace(fragment.to_string());
            }
            let target = parsed.path();

            // target does not start with `./`
            if !target.starts_with("./") {
                // error if target is invalid for exports
                if !is_imports || target.starts_with("../") || target.starts_with('/') {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: (*target).to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.join("package.json"),
                    });
                }
                // normalize and resolve as package specifier
                let target =
                    normalize_string_target(target_key, target, pattern_match, package_url)?;
                return self.package_resolve(package_url, &target, ctx);
            }
            // target starts with `./`
            else {
                let target =
                    normalize_string_target(target_key, target, pattern_match, package_url)?;
                if is_path_invalid_exports_target(Path::new(target.as_ref())) {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: target.to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.join("package.json"),
                    });
                }
                return Ok(Some(package_url.normalize_with(target.as_ref())));
            }
        }
        // resolve object target (conditions)
        else if let Some(target) = target.as_object() {
            for (key, target_value) in target.iter() {
                if key == "default" || conditions.iter().any(|condition| condition == key) {
                    let resolved = self.package_target_resolve(
                        package_url,
                        target_key,
                        target_value,
                        pattern_match,
                        is_imports,
                        conditions,
                        ctx,
                    );
                    if let Some(path) = resolved? {
                        return Ok(Some(path));
                    }
                }
            }
            return Ok(None);
        }
        // resolve array target (fallback)
        else if let Some(targets) = target.as_array() {
            if targets.is_empty() {
                return Err(ResolveError::PackagePathNotExported {
                    subpath: pattern_match.unwrap_or(".").to_string(),
                    package_path: package_url.to_path_buf(),
                    package_json_path: package_url.join("package.json"),
                    conditions: self.options.conditions.clone(),
                });
            }
            for (i, target_value) in targets.iter().enumerate() {
                let resolved = self.package_target_resolve(
                    package_url,
                    target_key,
                    target_value,
                    pattern_match,
                    is_imports,
                    conditions,
                    ctx,
                );
                if let Ok(Some(path)) = resolved {
                    return Ok(Some(path));
                } else if resolved.is_err() && i == targets.len() {
                    return resolved;
                }
            }
        }

        Ok(None)
    }

    /// Parse a package specifier into package name and subpath.
    pub(crate) fn parse_package_specifier(specifier: &str) -> (&str, &str) {
        // find first slash
        let mut separator_index = specifier.as_bytes().iter().position(|b| *b == b'/');

        // scoped packages have format `@scope/package-name/subpath`
        if specifier.starts_with('@') {
            if separator_index.is_none() || specifier.is_empty() {
                // fall through with no separator
            } else if let Some(first_slash) = separator_index {
                separator_index = specifier.as_bytes()[first_slash + 1..]
                    .iter()
                    .position(|b| *b == b'/')
                    .map(|offset| offset + first_slash + 1);
            }
        }

        // split at separator
        let package_name = separator_index.map_or(specifier, |index| &specifier[..index]);
        let package_subpath = separator_index.map_or("", |index| &specifier[index..]);
        (package_name, package_subpath)
    }

    /// Compare two pattern keys for specificity ordering.
    fn pattern_key_compare(key_a: &str, key_b: &str) -> Ordering {
        if key_a.is_empty() {
            return Ordering::Greater;
        }

        // ensure pattern keys are actual pattern keys
        debug_assert!(
            key_a.ends_with('/') || key_a.match_indices('*').count() == 1,
            "{key_a}"
        );
        debug_assert!(
            key_b.ends_with('/') || key_b.match_indices('*').count() == 1,
            "{key_b}"
        );

        // compare pattern keys
        let a_pos = key_a.bytes().position(|c| c == b'*');
        let base_length_a = a_pos.map_or(key_a.len(), |p| p + 1);
        let b_pos = key_b.bytes().position(|c| c == b'*');
        let base_length_b = b_pos.map_or(key_b.len(), |p| p + 1);
        if base_length_a > base_length_b {
            Ordering::Less
        } else if base_length_b > base_length_a || a_pos.is_none() {
            Ordering::Greater
        } else if b_pos.is_none() || key_a.len() > key_b.len() {
            Ordering::Less
        } else if key_b.len() > key_a.len() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}
