use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_session::{
    FileUpdate, Session, SourceSnapshot, SourceUpdate, SourceUpdateResult, open_repository_from_fs,
    open_repository_from_source,
};
use destack_source::{FileSystem, PhysicalFileSystem};
use destack_workspace::{DestackLayoutOverride, Environment, Ref, Revision, Settings};

use crate::{Error, Result};

/// Bridge session over one live language session.
#[derive(Debug)]
pub struct LanguageSession {
    /// Live language session.
    inner: Session,
}

impl LanguageSession {
    /// Open one language session from a physical filesystem path.
    pub fn open_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let file_system = Arc::new(PhysicalFileSystem::new());
        let repository = open_repository_from_fs(
            path,
            file_system,
            Environment::default(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;

        Self::open(repository)
    }

    /// Open one language session from an explicit source snapshot.
    pub fn open_source(root: impl Into<PathBuf>, source: SourceSnapshot) -> Result<Self> {
        let repository = open_repository_from_source(
            root.into(),
            source,
            Environment::default(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;

        Self::open(repository)
    }

    /// Return the current session revision.
    pub fn revision(&self) -> Result<Revision> {
        self.inner.revision(self.inner.head()).map_err(Error::from)
    }

    /// Return editable repository file paths at the current revision.
    pub fn files(&self) -> Result<Vec<String>> {
        let repository = self.inner.repository();
        let revision = self.inner.revision(self.inner.head())?;
        let mut files = repository
            .editable_file_logical_paths(revision)?
            .into_iter()
            .map(|(_, path)| repository.string_pool().get(path).to_string())
            .collect::<Vec<_>>();
        files.sort();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult> {
        self.inner
            .update(self.inner.head(), update)
            .map_err(Error::from)
    }

    /// Reload tracked files from this session filesystem.
    pub fn reload(&self) -> Result<Vec<FileUpdate>> {
        self.inner
            .reload_from_fs(self.inner.head())
            .map_err(Error::from)
    }

    /// Return the repository logical path for one file update.
    pub fn file_update_path(&self, update: &FileUpdate) -> String {
        if let Some(file) = update.file()
            && let Some(path) = file.path.as_deref()
        {
            return self.inner.repository_path(path);
        }

        if let Some(path) = update.uri().to_path_buf() {
            return self.inner.repository_path(&path);
        }

        update.uri().to_string()
    }

    /// Load one filesystem module path into the default session ref.
    pub fn load_module(&self, path: impl AsRef<Path>) -> Result<String> {
        let module = self
            .inner
            .load_module_from_fs(self.inner.head(), path.as_ref())?;

        Ok(format!("{module:?}"))
    }

    /// Open one bridge session from one prepared repository.
    fn open(repository: destack_workspace::Repository) -> Result<Self> {
        let repository = Arc::new(repository);
        let root = repository.workspace_root().to_path_buf();
        let head = Ref::for_workspace_root(&root);
        let compiler = Arc::new(Compiler::new(Arc::clone(&repository)));
        let linter = Arc::new(Linter::new(Arc::clone(&repository)));
        let query = Arc::new(Query::new(Arc::clone(&repository)));
        let worker_count = Session::default_worker_count();
        let inner = Session::new(
            root.clone(),
            root,
            repository,
            head,
            compiler,
            linter,
            query,
            worker_count,
            None,
        )?;

        Ok(Self { inner })
    }
}

/// Parse one displayed workspace revision.
pub fn parse_revision(value: &str) -> Result<Revision> {
    let Some(hex) = value.strip_prefix('r') else {
        return Err(Error::InvalidRevision {
            revision: value.to_string(),
        });
    };
    if hex.len() != 64 {
        return Err(Error::InvalidRevision {
            revision: value.to_string(),
        });
    }

    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = 2 * index;
        let end = start + 2;
        let Ok(parsed) = u8::from_str_radix(&hex[start..end], 16) else {
            return Err(Error::InvalidRevision {
                revision: value.to_string(),
            });
        };
        *byte = parsed;
    }

    Ok(Revision::new(bytes))
}

#[cfg(test)]
mod tests {
    use destack_session::{SourceFile, SourceSnapshot};

    use super::*;

    #[test]
    fn test_session_opens_source_snapshot() {
        let session = LanguageSession::open_source(
            "/workspace",
            SourceSnapshot::new(vec![
                SourceFile::text("destack.json", r#"{"name":"@test/app"}"#),
                SourceFile::text("src/index.ds", "export const value = 1;"),
            ]),
        )
        .unwrap();

        assert_eq!(
            session.files().unwrap(),
            vec!["destack.json", "src/index.ds"]
        );
    }

    #[test]
    fn test_session_updates_source_snapshot_revision() {
        let session = LanguageSession::open_source(
            "/workspace",
            SourceSnapshot::new(vec![
                SourceFile::text("destack.json", r#"{"name":"@test/app"}"#),
                SourceFile::text("src/index.ds", "export const value = 1;"),
            ]),
        )
        .unwrap();

        let result = session
            .update(SourceUpdate {
                base: None,
                edits: vec![destack_session::SourceEdit::EditText {
                    path: "src/index.ds".into(),
                    edits: vec![destack_session::TextEdit {
                        range: destack_session::TextRange { start: 21, end: 22 },
                        text: "2".to_string(),
                    }],
                }],
            })
            .unwrap();

        assert_eq!(result.files.len(), 1);
        assert_ne!(result.before, result.after);
    }
}
