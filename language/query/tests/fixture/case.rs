use std::path::{Path, PathBuf};

use destack_query::QueryMethod;
use indexmap::IndexMap;

use crate::MarkdownCase;

use super::{QueryFile, QueryResult, QueryRevision, QueryRun, QueryWorkspace};

/// One executable query case parsed from a Markdown fixture.
#[derive(Debug, Clone)]
pub(super) struct QueryCase {
    /// The Markdown document containing the fixture.
    pub(super) document: PathBuf,
    /// The Markdown section name.
    section: String,
    /// The case name.
    name: String,
    /// The workspace files in declaration order.
    pub(super) files: IndexMap<PathBuf, QueryFile>,
    /// The workspace revisions in execution order.
    pub(super) revisions: Vec<QueryRevision>,
}

impl QueryCase {
    /// Parse one Markdown case into an executable query case.
    pub(super) fn parse(path: &Path, markdown: MarkdownCase) -> Result<Self, String> {
        let method = Self::parse_method(path)?;
        let files = Self::index_files(Self::parse_files(&markdown)?)?;
        let revisions = QueryRevision::parse(&markdown, &files, method)?;

        Ok(Self {
            document: path.to_path_buf(),
            section: markdown.section,
            name: markdown.name,
            files,
            revisions,
        })
    }

    /// Return the stable test name for this fixture.
    pub(super) fn test_name(&self, root: &Path) -> Result<String, String> {
        let document = self.document.strip_prefix(root).map_err(|error| {
            format!(
                "query fixture '{}' is outside '{}': {error}",
                self.document.display(),
                root.display()
            )
        })?;

        Ok(format!(
            "{}/{}/{}",
            document.display(),
            slug(&self.section),
            slug(&self.name)
        ))
    }

    /// Execute and verify this fixture.
    pub(super) fn run(
        &self,
        workspace: &QueryWorkspace,
        is_blessing: bool,
    ) -> Result<QueryResult, String> {
        QueryRun::open(&self.files, workspace)?.run(&self.revisions, is_blessing)
    }

    /// Parse the query method named by one fixture file.
    fn parse_method(path: &Path) -> Result<QueryMethod, String> {
        let method_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("query fixture path '{}' has no UTF-8 stem", path.display()))?;
        let method = QueryMethod::from_name(method_name).ok_or_else(|| {
            format!(
                "query fixture '{}' has no registered method",
                path.display()
            )
        })?;

        Ok(method)
    }

    /// Parse every initial workspace file.
    fn parse_files(markdown: &MarkdownCase) -> Result<Vec<QueryFile>, String> {
        markdown
            .files
            .iter()
            .map(|file| QueryFile::parse(&file.path, &file.content))
            .collect()
    }

    /// Index unique workspace files in declaration order.
    fn index_files(files: Vec<QueryFile>) -> Result<IndexMap<PathBuf, QueryFile>, String> {
        let mut indexed = IndexMap::new();

        // require one exact entry for every path
        for file in files {
            let path = file.path.clone();
            if indexed.insert(path.clone(), file).is_some() {
                return Err(format!(
                    "query fixture declares file '{}' more than once",
                    path.display()
                ));
            }
        }
        if indexed.is_empty() {
            return Err("query fixture has no workspace files".to_string());
        }

        // require one program source
        if !indexed.values().any(QueryFile::is_code) {
            return Err("query fixture has no source modules".to_string());
        }

        Ok(indexed)
    }
}

/// Convert one Markdown heading into a stable test path segment.
fn slug(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
