use std::cell::RefCell;
use std::convert::AsRef;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};

use cfg_if::cfg_if;
use dyst_source::FileSystem;
use papaya::Equivalent;

use super::system::CachedFileSystem;
use crate::{PackageJson, ResolveContext, ResolveError, ResolveOptions, TsConfig};

// Per-thread pre-allocated path that is used to perform operations on paths more quickly.
// <https://github.com/parcel-bundler/parcel/blob/a53f8f3ba1025c7ea8653e9719e0a61ef9717079/crates/parcel-resolver/src/cache.rs#L394>
thread_local! {
    pub static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
}

/// Cached path.
#[derive(Clone)]
pub struct CachedPath(pub Arc<CachedPathState>);

/// Cached path implementation.
#[derive(Debug)]
pub struct CachedPathState {
    pub hash: u64,
    pub path: Box<Path>,
    pub parent: Option<Weak<CachedPathState>>,
    pub is_node_modules: bool,
    pub is_inside_node_modules: bool,

    pub meta: OnceLock<Option<(/* is_file */ bool, /* is_dir */ bool)>>, // None means not found.
    pub canonicalized_path: OnceLock<Weak<CachedPathState>>,
    pub node_modules: OnceLock<Option<Weak<CachedPathState>>>,
    pub package_json: OnceLock<Option<Arc<PackageJson>>>,
    pub tsconfig: OnceLock<Option<Arc<TsConfig>>>,
}

impl CachedPathState {
    pub fn new(
        hash: u64,
        path: Box<Path>,
        is_node_modules: bool,
        inside_node_modules: bool,
        parent: Option<Weak<Self>>,
    ) -> Self {
        Self {
            hash,
            path,
            parent,
            is_node_modules,
            is_inside_node_modules: inside_node_modules,
            meta: OnceLock::new(),
            canonicalized_path: OnceLock::new(),
            node_modules: OnceLock::new(),
            package_json: OnceLock::new(),
            tsconfig: OnceLock::new(),
        }
    }
}

impl Deref for CachedPath {
    type Target = CachedPathState;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl CachedPath {
    /// Get the path of the cached path.
    pub(crate) fn path(&self) -> &Path {
        &self.0.path
    }

    /// Convert the cached path to a path buffer.
    pub(crate) fn to_path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    /// Get the parent of the cached path.
    pub(crate) fn parent(&self) -> Option<Self> {
        self.0
            .parent
            .as_ref()
            .and_then(|weak| weak.upgrade().map(CachedPath))
    }

    /// Check if the cached path is a node_modules directory.
    pub(crate) fn is_node_modules(&self) -> bool {
        self.is_node_modules
    }

    /// Check if the cached path is inside a node_modules directory.
    pub(crate) fn inside_node_modules(&self) -> bool {
        self.is_inside_node_modules
    }

    /// Get the module directory of the cached path.
    pub(crate) fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &CachedFileSystem<Fs>,
        ctx: &mut ResolveContext,
    ) -> Option<Self> {
        let cached_path = cache.value(&self.path.join(module_name));
        cache.is_directory(&cached_path, ctx).then_some(cached_path)
    }

    /// Get the cached node_modules directory.
    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &CachedFileSystem<Fs>,
        ctx: &mut ResolveContext,
    ) -> Option<Self> {
        self.node_modules
            .get_or_init(|| {
                self.module_directory("node_modules", cache, ctx)
                    .map(|cp| Arc::downgrade(&cp.0))
            })
            .as_ref()
            .and_then(|weak| weak.upgrade().map(CachedPath))
    }

    /// Find package.json of a path by traversing parent directories.
    pub(crate) fn find_package_json<Fs: FileSystem>(
        &self,
        options: &ResolveOptions,
        cache: &CachedFileSystem<Fs>,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        let mut cache_value = self.clone();
        // Go up directories when the querying path is not a directory
        while !cache.is_directory(&cache_value, ctx) {
            if let Some(cv) = cache_value.parent() {
                cache_value = cv;
            } else {
                break;
            }
        }
        let mut cache_value = Some(cache_value);
        while let Some(cv) = cache_value {
            if let Some(package_json) = cache.get_package_json(&cv, options, ctx)? {
                return Ok(Some(package_json));
            }
            cache_value = cv.parent();
        }
        Ok(None)
    }

    /// Add an extension to the cached path.
    pub(crate) fn add_extension<Fs: FileSystem>(
        &self,
        ext: &str,
        cache: &CachedFileSystem<Fs>,
    ) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(ext);
            cache.value(path)
        })
    }

    /// Replace the extension of the cached path.
    pub(crate) fn replace_extension<Fs: FileSystem>(
        &self,
        ext: &str,
        cache: &CachedFileSystem<Fs>,
    ) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            let self_len = self.path.as_os_str().len();
            let self_bytes = self.path.as_os_str().as_encoded_bytes();
            let slice_to_copy = self
                .path
                .extension()
                .map_or(self_bytes, |previous_extension| {
                    &self_bytes[..self_len - previous_extension.len() - 1]
                });
            // SAFETY: ???
            s.push(unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(slice_to_copy) });
            s.push(ext);
            cache.value(path)
        })
    }

    /// Returns a new path by resolving the given subpath (including "." and ".." components) with this path.
    pub(crate) fn normalize_with<Fs: FileSystem, P: AsRef<Path>>(
        &self,
        subpath: P,
        cache: &CachedFileSystem<Fs>,
    ) -> Self {
        let subpath = subpath.as_ref();
        let mut components = subpath.components();
        let Some(head) = components.next() else {
            return cache.value(subpath);
        };
        if matches!(head, Component::Prefix(..) | Component::RootDir) {
            return cache.value(subpath);
        }
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self.path);
            for component in std::iter::once(head).chain(components) {
                match component {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        path.pop();
                    }
                    Component::Normal(c) => {
                        cfg_if! {
                            if #[cfg(target_family = "wasm")] {
                                // Need to trim the extra \0 introduces by https://github.com/nodejs/uvwasi/issues/262
                                path.push(c.to_string_lossy().trim_end_matches('\0'));
                            } else {
                                path.push(c);
                            }
                        }
                    }
                    Component::Prefix(..) | Component::RootDir => {
                        unreachable!("Path {:?} Subpath {:?}", self.path, subpath)
                    }
                }
            }

            cache.value(path)
        })
    }

    /// Normalize the root of the cached path.
    #[inline]
    #[cfg(windows)]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, cache: &CachedFileSystem<Fs>) -> Self {
        if self.path().as_os_str().as_encoded_bytes().last() == Some(&b'/') {
            let mut path_string = self.path.to_string_lossy().into_owned();
            path_string.pop();
            path_string.push('\\');
            cache.value(&PathBuf::from(path_string))
        } else {
            self.clone()
        }
    }

    /// Normalize the root of the cached path.
    #[inline]
    #[cfg(not(windows))]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, _cache: &CachedFileSystem<Fs>) -> Self {
        self.clone()
    }
}

impl CachedPath {
    /// Get the metadata of the cached path.
    fn metadata<Fs: FileSystem>(&self, fs: &Fs) -> Option<(bool, bool)> {
        *self.meta.get_or_init(|| {
            fs.metadata(&self.path)
                .ok()
                .map(|r| (r.is_file, r.is_directory))
        })
    }

    /// Check if the cached path is a file.
    pub(crate) fn is_file<Fs: FileSystem>(&self, fs: &Fs) -> Option<bool> {
        self.metadata(fs).map(|r| r.0)
    }

    /// Check if the cached path is a directory.
    pub(crate) fn is_directory<Fs: FileSystem>(&self, fs: &Fs) -> Option<bool> {
        self.metadata(fs).map(|r| r.1)
    }
}

impl Hash for CachedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for CachedPath {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
    }
}

impl Eq for CachedPath {}

impl fmt::Debug for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedPath")
            .field("path", &self.path)
            .finish()
    }
}

/// Borrowed cached path.
#[derive(Debug)]
pub(crate) struct BorrowedCachedPath<'a> {
    pub hash: u64,
    pub path: &'a Path,
}

impl Equivalent<CachedPath> for BorrowedCachedPath<'_> {
    fn equivalent(&self, other: &CachedPath) -> bool {
        self.path.as_os_str() == other.path().as_os_str()
    }
}

impl Hash for BorrowedCachedPath<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for BorrowedCachedPath<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
    }
}
