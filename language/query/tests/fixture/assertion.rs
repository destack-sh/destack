use std::ops::Range;
use std::path::{Path, PathBuf};

use destack_query::QueryMethod;
use indexmap::IndexMap;

use crate::QueryBlock;

use super::{
    FixturePosition, FixtureRange, QueryCall, QueryDecoratorScope, QueryFile, ResponseRows,
    parse_marked_file_tag, require_query_file, validate_query_path,
};

/// One query call and its expected result.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct QueryAssertion {
    /// The query call.
    pub(super) call: QueryCall,
    /// The expected response.
    pub(super) expected: QueryExpectation,
}

/// The expected result of one query call.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum QueryExpectation {
    /// Any successful response from the declared query method.
    Success,
    /// The exact response rows.
    Rows {
        /// The parsed response rows.
        rows: ResponseRows,
        /// The response content range in the Markdown file.
        content_range: Range<usize>,
        /// The exact fenced response body.
        body: String,
    },
    /// The complete files produced by one edit response.
    Files(Vec<QueryFile>),
}

impl QueryAssertion {
    /// Return this assertion with success required instead of an exact response.
    pub(super) fn require_success(&self) -> Self {
        Self {
            call: self.call.clone(),
            expected: QueryExpectation::Success,
        }
    }

    /// Parse one query call and its exact expected response.
    pub(super) fn parse(
        blocks: &[QueryBlock],
        index: usize,
        files: &IndexMap<PathBuf, QueryFile>,
        method: QueryMethod,
    ) -> Result<(Self, Option<Vec<QueryFile>>, usize), String> {
        let query = &blocks[index];
        let (request, response) = split_request_response(&query.content);
        let (language, applies_edit) = parse_query_language(&query.language);
        let call = QueryCall::parse(language, request, method)?;
        call.validate(files)?;
        let mut next_index = index + 1;
        if applies_edit && !call.is_edit() {
            return Err(format!("{} query cannot apply an edit", method.name()));
        }

        // parse response rows or complete edited files
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
            while let Some(output) = blocks.get(next_index) {
                if !is_after_file(output) {
                    break;
                }
                let file = parse_after_file(output, files)?;
                if outputs
                    .iter()
                    .any(|expected: &QueryFile| expected.path == file.path)
                {
                    return Err(format!(
                        "query expects file '{}' more than once",
                        file.path.display()
                    ));
                }
                outputs.push(file);
                next_index += 1;
            }
            if outputs.is_empty() {
                return Err("edit query has no complete 'after' files".to_string());
            }

            QueryExpectation::Files(outputs)
        } else {
            return Err("query has no response rows".to_string());
        };

        // retain applied output as the next revision
        let applied_files = if applies_edit {
            let QueryExpectation::Files(files) = &expected else {
                return Err("applied query has no complete edited files".to_string());
            };

            Some(files.clone())
        } else {
            None
        };

        Ok((Self { call, expected }, applied_files, next_index))
    }
}

impl QueryCall {
    /// Validate every file and anchor named by this fixture call.
    fn validate(&self, files: &IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
        match self {
            Self::Completion { position, .. }
            | Self::Hover { position }
            | Self::SignatureHelp { position }
            | Self::Highlight { position }
            | Self::GotoDefinition { position }
            | Self::GotoDeclaration { position }
            | Self::GotoTypeDefinition { position }
            | Self::GotoImplementation { position }
            | Self::FindReferences { position, .. }
            | Self::CallItem { position }
            | Self::RenameTarget { position }
            | Self::Rename { position, .. }
            | Self::Inline { position } => require_position(position, files),

            Self::InlayHints { range, .. }
            | Self::SemanticTokensRange { range }
            | Self::ExtractVariable { range, .. } => require_range(range, files),

            Self::CodeActions {
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

            Self::IncomingCalls { item: position }
            | Self::OutgoingCalls { item: position }
            | Self::TypeItem { position }
            | Self::Supertypes { item: position }
            | Self::Subtypes { item: position } => require_position(position, files),

            Self::CodeLenses { module }
            | Self::FoldingRanges { module }
            | Self::SemanticTokens { module }
            | Self::Outline { module }
            | Self::Links { module } => require_module(module, files),

            Self::SelectionRanges { positions } => {
                for position in positions {
                    require_position(position, files)?;
                }

                Ok(())
            }

            Self::Decorators {
                scope: QueryDecoratorScope::Module(module),
                ..
            } => require_module(module, files),

            Self::RenameFiles { renames } => {
                for rename in renames {
                    require_rename_source(&rename.old_path, files)?;
                    validate_query_path(&rename.new_path)?;
                }

                Ok(())
            }

            Self::SearchSymbols { .. }
            | Self::Decorators {
                scope: QueryDecoratorScope::Program,
                ..
            } => Ok(()),
        }
    }
}

/// Split the fixture-only apply marker from one query fence.
fn parse_query_language(language: &str) -> (&str, bool) {
    match language.strip_suffix(" apply") {
        Some(language) => (language, true),
        None => (language, false),
    }
}

/// Split request rows from exact response rows.
fn split_request_response(source: &str) -> (&str, Option<(usize, &str)>) {
    let mut offset = 0;

    // find the first response row
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
fn is_after_file(block: &QueryBlock) -> bool {
    after_file_tag(&block.language).is_some()
}

/// Parse one exact edited output file.
fn parse_after_file(
    block: &QueryBlock,
    files: &IndexMap<PathBuf, QueryFile>,
) -> Result<QueryFile, String> {
    let Some((language, path)) = after_file_tag(&block.language) else {
        return Err(format!(
            "edited file block '{}' must be '<language> <path> after'",
            block.language
        ));
    };
    let path = PathBuf::from(path);
    let file = require_query_file(&path, files)?;
    if !file.accepts_language(language) {
        return Err(format!(
            "edited file block '{}' uses the wrong language for '{}'",
            block.language,
            path.display()
        ));
    }

    QueryFile::parse(path, &block.content)
}

/// Parse one complete edited file tag.
fn after_file_tag(language: &str) -> Option<(&str, &str)> {
    parse_marked_file_tag(language, "after")
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

/// Require one declared code file and named position.
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
    let file = require_query_file(path, files)?;
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
    let file = require_query_file(path, files)?;
    if !file.is_code() {
        return Err(format!(
            "query module '{}' is not a source file",
            path.display()
        ));
    }

    Ok(())
}
