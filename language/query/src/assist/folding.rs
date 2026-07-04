use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::File;
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleQueryContext};

/// Kind of folding range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FoldingRangeKind {
    /// A comment block.
    Comment,
    /// An import section.
    Imports,
    /// A region.
    Region,
}

/// A foldable range in source code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FoldingRange {
    /// Start line.
    pub start_line: u32,
    /// End line.
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

    /// Create a comment folding range.
    pub fn comment(start_line: u32, end_line: u32) -> Self {
        let mut range = Self::new(start_line, end_line);
        range.kind = Some(FoldingRangeKind::Comment);

        range
    }
}

/// One contiguous line-comment block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineCommentBlock {
    /// The first line in the block.
    start_line: u32,
    /// The last line in the block.
    end_line: u32,
}

impl LineCommentBlock {
    /// Create a one-line comment block.
    fn new(line: u32) -> Self {
        Self {
            start_line: line,
            end_line: line,
        }
    }

    /// Return whether this block contains multiple lines.
    fn is_foldable(&self) -> bool {
        self.end_line > self.start_line
    }

    /// Extend this block if the line is contiguous.
    fn extend(&mut self, line: u32) -> bool {
        let is_next_line = line == self.end_line.saturating_add(1);
        if is_next_line {
            self.end_line = line;
        }

        is_next_line
    }

    /// Return this block as a folding range.
    fn folding_range(&self) -> Option<FoldingRange> {
        if !self.is_foldable() {
            return None;
        }

        Some(FoldingRange::comment(self.start_line, self.end_line))
    }
}

/// Pending line-comment folding state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LineCommentBlocks {
    /// The current line-comment block.
    current: Option<LineCommentBlock>,
}

impl LineCommentBlocks {
    /// Insert one line comment.
    fn insert(&mut self, line: u32, ranges: &mut Vec<FoldingRange>) {
        if let Some(block) = &mut self.current {
            if block.extend(line) {
                return;
            }
        }

        self.flush(ranges);
        self.current = Some(LineCommentBlock::new(line));
    }

    /// Flush the current block into the folding ranges.
    fn flush(&mut self, ranges: &mut Vec<FoldingRange>) {
        let Some(block) = self.current.take() else {
            return;
        };

        if let Some(range) = block.folding_range() {
            ranges.push(range);
        }
    }
}

/// Request folding ranges for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FoldingRangesRequest {
    /// The queried module.
    pub module: Module,
}

/// Response payload for folding ranges queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FoldingRangesResponse {
    /// Folding ranges.
    pub ranges: Vec<FoldingRange>,
}

impl ModuleQueryContext<'_> {
    /// Return folding ranges for a file.
    pub fn folding_ranges(&self) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();

        // collect declaration and comment folds
        self.collect_declaration_folding_ranges(&mut ranges);
        self.collect_comment_folding_ranges(&mut ranges);

        // order and deduplicate ranges
        ranges.sort_by_key(|range| (range.start_line, range.end_line));
        ranges.dedup_by(|left, right| {
            left.start_line == right.start_line && left.end_line == right.end_line
        });

        ranges
    }

    /// Collect declaration folding ranges.
    fn collect_declaration_folding_ranges(&self, ranges: &mut Vec<FoldingRange>) {
        let source_file = self.source_file();
        let view = self.view();

        // collect one foldable range per declaration body
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            if !Self::declaration_is_foldable(declaration) {
                continue;
            }

            let source_node_id = view.get_source(declaration_id);
            let span = self.tree().source_index.get(source_node_id);
            let Some((start_line, _)) = source_file.get_position(span.start) else {
                continue;
            };
            let Some((end_line, _)) = source_file.get_position(span.end) else {
                continue;
            };

            if end_line > start_line {
                ranges.push(FoldingRange::new(start_line, end_line));
            }
        }
    }

    /// Return whether a declaration can produce a folding range.
    fn declaration_is_foldable(declaration: &dir::Declaration) -> bool {
        matches!(
            declaration,
            dir::Declaration::Function { .. }
                | dir::Declaration::Class { .. }
                | dir::Declaration::Struct { .. }
                | dir::Declaration::Interface { .. }
                | dir::Declaration::Enum { .. }
                | dir::Declaration::Global { .. }
                | dir::Declaration::Extension { .. }
        )
    }

    /// Collect comment folding ranges.
    fn collect_comment_folding_ranges(&self, ranges: &mut Vec<FoldingRange>) {
        let source_file = self.source_file();
        let mut line_comment_blocks = LineCommentBlocks::default();

        // scan side tokens in source order
        for token in self.side_tokens() {
            if token.span.file != source_file.id {
                continue;
            }

            self.collect_token_folding_range(ranges, &mut line_comment_blocks, token, &source_file);
        }

        line_comment_blocks.flush(ranges);
    }

    /// Collect the folding range for one side token.
    fn collect_token_folding_range(
        &self,
        ranges: &mut Vec<FoldingRange>,
        line_comment_blocks: &mut LineCommentBlocks,
        token: &dir::TokenSpan,
        source_file: &File,
    ) {
        match token.token.ty() {
            dir::TokenType::LineComment | dir::TokenType::DocLineComment => {
                let Some((start_line, _)) = source_file.get_position(token.span.start) else {
                    return;
                };

                line_comment_blocks.insert(start_line, ranges);
            }
            dir::TokenType::BlockComment | dir::TokenType::DocBlockComment => {
                line_comment_blocks.flush(ranges);
                self.collect_block_comment_folding_range(ranges, token, source_file);
            }
            dir::TokenType::Whitespace | dir::TokenType::Newline => {}
            _ => {
                line_comment_blocks.flush(ranges);
            }
        }
    }

    /// Collect the folding range for one block comment.
    fn collect_block_comment_folding_range(
        &self,
        ranges: &mut Vec<FoldingRange>,
        token: &dir::TokenSpan,
        source_file: &File,
    ) {
        let Some((start_line, _)) = source_file.get_position(token.span.start) else {
            return;
        };
        let Some((end_line, _)) = source_file.get_position(token.span.end) else {
            return;
        };

        if end_line > start_line {
            ranges.push(FoldingRange::comment(start_line, end_line));
        }
    }
}
