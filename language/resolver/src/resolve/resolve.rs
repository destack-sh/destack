use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::{fmt, io};

use dyst_dir::{
    DsConfigId, ModuleSpecifier, Package, PackageId, PackageOptions, PackageType, Program,
    TsConfig, TsConfigId, TsConfigProjectReferences,
};
use dyst_source::{
    FileRegistry, FileSystem, LanguageOptions, PathExt, PhysicalFileSystem, SLASH_START,
};
use rustc_hash::FxHasher;

use crate::{
    Alias, AliasValue, Resolution, ResolutionContext, ResolveError, ResolveOptions, Restriction,
    TypeScriptOptionsDiscovery, TypeScriptOptionsReferences,
};

// thread-local pre-allocated path buffer for path manipulation
thread_local! {
    static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
}

/// Simple identity hasher for hash sets that already have pre-computed hashes.
#[derive(Debug, Default)]
struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("invalid use of IdentityHasher")
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Check if a path is inside a modules directory.
#[inline]
fn is_inside_modules(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, Component::Normal(name) if name == "node_modules"))
}

/// Append an extension to a path (e.g., `foo` + `.js` = `foo.js`).
fn append_extension(path: &Path, extension: &str) -> PathBuf {
    SCRATCH_PATH.with_borrow_mut(|scratch| {
        scratch.clear();
        let os_string = scratch.as_mut_os_string();
        os_string.push(path.as_os_str());
        os_string.push(extension);
        scratch.clone()
    })
}

/// Module resolver implementing Node.js-style resolution.
///
/// Resolves import specifiers to absolute file paths following the Node.js
/// resolution algorithm with extensions for TypeScript path mapping,
/// browser field substitution, and custom aliases.
pub struct Resolver {
    /// The Program holding file registries and filesystem access.
    pub program: Arc<Program>,
    /// Configuration options for resolution behavior.
    pub options: ResolveOptions,
}

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Create a new resolver with options in an existing program.
    pub fn new(program: Arc<Program>, options: ResolveOptions) -> Self {
        Self {
            program,
            options: options.sanitize(),
        }
    }

    /// Create a new resolver with physical file system in an empty program.
    pub fn blank(options: ResolveOptions) -> Self {
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let files = Arc::new(FileRegistry::new());
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            fs.clone(),
            files.clone(),
        ));
        Self::new(program, options)
    }

    /// Create a new resolver with a custom file system in an empty program.
    pub fn blank_with_fs(fs: Arc<dyn FileSystem>, options: ResolveOptions) -> Self {
        let files = Arc::new(FileRegistry::new());
        let program = Arc::new(Program::new(
            LanguageOptions::default(),
            fs.clone(),
            files.clone(),
        ));
        Self::new(program, options)
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolveOptions) -> Self {
        Self {
            program: self.program.clone(),
            options: options.sanitize(),
        }
    }

    /// Get the filesystem.
    #[inline]
    fn fs(&self) -> &dyn FileSystem {
        self.program.fs.as_ref()
    }

    /// Check if a path is a file.
    #[inline]
    fn is_file(&self, path: &Path, ctx: &mut ResolutionContext) -> bool {
        match self.fs().metadata(path) {
            Ok(meta) if meta.is_file => {
                ctx.track_found_dependency(path);
                true
            }
            _ => {
                ctx.track_missing_dependency(path);
                false
            }
        }
    }

    /// Check if a path is a directory.
    #[inline]
    fn is_directory(&self, path: &Path, ctx: &mut ResolutionContext) -> bool {
        match self.fs().metadata(path) {
            Ok(meta) if meta.is_directory => true,
            _ => {
                ctx.track_missing_dependency(path);
                false
            }
        }
    }

    /// Canonicalize a path, resolving all symlinks.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        // track visited paths for circular symlink detection
        let mut visited = HashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());
        let result = self
            .canonicalize_with_visited(path, &mut visited)
            .or_else(|err| {
                // fallback: try direct FS canonicalize
                self.fs().canonicalize(path).map_err(|_| err)
            })?;

        #[cfg(target_os = "windows")]
        let result = dyst_source::file::windows::strip_windows_prefix(result).unwrap_or(result);

        Ok(result)
    }

    /// Canonicalize with circular symlink detection.
    fn canonicalize_with_visited(
        &self,
        path: &Path,
        visited: &mut HashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> Result<PathBuf, ResolveError> {
        // compute hash for circular detection
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };

        // check for circular symlink
        if !visited.insert(hash) {
            return Err(ResolveError::IoError {
                path: path.to_path_buf(),
                kind: io::ErrorKind::NotFound,
            });
        }

        // resolve parent and normalize
        let canonical = path.parent().map_or_else(
            || Ok(Self::normalize_root(path)),
            |parent| {
                self.canonicalize_with_visited(parent, visited)
                    .and_then(|parent_canonical| {
                        let normalized = parent_canonical
                            .normalize_with(path.strip_prefix(parent).unwrap_or(Path::new("")));

                        if self.fs().symlink_metadata(path).is_ok_and(|m| m.is_symlink) {
                            let link = self.fs().resolve_symlink(&normalized).map_err(|error| {
                                ResolveError::IoError {
                                    path: normalized.clone(),
                                    kind: error.kind(),
                                }
                            })?;
                            if link.is_absolute() {
                                return self.canonicalize_with_visited(&link.normalize(), visited);
                            } else if let Some(dir) = normalized.parent() {
                                return self.canonicalize_with_visited(
                                    &dir.normalize_with(&link),
                                    visited,
                                );
                            }
                            debug_assert!(
                                false,
                                "Failed to get path parent for {}.",
                                normalized.display()
                            );
                        }

                        Ok(normalized)
                    })
            },
        )?;

        visited.remove(&hash);
        Ok(canonical)
    }

    /// Normalize root path (Windows-specific handling).
    #[inline]
    #[cfg(windows)]
    fn normalize_root(path: &Path) -> PathBuf {
        if path.as_os_str().as_encoded_bytes().last() == Some(&b'/') {
            let mut path_string = path.to_string_lossy().into_owned();
            path_string.pop();
            path_string.push('\\');
            PathBuf::from(path_string)
        } else {
            path.to_path_buf()
        }
    }

    #[inline]
    #[cfg(not(windows))]
    fn normalize_root(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    /// Find the nearest package.json by walking up parent directories.
    ///
    /// Returns the PackageId if found, or None if no package.json exists in any parent.
    pub fn find_package(&self, path: &Path) -> Option<PackageId> {
        let mut ctx = ResolutionContext::default();
        let package_id = self.find_package_json_id(path, &mut ctx).ok()??;
        Some(package_id)
    }

    /// Find the nearest tsconfig.json by walking up parent directories.
    ///
    /// Returns the TsConfigId if found, or None if no tsconfig.json exists in any parent.
    pub fn find_tsconfig_id(&self, path: &Path) -> Option<TsConfigId> {
        let mut ctx = ResolutionContext::default();
        let tsconfig_id = self.find_tsconfig(path, &mut ctx).ok()??;
        Some(tsconfig_id)
    }

    /// Find the nearest dsconfig.json by walking up parent directories.
    ///
    /// Returns the DsConfigId if found, or None if no dsconfig.json exists in any parent.
    pub fn find_dsconfig(&self, path: &Path) -> Option<DsConfigId> {
        // check if already in registry
        if let Some(dsconfig_id) = self.program.dsconfigs.get_id_by_path(path) {
            return Some(dsconfig_id);
        }

        // walk up parent directories looking for dsconfig.json
        let mut current = Some(path.to_path_buf());
        while let Some(dir) = current {
            let dsconfig_path = dir.join("dsconfig.json");
            if self.fs().metadata(&dsconfig_path).is_ok_and(|m| m.is_file) {
                // check registry
                if let Some(dsconfig_id) = self.program.dsconfigs.get_id_by_path(&dsconfig_path) {
                    return Some(dsconfig_id);
                }
                // nocheckin TODO @Incomplete: should load and register dsconfig here
                return None;
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
        None
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

    /// Load package.json from a directory, returning the PackageId.
    ///
    /// The directory path is the path to the directory containing package.json.
    fn load_package_json_id(
        &self,
        directory: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PackageId>, ResolveError> {
        // check registry first
        if let Some(package_id) = self.program.packages.get_id_by_path(directory) {
            let package = self.program.packages.get(package_id);
            let package_guard = package.read();
            ctx.track_found_dependency(&package_guard.options.path);
            return Ok(Some(package_id));
        }

        let package_json_path = directory.join("package.json");

        // try to read the file
        let package_json_bytes = match self.fs().read(&package_json_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                ctx.track_missing_dependency(&package_json_path);
                return Ok(None);
            }
        };

        // determine real path
        let real_path = if self.options.canonicalize_symlinks {
            self.canonicalize(directory)?.join("package.json")
        } else {
            package_json_path.clone()
        };

        // get a FileId for the package.json file
        // nocheckin TODO @Incomplete: Should properly register the file in FileRegistry with content
        let file_id = self.program.files.next_id();

        // parse `package.json` file
        let package_options = PackageOptions::parse(
            file_id,
            package_json_path.clone(),
            real_path.clone(),
            package_json_bytes,
        )
        .map_err(|_| ResolveError::InvalidPackageJson {
            path: package_json_path.clone(),
        })?;

        // create and store Package in registry
        let package_id = self.program.packages.next_id();
        let package = Package {
            id: package_id,
            file_id,
            path: directory.to_path_buf(),
            realpath: real_path.parent().unwrap_or(directory).to_path_buf(),
            name: package_options.content.name.clone(),
            version: package_options.content.version.clone(),
            ty: package_options.content.ty.unwrap_or(PackageType::CommonJs),
            options: package_options,
            main_tsconfig_id: None,
            main_dsconfig_id: None,
        };
        self.program.packages.insert(package);

        ctx.track_found_dependency(&package_json_path);
        Ok(Some(package_id))
    }

    /// Get PackageOptions for a PackageId.
    #[inline]
    fn get_package_options(&self, package_id: PackageId) -> PackageOptions {
        let package = self.program.packages.get(package_id);
        let package_guard = package.read();
        package_guard.options.clone()
    }

    /// Find and return package.json options (convenience method).
    fn find_package_json(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PackageOptions>, ResolveError> {
        if let Some(package_id) = self.find_package_json_id(path, ctx)? {
            Ok(Some(self.get_package_options(package_id)))
        } else {
            Ok(None)
        }
    }

    /// Load and return package.json options (convenience method).
    fn load_package_json(
        &self,
        directory: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PackageOptions>, ResolveError> {
        if let Some(package_id) = self.load_package_json_id(directory, ctx)? {
            Ok(Some(self.get_package_options(package_id)))
        } else {
            Ok(None)
        }
    }

    /// Resolve a `tsconfig.json` file.
    ///
    /// The path can be:
    /// * Path to a file with `.json` extension.
    /// * Path to a file without `.json` extension, `.json` will be appended to filename.
    /// * Path to a directory, where the filename is defaulted to `tsconfig.json`
    pub fn resolve_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<TsConfigId, ResolveError> {
        let path = path.as_ref();
        self.load_tsconfig(
            true,
            path,
            &TypeScriptOptionsReferences::Automatic,
            &mut TypeScriptOptionsResolveContext::default(),
        )
    }

    /// Get TsConfig for a TsConfigId.
    #[inline]
    pub fn get_tsconfig(&self, tsconfig_id: TsConfigId) -> TsConfig {
        let tsconfig = self.program.tsconfigs.get(tsconfig_id);
        let tsconfig_guard = tsconfig.read();
        (*tsconfig_guard).clone()
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

    /// Perform the resolution.
    fn resolve_in_context(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Resolution, ResolveError> {
        ctx.is_fully_specified = self.options.is_fully_specified;

        let resolved_path = self.require(path, specifier, ctx)?;
        let path = self.load_realpath(&resolved_path)?;

        let resolution = Resolution {
            path,
            query: ctx.query.take(),
            fragment: ctx.fragment.take(),
        };
        Ok(resolution)
    }

    /// Find the nearest package.json by walking up parent directories.
    ///
    /// Returns the PackageId if found.
    fn find_package_json_id(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PackageId>, ResolveError> {
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
            if let Some(package_id) = self.load_package_json_id(&dir, ctx)? {
                return Ok(Some(package_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }

    /// Require `specifier` from a module at `path`.
    /// <https://nodejs.org/api/modules.html#all-together>
    fn require(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
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
            if let Ok(resolved) = self.require_without_parse(path, &candidate, ctx) {
                return Ok(resolved);
            }
            ctx.fragment.replace(fragment);
        }

        self.require_without_parse(path, parsed.path(), ctx)
    }

    /// Require `specifier` from a module at `path` without parsing.
    fn require_without_parse(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
        // check tsconfig paths
        if let Some(resolved) =
            self.load_tsconfig_paths(path, specifier, &mut ResolutionContext::default())?
        {
            return Ok(resolved);
        }

        // check alias
        if let Some(resolved) = self.load_alias(path, specifier, &self.options.alias, ctx)? {
            return Ok(resolved);
        }

        // resolve file protocol
        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let specifier = resolve_file_protocol(specifier)?;
                let specifier = specifier.as_ref();
            }
        };

        let result = match Path::new(&specifier).components().next() {
            // absolute path
            Some(Component::RootDir | Component::Prefix(_)) => {
                self.require_absolute(path, specifier, ctx)
            }
            // relative path
            Some(Component::CurDir | Component::ParentDir) => {
                self.require_relative(path, specifier, ctx)
            }
            // internal package import
            Some(Component::Normal(_)) if specifier.as_bytes()[0] == b'#' => {
                self.require_hash(path, specifier, ctx)
            }
            // bare specifier (module)
            _ => self.require_bare(path, specifier, ctx),
        };

        result.or_else(|err| {
            if err.is_ignore() {
                return Err(err);
            }
            // check fallback alias
            self.load_alias(path, specifier, &self.options.fallback, ctx)
                .and_then(|value| value.ok_or(err))
        })
    }

    /// Resolve an absolute path specifier (starting with `/` or drive letter).
    fn require_absolute(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
        // make sure only path prefixes gets called
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
        );

        // try to load from package itself or node_modules
        if !self.options.prefer_relative
            && self.options.prefer_absolute
            && let Ok(path) = self.load_package_self_or_modules(path, specifier, ctx)
        {
            return Ok(path);
        }

        // try to load from roots
        if let Some(path) = self.load_roots(path, specifier, ctx) {
            return Ok(path);
        }

        // try to load as file or directory
        let path = Path::new(specifier).to_path_buf();
        if let Some(path) = self.load_as_file_or_directory(&path, specifier, ctx)? {
            return Ok(path);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a relative path specifier (starting with `./` or `../`).
    fn require_relative(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
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
        let path_with_specifier = path.normalize_with(specifier);

        // load as file or directory
        if let Some(result) = self.load_as_file_or_directory(
            &path_with_specifier,
            // ensure resolve directory only when specifier is `.`
            if specifier == "." { "./" } else { specifier },
            ctx,
        )? {
            return Ok(result);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a hash-prefixed specifier against package.json imports.
    fn require_hash(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
        debug_assert_eq!(specifier.chars().next(), Some('#'));

        // load package imports
        self.load_package_imports(path, specifier, ctx)?
            .map_or_else(
                || {
                    Err(ResolveError::NotFound {
                        specifier: specifier.to_string(),
                    })
                },
                Ok,
            )
    }

    /// Resolve a bare specifier by searching node_modules directories.
    fn require_bare(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
        // make sure no other path prefixes gets called
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::Normal(_)))
        );

        // prefer relative path
        if self.options.prefer_relative
            && let Ok(path) = self.require_relative(path, specifier, ctx)
        {
            Ok(path)
        }
        // try package itself or modules
        else {
            self.load_package_self_or_modules(path, specifier, ctx)
        }
    }

    /// Try to resolve from the package itself (self-reference) or node_modules.
    fn load_package_self_or_modules(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<PathBuf, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.is_fully_specified = false;
        }

        // try to load from the package itself
        if let Some(path) = self.load_package_self(path, specifier, ctx)? {
            return Ok(path);
        }

        // try to load from node_modules
        if let Some(path) = self.load_modules(path, specifier, package_name, subpath, ctx)? {
            return Ok(path);
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
    fn load_package_imports(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // find the closest package scope to the directory
        let Some(package_json) = self.find_package_json(path, ctx)? else {
            return Ok(None);
        };

        // check if the package has imports
        if let Some(path) = self.package_imports_resolve(specifier, &package_json, ctx)? {
            self.resolve_esm_match(specifier, &path, ctx)
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try to resolve a path as a file with optional extension adding.
    fn load_as_file(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try extension alias
        if let Some(path) = self.load_with_extension_alias(path, ctx)? {
            Ok(Some(path))
        }
        // if the path is a file, load it as its file extension format
        else if self.options.enforce_extension.is_disabled()
            && let Some(path) = self.load_alias_or_file(path, ctx)?
        {
            Ok(Some(path))
        }
        // try extensions (like .js, .json, .node, etc.)
        else if let Some(path) = self.load_extensions(path, &self.options.extensions, ctx)? {
            Ok(Some(path))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try to resolve a path as a directory via main files or index files.
    fn load_as_directory(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // check for package.json in the directory
        if let Some(package_json) = self.load_package_json(path, ctx)? {
            if let Some(main_field) = package_json.content.main.as_deref() {
                let main_field = if main_field.starts_with("./") || main_field.starts_with("../") {
                    Cow::Borrowed(main_field)
                } else {
                    Cow::Owned(format!("./{main_field}"))
                };

                let main_path = path.normalize_with(main_field.as_ref());

                // try to load as file
                if let Some(result) = self.load_as_file(&main_path, ctx)? {
                    return Ok(Some(result));
                }

                // try to load index file
                if let Some(result) = self.load_index(&main_path, ctx)? {
                    return Ok(Some(result));
                }
            }

            // allow `exports` field in `require('../directory')`
            if let Some(exports) = package_json.content.exports.as_ref()
                && let Some(path) = self.package_exports_resolve(path, ".", exports, ctx)?
            {
                return Ok(Some(path));
            }
        }

        // try to load index file
        self.load_index(path, ctx)
    }

    /// Load a path as either a file or a directory.
    /// Try to resolve a path as either a file or a directory.
    fn load_as_file_or_directory(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // special mode: resolve to directory itself
        if self.options.resolve_to_directory && self.is_directory(path, ctx) {
            return Ok(Some(path.to_path_buf()));
        }

        // try as file (unless specifier ends with `/`)
        if !specifier.ends_with('/')
            && let Some(resolved) = self.load_as_file(path, ctx)?
        {
            Ok(Some(resolved))
        }
        // try as directory
        else if self.is_directory(path, ctx)
            && let Some(resolved) = self.load_as_directory(path, ctx)?
        {
            Ok(Some(resolved))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try appending configured extensions to resolve a path.
    fn load_extensions(
        &self,
        path: &Path,
        extensions: &[String],
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if ctx.is_fully_specified {
            return Ok(None);
        }
        for extension in extensions {
            let extended_path = append_extension(path, extension);
            if let Some(result) = self.load_alias_or_file(&extended_path, ctx)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Load the real path (resolving symlinks if needed).
    fn load_realpath(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        if self.options.canonicalize_symlinks {
            self.canonicalize(path)
        } else {
            Ok(path.to_path_buf())
        }
    }

    /// Check if a resolved path passes all configured restrictions.
    fn check_restrictions(&self, path: &Path) -> bool {
        /// Check if a path is inside a restricted path.
        /// See <https://github.com/webpack/enhanced-resolve/blob/a998c7d218b7a9ec2461fc4fddd1ad5dd7687485/lib/RestrictionsPlugin.js#L19-L24>
        fn is_in_restricted(path: &Path, parent: &Path) -> bool {
            if !path.starts_with(parent) {
                return false;
            }
            if path.as_os_str().len() == parent.as_os_str().len() {
                return true;
            }
            path.strip_prefix(parent)
                .is_ok_and(|p| p == Path::new("./"))
        }

        // check all restrictions
        for restriction in &self.options.restrictions {
            match restriction {
                Restriction::Path(restricted_path) => {
                    if !is_in_restricted(path, restricted_path) {
                        return false;
                    }
                }
                Restriction::Function(f) => {
                    if !f(path) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Try to resolve a directory by looking for index files.
    fn load_index(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try every main file
        for main_file in &self.options.main_files {
            // resolve main file
            let resolved_path = path.normalize_with(main_file);
            if self.options.enforce_extension.is_disabled()
                && let Some(resolved) = self.load_browser_field_or_alias(&resolved_path, ctx)?
                && self.check_restrictions(&resolved)
            {
                return Ok(Some(resolved));
            }

            // try extensions
            if let Some(resolved) =
                self.load_extensions(&resolved_path, &self.options.extensions, ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve via browser field or alias mappings.
    fn load_browser_field_or_alias(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try browser field
        if let Some(package_json) = self.find_package_json(path, ctx)?
            && let Some(path) = self.load_browser_field(path, None, &package_json, ctx)?
        {
            return Ok(Some(path));
        }

        // try file as alias
        if !self.options.alias.is_empty() {
            let alias_specifier = path.to_string_lossy();
            if let Some(path) = self.load_alias(path, &alias_specifier, &self.options.alias, ctx)? {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// Try to resolve via alias mapping, falling back to direct file check.
    fn load_alias_or_file(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if let Some(resolved) = self.load_browser_field_or_alias(path, ctx)? {
            return Ok(Some(resolved));
        }
        if self.is_file(path, ctx) && self.check_restrictions(path) {
            return Ok(Some(path.to_path_buf()));
        }
        Ok(None)
    }

    /// Search node_modules directories walking up from the given path.
    fn load_modules(
        &self,
        path: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
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
                // (try to interpret X as a combination of NAME and SUBPATH,
                //  name may have a @scope/ prefix and the subpath begins with a slash (`/`))
                if !package_name.is_empty() {
                    let package_path = module_dir.normalize_with(package_name);
                    // try <foo>/node_modules/package_name
                    if self.is_directory(&package_path, ctx) {
                        // load package exports
                        if let Some(resolved) =
                            self.load_package_exports(specifier, subpath, &package_path, ctx)?
                        {
                            return Ok(Some(resolved));
                        }
                    }
                    // foo/node_modules/package_name is not a directory, so useless to check inside it
                    else {
                        if !subpath.is_empty() {
                            current = current_path.parent().map(|p| p.to_path_buf());
                            continue;
                        }
                        // skip if the directory lead to the scope package does not exist
                        // i.e. `foo/node_modules/@scope` is not a directory for `foo/node_modules/@scope/package`
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
                if self.options.resolve_to_directory {
                    return Ok(self
                        .is_directory(&resolved_path, ctx)
                        .then(|| resolved_path.to_path_buf()));
                }

                // load directory
                if self.is_directory(&resolved_path, ctx) {
                    if let Some(resolved) = self.load_browser_field_or_alias(&resolved_path, ctx)? {
                        return Ok(Some(resolved));
                    }
                    if let Some(resolved) = self.load_as_directory(&resolved_path, ctx)? {
                        return Ok(Some(resolved));
                    }
                }
                // load file
                else if let Some(resolved) = self.load_as_file(&resolved_path, ctx)? {
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
        ctx: &mut ResolutionContext,
    ) -> Option<PathBuf> {
        if path
            .components()
            .next_back()
            .is_some_and(|c| c.as_os_str() == module_name)
        {
            return Some(path.to_path_buf());
        }

        let subdir = path.join(module_name);
        if self.is_directory(&subdir, ctx) {
            Some(subdir)
        } else {
            None
        }
    }

    /// Try to resolve a specifier via package.json exports field.
    fn load_package_exports(
        &self,
        specifier: &str,
        subpath: &str,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // check if package.json exists
        let Some(package_json) = self.load_package_json(path, ctx)? else {
            return Ok(None);
        };

        // resolve exports
        if let Some(exports) = package_json.content.exports.as_ref()
            && let Some(path) =
                self.package_exports_resolve(path, &format!(".{subpath}"), exports, ctx)?
        {
            return self.resolve_esm_match(specifier, &path, ctx);
        }

        Ok(None)
    }

    /// Try to resolve a self-reference (package importing itself).
    fn load_package_self(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // find the closest package scope to the directory
        let Some(package_json) = self.find_package_json(path, ctx)? else {
            return Ok(None);
        };

        // check if the package name matches the specifier
        if let Some(subpath) = package_json
            .content
            .name
            .as_ref()
            .and_then(|package_name| Self::strip_package_name(specifier, package_name.as_str()))
        {
            let package_url = package_json
                .path
                .parent()
                .unwrap_or_else(|| {
                    panic!(
                        "package.json path is not in a directory: {}",
                        package_json.path.display()
                    )
                })
                .to_path_buf();
            if let Some(exports) = package_json.content.exports.as_ref()
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
        self.load_browser_field(path, Some(specifier), &package_json, ctx)
    }

    /// Implements `RESOLVE_ESM_MATCH` (MATCH).
    fn resolve_esm_match(
        &self,
        specifier: &str,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // if the file at path exists, load it as its extension format
        // non-compliant ESM can result in a directory, so directory is tried as well.
        if let Some(path) = self.load_as_file_or_directory(path, "", ctx)? {
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
        package_json: &'a PackageOptions,
        path: &Path,
        request: Option<&str>,
    ) -> Result<Option<&'a str>, ResolveError> {
        if let Some(object) = package_json
            .content
            .browser
            .as_ref()
            .and_then(|v| v.as_object())
        {
            // find matching key in object
            if let Some(request) = request {
                if let Some(value) = object.get(request) {
                    return match value {
                        serde_json::Value::String(s) => Ok(Some(s.as_str())),
                        serde_json::Value::Bool(false) => Err(ResolveError::Ignored {
                            path: path.to_path_buf(),
                        }),
                        _ => Ok(None),
                    };
                }
            }
            // find first matching key
            else {
                let directory = package_json.path.parent().unwrap_or_else(|| {
                    panic!(
                        "package.json path is not in a directory: {}",
                        package_json.path.display()
                    )
                });
                for (key, value) in object {
                    let joined = directory.normalize_with(key.as_str());
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

        // not found
        Ok(None)
    }

    /// Resolves the browser field or alias field in the package.json.
    fn load_browser_field(
        &self,
        path: &Path,
        module_specifier: Option<&str>,
        package_json: &PackageOptions,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if ctx.is_fully_specified {
            return Ok(None);
        }

        // bail if there is no new browser specifier
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
                return if self.is_file(path, ctx) {
                    if self.check_restrictions(path) {
                        Ok(Some(path.to_path_buf()))
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

        // resolve alias
        ctx.resolving_alias = Some(new_specifier.to_string());
        ctx.is_fully_specified = false;
        let package_url = package_json.path.parent().unwrap().to_path_buf();
        self.require(&package_url, new_specifier, ctx).map(Some)
    }

    /// Resolves aliases and fallbacks from the options.
    fn load_alias(
        &self,
        path: &Path,
        specifier: &str,
        aliases: &Alias,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        for (alias_key_raw, specifiers) in aliases {
            let mut alias_key_has_wildcard = false;
            let alias_key = {
                // exact match
                if let Some(alias_key) = alias_key_raw.strip_suffix('$') {
                    if alias_key != specifier {
                        continue;
                    }
                    alias_key
                }
                // wildcard pattern match
                else if alias_key_raw.contains('*') {
                    alias_key_has_wildcard = true;
                    alias_key_raw
                }
                // directory pattern match
                else {
                    let strip_package_name = Self::strip_package_name(specifier, alias_key_raw);
                    if strip_package_name.is_none() {
                        continue;
                    }
                    alias_key_raw
                }
            };

            // it should stop resolving when all of the tried alias values failed to resolve.
            let mut should_stop = false;
            for r in specifiers {
                match r {
                    AliasValue::Path(alias_value) => {
                        if let Some(path) = self.load_alias_value(
                            path,
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
                        let ignored_path = path.normalize_with(alias_key);
                        return Err(ResolveError::Ignored { path: ignored_path });
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

    /// Resolve an alias value by substituting the matched portion and resolving.
    fn load_alias_value(
        &self,
        path: &Path,
        alias_key: &str,
        alias_key_has_wildcard: bool,
        alias_value: &str,
        request: &str,
        ctx: &mut ResolutionContext,
        should_stop: &mut bool,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // skip if request matches alias_value exactly or is a subpath of it
        if request == alias_value
            || request
                .strip_prefix(alias_value)
                .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return Ok(None);
        }

        // build the new specifier by substituting the alias
        let new_specifier = if alias_key_has_wildcard {
            // wildcard alias: `@/*` -> `./src/*`
            let Some(matched) = alias_key.split_once('*').and_then(|(prefix, suffix)| {
                request
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(suffix))
            }) else {
                return Ok(None);
            };

            // substitute wildcard in alias value if present
            if alias_value.contains('*') {
                Cow::Owned(alias_value.replacen('*', matched, 1))
            } else {
                Cow::Borrowed(alias_value)
            }
        }
        // non-wildcard alias: concatenate tail
        else {
            let tail = &request[alias_key.len()..];
            if tail.is_empty() {
                Cow::Borrowed(alias_value)
            } else {
                let alias_path = Path::new(alias_value).normalize();
                // don't append tail if alias_value is already a file
                if self.is_file(&alias_path, ctx) {
                    return Ok(None);
                }
                // strip leading slash and normalize
                let tail = tail.trim_start_matches(SLASH_START);
                if tail.is_empty() {
                    Cow::Borrowed(alias_value)
                } else {
                    let normalized = alias_path.normalize_with(tail);
                    Cow::Owned(normalized.to_string_lossy().to_string())
                }
            }
        };

        // resolve the substituted specifier
        *should_stop = true;
        ctx.is_fully_specified = false;
        match self.require(path, new_specifier.as_ref(), ctx) {
            Ok(resolved) => Ok(Some(resolved)),
            Err(ResolveError::NotFound { .. } | ResolveError::MatchedAliasNotFound { .. }) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Try to resolve via extension alias (e.g., mapping `.js` to `.ts`).
    fn load_with_extension_alias(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // no extension alias configured or found
        if self.options.extension_alias.is_empty() {
            return Ok(None);
        }
        let Some(path_extension) = path.extension() else {
            return Ok(None);
        };
        let Some(file_name) = path.file_name() else {
            return Ok(None);
        };
        let Some(path_extension_str) = path_extension.to_str() else {
            return Ok(None);
        };

        // get the extension alias mapping
        let extension_key = format!(".{path_extension_str}");
        let Some(extensions) = self.options.extension_alias.get(&extension_key) else {
            return Ok(None);
        };

        ctx.is_fully_specified = true;
        for extension in extensions {
            // extension has leading dot (e.g., ".ts"), but with_extension needs without dot
            let extension = extension.strip_prefix('.').unwrap_or(extension);
            let path_with_ext = path.with_extension(extension);
            if let Some(result) = self.load_alias_or_file(&path_with_ext, ctx)? {
                ctx.is_fully_specified = false;
                return Ok(Some(result));
            }
        }

        // bail if path is module directory (like `ipaddr.js`)
        if !self.is_file(path, ctx) {
            ctx.is_fully_specified = false;
            return Ok(None);
        } else if !self.check_restrictions(path) {
            return Ok(None);
        }

        // error
        let dir = path.parent().unwrap().to_path_buf();
        let filename_without_extension = Path::new(file_name).with_extension("");
        let filename_without_extension = filename_without_extension.to_string_lossy();
        let files = extensions
            .iter()
            .map(|ext| format!("{filename_without_extension}{ext}"))
            .collect::<Vec<_>>()
            .join(",");
        Err(ResolveError::ExtensionAliasNotFound {
            filename: file_name.to_string_lossy().to_string(),
            tried: files,
            dir,
        })
    }

    /// Resolves server-relative URLs using the configured roots.
    /// Resolve a specifier against configured root directories.
    ///
    /// Root directories allow absolute-style imports (starting with `/`) to resolve
    /// relative to project roots rather than the filesystem root.
    fn load_roots(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Option<PathBuf> {
        // bail if no roots configured
        if self.options.roots.is_empty() {
            return None;
        }

        // only handle specifiers starting with `/`
        let relative_specifier = specifier.strip_prefix(SLASH_START)?;

        // bare `/` resolves to the current directory if it's a root
        if relative_specifier.is_empty() {
            let is_root = self.options.roots.iter().any(|root| root.as_path() == path);
            if is_root && let Ok(resolved) = self.require_relative(path, "./", ctx) {
                return Some(resolved);
            }
        }
        // `/path` tries each root directory in order
        else {
            for root in &self.options.roots {
                if let Ok(resolved) = self.require_relative(root, relative_specifier, ctx) {
                    return Some(resolved);
                }
            }
        }

        None
    }

    /// Load and parse a tsconfig.json file recursively, using the registry for caching.
    fn load_tsconfig(
        &self,
        is_root: bool,
        path: &Path,
        references: &TypeScriptOptionsReferences,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<TsConfigId, ResolveError> {
        // check if already in registry
        if let Some(tsconfig_id) = self.program.tsconfigs.get_id_by_path(path) {
            return Ok(tsconfig_id);
        }

        // get IDs for this tsconfig
        let tsconfig_id = self.program.tsconfigs.next_id();
        let file_id = self.program.files.next_id();

        // read and parse the tsconfig
        let mut tsconfig = self.read_tsconfig(tsconfig_id, file_id, is_root, path)?;

        if ctx.is_already_extended(&tsconfig.path) {
            return Err(ResolveError::TsConfigCircular {
                paths: ctx.get_extended_configs_with(tsconfig.path.to_path_buf()),
            });
        }

        // extend tsconfig
        let extended_tsconfig_paths = tsconfig
            .content
            .extends()
            .map(|specifier| {
                self.get_extended_tsconfig_path(&tsconfig.directory, &tsconfig, specifier)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !extended_tsconfig_paths.is_empty() {
            ctx.with_extended_file(tsconfig.path.to_owned(), |ctx| {
                for extended_tsconfig_path in extended_tsconfig_paths {
                    let extended_tsconfig_id = self.load_tsconfig(
                        false,
                        &extended_tsconfig_path,
                        &TypeScriptOptionsReferences::Disabled,
                        ctx,
                    )?;
                    let extended = self.program.tsconfigs.get(extended_tsconfig_id);
                    let extended_guard = extended.read();
                    tsconfig.extend_from(&extended_guard);
                }
                Result::Ok::<(), ResolveError>(())
            })?;
        }

        // load the given references into this tsconfig
        match references {
            TypeScriptOptionsReferences::Disabled => {
                tsconfig.content.references.drain(..);
            }
            TypeScriptOptionsReferences::Automatic => {}
            TypeScriptOptionsReferences::Paths(paths) => {
                tsconfig.content.references = paths
                    .iter()
                    .map(|path| TsConfigProjectReferences {
                        path: path.clone(),
                        tsconfig: None,
                    })
                    .collect();
            }
        }
        if !tsconfig.content.references.is_empty() {
            let current_path = tsconfig.path.to_path_buf();
            for reference in tsconfig.content.references.iter_mut() {
                // read the reference tsconfig
                let reference_tsconfig_path = tsconfig.directory.normalize_with(&reference.path);
                let reference_tsconfig_id = self.program.tsconfigs.next_id();
                let reference_file_id = self.program.files.next_id();
                let mut referenced_tsconfig = self.read_tsconfig(
                    reference_tsconfig_id,
                    reference_file_id,
                    true,
                    &reference_tsconfig_path,
                )?;

                // error if reference tsconfig points to itself
                if referenced_tsconfig.path == current_path {
                    return Err(ResolveError::TsConfigSelfReference {
                        path: referenced_tsconfig.path.to_path_buf(),
                    });
                }

                // extend the tsconfig with the reference tsconfig
                self.extend_tsconfig(
                    &referenced_tsconfig.directory.to_path_buf(),
                    &mut referenced_tsconfig,
                    ctx,
                )?;

                // insert the tsconfig
                let reference_tsconfig = Arc::new(referenced_tsconfig.build());
                self.program.tsconfigs.insert((*reference_tsconfig).clone());
                reference.set_tsconfig(reference_tsconfig);
            }
        }

        // store in registry
        let built = tsconfig.build();
        self.program.tsconfigs.insert(built);

        Ok(tsconfig_id)
    }

    /// Read and parse a tsconfig.json file.
    fn read_tsconfig(
        &self,
        tsconfig_id: TsConfigId,
        file_id: dyst_source::FileId,
        root: bool,
        path: &Path,
    ) -> Result<TsConfig, ResolveError> {
        // resolve path to actual tsconfig file
        let meta = self.fs().metadata(path).ok();
        let tsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_directory) {
            Cow::Owned(path.join("tsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };

        // read file
        let mut tsconfig_string = self.fs().read_to_string(&tsconfig_path).map_err(|_| {
            ResolveError::TsConfigNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // parse
        TsConfig::parse(
            tsconfig_id,
            file_id,
            root,
            &tsconfig_path,
            &mut tsconfig_string,
        )
        .map_err(|_| ResolveError::TsConfigInvalid {
            path: tsconfig_path.to_path_buf(),
        })
    }

    /// Extend a tsconfig with inherited configurations.
    fn extend_tsconfig(
        &self,
        directory: &Path,
        tsconfig: &mut TsConfig,
        ctx: &mut TypeScriptOptionsResolveContext,
    ) -> Result<(), ResolveError> {
        let extended_tsconfig_paths = tsconfig
            .content
            .extends()
            .map(|specifier| self.get_extended_tsconfig_path(directory, tsconfig, specifier))
            .collect::<Result<Vec<_>, _>>()?;
        for extended_tsconfig_path in extended_tsconfig_paths {
            let extended_tsconfig_id = self.load_tsconfig(
                false,
                &extended_tsconfig_path,
                &TypeScriptOptionsReferences::Disabled,
                ctx,
            )?;
            let extended = self.program.tsconfigs.get(extended_tsconfig_id);
            let extended_guard = extended.read();
            tsconfig.extend_from(&extended_guard);
        }
        Ok(())
    }

    /// Resolve the specifier using tsconfig `paths` configuration.
    fn load_tsconfig_paths(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if is_inside_modules(path) {
            return Ok(None);
        }

        let tsconfig_id = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TypeScriptOptionsDiscovery::Manual(tsconfig_options)) => self.load_tsconfig(
                true,
                &tsconfig_options.config_file,
                &tsconfig_options.references,
                &mut TypeScriptOptionsResolveContext::default(),
            )?,
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                let Some(tsconfig_id) = self.find_tsconfig(path, ctx)? else {
                    return Ok(None);
                };
                tsconfig_id
            }
        };

        let tsconfig = self.get_tsconfig(tsconfig_id);
        let paths = tsconfig.resolve(path, specifier);
        for resolved in paths {
            if let Some(resolution) = self.load_as_file_or_directory(&resolved, ".", ctx)? {
                return Ok(Some(resolution));
            }
        }
        Ok(None)
    }

    /// Find tsconfig.json of a path by traversing parent directories.
    pub(crate) fn find_tsconfig(
        &self,
        path: &Path,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<TsConfigId>, ResolveError> {
        // don't discover tsconfig for paths inside node_modules
        if is_inside_modules(path) {
            return Ok(None);
        }
        // skip non-absolute paths (e.g. virtual modules)
        if !path.is_absolute() {
            return Ok(None);
        }

        // walk up parent directories looking for tsconfig.json
        let mut current = Some(path.to_path_buf());
        while let Some(dir) = current {
            let tsconfig_path = dir.join("tsconfig.json");
            if self.is_file(&tsconfig_path, ctx) {
                let tsconfig_id = self.resolve_tsconfig(&tsconfig_path)?;
                return Ok(Some(tsconfig_id));
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
        Ok(None)
    }

    /// Resolves the path of an extended tsconfig file.
    fn get_extended_tsconfig_path(
        &self,
        directory: &Path,
        tsconfig: &TsConfig,
        specifier: &str,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            None => Err(ResolveError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: None,
            }),
            Some(b'/') => Ok(PathBuf::from(specifier)),
            Some(b'.') => Ok(tsconfig.directory.normalize_with(specifier)),
            _ => self
                .with_options(ResolveOptions {
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    ..ResolveOptions::default()
                })
                .load_package_self_or_modules(
                    directory,
                    specifier,
                    &mut ResolutionContext::default(),
                )
                .map(|p| p.to_path_buf())
                .map_err(|err| match err {
                    ResolveError::NotFound { .. } => ResolveError::TsConfigNotFound {
                        path: PathBuf::from(specifier),
                    },
                    _ => err,
                }),
        }
    }

    /// Resolve a bare package specifier by searching node_modules directories.
    fn package_resolve(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);

        // iterate over all possible node_modules directories
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

                // if the folder at packageURL does not exist, then continue
                if self.is_directory(&package_path, ctx) {
                    // load `package.json`
                    if let Some(package_json) = self.load_package_json(&package_path, ctx)? {
                        // resolve exports
                        if let Some(exports) = package_json.content.exports.as_ref()
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
                            && let Some(main_field) = package_json.content.main.as_deref()
                        {
                            let main_path = package_path.normalize_with(main_field);
                            if self.is_file(&main_path, ctx) && self.check_restrictions(&main_path)
                            {
                                return Ok(Some(main_path));
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
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        let conditions = &self.options.conditions;

        // validate exports
        // (cannot mix starting with "." and not starting with ".")
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
            package_path: package_url.to_path_buf(),
            package_json_path: package_url.join("package.json"),
            conditions: self.options.conditions.clone(),
        })
    }

    /// Resolve an imports specifier against package.json imports field.
    fn package_imports_resolve(
        &self,
        specifier: &str,
        package_json: &PackageOptions,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        debug_assert!(specifier.starts_with('#'), "{specifier}");

        // bail if no imports are configured
        let Some(imports) = package_json.content.imports.as_ref() else {
            return Ok(None);
        };

        // error if specifier is invalid
        if specifier == "#" || specifier.starts_with("#/") {
            return Err(ResolveError::InvalidModuleSpecifier {
                specifier: specifier.to_string(),
                package_path: package_json.path.to_path_buf(),
            });
        }

        // resolve imports
        if let Some(path) = self.package_imports_exports_resolve(
            specifier,
            imports,
            &package_json.directory,
            /* is_imports */ true,
            &self.options.conditions,
            ctx,
        )? {
            Ok(Some(path))
        } else {
            Err(ResolveError::PackageImportNotDefined {
                specifier: specifier.to_string(),
                package_path: package_json.path.to_path_buf(),
            })
        }
    }

    /// Resolve a key against an imports or exports mapping object.
    pub(crate) fn package_imports_exports_resolve(
        &self,
        match_key: &str,
        match_obj: &serde_json::Map<String, serde_json::Value>,
        package_url: &Path,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolutionContext,
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

        // pattern match
        // find the best matching key in the match object
        let mut best_target = None;
        let mut best_match = "";
        let mut best_key = "";
        for (expansion_key, target_key) in match_obj.iter() {
            // ignore invalid mappings (wildcard expansion without wildcard target)
            if expansion_key.ends_with('*') && target_key.as_str().is_some_and(|s| !s.contains('*'))
            {
                continue;
            }

            if expansion_key.starts_with("./") || expansion_key.starts_with('#') {
                // wildcard pattern match
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
                }
                // directory pattern match
                else if expansion_key.ends_with('/')
                    && match_key.starts_with(expansion_key)
                    && Self::pattern_key_compare(best_key, expansion_key).is_gt()
                {
                    best_target = Some(target_key);
                    best_match = &match_key[expansion_key.len()..];
                    best_key = expansion_key;
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
    #[allow(clippy::too_many_arguments)]
    fn package_target_resolve(
        &self,
        package_url: &Path,
        target_key: &str,
        target: &serde_json::Value,
        pattern_match: Option<&str>,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolutionContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        /// Normalizes a string target (like `./foo.js` or `../bar.js` or `#baz`).
        fn normalize_string_target<'a>(
            target_key: &'a str,
            target: &'a str,
            pattern_match: Option<&'a str>,
            package_url: &Path,
        ) -> Result<Cow<'a, str>, ResolveError> {
            let target = if let Some(pattern_match) = pattern_match {
                if !target_key.contains('*') && !target.contains('*') {
                    // enhanced-resolve behaviour
                    if target_key.ends_with('/') && target.ends_with('/') {
                        Cow::Owned(format!("{target}{pattern_match}"))
                    } else {
                        return Err(ResolveError::InvalidPackageConfigDirectory {
                            path: package_url.join("package.json"),
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
            // parse target
            let target = ModuleSpecifier::parse(target);
            if let Some(query) = &target.query {
                ctx.query.replace(query.to_string());
            }
            if let Some(fragment) = &target.fragment {
                ctx.fragment.replace(fragment.to_string());
            }
            let target = target.path();

            // path does not start with `./`
            if !target.starts_with("./") {
                // error if target is not a valid package target
                // (exports cannot start with `./`, and nothing can start with `/` or `../`)
                if !is_imports || target.starts_with("../") || target.starts_with('/') {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: (*target).to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.join("package.json"),
                    });
                }
                // normalize and resolve
                let target =
                    normalize_string_target(target_key, target, pattern_match, package_url)?;
                return self.package_resolve(package_url, &target, ctx);
            }
            // path starts with `./`
            else {
                // normalize target
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
            // error if no fallback targets are configured
            if targets.is_empty() {
                return Err(ResolveError::PackagePathNotExported {
                    subpath: pattern_match.unwrap_or(".").to_string(),
                    package_path: package_url.to_path_buf(),
                    package_json_path: package_url.join("package.json"),
                    conditions: self.options.conditions.clone(),
                });
            }
            // resolve the fallback targets
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

                // return the first successful path
                if let Ok(Some(path)) = resolved {
                    return Ok(Some(path));
                }
                // error if last (=all) fallback targets failed
                else if resolved.is_err() && i == targets.len() {
                    return resolved;
                }
            }
        }

        Ok(None)
    }

    /// Parse a package specifier into package name and subpath.
    ///
    /// Examples: `lodash` -> (`lodash`, ``), `@scope/pkg/file` -> (`@scope/pkg`, `/file`).
    fn parse_package_specifier(specifier: &str) -> (&str, &str) {
        // find first slash
        let mut separator_index = specifier.as_bytes().iter().position(|b| *b == b'/');

        // scoped packages have format `@scope/package-name/subpath`
        if specifier.starts_with('@') {
            // invalid: empty or no slash after scope
            if separator_index.is_none() || specifier.is_empty() {
                // fall through with no separator
            }
            // find second slash (end of package name)
            else if let Some(first_slash) = separator_index {
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

        // ensure pattern keys end with '/' or contain exactly one '*'
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

    /// Check if a tsconfig path has already been extended in this chain.
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
            .map_err(|()| ResolveError::UnsupportedPath {
                path: PathBuf::from(specifier),
            })
    } else {
        Ok(Cow::Borrowed(specifier))
    }
}
