use crate::core::QueryModule;
use destack_dir as dir;
use serde::{Deserialize, Serialize};

use crate::core::ModuleQueryContext;

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
    /// The queried module.
    pub module: QueryModule,
}

/// Response payload for folding ranges queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldingRangesResponse {
    /// Folding ranges.
    pub ranges: Vec<FoldingRange>,
}

impl ModuleQueryContext<'_> {
    /// Get folding ranges for a file.
    pub fn folding_ranges(&self) -> Vec<FoldingRange> {
        let Some(source_file) = self
            .repository()
            .file(self.revision(), self.file_id())
            .ok()
            .flatten()
        else {
            return Vec::new();
        };
        let dir_tree = self.dir().view();
        let mut ranges = Vec::new();

        // collect declaration body ranges
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            let should_fold = matches!(
                declaration,
                dir::Declaration::Function { .. }
                    | dir::Declaration::Class { .. }
                    | dir::Declaration::Struct { .. }
                    | dir::Declaration::Interface { .. }
                    | dir::Declaration::Enum { .. }
                    | dir::Declaration::Global { .. }
                    | dir::Declaration::Extension { .. }
            );
            if !should_fold {
                continue;
            }

            let source_node_id = dir_tree.get_source(declaration_id);
            let span = self.dir().tree().source_index.get(source_node_id);
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

        // collect comment block ranges
        add_comment_folding_ranges(&mut ranges, self.dir().side_tokens(), &source_file);

        // order and deduplicate ranges
        ranges.sort_by_key(|range| (range.start_line, range.end_line));
        ranges.dedup_by(|left, right| {
            left.start_line == right.start_line && left.end_line == right.end_line
        });

        ranges
    }
}

/// Add comment folding ranges for the given token stream.
fn add_comment_folding_ranges(
    ranges: &mut Vec<FoldingRange>,
    tokens: &[dir::TokenSpan],
    source_file: &destack_source::File,
) {
    // track line comment runs
    let mut line_comment_block: Option<(u32, u32)> = None;
    for token in tokens {
        let span = token.span;
        if span.file != source_file.id {
            continue;
        }

        match token.token.ty() {
            dir::TokenType::LineComment | dir::TokenType::DocLineComment => {
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
            dir::TokenType::BlockComment | dir::TokenType::DocBlockComment => {
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
            dir::TokenType::Whitespace | dir::TokenType::Newline => {}
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
