use destack_ast::{self as ast, TokenType};
use destack_dir::Declaration;
use destack_source::{FileId, Uri};
use serde::{Deserialize, Serialize};

use crate::common::{get_module_by_file_id, program_for_module};
use destack_workspace::Session;

/// Kind of folding range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoldingRangeKind {
    /// A comment block.
    Comment,
    /// An import section.
    Imports,
    /// A region (explicit fold marker).
    Region,
}

/// A foldable range in source code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldingRange {
    /// Start line (0-indexed).
    pub start_line: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// Optional start character.
    pub start_character: Option<u32>,
    /// Optional end character.
    pub end_character: Option<u32>,
    /// The kind of folding range.
    pub kind: Option<FoldingRangeKind>,
    /// Text to show when collapsed.
    pub collapsed_text: Option<String>,
}

impl FoldingRange {
    /// Create a folding range.
    pub fn new(start_line: u32, end_line: u32) -> Self {
        Self {
            start_line,
            end_line,
            start_character: None,
            end_character: None,
            kind: None,
            collapsed_text: None,
        }
    }

    /// Set the kind.
    pub fn with_kind(mut self, kind: FoldingRangeKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Set the collapsed text.
    pub fn with_collapsed_text(mut self, text: impl Into<String>) -> Self {
        self.collapsed_text = Some(text.into());
        self
    }
}

/// Request folding ranges for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldingRangesRequest {
    /// The document URI.
    pub uri: Uri,
}

/// Response payload for folding ranges queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldingRangesResponse {
    /// Folding ranges.
    pub ranges: Vec<FoldingRange>,
}

/// Get folding ranges for a file.
///
/// Returns foldable regions for:
/// - Function bodies
/// - Class/struct/interface/enum bodies
/// - Namespace blocks
pub fn folding_ranges(session: &Session, file: FileId) -> Vec<FoldingRange> {
    // prefer using dir aware ranges when available
    if let Some(ranges) = folding_ranges_with_dir(session, file) {
        return ranges;
    }

    // fall back to ast only folding ranges
    folding_ranges_with_ast(session, file)
}

/// Build folding ranges using the DIR context when available.
fn folding_ranges_with_dir(session: &Session, file: FileId) -> Option<Vec<FoldingRange>> {
    crate::with_query_context_for_file(session, file, |ctx| {
        // resolve the source file and dir tree
        let source_file = session.files.get(ctx.file_id);
        let dir_tree = ctx.tree();

        // collect folding ranges from declarations
        let mut ranges = Vec::new();

        // iterate through all declarations and create folding ranges
        for (decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // only fold declarations with foldable bodies
            let should_fold = matches!(
                declaration,
                Declaration::Function { .. }
                    | Declaration::Class { .. }
                    | Declaration::Struct { .. }
                    | Declaration::Interface { .. }
                    | Declaration::Enum { .. }
                    | Declaration::Global { .. }
                    | Declaration::Namespace { .. }
                    | Declaration::Extension { .. }
            );
            if !should_fold {
                continue;
            }

            // resolve the declaration span
            let ast_node_id = dir_tree.get_source(decl_id.id);
            let span = ctx.ast.tree.source_map.get(ast_node_id);

            // convert the span to line numbers
            let Some((start_line, _)) = source_file.get_position(span.start) else {
                continue;
            };
            let Some((end_line, _)) = source_file.get_position(span.end) else {
                continue;
            };

            // skip single line declarations
            if end_line > start_line {
                ranges.push(FoldingRange::new(start_line, end_line));
            }
        }

        // collect folding ranges for comment blocks
        add_comment_folding_ranges(&mut ranges, &ctx.ast.side_tokens, &source_file);

        // sort ranges by start and end line
        ranges.sort_by_key(|range| (range.start_line, range.end_line));

        // drop duplicate folding ranges
        ranges.dedup_by(|left, right| {
            left.start_line == right.start_line && left.end_line == right.end_line
        });
        ranges
    })
}

/// Build folding ranges from the AST when DIR is unavailable.
fn folding_ranges_with_ast(session: &Session, file: FileId) -> Vec<FoldingRange> {
    // resolve the module ast
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.as_ref();
    let program = program_for_module(session, module);
    let Some(ast) = program.artifacts.ast(module.id) else {
        return Vec::new();
    };

    // resolve the source file
    let source_file = session.files.get(file);

    // collect folding ranges from declarations
    let mut ranges = Vec::new();

    // iterate through all declarations and create folding ranges
    for declaration_id in ast.tree.iter_nodes::<ast::Declaration>() {
        let declaration = ast.tree.get(declaration_id);

        // only fold declarations with foldable bodies
        let should_fold = matches!(
            declaration,
            ast::Declaration::Function { .. }
                | ast::Declaration::Class { .. }
                | ast::Declaration::Struct { .. }
                | ast::Declaration::Interface { .. }
                | ast::Declaration::Enum { .. }
                | ast::Declaration::Global { .. }
                | ast::Declaration::Namespace { .. }
                | ast::Declaration::Extension { .. }
        );
        if !should_fold {
            continue;
        }

        // resolve the declaration span
        let span = ast.tree.source_map.get(declaration_id.id);

        // convert the span to line numbers
        let Some((start_line, _)) = source_file.get_position(span.start) else {
            continue;
        };
        let Some((end_line, _)) = source_file.get_position(span.end) else {
            continue;
        };

        // skip single line declarations
        if end_line > start_line {
            ranges.push(FoldingRange::new(start_line, end_line));
        }
    }

    // collect folding ranges for comment blocks
    add_comment_folding_ranges(&mut ranges, &ast.side_tokens, &source_file);

    // sort ranges by start and end line
    ranges.sort_by_key(|range| (range.start_line, range.end_line));

    // drop duplicate folding ranges
    ranges.dedup_by(|left, right| {
        left.start_line == right.start_line && left.end_line == right.end_line
    });

    ranges
}

/// Add comment folding ranges for the given token stream.
fn add_comment_folding_ranges(
    ranges: &mut Vec<FoldingRange>,
    tokens: &[ast::TokenSpan],
    source_file: &destack_source::File,
) {
    // track line comment runs
    let mut line_comment_block: Option<(u32, u32)> = None;
    for token in tokens {
        let span = token.span;
        if span.file != source_file.id {
            continue;
        }

        match token.token.ty {
            TokenType::LineComment | TokenType::DocLineComment => {
                let Some((start_line, _)) = source_file.get_position(span.start) else {
                    continue;
                };
                match line_comment_block {
                    Some((block_start, block_end)) => {
                        if start_line == block_end.saturating_add(1) {
                            line_comment_block = Some((block_start, start_line));
                        } else {
                            if block_end > block_start {
                                ranges.push(
                                    FoldingRange::new(block_start, block_end)
                                        .with_kind(FoldingRangeKind::Comment),
                                );
                            }
                            line_comment_block = Some((start_line, start_line));
                        }
                    }
                    None => {
                        line_comment_block = Some((start_line, start_line));
                    }
                }
            }
            TokenType::BlockComment | TokenType::DocBlockComment => {
                if let Some((block_start, block_end)) =
                    line_comment_block.take().filter(|(start, end)| end > start)
                {
                    ranges.push(
                        FoldingRange::new(block_start, block_end)
                            .with_kind(FoldingRangeKind::Comment),
                    );
                }
                let Some((start_line, _)) = source_file.get_position(span.start) else {
                    continue;
                };
                let Some((end_line, _)) = source_file.get_position(span.end) else {
                    continue;
                };
                if end_line > start_line {
                    ranges.push(
                        FoldingRange::new(start_line, end_line)
                            .with_kind(FoldingRangeKind::Comment),
                    );
                }
            }
            TokenType::Whitespace | TokenType::Newline => {}
            _ => {
                if let Some((block_start, block_end)) =
                    line_comment_block.take().filter(|(start, end)| end > start)
                {
                    ranges.push(
                        FoldingRange::new(block_start, block_end)
                            .with_kind(FoldingRangeKind::Comment),
                    );
                }
            }
        }
    }

    if let Some((block_start, block_end)) =
        line_comment_block.take().filter(|(start, end)| end > start)
    {
        ranges.push(FoldingRange::new(block_start, block_end).with_kind(FoldingRangeKind::Comment));
    }
}
