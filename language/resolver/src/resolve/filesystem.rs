use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use destack_source::strip_windows_prefix;
use destack_source::{FileMetadata, PathExt};

use crate::{
    Resolver, ResolverContext, ResolverError, ResolverResult, ResolverSource, Restriction,
};

/// The identity hasher for hash sets with precomputed hashes.
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

impl Resolver {
    /// Check if one path lies inside one restricted path.
    fn is_in_restricted_path(path: &Path, parent: &Path) -> bool {
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
                .is_ok_and(|path| path == Path::new("./"))
        }
    }

    /// Append an extension to a path (e.g., `foo` + `.js` = `foo.js`).
    pub(crate) fn append_extension(path: &Path, extension: &str) -> PathBuf {
        let mut os_string = path.as_os_str().to_os_string();
        os_string.push(extension);
        PathBuf::from(os_string)
    }

    /// Normalize one Windows path and reject unsupported DOS device forms.
    #[cfg(target_os = "windows")]
    fn normalize_windows_path(path: &Path) -> ResolverResult<PathBuf> {
        let normalized = path.normalize();
        strip_windows_prefix(normalized.clone())
            .map(|path| path.normalize())
            .map_err(|_| ResolverError::UnsupportedPath { path: normalized })
    }

    /// Read symlink metadata from one path.
    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.fs().symlink_metadata(path)
    }

    /// Canonicalize one path.
    pub(crate) fn canonicalize_path(&self, path: &Path) -> io::Result<PathBuf> {
        self.fs().canonicalize(path)
    }

    /// Resolve one symbolic link target path.
    pub(crate) fn resolve_symlink_path(&self, path: &Path) -> io::Result<PathBuf> {
        self.fs().resolve_symlink(path)
    }

    /// Canonicalize a path, resolving all symlinks.
    pub(crate) fn canonicalize(&self, path: &Path) -> ResolverResult<PathBuf> {
        // track visited paths for circular symlink detection
        let mut visited = HashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());
        let result = match self.canonicalize_recursive(path, &mut visited) {
            Ok(result) => result,

            // keep unsupported windows path semantics intact
            Err(error @ ResolverError::UnsupportedPath { .. }) => return Err(error),

            // preserve direct canonicalization semantics
            Err(error) => self.canonicalize_path(path).map_err(|_| error)?,
        };

        #[cfg(target_os = "windows")]
        let result = Self::normalize_windows_path(&result)?;

        Ok(result)
    }

    /// Canonicalize a path with one visited set for cycle detection.
    fn canonicalize_recursive(
        &self,
        path: &Path,
        visited: &mut HashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> ResolverResult<PathBuf> {
        // use a hash of the path for cycle detection
        let hash = {
            use std::hash::Hasher as _;
            let mut hasher = rustc_hash::FxHasher::default();
            path.hash(&mut hasher);
            hasher.finish()
        };
        if !visited.insert(hash) {
            return Err(ResolverError::RecursiveDependency { depth: 0 });
        }

        // canonicalize the current path through its parent chain
        let result = if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            self.canonicalize_recursive(parent, visited)
                .and_then(|parent_canonical| {
                    let child_path =
                        path.strip_prefix(parent)
                            .map_err(|_| ResolverError::UnsupportedPath {
                                path: path.to_path_buf(),
                            })?;
                    let normalized = parent_canonical.normalize_with(child_path);

                    #[cfg(target_os = "windows")]
                    let normalized = Self::normalize_windows_path(&normalized)?;

                    // follow symlink targets explicitly to match oxc semantics on windows
                    if self
                        .symlink_metadata(path)
                        .is_ok_and(|metadata| metadata.is_symlink)
                    {
                        let link = self.resolve_symlink_path(path).map_err(|error| {
                            #[cfg(target_os = "windows")]
                            if error.kind() == io::ErrorKind::InvalidInput {
                                return ResolverError::UnsupportedPath {
                                    path: normalized.clone(),
                                };
                            }

                            ResolverError::IoError {
                                path: normalized.clone(),
                                kind: error.kind(),
                            }
                        })?;

                        // absolute symlink target
                        if link.is_absolute() {
                            let link = link.normalize();
                            #[cfg(target_os = "windows")]
                            let link = Self::normalize_windows_path(&link)?;
                            return self.canonicalize_recursive(&link, visited);
                        }

                        // relative symlink target
                        if let Some(directory) = normalized.parent() {
                            let link = directory.normalize_with(&link);
                            #[cfg(target_os = "windows")]
                            let link = Self::normalize_windows_path(&link)?;
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

    /// Normalize a root path without any platform specific rewrite.
    #[cfg(not(target_os = "windows"))]
    fn normalize_root(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    /// Finalize one resolved path by applying symlink canonicalization if configured.
    pub(crate) fn finalize_path(
        &self,
        path: &Path,
        _ctx: &ResolverContext,
    ) -> ResolverResult<PathBuf> {
        if self.source == ResolverSource::Revision {
            return Ok(Self::normalize_root(path));
        }

        if self.options.canonicalize_symlinks {
            self.canonicalize(path)
        } else {
            Ok(Self::normalize_root(path))
        }
    }

    /// Check if a resolved path passes all configured restrictions.
    pub(crate) fn check_restrictions(&self, path: &Path) -> bool {
        // check all restrictions
        for restriction in &self.options.restrictions {
            match restriction {
                Restriction::Path(restricted_path) => {
                    if !Self::is_in_restricted_path(path, restricted_path) {
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
