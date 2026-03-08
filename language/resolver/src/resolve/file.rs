use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::io;
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "windows")]
use destack_source::strip_windows_prefix;
use destack_source::{FileMetadata, PathExt};

#[cfg(not(target_arch = "wasm32"))]
use pnp::fs::{VPath, VPathInfo, ZipCache};

use crate::{ResolveContext, ResolveError, Resolver, Restriction};

// thread-local pre-allocated path buffer for path manipulation
thread_local! {
    pub(crate) static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
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

/// Check if a path is inside a modules directory (node_modules).
#[inline]
pub(crate) fn is_inside_modules(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, Component::Normal(name) if name == "node_modules"))
}

/// Append an extension to a path (e.g., `foo` + `.js` = `foo.js`).
pub(crate) fn append_extension(path: &Path, extension: &str) -> PathBuf {
    SCRATCH_PATH.with_borrow_mut(|scratch| {
        scratch.clear();
        let os_string = scratch.as_mut_os_string();
        os_string.push(path.as_os_str());
        os_string.push(extension);
        scratch.clone()
    })
}

impl Resolver {
    /// Normalize one Windows path and reject unsupported DOS device forms.
    #[cfg(target_os = "windows")]
    fn normalize_windows_path(path: &Path) -> Result<PathBuf, ResolveError> {
        let normalized = path.normalize();
        strip_windows_prefix(normalized.clone())
            .map(|path| path.normalize())
            .map_err(|_| ResolveError::UnsupportedPath { path: normalized })
    }

    /// Read a path as bytes, with optional Yarn PnP virtual/zip support.
    pub(crate) fn read_path(&self, path: &Path) -> io::Result<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => self
                    .state
                    .pnp_lru
                    .read(info.physical_base_path(), info.zip_path.as_str()),
                VPath::Virtual(info) => self.read_path(&info.physical_base_path()),
                VPath::Native(_) => self.fs().read(path),
            };
        }

        self.fs().read(path)
    }

    /// Read a path as UTF-8 text, with optional Yarn PnP virtual/zip support.
    pub(crate) fn read_path_to_string(&self, path: &Path) -> io::Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => self
                    .state
                    .pnp_lru
                    .read_to_string(info.physical_base_path(), info.zip_path.as_str()),
                VPath::Virtual(info) => self.read_path_to_string(&info.physical_base_path()),
                VPath::Native(_) => self.fs().read_to_string(path),
            };
        }

        let bytes = self.read_path(path)?;
        destack_source::validate_utf8_string(bytes)
    }

    /// Read metadata from one path, with optional Yarn PnP virtual/zip support.
    pub(crate) fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => {
                    let file_type = self
                        .state
                        .pnp_lru
                        .file_type(info.physical_base_path(), info.zip_path.as_str())?;
                    Ok(match file_type {
                        pnp::fs::FileType::File => FileMetadata::new(true, false, false, 0, None),
                        pnp::fs::FileType::Directory => {
                            FileMetadata::new(false, true, false, 0, None)
                        }
                    })
                }
                VPath::Virtual(info) => self.metadata(&info.physical_base_path()),
                VPath::Native(_) => self.fs().metadata(path),
            };
        }

        self.fs().metadata(path)
    }

    /// Read symlink metadata from one path, with optional Yarn PnP virtual/zip support.
    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => {
                    let file_type = self
                        .state
                        .pnp_lru
                        .file_type(info.physical_base_path(), info.zip_path.as_str())?;
                    Ok(match file_type {
                        pnp::fs::FileType::File => FileMetadata::new(true, false, false, 0, None),
                        pnp::fs::FileType::Directory => {
                            FileMetadata::new(false, true, false, 0, None)
                        }
                    })
                }
                VPath::Virtual(info) => self.symlink_metadata(&info.physical_base_path()),
                VPath::Native(_) => self.fs().symlink_metadata(path),
            };
        }

        self.fs().symlink_metadata(path)
    }

    /// Canonicalize one path, with optional Yarn PnP virtual/zip support.
    pub(crate) fn canonicalize_path(&self, path: &Path) -> io::Result<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => self
                    .fs()
                    .canonicalize(&info.physical_base_path())
                    .map(|base| base.join(info.zip_path)),
                VPath::Virtual(info) => self.canonicalize_path(&info.physical_base_path()),
                VPath::Native(path) => self.fs().canonicalize(&path),
            };
        }

        self.fs().canonicalize(path)
    }

    /// Resolve one symbolic link target path, with optional Yarn PnP virtual/zip support.
    pub(crate) fn resolve_symlink_path(&self, path: &Path) -> io::Result<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp {
            return match VPath::from(path)? {
                VPath::Zip(info) => self
                    .fs()
                    .resolve_symlink(&info.physical_base_path().join(info.zip_path)),
                VPath::Virtual(info) => self.resolve_symlink_path(&info.physical_base_path()),
                VPath::Native(path) => self.fs().resolve_symlink(&path),
            };
        }

        self.fs().resolve_symlink(path)
    }

    /// Check if a path is a file.
    #[inline]
    pub(crate) fn is_file(&self, path: &Path, ctx: &mut ResolveContext) -> bool {
        match self.metadata(path) {
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
    pub(crate) fn is_directory(&self, path: &Path, ctx: &mut ResolveContext) -> bool {
        match self.metadata(path) {
            Ok(meta) if meta.is_directory => {
                ctx.track_found_dependency(path);
                true
            }
            _ => {
                ctx.track_missing_dependency(path);
                false
            }
        }
    }

    /// Canonicalize a path, resolving all symlinks.
    pub(crate) fn canonicalize(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        // track visited paths for circular symlink detection
        let mut visited = HashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());
        let result = self
            .canonicalize_recursive(path, &mut visited)
            .or_else(|err| {
                // fallback: try direct FS canonicalize
                self.canonicalize_path(path).map_err(|_| err)
            })?;

        #[cfg(target_os = "windows")]
        let result = Self::normalize_windows_path(&result)?;

        Ok(result)
    }

    /// Canonicalize a path with a set of already-visited paths for cycle detection.
    fn canonicalize_recursive(
        &self,
        path: &Path,
        visited: &mut HashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> Result<PathBuf, ResolveError> {
        // use a hash of the path for cycle detection
        let hash = {
            use std::hash::Hasher as _;
            let mut hasher = rustc_hash::FxHasher::default();
            path.hash(&mut hasher);
            hasher.finish()
        };
        if !visited.insert(hash) {
            return Err(ResolveError::RecursiveDependency { depth: 0 });
        }

        // canonicalize the current path through its parent chain
        let result = if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            self.canonicalize_recursive(parent, visited)
                .and_then(|parent_canonical| {
                    let normalized = parent_canonical
                        .normalize_with(path.strip_prefix(parent).unwrap_or(Path::new("")));

                    // follow symlink targets explicitly to match oxc semantics on windows
                    if self
                        .symlink_metadata(path)
                        .is_ok_and(|metadata| metadata.is_symlink)
                    {
                        let link = self.resolve_symlink_path(&normalized).map_err(|error| {
                            #[cfg(target_os = "windows")]
                            if error.kind() == io::ErrorKind::InvalidInput {
                                return ResolveError::UnsupportedPath {
                                    path: normalized.clone(),
                                };
                            }

                            ResolveError::IoError {
                                path: normalized.clone(),
                                kind: error.kind(),
                            }
                        })?;

                        // absolute symlink target
                        if link.is_absolute() {
                            let link = link.normalize();

                            return self.canonicalize_recursive(&link, visited);
                        }

                        // relative symlink target
                        if let Some(directory) = normalized.parent() {
                            let link = directory.normalize_with(&link);

                            return self.canonicalize_recursive(&link, visited);
                        }
                    }

                    Ok(normalized)
                })
        } else {
            Ok(path.to_path_buf())
        };

        // remove this path from the current chain while unwinding recursion
        visited.remove(&hash);
        result
    }

    /// Normalize a root path to remove extended path prefix on Windows.
    #[cfg(target_os = "windows")]
    fn normalize_root(path: &Path) -> PathBuf {
        path.normalize()
    }

    #[cfg(not(target_os = "windows"))]
    fn normalize_root(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    /// Load the real path (resolving symlinks if needed).
    pub(crate) fn load_realpath(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        if self.options.canonicalize_symlinks {
            self.canonicalize(path)
        } else {
            Ok(Self::normalize_root(path))
        }
    }

    /// Check if a resolved path passes all configured restrictions.
    pub(crate) fn check_restrictions(&self, path: &Path) -> bool {
        /// Check if a path is inside a restricted path.
        /// See <https://github.com/webpack/enhanced-resolve/blob/a998c7d218b7a9ec2461fc4fddd1ad5dd7687485/lib/RestrictionsPlugin.js#L19-L24>
        fn is_in_restricted(path: &Path, parent: &Path) -> bool {
            // not a prefix
            if !path.starts_with(parent) {
                false
            }
            // exact path match
            else if path.as_os_str().len() == parent.as_os_str().len() {
                true
            }
            // relative path match
            else {
                path.strip_prefix(parent)
                    .is_ok_and(|p| p == Path::new("./"))
            }
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
}
