use std::path::{Path, PathBuf};

use destack_query::QueryMethodId;
use indexmap::IndexMap;

use crate::core::MarkdownSuiteEntry;
use crate::mdtest::{MdTestCase, RawCodeBlock};

use super::{
    FixturePosition, FixtureRange, QueryAssertion, QueryCall, QueryDecoratorScope,
    QueryExpectation, QueryFile, QueryRun, QueryWorkspace, ResponseRows, ResponseUpdate,
    validate_query_path,
};

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
    /// The query assertions in execution order.
    pub(super) assertions: Vec<QueryAssertion>,
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
        let (name, is_ignored) = parse_name(&markdown.name)?;
        let method = fixture_method(path)?;
        let files = index_files(parse_files(&markdown)?)?;
        let assertions = parse_assertions(&markdown, &files, method)?;
        if assertions.is_empty() {
            return Err("query fixture has no assertions".to_string());
        }

        Ok(Self {
            section: markdown.section,
            name,
            is_ignored,
            files,
            assertions,
        })
    }

    /// Execute and verify this fixture.
    pub(super) fn run(
        &self,
        workspace: &QueryWorkspace,
        is_blessing: bool,
    ) -> Result<Vec<ResponseUpdate>, String> {
        QueryRun::open(self, workspace)?.run(is_blessing)
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

/// Return the query method named by one fixture file.
fn fixture_method(path: &Path) -> Result<QueryMethodId, String> {
    let method_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("query fixture path '{}' has no UTF-8 stem", path.display()))?;
    let method = QueryMethodId::from_name(method_name).ok_or_else(|| {
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

/// Parse every workspace file.
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

/// Parse every query call and exact expected response.
fn parse_assertions(
    markdown: &MdTestCase,
    files: &IndexMap<PathBuf, QueryFile>,
    method: QueryMethodId,
) -> Result<Vec<QueryAssertion>, String> {
    if !markdown.bullet_items.is_empty() {
        return Err("query fixture has unsupported bullet expectations".to_string());
    }

    let mut assertions = Vec::new();
    let mut index = 0;

    // consume each call with response rows or one complete edited file set
    while let Some(query) = markdown.extra_blocks.get(index) {
        if !query.language.starts_with("query ") {
            return Err(format!(
                "query fixture expected a 'query <method> ...' block, found '{}'",
                query.language
            ));
        }
        let (request, response) = split_request_response(&query.content);
        let call = QueryCall::parse(&query.language, request, method)?;
        validate_call(&call, files)?;
        index += 1;

        let expected = if let Some((response_offset, response)) = response {
            let content_range = query.content_range.clone().ok_or_else(|| {
                "query response must have a non-empty fenced block body".to_string()
            })?;
            let content_range = content_range.start + response_offset..content_range.end;

            QueryExpectation::Rows {
                rows: ResponseRows::parse(response, method.name())?,
                content_range,
                body: response.to_string(),
            }
        } else if call.is_edit() {
            let mut outputs = Vec::new();
            while let Some(output) = markdown.extra_blocks.get(index) {
                if !is_after_file(output) {
                    break;
                }
                outputs.push(parse_after_file(output, files)?);
                index += 1;
            }
            if outputs.is_empty() {
                return Err("edit query has no complete 'after' files".to_string());
            }

            QueryExpectation::Files(outputs)
        } else {
            return Err("query has no response rows".to_string());
        };
        assertions.push(QueryAssertion { call, expected });
    }

    Ok(assertions)
}

/// Split request rows from exact response rows.
fn split_request_response(source: &str) -> (&str, Option<(usize, &str)>) {
    // find the first response row
    let mut offset = 0;

    for line in source.split_inclusive('\n') {
        if line.starts_with('@') {
            let response = &source[offset..];

            return (&source[..offset], Some((offset, response)));
        }
        offset += line.len();
    }

    (source, None)
}

/// Return whether one raw block is a complete edited file.
fn is_after_file(block: &RawCodeBlock) -> bool {
    after_file_tag(&block.language).is_some()
}

/// Parse one exact edited output file.
fn parse_after_file(
    block: &RawCodeBlock,
    files: &IndexMap<PathBuf, QueryFile>,
) -> Result<QueryFile, String> {
    let Some((language, path)) = after_file_tag(&block.language) else {
        return Err(format!(
            "edited file block '{}' must be '<language> <path> after'",
            block.language
        ));
    };
    let path = PathBuf::from(path);
    let file = require_file(&path, files)?;
    if !file.accepts_language(language) {
        return Err(format!(
            "edited file block '{}' uses the wrong language for '{}'",
            block.language,
            path.display()
        ));
    }

    Ok(QueryFile {
        path,
        source: block.content.clone(),
        anchors: IndexMap::new(),
    })
}

/// Parse one complete edited file tag.
fn after_file_tag(language: &str) -> Option<(&str, &str)> {
    let mut words = language.split_whitespace();
    let language = words.next()?;
    let path = words.next()?;
    let marker = words.next()?;
    if marker != "after" || words.next().is_some() {
        return None;
    }

    Some((language, path))
}

/// Validate every file and anchor named by one query call.
fn validate_call(call: &QueryCall, files: &IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
    match call {
        QueryCall::Completion { position, .. }
        | QueryCall::Hover { position }
        | QueryCall::SignatureHelp { position }
        | QueryCall::Highlight { position }
        | QueryCall::GotoDefinition { position }
        | QueryCall::GotoDeclaration { position }
        | QueryCall::GotoTypeDefinition { position }
        | QueryCall::GotoImplementation { position }
        | QueryCall::FindReferences { position, .. }
        | QueryCall::CallItem { position }
        | QueryCall::RenameTarget { position }
        | QueryCall::Rename { position, .. }
        | QueryCall::Inline { position } => require_position(position, files),

        QueryCall::InlayHints { range, .. }
        | QueryCall::SemanticTokensRange { range }
        | QueryCall::ExtractVariable { range, .. } => require_range(range, files),

        QueryCall::CodeActions {
            range, diagnostics, ..
        } => {
            require_range(range, files)?;
            if let Some(diagnostics) = diagnostics {
                for diagnostic in diagnostics {
                    require_range(&diagnostic.range, files)?;
                }
            }

            Ok(())
        }

        QueryCall::IncomingCalls { item: position }
        | QueryCall::OutgoingCalls { item: position }
        | QueryCall::TypeItem { position }
        | QueryCall::Supertypes { item: position }
        | QueryCall::Subtypes { item: position } => require_position(position, files),

        QueryCall::CodeLenses { module }
        | QueryCall::FoldingRanges { module }
        | QueryCall::SemanticTokens { module }
        | QueryCall::Outline { module }
        | QueryCall::Links { module } => require_module(module, files),

        QueryCall::SelectionRanges { positions } => {
            for position in positions {
                require_position(position, files)?;
            }

            Ok(())
        }

        QueryCall::Decorators {
            scope: QueryDecoratorScope::Module(module),
            ..
        } => require_module(module, files),

        QueryCall::RenameFiles { renames } => {
            for rename in renames {
                require_rename_source(&rename.old_path, files)?;
                validate_query_path(&rename.new_path)?;
            }

            Ok(())
        }

        QueryCall::SearchSymbols { .. }
        | QueryCall::Decorators {
            scope: QueryDecoratorScope::Program,
            ..
        } => Ok(()),
    }
}

/// Require one declared file or directory rename source.
fn require_rename_source(path: &Path, files: &IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
    validate_query_path(path)?;

    let is_declared = files.values().any(|file| file.path.starts_with(path));
    if !is_declared {
        return Err(format!(
            "query rename source '{}' is not declared",
            path.display()
        ));
    }

    Ok(())
}

/// Require one declared code file and named anchor.
fn require_position(
    position: &FixturePosition,
    files: &IndexMap<PathBuf, QueryFile>,
) -> Result<(), String> {
    require_anchor(&position.file, &position.anchor, files)
}

/// Require one declared code file and named range.
fn require_range(range: &FixtureRange, files: &IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
    require_anchor(&range.file, &range.anchor, files)
}

/// Require one declared code file and named anchor.
fn require_anchor(
    path: &Path,
    anchor: &str,
    files: &IndexMap<PathBuf, QueryFile>,
) -> Result<(), String> {
    let file = require_file(path, files)?;
    if !file.is_code() {
        return Err(format!(
            "query anchor '{}#{anchor}' is not in a source module",
            path.display()
        ));
    }
    file.anchor(anchor)?;

    Ok(())
}

/// Require one declared source module.
fn require_module(path: &Path, files: &IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
    let file = require_file(path, files)?;
    if !file.is_code() {
        return Err(format!(
            "query module '{}' is not a source file",
            path.display()
        ));
    }

    Ok(())
}

/// Require one declared query file.
fn require_file<'a>(
    path: &Path,
    files: &'a IndexMap<PathBuf, QueryFile>,
) -> Result<&'a QueryFile, String> {
    validate_query_path(path)?;

    files
        .get(path)
        .ok_or_else(|| format!("query file '{}' is not declared", path.display()))
}
