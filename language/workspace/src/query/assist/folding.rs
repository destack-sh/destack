use destack_dir::Declaration;
use destack_source::{FileId, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::with_query_context_for_file;

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
    with_query_context_for_file(session, file, |ctx| {
        // resolve the source file and dir tree
        let source_file = session.files.get(ctx.file_id);
        let dir_tree = ctx.tree();

        // collect folding ranges from declarations
        let mut ranges = Vec::new();

        // iterate through all declarations and create folding ranges
        for (decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // only fold declarations with bodies
            let should_fold = matches!(
                declaration,
                Declaration::Function { .. }
                    | Declaration::Class { .. }
                    | Declaration::Struct { .. }
                    | Declaration::Interface { .. }
                    | Declaration::Enum { .. }
                    | Declaration::Global { .. }
                    | Declaration::Namespace { .. }
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

        // sort ranges by start and end line
        ranges.sort_by_key(|range| (range.start_line, range.end_line));

        // drop duplicate folding ranges
        ranges.dedup_by(|left, right| {
            left.start_line == right.start_line && left.end_line == right.end_line
        });

        ranges
    })
    .unwrap_or_default()
}
