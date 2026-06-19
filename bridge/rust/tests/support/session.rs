use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

/// One opened bridge test workspace.
pub(crate) struct WorkspaceFixture {
    /// The open language workspace.
    pub(crate) workspace: destack::Workspace,
}

/// One opened bridge test repository.
pub(crate) struct RepositoryFixture {
    /// The open language repository.
    pub(crate) repository: destack::Repository,
}

/// Open a memory source with explicit test files.
pub(crate) fn open_workspace(files: &[(&str, &str)]) -> destack::Result<WorkspaceFixture> {
    // open the repository first so workspace and session tests share setup
    let repository = open_repository(files)?;
    let workspace = repository.repository.workspace()?;

    Ok(WorkspaceFixture { workspace })
}

/// Open a memory repository with explicit test files.
pub(crate) fn open_repository(files: &[(&str, &str)]) -> destack::Result<RepositoryFixture> {
    // build one isolated memory source for this test
    let root = create_root();
    let edits = files
        .iter()
        .map(|(path, text)| destack::Edit::SetText {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    let source = destack::Source::memory(root.to_string_lossy().into_owned(), edits);

    // open the public bridge repository over the prepared source
    let repository = destack::Repository::open(source)?;

    Ok(RepositoryFixture { repository })
}

fn create_root() -> PathBuf {
    // allocate a deterministic but unique temp root
    let index = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "destack-bridge-rust-tests-{}-{index}",
        std::process::id()
    ));

    // create a writable cache parent for repository state
    std::fs::create_dir_all(&root).expect("create bridge test root");

    root
}
