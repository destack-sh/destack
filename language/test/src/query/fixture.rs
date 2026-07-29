use std::path::{Path, PathBuf};

use destack_query::QueryMethod;
use indexmap::IndexMap;

use crate::core::MarkdownSuiteEntry;
use crate::mdtest::MdTestCase;

use super::{QueryFile, QueryFixtureResult, QueryRevision, QueryRun, QueryWorkspace};

/// One Markdown query fixture and its executable case.
#[derive(Debug, Clone)]
pub(super) struct QueryFixture {
    /// The Markdown section name.
    section: String,
    /// The case name.
    name: String,
    /// Whether the case is ignored by default.
    is_ignored: bool,
    /// The workspace files in declaration order.
    pub(super) files: IndexMap<PathBuf, QueryFile>,
    /// The workspace revisions in execution order.
    pub(super) revisions: Vec<QueryRevision>,
}

impl QueryFixture {
    /// Parse one Markdown case into the query fixture language.
    pub(super) fn parse(path: &Path, markdown: MdTestCase) -> Result<Self, String> {
        if !markdown.options.is_empty() {
            return Err("query fixture has unsupported test options".to_string());
        }
        if markdown.skip {
            return Err("query fixtures cannot be skipped".to_string());
        }
        let (name, is_ignored) = Self::parse_name(&markdown.name)?;
        let method = Self::parse_method(path)?;
        let files = Self::index_files(Self::parse_files(&markdown)?)?;
        let revisions = QueryRevision::parse(&markdown, &files, method)?;

        Ok(Self {
            section: markdown.section,
            name,
            is_ignored,
            files,
            revisions,
        })
    }

    /// Execute and verify this fixture.
    pub(super) fn run(
        &self,
        workspace: &QueryWorkspace,
        is_blessing: bool,
    ) -> Result<QueryFixtureResult, String> {
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

    /// Parse one fixture name and its optional ignored annotation.
    fn parse_name(name: &str) -> Result<(String, bool), String> {
        let Some(name) = name.strip_prefix("[ignored]") else {
            return Ok((name.to_string(), false));
        };
        let Some(name) = name.strip_prefix(' ') else {
            return Err("query fixture '[ignored]' annotation must precede a name".to_string());
        };
        if name.is_empty() {
            return Err("query fixture '[ignored]' annotation must precede a name".to_string());
        }

        Ok((name.to_string(), true))
    }

    /// Parse every initial workspace file.
    fn parse_files(markdown: &MdTestCase) -> Result<Vec<QueryFile>, String> {
        markdown
            .files
            .iter()
            .map(|file| {
                if !file.options.is_empty() {
                    return Err(format!(
                        "query file '{}' has unsupported options",
                        file.path
                    ));
                }

                QueryFile::parse(&file.path, &file.content)
            })
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

impl MarkdownSuiteEntry for QueryFixture {
    /// Return the fixture section.
    fn section(&self) -> &str {
        &self.section
    }

    /// Return the fixture name.
    fn name(&self) -> &str {
        &self.name
    }

    /// Return whether the fixture is skipped by default.
    fn is_skipped(&self) -> bool {
        self.is_ignored
    }
}
