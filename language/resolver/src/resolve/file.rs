use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::path::{Component, Path, PathBuf};

use dyst_source::{FileSystem, PathExt};

use crate::{ResolutionContext, ResolveError, Resolver, Restriction};

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
    /// Get the filesystem.
    #[inline]
    pub(crate) fn fs(&self) -> &dyn FileSystem {
        self.program.fs.as_ref()
    }

    /// Check if a path is a file.
    #[inline]
    pub(crate) fn is_file(&self, path: &Path, ctx: &mut ResolutionContext) -> bool {
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
    pub(crate) fn is_directory(&self, path: &Path, ctx: &mut ResolutionContext) -> bool {
        match self.fs().metadata(path) {
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
                self.fs().canonicalize(path).map_err(|_| err)
            })?;

        #[cfg(target_os = "windows")]
        let result = Self::normalize_root(&result);

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

        // try to get parent and canonicalize recursively
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            self.canonicalize_recursive(parent, visited)
                .and_then(|parent_canonical| {
                    let normalized = parent_canonical
                        .normalize_with(path.strip_prefix(parent).unwrap_or(Path::new("")));

                    // follow symlinks if the filesystem supports it
                    if self.fs().symlink_metadata(path).is_ok_and(|m| m.is_symlink) {
                        // try to canonicalize via the filesystem directly
                        if let Ok(canonical) = self.fs().canonicalize(&normalized) {
                            return self.canonicalize_recursive(&canonical, visited);
                        }
                    }

                    Ok(normalized)
                })
        } else {
            Ok(path.to_path_buf())
        }
    }

    /// Normalize a root path to remove extended path prefix on Windows.
    #[cfg(target_os = "windows")]
    fn normalize_root(path: &Path) -> PathBuf {
        const VERBATIM: &str = r"\\?\";
        let path_str = path.to_string_lossy();
        if path_str.starts_with(VERBATIM) {
            PathBuf::from(&path_str[VERBATIM.len()..])
        } else {
            path.to_path_buf()
        }
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
