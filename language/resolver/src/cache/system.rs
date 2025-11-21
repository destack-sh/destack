use std::borrow::Cow;
use std::collections::HashSet as StdHashSet;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cfg_if::cfg_if;
use papaya::{HashMap, HashSet};
use rustc_hash::FxHasher;

use super::hasher::IdentityHasher;
use super::path::{BorrowedCachedPath, CachedPath, CachedPathState};
use crate::{JSONError, ResolutionContext, ResolveError, ResolveOptions};
use dyst_dir::{PackageJson, TsConfigJson};
use dyst_source::{FileSystem, PathExt};

/// A cached file system implementation.
#[derive(Debug, Default)]
pub struct CachedFileSystem<Fs> {
    /// The underlying file system.
    pub(crate) fs: Fs,
    /// The cached paths.
    pub(crate) paths: HashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    /// The cached tsconfigs.
    pub(crate) tsconfigs: HashMap<PathBuf, Arc<TsConfigJson>, BuildHasherDefault<FxHasher>>,
}

impl<Fs: FileSystem> CachedFileSystem<Fs> {
    /// Clears the caches.
    pub fn clear(&self) {
        self.paths.pin().clear();
        self.tsconfigs.pin().clear();
    }

    /// Gets the cached path for a given path.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn value(&self, path: &Path) -> CachedPath {
        // calculate hash
        // `Path::hash` is slow: https://doc.rust-lang.org/std/path/struct.Path.html#impl-Hash-for-Path
        // `path.as_os_str()` hash is not stable because we may joined a path like `foo/bar` and `foo\\bar` on windows.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };

        // check cache
        let paths = self.paths.pin();
        if let Some(entry) = paths.get(&BorrowedCachedPath { hash, path }) {
            return entry.clone();
        }

        // create new cached path
        let parent = path.parent().map(|p| self.value(p));
        let is_node_modules = path
            .file_name()
            .as_ref()
            .is_some_and(|&name| name == "node_modules");
        let inside_node_modules = is_node_modules
            || parent
                .as_ref()
                .is_some_and(|parent| parent.is_inside_node_modules);
        let parent_weak = parent.as_ref().map(|p| Arc::downgrade(&p.0));

        let cached_path = CachedPath(Arc::new(CachedPathState::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            is_node_modules,
            inside_node_modules,
            parent_weak,
        )));

        paths.insert(cached_path.clone());
        cached_path
    }

    /// Canonicalizes the cached path.
    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let cached_path = self.canonicalize_impl(path)?;
        let path = cached_path.to_path_buf();

        cfg_if! {
            if #[cfg(target_os = "windows")] {
                dyst_source::file::windows::strip_windows_prefix(path)
            } else {
                Ok(path)
            }
        }
    }

    /// Checks if the cached path is a file.
    pub(crate) fn is_file(&self, path: &CachedPath, ctx: &mut ResolutionContext) -> bool {
        if path.is_file(&self.fs).is_some_and(|b| b) {
            ctx.add_found_dependency_maybe(path.path());
            true
        } else {
            ctx.add_missing_dependency_maybe(path.path());
            false
        }
    }

    /// Checks if the cached path is a directory.
    pub(crate) fn is_directory(&self, path: &CachedPath, ctx: &mut ResolutionContext) -> bool {
        path.is_directory(&self.fs).map_or_else(
            || {
                ctx.add_missing_dependency_maybe(path.path());
                false
            },
            |b| b,
        )
    }

    /// Gets the `package.json` of the path.
    pub(crate) fn get_package_json(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut ResolutionContext,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        let result = path
            .package_json
            .get_or_try_init(|| {
                let package_json_path = path.path.join("package.json");
                let Ok(package_json_bytes) = self.fs.read(&package_json_path) else {
                    return Ok(None);
                };

                let real_path = if options.canonicalize_symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };

                PackageJson::parse(package_json_path.clone(), real_path, package_json_bytes)
                    .map(|package_json| Some(Arc::new(package_json)))
                    .map_err(|error| ResolveError::Json {
                        error: JSONError {
                            path: package_json_path,
                            message: error.to_string(),
                            line: error.line(),
                            column: error.column(),
                        },
                    })
            })
            .cloned();

        // track dependencies
        // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
        match &result {
            Ok(Some(package_json)) => {
                ctx.add_found_dependency_maybe(&package_json.path);
            }
            Ok(None) => {
                if let Some(deps) = &mut ctx.missing_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
            Err(_) => {
                if let Some(deps) = &mut ctx.found_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
        }

        result
    }

    /// Gets the `tsconfig.json` of the path.
    pub(crate) fn get_tsconfig_json<F: FnOnce(&mut TsConfigJson) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        modify: F,
    ) -> Result<Arc<TsConfigJson>, ResolveError> {
        // check cache
        let tsconfigs = self.tsconfigs.pin();
        if let Some(tsconfig) = tsconfigs.get(path) {
            return Ok(Arc::clone(tsconfig));
        }

        // resolve path
        let meta = self.fs.metadata(path).ok();
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
        let mut tsconfig_string = self.fs.read_to_string(&tsconfig_path).map_err(|_| {
            ResolveError::TypeScriptOptionsNotFound {
                path: path.to_path_buf(),
            }
        })?;

        // parse
        let mut tsconfig = TsConfigJson::parse(root, &tsconfig_path, &mut tsconfig_string)
            .map_err(|error| ResolveError::Json {
                error: JSONError {
                    path: tsconfig_path.to_path_buf(),
                    message: error.to_string(),
                    line: error.line(),
                    column: error.column(),
                },
            })?;

        modify(&mut tsconfig)?;

        let tsconfig = Arc::new(tsconfig.build());
        tsconfigs.insert(path.to_path_buf(), Arc::clone(&tsconfig));
        Ok(tsconfig)
    }
}

impl<Fs: FileSystem> CachedFileSystem<Fs> {
    /// Creates a new cached file system.
    pub fn new(fs: Fs) -> Self {
        Self {
            fs,
            paths: HashSet::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            tsconfigs: HashMap::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
        }
    }

    /// Returns the canonical path, resolving all symbolic links.
    ///
    /// <https://github.com/parcel-bundler/parcel/blob/4d27ec8b8bd1792f536811fef86e74a31fa0e704/crates/parcel-resolver/src/cache.rs#L232>
    pub(crate) fn canonicalize_impl(&self, path: &CachedPath) -> Result<CachedPath, ResolveError> {
        // track visited paths for circular symlink detection
        // (each canonicalization chain gets its own visited set for circular symlink detection)
        let mut visited = StdHashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());
        self.canonicalize_with_visited(path, &mut visited)
            .or_else(|err| {
                // Fallback: if canonicalization fails and path's cache was cleared,
                // try direct FS canonicalize without caching the result
                self.fs
                    .canonicalize(path.path())
                    .map(|canonical| self.value(&canonical))
                    .map_err(|_| err)
            })
    }

    /// Internal helper for canonicalization with circular symlink detection.
    fn canonicalize_with_visited(
        &self,
        path: &CachedPath,
        visited: &mut StdHashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> Result<CachedPath, ResolveError> {
        // check cache first
        if let Some(weak) = path.canonicalized_path.get() {
            return weak.upgrade().map(CachedPath).ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Cached path no longer exists").into()
            });
        }

        // check for circular symlink by tracking visited paths in the current canonicalization chain
        if !visited.insert(path.hash) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        // resolve parent and normalize
        let cached_path = path.parent().map_or_else(
            || Ok(path.normalize_root(self)),
            |parent| {
                self.canonicalize_with_visited(&parent, visited)
                    .and_then(|parent_canonical| {
                        let normalized = parent_canonical
                            .normalize_with(path.path().strip_prefix(parent.path()).unwrap(), self);

                        if self
                            .fs
                            .symlink_metadata(path.path())
                            .is_ok_and(|m| m.is_symlink)
                        {
                            let link = self.fs.read_link(normalized.path())?;
                            if link.is_absolute() {
                                return self.canonicalize_with_visited(
                                    &self.value(&link.normalize()),
                                    visited,
                                );
                            } else if let Some(dir) = normalized.parent() {
                                // use the path directory to resolve relative symlink
                                return self.canonicalize_with_visited(
                                    &dir.normalize_with(&link, self),
                                    visited,
                                );
                            }
                            debug_assert!(
                                false,
                                "Failed to get path parent for {}.",
                                normalized.path().display()
                            );
                        }

                        Ok(normalized)
                    })
            },
        )?;

        // cache the result before removing from visited set
        let _ = path.canonicalized_path.set(Arc::downgrade(&cached_path.0));
        visited.remove(&path.hash);

        Ok(cached_path)
    }
}
