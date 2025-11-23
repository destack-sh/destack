use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::{fmt, iter};

use dyst_dir::{ModuleSpecifier, PackageJson, TsConfigJson, TsProjectReferences};
use dyst_source::{FileSystem, MemoryFileSystem, PathExt, PhysicalFileSystem, SLASH_START};

use crate::{
    Alias, AliasValue, CachedFileSystem, CachedPath, Resolution, ResolutionContext, ResolveError,
    ResolveOptions, Restriction, TypeScriptOptionsDiscovery, TypeScriptOptionsReferences,
};

/// A resolver with a cache backed by a file system.
pub struct Resolver<Fs> {
    /// The resolution options.
    pub options: ResolveOptions,
    /// The cache backed by the file system.
    pub cache: Arc<CachedFileSystem<Fs>>,
}

pub type PhysicalResolver = Resolver<PhysicalFileSystem>;
pub type MemoryResolver = Resolver<MemoryFileSystem>;
pub type ResolveResult = Result<Option<CachedPath>, ResolveError>;

impl<Fs> fmt::Debug for Resolver<Fs> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

impl<Fs: FileSystem> Default for Resolver<Fs> {
    fn default() -> Self {
        Self::new(ResolveOptions::default())
    }
}

impl<Fs: FileSystem> Resolver<Fs> {
    /// Creates a new resolver with default options.
    pub fn new(options: ResolveOptions) -> Self {
        let fs = Fs::new();
        let cache = Arc::new(CachedFileSystem::new(fs));
        Self {
            options: options.sanitize(),
            cache,
        }
    }
}

impl<Fs: FileSystem> Resolver<Fs> {
    /// Creates a new resolver from a file system and options.
    pub fn from_file_system(file_system: Fs, options: ResolveOptions) -> Self {
        Self {
            cache: Arc::new(CachedFileSystem::new(file_system)),
            options: options.sanitize(),
        }
    }

    /// Clones the resolver using the same underlying cache.
    pub fn clone_with_options(&self, options: ResolveOptions) -> Self {
        Self {
            options: options.sanitize(),
            cache: Arc::clone(&self.cache),
        }
    }

    /// Resolves specifier at an absolute path to a `directory`.
    pub fn resolve<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = ResolutionContext::default();
        self.resolve_in_context(directory.as_ref(), specifier, &mut ctx)
    }

    /// Resolves a `tsconfig.json` file.
    ///
    /// The path can be:
    /// * Path to a file with `.json` extension.
    /// * Path to a file without `.json` extension, `.json` will be appended to filename.
    /// * Path to a directory, where the filename is defaulted to `tsconfig.json`
    pub fn resolve_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Arc<TsConfigJson>, ResolveError> {
        let path = path.as_ref();
        self.load_tsconfig(
            true,
            path,
            &TypeScriptOptionsReferences::Auto,
            &mut TypeScriptOptionsResolveContext::default(),
        )
    }

    /// Resolves `specifier` at absolute `path` with a custom [ResolveContext].
    pub fn resolve_with_context<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
        resolve_context: &mut ResolutionContext,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = ResolutionContext::default();
        ctx.found_dependencies.replace(vec![]);
        ctx.missing_dependencies.replace(vec![]);

        let result = self.resolve_in_context(directory.as_ref(), specifier, &mut ctx);

        // append dependencies
        if let Some(deps) = &mut ctx.found_dependencies {
            resolve_context
                .found_dependencies
                .get_or_insert_with(Vec::new)
                .append(deps);
        }
        if let Some(deps) = &mut ctx.missing_dependencies {
            resolve_context
                .missing_dependencies
                .get_or_insert_with(Vec::new)
                .append(deps);
        }

        result
    }

    /// Performs the resolution.
    fn resolve_in_context(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Resolution, ResolveError> {
        ctx.is_fully_specified = self.options.is_fully_specified;

        let cached_path = self.cache.value(path);
        let cached_path = self.require(&cached_path, specifier, ctx)?;
        let path = self.load_realpath(&cached_path)?;

        let resolution = Resolution {
            path,
            query: ctx.query.take(),
            fragment: ctx.fragment.take(),
        };
        Ok(resolution)
    }

    /// Finds the `package.json` for a resolved package.
    fn find_package_json_for_a_package(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        // if we are inside node_modules, find the nearest package.json by walking up
        if cached_path.is_inside_node_modules {
            let mut last = None;
            for cp in iter::successors(Some(cached_path.clone()), CachedPath::parent) {
                if cp.is_node_modules {
                    break;
                }
                if self.cache.is_directory(&cp, ctx)
                    && let Some(package_json) =
                        self.cache.get_package_json(&cp, &self.options, ctx)?
                {
                    last = Some(package_json);
                }
            }
            Ok(last)
        } else {
            // otherwise, just use the closest package.json
            cached_path.find_package_json(&self.options, self.cache.as_ref(), ctx)
        }
    }

    /// Requires `specifier` from a module at `path`.
    /// <https://nodejs.org/api/modules.html#all-together>
    fn require(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        ctx.check_depth()?;

        // parse query and fragment identifiers
        let parsed = ModuleSpecifier::parse(specifier);
        if let Some(query) = &parsed.query {
            ctx.query.replace(query.to_string());
        }
        if let Some(fragment) = &parsed.fragment {
            ctx.fragment.replace(fragment.to_string());
        }

        // if there is a fragment but no query, it might be part of the filename
        if ctx.fragment.is_some() && ctx.query.is_none() {
            let base_path = parsed.path();
            let fragment = ctx.fragment.take().unwrap();
            let candidate = format!("{base_path}{fragment}");
            if let Ok(path) = self.require_without_parse(cached_path, &candidate, ctx) {
                return Ok(path);
            }
            ctx.fragment.replace(fragment);
        }

        self.require_without_parse(cached_path, parsed.path(), ctx)
    }

    /// Requires `specifier` from a module at `path` without parsing.
    fn require_without_parse(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        // check tsconfig paths
        if let Some(path) =
            self.load_tsconfig_paths(cached_path, specifier, &mut ResolutionContext::default())?
        {
            return Ok(path);
        }

        // check alias
        if let Some(path) = self.load_alias(cached_path, specifier, &self.options.alias, ctx)? {
            return Ok(path);
        }

        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let specifier = resolve_file_protocol(specifier)?;
                let specifier = specifier.as_ref();
            }
        };

        let result = match Path::new(&specifier).components().next() {
            // absolute path
            Some(Component::RootDir | Component::Prefix(_)) => {
                self.require_absolute(cached_path, specifier, ctx)
            }
            // relative path
            Some(Component::CurDir | Component::ParentDir) => {
                self.require_relative(cached_path, specifier, ctx)
            }
            // internal package import
            Some(Component::Normal(_)) if specifier.as_bytes()[0] == b'#' => {
                self.require_hash(cached_path, specifier, ctx)
            }
            // bare specifier (module)
            _ => self.require_bare(cached_path, specifier, ctx),
        };

        result.or_else(|err| {
            if err.is_ignore() {
                return Err(err);
            }
            // check fallback alias
            self.load_alias(cached_path, specifier, &self.options.fallback, ctx)
                .and_then(|value| value.ok_or(err))
        })
    }

    /// Requires an absolute path.
    fn require_absolute(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        // make sure only path prefixes gets called
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
        );

        if !self.options.prefer_relative
            && self.options.prefer_absolute
            && let Ok(path) = self.load_package_self_or_node_modules(cached_path, specifier, ctx)
        {
            return Ok(path);
        }

        if let Some(path) = self.load_roots(cached_path, specifier, ctx) {
            return Ok(path);
        }

        // load as file or directory
        let path = self.cache.value(Path::new(specifier));
        if let Some(path) = self.load_as_file_or_directory(&path, specifier, ctx)? {
            return Ok(path);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Requires a relative path.
    fn require_relative(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        // make sure only relative or normal paths gets called
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(
                    c,
                    Component::CurDir | Component::ParentDir | Component::Normal(_)
                ))
        );

        let cached_path = cached_path.normalize_with(specifier, self.cache.as_ref());

        // load as file or directory
        if let Some(path) = self.load_as_file_or_directory(
            &cached_path,
            // ensure resolve directory only when specifier is `.`
            if specifier == "." { "./" } else { specifier },
            ctx,
        )? {
            return Ok(path);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Requires a hash path (imports).
    fn require_hash(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        debug_assert_eq!(specifier.chars().next(), Some('#'));

        // load package imports
        self.load_package_imports(cached_path, specifier, ctx)?
            .map_or_else(
                || {
                    Err(ResolveError::NotFound {
                        specifier: specifier.to_string(),
                    })
                },
                Ok,
            )
    }

    /// Requires a bare specifier (node_modules).
    fn require_bare(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        // make sure no other path prefixes gets called
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::Normal(_)))
        );

        if self.options.prefer_relative
            && let Ok(path) = self.require_relative(cached_path, specifier, ctx)
        {
            return Ok(path);
        }

        self.load_package_self_or_node_modules(cached_path, specifier, ctx)
    }

    /// Loads the package itself or resolves from node_modules.
    fn load_package_self_or_node_modules(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<CachedPath, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.is_fully_specified = false;
        }

        // try to load from the package itself
        if let Some(path) = self.load_package_self(cached_path, specifier, ctx)? {
            return Ok(path);
        }

        // try to load from node_modules
        if let Some(path) =
            self.load_node_modules(cached_path, specifier, package_name, subpath, ctx)?
        {
            return Ok(path);
        }

        // abnormal relative specifier like `jest-runner-../../..`
        if specifier.contains("/../..") || specifier.contains("../../") {
            let path = Path::new(specifier).normalize_relative();
            let mut owned = path.to_string_lossy().into_owned();

            if specifier.ends_with('/') {
                owned += "/";
            }

            let specifier_owned = Some(owned);
            let normalized_specifier = specifier_owned.as_deref().unwrap();

            let (package_name, subpath) = Self::parse_package_specifier(normalized_specifier);

            if package_name == ".."
                && let Some(path) = self.load_node_modules(
                    cached_path,
                    normalized_specifier,
                    package_name,
                    subpath,
                    ctx,
                )?
            {
                return Ok(path);
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Implements `LOAD_PACKAGE_IMPORTS` (X, DIR).
    fn load_package_imports(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // find the closest package scope to the directory
        let Some(package_json) =
            cached_path.find_package_json(&self.options, self.cache.as_ref(), ctx)?
        else {
            return Ok(None);
        };

        // check if the package has imports
        if let Some(path) = self.package_imports_resolve(specifier, &package_json, ctx)? {
            return self.resolve_esm_match(specifier, &path, ctx);
        }

        Ok(None)
    }

    /// Loads a path as a file.
    fn load_as_file(&self, cached_path: &CachedPath, ctx: &mut ResolutionContext) -> ResolveResult {
        // try extension alias
        if let Some(path) = self.load_extension_alias(cached_path, ctx)? {
            return Ok(Some(path));
        }

        if self.options.enforce_extension.is_disabled() {
            // if the path is a file, load it as its file extension format
            if let Some(path) = self.load_alias_or_file(cached_path, ctx)? {
                return Ok(Some(path));
            }
        }

        // try extensions (like .js, .json, .node, etc.)
        if let Some(path) = self.load_extensions(cached_path, &self.options.extensions, ctx)? {
            return Ok(Some(path));
        }

        Ok(None)
    }

    /// Loads a path as a directory.
    fn load_as_directory(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // check for package.json in the directory
        if let Some(package_json) = self
            .cache
            .get_package_json(cached_path, &self.options, ctx)?
        {
            if let Some(main_field) = package_json.main.as_deref() {
                let main_field = if main_field.starts_with("./") || main_field.starts_with("../") {
                    Cow::Borrowed(main_field)
                } else {
                    Cow::Owned(format!("./{main_field}"))
                };

                let cached_path =
                    cached_path.normalize_with(main_field.as_ref(), self.cache.as_ref());

                // try to load as file
                if let Some(path) = self.load_as_file(&cached_path, ctx)? {
                    return Ok(Some(path));
                }

                // try to load index file
                if let Some(path) = self.load_index(&cached_path, ctx)? {
                    return Ok(Some(path));
                }
            }

            // allow `exports` field in `require('../directory')`
            if let Some(exports) = package_json.exports.as_ref()
                && let Some(path) = self.package_exports_resolve(cached_path, ".", exports, ctx)?
            {
                return Ok(Some(path));
            }
        }

        // try to load index file
        self.load_index(cached_path, ctx)
    }

    /// Loads a path as either a file or a directory.
    fn load_as_file_or_directory(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if self.options.resolve_directory {
            return Ok(self
                .cache
                .is_directory(cached_path, ctx)
                .then(|| cached_path.clone()));
        }
        // file
        if !specifier.ends_with('/')
            && let Some(path) = self.load_as_file(cached_path, ctx)?
        {
            Ok(Some(path))
        }
        // directory
        else if self.cache.is_directory(cached_path, ctx)
            && let Some(path) = self.load_as_directory(cached_path, ctx)?
        {
            Ok(Some(path))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Loads extensions.
    fn load_extensions(
        &self,
        path: &CachedPath,
        extensions: &[String],
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if ctx.is_fully_specified {
            return Ok(None);
        }
        for extension in extensions {
            let cached_path = path.add_extension(extension, self.cache.as_ref());
            if let Some(path) = self.load_alias_or_file(&cached_path, ctx)? {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// Loads the real path (resolving symlinks if needed).
    fn load_realpath(&self, cached_path: &CachedPath) -> Result<PathBuf, ResolveError> {
        if self.options.canonicalize_symlinks {
            self.cache.canonicalize(cached_path)
        } else {
            Ok(cached_path.to_path_buf())
        }
    }

    /// Checks restrictions.
    fn check_restrictions(&self, path: &Path) -> bool {
        // https://github.com/webpack/enhanced-resolve/blob/a998c7d218b7a9ec2461fc4fddd1ad5dd7687485/lib/RestrictionsPlugin.js#L19-L24
        fn is_inside(path: &Path, parent: &Path) -> bool {
            if !path.starts_with(parent) {
                return false;
            }
            if path.as_os_str().len() == parent.as_os_str().len() {
                return true;
            }
            path.strip_prefix(parent)
                .is_ok_and(|p| p == Path::new("./"))
        }
        for restriction in &self.options.restrictions {
            match restriction {
                Restriction::Path(restricted_path) => {
                    if !is_inside(path, restricted_path) {
                        return false;
                    }
                }
                Restriction::Fn(f) => {
                    if !f(path) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Loads an index file.
    fn load_index(&self, cached_path: &CachedPath, ctx: &mut ResolutionContext) -> ResolveResult {
        for main_file in &self.options.main_files {
            let cached_path = cached_path.normalize_with(main_file, self.cache.as_ref());
            if self.options.enforce_extension.is_disabled()
                && let Some(path) = self.load_browser_field_or_alias(&cached_path, ctx)?
                && self.check_restrictions(path.path())
            {
                return Ok(Some(path));
            }

            // try extensions
            if let Some(path) = self.load_extensions(&cached_path, &self.options.extensions, ctx)? {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// Loads a browser field or an alias.
    fn load_browser_field_or_alias(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if let Some(package_json) =
            cached_path.find_package_json(&self.options, self.cache.as_ref(), ctx)?
            && let Some(path) = self.load_browser_field(cached_path, None, &package_json, ctx)?
        {
            return Ok(Some(path));
        }

        // try file as alias
        if !self.options.alias.is_empty() {
            let alias_specifier = cached_path.path().to_string_lossy();
            if let Some(path) =
                self.load_alias(cached_path, &alias_specifier, &self.options.alias, ctx)?
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// Loads an alias or a file.
    fn load_alias_or_file(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if let Some(path) = self.load_browser_field_or_alias(cached_path, ctx)? {
            return Ok(Some(path));
        }
        if self.cache.is_file(cached_path, ctx) && self.check_restrictions(cached_path.path()) {
            return Ok(Some(cached_path.clone()));
        }
        Ok(None)
    }

    /// Loads node modules.
    fn load_node_modules(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // check each module directory (node_modules)
        for module_name in &self.options.modules {
            for cached_path in std::iter::successors(Some(cached_path.clone()), CachedPath::parent)
            {
                // skip if /path/to/node_modules does not exist
                if !self.cache.is_directory(&cached_path, ctx) {
                    continue;
                }

                let Some(cached_path) = self.get_module_directory(&cached_path, module_name, ctx)
                else {
                    continue;
                };

                // optimize node_modules lookup by inspecting whether the package exists
                // try to interpret X as a combination of NAME and SUBPATH where the name
                // may have a @scope/ prefix and the subpath begins with a slash (`/`).
                if !package_name.is_empty() {
                    let cached_path = cached_path.normalize_with(package_name, self.cache.as_ref());
                    // try foo/node_modules/package_name
                    if self.cache.is_directory(&cached_path, ctx) {
                        // load package exports
                        if let Some(path) =
                            self.load_package_exports(specifier, subpath, &cached_path, ctx)?
                        {
                            return Ok(Some(path));
                        }
                    } else {
                        // foo/node_modules/package_name is not a directory, so useless to check inside it
                        if !subpath.is_empty() {
                            continue;
                        }
                        // skip if the directory lead to the scope package does not exist
                        // i.e. `foo/node_modules/@scope` is not a directory for `foo/node_modules/@scope/package`
                        if package_name.starts_with('@')
                            && let Some(path) = cached_path.parent().as_ref()
                            && !self.cache.is_directory(path, ctx)
                        {
                            continue;
                        }
                    }
                }

                // try as file or directory for all other cases
                let cached_path = cached_path.normalize_with(specifier, self.cache.as_ref());

                if self.options.resolve_directory {
                    return Ok(self
                        .cache
                        .is_directory(&cached_path, ctx)
                        .then(|| cached_path.clone()));
                }

                // load directory
                if self.cache.is_directory(&cached_path, ctx) {
                    if let Some(path) = self.load_browser_field_or_alias(&cached_path, ctx)? {
                        return Ok(Some(path));
                    }
                    if let Some(path) = self.load_as_directory(&cached_path, ctx)? {
                        return Ok(Some(path));
                    }
                }
                // load file
                else if let Some(path) = self.load_as_file(&cached_path, ctx)? {
                    return Ok(Some(path));
                }
            }
        }
        Ok(None)
    }

    /// Gets the module directory.
    fn get_module_directory(
        &self,
        cached_path: &CachedPath,
        module_name: &str,
        ctx: &mut ResolutionContext,
    ) -> Option<CachedPath> {
        if module_name == "node_modules" {
            cached_path.cached_node_modules(self.cache.as_ref(), ctx)
        } else if cached_path.path().components().next_back()
            == Some(Component::Normal(OsStr::new(module_name)))
        {
            Some(cached_path.clone())
        } else {
            cached_path.module_directory(module_name, self.cache.as_ref(), ctx)
        }
    }

    /// Loads package exports.
    fn load_package_exports(
        &self,
        specifier: &str,
        subpath: &str,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // check if package.json exists
        let Some(package_json) = self
            .cache
            .get_package_json(cached_path, &self.options, ctx)?
        else {
            return Ok(None);
        };

        if let Some(exports) = package_json.exports.as_ref()
            && let Some(path) =
                self.package_exports_resolve(cached_path, &format!(".{subpath}"), exports, ctx)?
        {
            return self.resolve_esm_match(specifier, &path, ctx);
        }

        Ok(None)
    }

    /// Loads the package itself.
    fn load_package_self(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // find the closest package scope to the directory
        let Some(package_json) =
            cached_path.find_package_json(&self.options, self.cache.as_ref(), ctx)?
        else {
            return Ok(None);
        };

        // check if the package name matches the specifier
        if let Some(subpath) = package_json
            .name
            .as_ref()
            .and_then(|package_name| Self::strip_package_name(specifier, package_name.as_str()))
        {
            let package_url = self.cache.value(package_json.path.parent().unwrap());
            if let Some(exports) = package_json.exports.as_ref()
                && let Some(cached_path) = self.package_exports_resolve(
                    &package_url,
                    &format!(".{subpath}"),
                    exports,
                    ctx,
                )?
            {
                return self.resolve_esm_match(specifier, &cached_path, ctx);
            }
        }

        // fallback to browser field
        self.load_browser_field(cached_path, Some(specifier), &package_json, ctx)
    }

    /// Implements `RESOLVE_ESM_MATCH` (MATCH).
    fn resolve_esm_match(
        &self,
        specifier: &str,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        // if the file at path exists, load it as its extension format
        // non-compliant ESM can result in a directory, so directory is tried as well.
        if let Some(path) = self.load_as_file_or_directory(cached_path, "", ctx)? {
            return Ok(Some(path));
        }

        // not found
        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolves the request string for this `package.json` by looking at the "browser" field.
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    fn resolve_browser_field<'a>(
        &self,
        package_json: &'a PackageJson,
        path: &Path,
        request: Option<&str>,
    ) -> Result<Option<&'a str>, ResolveError> {
        if let Some(object) = package_json.browser.as_ref().and_then(|v| v.as_object()) {
            if let Some(request) = request {
                // Find matching key in object
                if let Some(value) = object.get(request) {
                    return match value {
                        serde_json::Value::String(s) => Ok(Some(s.as_str())),
                        serde_json::Value::Bool(false) => Err(ResolveError::Ignored {
                            path: path.to_path_buf(),
                        }),
                        _ => Ok(None),
                    };
                }
            } else {
                let dir = package_json.path.parent().unwrap();
                for (key, value) in object {
                    let joined = dir.normalize_with(key.as_str());
                    if joined == path {
                        return match value {
                            serde_json::Value::String(s) => Ok(Some(s.as_str())),
                            serde_json::Value::Bool(false) => Err(ResolveError::Ignored {
                                path: path.to_path_buf(),
                            }),
                            _ => Ok(None),
                        };
                    }
                }
            }
        }
        Ok(None)
    }

    /// Resolves the browser field or alias field in the package.json.
    fn load_browser_field(
        &self,
        cached_path: &CachedPath,
        module_specifier: Option<&str>,
        package_json: &PackageJson,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if ctx.is_fully_specified {
            return Ok(None);
        }

        let path = cached_path.path();
        let Some(new_specifier) =
            self.resolve_browser_field(package_json, path, module_specifier)?
        else {
            return Ok(None);
        };

        // abort when resolving recursive module
        if module_specifier.is_some_and(|s| s == new_specifier) {
            return Ok(None);
        }

        if ctx
            .resolving_alias
            .as_ref()
            .is_some_and(|s| s == new_specifier)
        {
            // complete when resolving to self `{"./a.js": "./a.js"}`
            if new_specifier
                .strip_prefix("./")
                .filter(|s| path.ends_with(Path::new(s)))
                .is_some()
            {
                return if self.cache.is_file(cached_path, ctx) {
                    if self.check_restrictions(cached_path.path()) {
                        Ok(Some(cached_path.clone()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(ResolveError::NotFound {
                        specifier: new_specifier.to_string(),
                    })
                };
            }
            return Err(ResolveError::RecursiveDependency { depth: ctx.depth });
        }

        ctx.resolving_alias = Some(new_specifier.to_string());
        ctx.is_fully_specified = false;
        let package_url = self.cache.value(package_json.path.parent().unwrap());
        self.require(&package_url, new_specifier, ctx).map(Some)
    }

    /// Resolves aliases and fallbacks from the options.
    fn load_alias(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        aliases: &Alias,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        for (alias_key_raw, specifiers) in aliases {
            let mut alias_key_has_wildcard = false;
            let alias_key = if let Some(alias_key) = alias_key_raw.strip_suffix('$') {
                if alias_key != specifier {
                    continue;
                }
                alias_key
            } else if alias_key_raw.contains('*') {
                alias_key_has_wildcard = true;
                alias_key_raw
            } else {
                let strip_package_name = Self::strip_package_name(specifier, alias_key_raw);
                if strip_package_name.is_none() {
                    continue;
                }
                alias_key_raw
            };

            // it should stop resolving when all of the tried alias values failed to resolve.
            let mut should_stop = false;
            for r in specifiers {
                match r {
                    AliasValue::Path(alias_value) => {
                        if let Some(path) = self.load_alias_value(
                            cached_path,
                            alias_key,
                            alias_key_has_wildcard,
                            alias_value,
                            specifier,
                            ctx,
                            &mut should_stop,
                        )? {
                            return Ok(Some(path));
                        }
                    }
                    AliasValue::Ignore => {
                        let cached_path =
                            cached_path.normalize_with(alias_key, self.cache.as_ref());
                        return Err(ResolveError::Ignored {
                            path: cached_path.to_path_buf(),
                        });
                    }
                }
            }
            if should_stop {
                return Err(ResolveError::MatchedAliasNotFound {
                    specifier: specifier.to_string(),
                    alias_key: alias_key.to_string(),
                });
            }
        }
        Ok(None)
    }

    /// Resolves a specific alias value against the request.
    #[allow(clippy::too_many_arguments)]
    fn load_alias_value(
        &self,
        cached_path: &CachedPath,
        alias_key: &str,
        alias_key_has_wild_card: bool,
        alias_value: &str,
        request: &str,
        ctx: &mut ResolutionContext,
        should_stop: &mut bool,
    ) -> ResolveResult {
        if request != alias_value
            && !request
                .strip_prefix(alias_value)
                .is_some_and(|prefix| prefix.starts_with('/'))
        {
            let new_specifier = if alias_key_has_wild_card {
                // resolve wildcard, e.g. `@/*` -> `./src/*`
                let Some(alias_key) = alias_key.split_once('*').and_then(|(prefix, suffix)| {
                    request
                        .strip_prefix(prefix)
                        .and_then(|specifier| specifier.strip_suffix(suffix))
                }) else {
                    return Ok(None);
                };

                if alias_value.contains('*') {
                    Cow::Owned(alias_value.replacen('*', alias_key, 1))
                } else {
                    Cow::Borrowed(alias_value)
                }
            } else {
                let tail = &request[alias_key.len()..];
                if tail.is_empty() {
                    Cow::Borrowed(alias_value)
                } else {
                    let alias_path = Path::new(alias_value).normalize();
                    // must not append anything to alias_value if it is a file.
                    let cached_alias_path = self.cache.value(&alias_path);
                    if self.cache.is_file(&cached_alias_path, ctx) {
                        return Ok(None);
                    }
                    // remove the leading slash so the final path is concatenated.
                    let tail = tail.trim_start_matches(SLASH_START);
                    if tail.is_empty() {
                        Cow::Borrowed(alias_value)
                    } else {
                        let normalized = alias_path.normalize_with(tail);
                        Cow::Owned(normalized.to_string_lossy().to_string())
                    }
                }
            };

            *should_stop = true;
            ctx.is_fully_specified = false;
            return match self.require(cached_path, new_specifier.as_ref(), ctx) {
                Err(ResolveError::NotFound { .. } | ResolveError::MatchedAliasNotFound { .. }) => {
                    Ok(None)
                }
                Ok(path) => return Ok(Some(path)),
                Err(err) => return Err(err),
            };
        }
        Ok(None)
    }

    /// Loads the extension alias mapping (e.g. mapping .js to .ts).
    fn load_extension_alias(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if self.options.extension_alias.is_empty() {
            return Ok(None);
        }

        let Some(path_extension) = cached_path.path().extension() else {
            return Ok(None);
        };

        let Some(path_extension_str) = path_extension.to_str() else {
            return Ok(None);
        };

        let extension_key = format!(".{path_extension_str}");
        let Some(extensions) = self.options.extension_alias.get(&extension_key) else {
            return Ok(None);
        };

        let path = cached_path.path();
        let Some(filename) = path.file_name() else {
            return Ok(None);
        };

        ctx.is_fully_specified = true;
        for extension in extensions {
            let cached_path = cached_path.replace_extension(extension, self.cache.as_ref());
            if let Some(path) = self.load_alias_or_file(&cached_path, ctx)? {
                ctx.is_fully_specified = false;
                return Ok(Some(path));
            }
        }

        // bail if path is module directory such as `ipaddr.js`
        if !self.cache.is_file(cached_path, ctx) {
            ctx.is_fully_specified = false;
            return Ok(None);
        } else if !self.check_restrictions(cached_path.path()) {
            return Ok(None);
        }

        // create a meaningful error message
        let dir = path.parent().unwrap().to_path_buf();
        let filename_without_extension = Path::new(filename).with_extension("");
        let filename_without_extension = filename_without_extension.to_string_lossy();
        let files = extensions
            .iter()
            .map(|ext| format!("{filename_without_extension}{ext}"))
            .collect::<Vec<_>>()
            .join(",");
        Err(ResolveError::ExtensionAlias {
            filename: filename.to_string_lossy().to_string(),
            tried: files,
            dir,
        })
    }

    /// Resolves server-relative URLs using the configured roots.
    fn load_roots(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Option<CachedPath> {
        if self.options.roots.is_empty() {
            return None;
        }
        if let Some(specifier) = specifier.strip_prefix(SLASH_START) {
            if specifier.is_empty() {
                if self
                    .options
                    .roots
                    .iter()
                    .any(|root| root.as_path() == cached_path.path())
                    && let Ok(path) = self.require_relative(cached_path, "./", ctx)
                {
                    return Some(path);
                }
            } else {
                for root in &self.options.roots {
                    let cached_path = self.cache.value(root);
                    if let Ok(path) = self.require_relative(&cached_path, specifier, ctx) {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    /// Loads and parses a tsconfig.json file recursively.
    fn load_tsconfig(
        &self,
        root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<Arc<TsConfigJson>, ResolveError> {
        self.cache.get_tsconfig_json(root, path, |tsconfig| {
            let directory = self.cache.value(tsconfig.directory());

            if ctx.is_already_extended(&tsconfig.path) {
                return Err(ResolveError::TypeScriptOptionsCircular {
                    paths: ctx.get_extended_configs_with(tsconfig.path.to_path_buf()),
                });
            }

            // extend tsconfig
            let extended_tsconfig_paths = tsconfig
                .extends()
                .map(|specifier| self.get_extended_tsconfig_path(&directory, tsconfig, specifier))
                .collect::<Result<Vec<_>, _>>()?;
            if !extended_tsconfig_paths.is_empty() {
                ctx.with_extended_file(tsconfig.path.to_owned(), |ctx| {
                    for extended_tsconfig_path in extended_tsconfig_paths {
                        let extended_tsconfig = self.load_tsconfig(
                            /* root */ false,
                            &extended_tsconfig_path,
                            &TypeScriptOptionsReferences::Disabled,
                            ctx,
                        )?;
                        tsconfig.extend_from(&extended_tsconfig);
                    }
                    Result::Ok::<(), ResolveError>(())
                })?;
            }

            // loads the given references into this tsconfig
            match references {
                TypeScriptOptionsReferences::Disabled => {
                    tsconfig.references.drain(..);
                }
                TypeScriptOptionsReferences::Auto => {}
                TypeScriptOptionsReferences::Paths(paths) => {
                    tsconfig.references = paths
                        .iter()
                        .map(|path| TsProjectReferences {
                            path: path.clone(),
                            tsconfig: None,
                        })
                        .collect();
                }
            }
            if !tsconfig.references.is_empty() {
                let path = tsconfig.path.to_path_buf();
                let directory = tsconfig.directory().to_path_buf();
                for reference in tsconfig.references.iter_mut() {
                    let reference_tsconfig_path = directory.normalize_with(&reference.path);
                    let tsconfig = self.cache.get_tsconfig_json(
                        /* root */ true,
                        &reference_tsconfig_path,
                        |reference_tsconfig| {
                            if reference_tsconfig.path == path {
                                return Err(ResolveError::TypeScriptOptionsSelfReference {
                                    path: reference_tsconfig.path.to_path_buf(),
                                });
                            }
                            self.extend_tsconfig(
                                &self.cache.value(reference_tsconfig.directory()),
                                reference_tsconfig,
                                ctx,
                            )?;
                            Ok(())
                        },
                    )?;
                    reference.set_tsconfig(tsconfig);
                }
            }
            Ok(())
        })
    }

    /// Extends the current tsconfig with another configuration.
    fn extend_tsconfig(
        &self,
        directory: &CachedPath,
        tsconfig: &mut TsConfigJson,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<(), ResolveError> {
        let extended_tsconfig_paths = tsconfig
            .extends()
            .map(|specifier| self.get_extended_tsconfig_path(directory, tsconfig, specifier))
            .collect::<Result<Vec<_>, _>>()?;

        for extended_tsconfig_path in extended_tsconfig_paths {
            let extended_tsconfig = self.load_tsconfig(
                /* root */ false,
                &extended_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
            )?;
            tsconfig.extend_from(&extended_tsconfig);
        }
        Ok(())
    }

    /// Resolves the specifier using tsconfig `paths` configuration.
    fn load_tsconfig_paths(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        if cached_path.is_inside_node_modules {
            return Ok(None);
        }

        let tsconfig = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => {
                let tsconfig = self.load_tsconfig(
                    /* root */ true,
                    &tsconfig_options.config_file,
                    &tsconfig_options.references,
                    &mut TypeScriptOptionsResolveContext::default(),
                )?;
                // cache the loaded tsconfig in the path's directory
                let tsconfig_dir = self.cache.value(tsconfig.directory());
                _ = tsconfig_dir
                    .tsconfig
                    .get_or_init(|| Some(Arc::clone(&tsconfig)));
                tsconfig
            }
            Some(TypeScriptOptionsDiscovery::Auto) => {
                let Some(tsconfig) = self.find_tsconfig(cached_path, ctx)? else {
                    return Ok(None);
                };
                tsconfig
            }
        };

        let paths = tsconfig.resolve(cached_path.path(), specifier);
        for path in paths {
            let resolved_path = self.cache.value(&path);
            if let Some(resolution) = self.load_as_file_or_directory(&resolved_path, ".", ctx)? {
                // cache the tsconfig in the resolved path
                _ = resolved_path
                    .tsconfig
                    .get_or_init(|| Some(Arc::clone(&tsconfig)));
                return Ok(Some(resolution));
            }
        }
        Ok(None)
    }

    /// Find tsconfig.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    pub(crate) fn find_tsconfig(
        &self,
        cached_path: &CachedPath,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<Arc<TsConfigJson>>, ResolveError> {
        // don't discover tsconfig for paths inside node_modules
        if cached_path.is_inside_node_modules {
            return Ok(None);
        }
        // skip non-absolute paths (e.g. virtual modules)
        if !cached_path.path.is_absolute() {
            return Ok(None);
        }

        let mut cache_value = Some(cached_path.clone());
        while let Some(cv) = cache_value {
            if let Some(tsconfig) = cv.tsconfig.get_or_try_init(|| {
                let tsconfig_path = cv.path.join("tsconfig.json");
                let tsconfig_path = self.cache.value(&tsconfig_path);
                if self.cache.is_file(&tsconfig_path, ctx) {
                    self.resolve_tsconfig(tsconfig_path.path()).map(Some)
                } else {
                    Ok(None)
                }
            })? {
                return Ok(Some(Arc::clone(tsconfig)));
            }
            cache_value = cv.parent();
        }
        Ok(None)
    }

    /// Resolves the path of an extended tsconfig file.
    fn get_extended_tsconfig_path(
        &self,
        directory: &CachedPath,
        tsconfig: &TsConfigJson,
        specifier: &str,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            None => Err(ResolveError::Specifier {
                specifier: specifier.to_string(),
                message: None,
            }),
            Some(b'/') => Ok(PathBuf::from(specifier)),
            Some(b'.') => Ok(tsconfig.directory().normalize_with(specifier)),
            _ => self
                .clone_with_options(ResolveOptions {
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    ..ResolveOptions::default()
                })
                .load_package_self_or_node_modules(
                    directory,
                    specifier,
                    &mut ResolutionContext::default(),
                )
                .map(|p| p.to_path_buf())
                .map_err(|err| match err {
                    ResolveError::NotFound { .. } => ResolveError::TypeScriptOptionsNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => err,
                }),
        }
    }

    /// Implements `PACKAGE_RESOLVE` (packageSpecifier, parentURL).
    fn package_resolve(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);

        // iterate over all possible node_modules directories
        for module_name in &self.options.modules {
            for cached_path in std::iter::successors(Some(cached_path.clone()), CachedPath::parent)
            {
                // check if the module directory exists
                let Some(cached_path) = self.get_module_directory(&cached_path, module_name, ctx)
                else {
                    continue;
                };

                // check if the package exists in the module directory
                let cached_path = cached_path.normalize_with(package_name, self.cache.as_ref());

                // if the folder at packageURL does not exist, then continue
                if self.cache.is_directory(&cached_path, ctx) {
                    // load package.json
                    if let Some(package_json) =
                        self.cache
                            .get_package_json(&cached_path, &self.options, ctx)?
                    {
                        if let Some(exports) = package_json.exports.as_ref()
                            && let Some(path) = self.package_exports_resolve(
                                &cached_path,
                                &format!(".{subpath}"),
                                exports,
                                ctx,
                            )?
                        {
                            return Ok(Some(path));
                        }

                        if subpath == "."
                            && let Some(main_field) = package_json.main.as_deref()
                        {
                            let cached_path =
                                cached_path.normalize_with(main_field, self.cache.as_ref());
                            if self.cache.is_file(&cached_path, ctx)
                                && self.check_restrictions(cached_path.path())
                            {
                                return Ok(Some(cached_path));
                            }
                        }
                    }

                    let subpath = format!(".{subpath}");
                    ctx.is_fully_specified = false;
                    return self.require(&cached_path, &subpath, ctx).map(Some);
                }
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Implements `PACKAGE_EXPORTS_RESOLVE` (packageURL, subpath, exports, conditions).
    pub(crate) fn package_exports_resolve(
        &self,
        package_url: &CachedPath,
        subpath: &str,
        exports: &serde_json::Value,
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        let conditions = &self.options.conditions;

        // validate exports: cannot mix starting with "." and not starting with "."
        if let Some(map) = exports.as_object() {
            let mut has_dot = false;
            let mut without_dot = false;
            for key in map.keys() {
                let starts_with_dot_or_hash = key.starts_with(['.', '#']);
                has_dot = has_dot || starts_with_dot_or_hash;
                without_dot = without_dot || !starts_with_dot_or_hash;
                if has_dot && without_dot {
                    return Err(ResolveError::PackageJsonInvalid {
                        path: package_url.path().join("package.json"),
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
                    /* is_imports */ false,
                    conditions,
                    ctx,
                )?;
                if let Some(path) = resolved {
                    return Ok(Some(path));
                }
            }
        }

        // resolve subpath export
        if let Some(exports) = exports.as_object() {
            let match_key = &subpath;
            if let Some(path) = self.package_imports_exports_resolve(
                match_key,
                exports,
                package_url,
                /* is_imports */ false,
                conditions,
                ctx,
            )? {
                return Ok(Some(path));
            }
        }

        // package path not exported
        Err(ResolveError::PackagePathNotExported {
            subpath: subpath.to_string(),
            package_path: package_url.path().to_path_buf(),
            package_json_path: package_url.path().join("package.json"),
            conditions: self.options.conditions.clone(),
        })
    }

    /// Implements `PACKAGE_IMPORTS_RESOLVE` (specifier, parentURL, conditions).
    fn package_imports_resolve(
        &self,
        specifier: &str,
        package_json: &PackageJson,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<CachedPath>, ResolveError> {
        debug_assert!(specifier.starts_with('#'), "{specifier}");

        let Some(imports) = package_json.imports.as_ref() else {
            return Ok(None);
        };

        if specifier == "#" || specifier.starts_with("#/") {
            return Err(ResolveError::InvalidModuleSpecifier {
                specifier: specifier.to_string(),
                package_path: package_json.path.to_path_buf(),
            });
        }

        if let Some(path) = self.package_imports_exports_resolve(
            specifier,
            imports,
            &self.cache.value(&package_json.directory),
            /* is_imports */ true,
            &self.options.conditions,
            ctx,
        )? {
            return Ok(Some(path));
        }

        Err(ResolveError::PackageImportNotDefined {
            specifier: specifier.to_string(),
            package_path: package_json.path.to_path_buf(),
        })
    }

    /// Implements `PACKAGE_IMPORTS_EXPORTS_RESOLVE` (matchKey, matchObj, packageURL, isImports, conditions).
    pub(crate) fn package_imports_exports_resolve(
        &self,
        match_key: &str,
        match_obj: &serde_json::Map<String, serde_json::Value>,
        package_url: &CachedPath,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
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

        let mut best_target = None;
        let mut best_match = "";
        let mut best_key = "";

        // pattern match
        // find the best matching key in the match object
        for (expansion_key, target_key) in match_obj.iter() {
            // ignore invalid mappings (wildcard expansion without wildcard target)
            if expansion_key.ends_with('*') && target_key.as_str().is_some_and(|s| !s.contains('*'))
            {
                continue;
            }

            if expansion_key.starts_with("./") || expansion_key.starts_with('#') {
                if let Some((pattern_base, pattern_trailer)) = expansion_key.split_once('*') {
                    if match_key.starts_with(pattern_base)
                        && !pattern_trailer.contains('*')
                        && (pattern_trailer.is_empty()
                            || (match_key.len() >= expansion_key.len()
                                && match_key.ends_with(pattern_trailer)))
                        && Self::pattern_key_compare(best_key, expansion_key).is_gt()
                    {
                        best_target = Some(target_key);
                        best_match =
                            &match_key[pattern_base.len()..match_key.len() - pattern_trailer.len()];
                        best_key = expansion_key;
                    }
                } else if expansion_key.ends_with('/')
                    && match_key.starts_with(expansion_key)
                    && Self::pattern_key_compare(best_key, expansion_key).is_gt()
                {
                    best_target = Some(target_key);
                    best_match = &match_key[expansion_key.len()..];
                    best_key = expansion_key;
                }
            }
        }

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

    /// Implements `PACKAGE_TARGET_RESOLVE` (packageURL, target, patternMatch, isImports, conditions).
    #[allow(clippy::too_many_arguments)]
    fn package_target_resolve(
        &self,
        package_url: &CachedPath,
        target_key: &str,
        target: &serde_json::Value,
        pattern_match: Option<&str>,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolutionContext,
    ) -> ResolveResult {
        fn normalize_string_target<'a>(
            target_key: &'a str,
            target: &'a str,
            pattern_match: Option<&'a str>,
            package_url: &CachedPath,
        ) -> Result<Cow<'a, str>, ResolveError> {
            let target = if let Some(pattern_match) = pattern_match {
                if !target_key.contains('*') && !target.contains('*') {
                    // enhanced-resolve behaviour
                    if target_key.ends_with('/') && target.ends_with('/') {
                        Cow::Owned(format!("{target}{pattern_match}"))
                    } else {
                        return Err(ResolveError::InvalidPackageConfigDirectory {
                            path: package_url.path().join("package.json"),
                        });
                    }
                } else {
                    Cow::Owned(target.replace('*', pattern_match))
                }
            } else {
                Cow::Borrowed(target)
            };
            Ok(target)
        }

        // resolve string target
        if let Some(target) = target.as_str() {
            let parsed = ModuleSpecifier::parse(target);
            if let Some(query) = &parsed.query {
                ctx.query.replace(query.to_string());
            }
            if let Some(fragment) = &parsed.fragment {
                ctx.fragment.replace(fragment.to_string());
            }
            let target = parsed.path();

            if !target.starts_with("./") {
                if !is_imports || target.starts_with("../") || target.starts_with('/') {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: (*target).to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.path().join("package.json"),
                    });
                }
                let target =
                    normalize_string_target(target_key, target, pattern_match, package_url)?;
                return self.package_resolve(package_url, &target, ctx);
            }

            // normalize target
            let target = normalize_string_target(target_key, target, pattern_match, package_url)?;
            if is_path_invalid_exports_target(Path::new(target.as_ref())) {
                return Err(ResolveError::InvalidPackageTarget {
                    target: target.to_string(),
                    name: target_key.to_string(),
                    package_path: package_url.path().join("package.json"),
                });
            }

            return Ok(Some(
                package_url.normalize_with(target.as_ref(), self.cache.as_ref()),
            ));
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
                    package_path: package_url.path().to_path_buf(),
                    package_json_path: package_url.path().join("package.json"),
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

                if resolved.is_err() && i == targets.len() {
                    return resolved;
                }
                if let Ok(Some(path)) = resolved {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    /// Parses the package specifier (like `@scope/package-name/file` into `@scope/package-name` and `file`)
    fn parse_package_specifier(specifier: &str) -> (&str, &str) {
        let mut separator_index = specifier.as_bytes().iter().position(|b| *b == b'/');
        // valid package name
        if specifier.starts_with('@') {
            if separator_index.is_none() || specifier.is_empty() {
                // invalid package name
            } else if let Some(index) = &separator_index {
                separator_index = specifier.as_bytes()[*index + 1..]
                    .iter()
                    .position(|b| *b == b'/')
                    .map(|i| i + *index + 1);
            }
        }
        let package_name =
            separator_index.map_or(specifier, |separator_index| &specifier[..separator_index]);
        let package_subpath =
            separator_index.map_or("", |separator_index| &specifier[separator_index..]);
        (package_name, package_subpath)
    }

    /// Implements `PATTERN_KEY_COMPARE` (keyA, keyB).
    fn pattern_key_compare(key_a: &str, key_b: &str) -> Ordering {
        if key_a.is_empty() {
            return Ordering::Greater;
        }

        debug_assert!(
            key_a.ends_with('/') || key_a.match_indices('*').count() == 1,
            "{key_a}"
        );
        debug_assert!(
            key_b.ends_with('/') || key_b.match_indices('*').count() == 1,
            "{key_b}"
        );

        let a_pos = key_a.bytes().position(|c| c == b'*');
        let base_length_a = a_pos.map_or(key_a.len(), |p| p + 1);
        let b_pos = key_b.bytes().position(|c| c == b'*');
        let base_length_b = b_pos.map_or(key_b.len(), |p| p + 1);

        if base_length_a > base_length_b {
            return Ordering::Less;
        }
        if base_length_b > base_length_a {
            return Ordering::Greater;
        }

        if a_pos.is_none() {
            return Ordering::Greater;
        }
        if b_pos.is_none() {
            return Ordering::Less;
        }

        if key_a.len() > key_b.len() {
            return Ordering::Less;
        }
        if key_b.len() > key_a.len() {
            return Ordering::Greater;
        }

        Ordering::Equal
    }

    /// Strips the package name from the specifier.
    fn strip_package_name<'a>(specifier: &'a str, package_name: &'a str) -> Option<&'a str> {
        specifier
            .strip_prefix(package_name)
            .filter(|tail| tail.is_empty() || tail.starts_with(SLASH_START))
    }
}

fn is_path_invalid_exports_target(path: &Path) -> bool {
    path.components().enumerate().any(|(index, c)| match c {
        Component::ParentDir => true,
        Component::CurDir => index > 0,
        Component::Normal(c) => c.eq_ignore_ascii_case("node_modules"),
        _ => false,
    })
}

#[derive(Default)]
struct TypeScriptOptionsResolveContext {
    extended_configs: Vec<PathBuf>,
}

impl TypeScriptOptionsResolveContext {
    /// Executes a closure with a new extended file added to the context.
    fn with_extended_file<F, T>(&mut self, path: PathBuf, f: F) -> Result<T, ResolveError>
    where
        F: FnOnce(&mut Self) -> Result<T, ResolveError>,
    {
        self.extended_configs.push(path);
        let result = f(self);
        self.extended_configs.pop();
        result
    }

    /// Checks if a file has already been extended in the current context.
    fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs
            .iter()
            .any(|extended| extended == path)
    }

    /// Returns the list of extended configurations including the given path.
    fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = self.extended_configs.clone();
        configs.push(path);
        configs
    }
}

/// Resolves a file protocol URL to a file path.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_file_protocol(specifier: &str) -> Result<Cow<'_, str>, ResolveError> {
    if specifier.starts_with("file://") {
        url::Url::parse(specifier)
            .map_err(|_| ())
            .and_then(|url| {
                url.to_file_path().map(|path| {
                    let mut result = path.to_string_lossy().to_string();
                    // preserve query and fragment from the url
                    if let Some(query) = url.query() {
                        result.push('?');
                        result.push_str(query);
                    }
                    if let Some(fragment) = url.fragment() {
                        result.push('#');
                        result.push_str(fragment);
                    }
                    Cow::Owned(result)
                })
            })
            .map_err(|()| ResolveError::PathNotSupported {
                path: PathBuf::from(specifier),
            })
    } else {
        Ok(Cow::Borrowed(specifier))
    }
}
