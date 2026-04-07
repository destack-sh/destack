use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::MemoryCacheStore;
use destack_compiler::Compiler;
use destack_source::{FileContent, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};
use destack_workspace::{Change, Edit, FormatterOptions, LinterOptions, Ref, Repository, Revision};

/// One shared in memory workspace for suite execution.
#[derive(Debug)]
pub struct SharedMemoryWorkspace {
    /// The shared repository.
    repository: Arc<Repository>,
    /// The shared in memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The root directory prefix for allocated cases.
    root: PathBuf,
    /// The next unique case id.
    next_id: AtomicUsize,
}

impl SharedMemoryWorkspace {
    /// Create one shared in memory workspace rooted at the given path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("failed to create core workspace root");
        let repository = Arc::new(
            Repository::open_root_from_fs(root.clone(), fs.clone())
                .expect("failed to import repository from core workspace fs")
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );

        Self {
            repository,
            fs,
            root,
            next_id: AtomicUsize::new(0),
        }
    }

    /// Return the shared repository.
    pub fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the shared in memory file system.
    pub fn fs(&self) -> Arc<MemoryFileSystem> {
        self.fs.clone()
    }

    /// Return the workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Allocate one unique case root under the workspace root.
    pub fn allocate_root(&self, label: &str) -> PathBuf {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.root.join(format!("{label}-{id}"))
    }
}

/// Open one repository over one workspace root with explicit formatter and linter options.
pub fn open_repository_with_options(
    root: PathBuf,
    fs: Arc<dyn FileSystem>,
    formatter: FormatterOptions,
    linter: LinterOptions,
) -> Arc<Repository> {
    // materialize the workspace root before importing it
    fs.create_dir_all(&root)
        .expect("failed to create core workspace root");

    Arc::new(
        Repository::open_root_from_fs(root, fs)
            .expect("failed to import repository from core workspace fs")
            .with_cache_store(Arc::new(MemoryCacheStore::new()))
            .with_formatter(formatter)
            .with_linter(linter),
    )
}

/// Return the current workspace revision for one repository.
pub fn current_workspace_revision(repository: &Repository) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());

    repository
        .current(&reference)
        .expect("missing current workspace revision")
}

/// Apply one full file write to the current workspace revision.
pub fn write_workspace_file(
    repository: &Repository,
    path: &Path,
    content: FileContent,
) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());
    let logical_path = repository.normalize_workspace_path(path);
    let change = Change::single(Edit::SetFile {
        logical_path,
        content,
    });

    repository
        .apply(&reference, change)
        .expect("failed to apply workspace file change")
}

/// Apply one text file write to the current workspace revision.
pub fn write_workspace_text_file(repository: &Repository, path: &Path, content: &str) -> Revision {
    write_workspace_file(
        repository,
        path,
        FileContent::Text {
            content: content.to_string(),
        },
    )
}

/// Remember the default profile for one module in the compiler cache.
pub fn remember_default_profile_for_module(
    repository: &Repository,
    compiler: &Compiler,
    revision: Revision,
    module_id: ModuleId,
) -> ProfileId {
    let profile = repository
        .default_profile_for_module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"));

    compiler.remember_profile(profile)
}

/// Remember the target profile for one module in the compiler cache.
pub fn remember_profile_for_target_or_default(
    repository: &Repository,
    compiler: &Compiler,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
) -> ProfileId {
    let profile = repository
        .profile_for_target_or_default(revision, module_id, target_id)
        .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"));

    compiler.remember_profile(profile)
}
