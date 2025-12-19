use destack_dir::Declaration;
use destack_source::FileId;

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// Kind of folding range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldingRangeKind {
    /// A comment block.
    Comment,
    /// An import section.
    Imports,
    /// A region (explicit fold marker).
    Region,
}

/// A foldable range in source code.
#[derive(Debug, Clone)]
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

/// Get folding ranges for a file.
///
/// Returns foldable regions for:
/// - Function bodies
/// - Class/struct/interface/enum bodies
/// - Namespace blocks
pub fn folding_ranges(session: &Session, file: FileId) -> Vec<FoldingRange> {
    // get module AST/DIR
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return Vec::new();
    };

    // get the file for position conversion
    let source_file = session.files.get(file);
    let dir_tree = dir.tree.read();

    let mut ranges = Vec::new();

    // iterate through all declarations and create folding ranges
    for (decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        // only fold declarations with bodies (multi-line)
        let should_fold = matches!(
            declaration,
            Declaration::Function { .. }
                | Declaration::Class { .. }
                | Declaration::Struct { .. }
                | Declaration::Interface { .. }
                | Declaration::Enum { .. }
                | Declaration::Namespace { .. }
        );

        if !should_fold {
            continue;
        }

        // get the span of this declaration
        let ast_node_id = dir_tree.get_source(decl_id.id);
        let span = ast.tree.source_map.get(ast_node_id);

        // convert to line numbers
        let Some((start_line, _)) = source_file.get_position(span.start) else {
            continue;
        };
        let Some((end_line, _)) = source_file.get_position(span.end) else {
            continue;
        };

        // only fold if spans multiple lines
        if end_line > start_line {
            ranges.push(FoldingRange::new(start_line, end_line));
        }
    }

    // sort by start line
    ranges.sort_by_key(|r| r.start_line);

    ranges
}
