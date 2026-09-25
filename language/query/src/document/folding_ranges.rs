use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{File, FileId, NodeSpanRegion, NodeSpanType, Span};

use crate::{Module, ModuleQueryContext, QueryError, QueryResult};

/// Kind of folding range.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum FoldingRangeKind {
    /// A comment block.
    Comment,
    /// An import section.
    Imports,
    /// An explicit source region.
    Region,
}

/// A foldable range in source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    fn new(start_line: u32, end_line: u32) -> Self {
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
    fn comment(start_line: u32, end_line: u32) -> Self {
        let mut range = Self::new(start_line, end_line);
        range.kind = Some(FoldingRangeKind::Comment);

        range
    }
}

/// A folding ranges request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FoldingRangesRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
}

/// A folding ranges response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FoldingRangesResponse {
    /// Folding ranges.
    pub ranges: Vec<FoldingRange>,
}

impl ModuleQueryContext<'_> {
    /// Return folding ranges for a file.
    pub fn folding_ranges(
        &self,
        request: FoldingRangesRequest,
    ) -> QueryResult<FoldingRangesResponse> {
        let file_id = request.file_id;
        let mut ranges = Vec::new();

        // collect authored source folds
        self.collect_import_folding_ranges(file_id, &mut ranges)?;
        self.collect_node_folding_ranges(file_id, &mut ranges)?;
        self.collect_comment_folding_ranges(file_id, &mut ranges)?;
        self.collect_region_folding_ranges(file_id, &mut ranges)?;

        // retain source order and merge equivalent source containers
        ranges.sort_by(|left, right| {
            left.start_line
                .cmp(&right.start_line)
                .then(right.end_line.cmp(&left.end_line))
                .then(left.kind.cmp(&right.kind))
                .then(left.collapsed_text.cmp(&right.collapsed_text))
        });
        ranges.dedup();

        Ok(FoldingRangesResponse { ranges })
    }
}

/// One explicit source region marker.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RegionMarker {
    /// Start a region with optional collapsed text.
    Start(Option<String>),
    /// End the innermost region.
    End,
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

    /// Extend this block if the line is contiguous.
    fn extend(&mut self, line: u32) -> bool {
        let is_next_line = self.end_line + 1 == line;
        if is_next_line {
            self.end_line = line;
        }

        is_next_line
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
        if let Some(block) = &mut self.current
            && block.extend(line)
        {
            return;
        }

        self.flush(ranges);
        self.current = Some(LineCommentBlock::new(line));
    }

    /// Flush the current block into the folding ranges.
    fn flush(&mut self, ranges: &mut Vec<FoldingRange>) {
        let Some(block) = self.current.take() else {
            return;
        };

        if block.end_line > block.start_line {
            ranges.push(FoldingRange::comment(block.start_line, block.end_line));
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Collect contiguous top-level import folding ranges.
    fn collect_import_folding_ranges(
        &self,
        file_id: FileId,
        ranges: &mut Vec<FoldingRange>,
    ) -> QueryResult<()> {
        let file = self.read_file(file_id)?;
        let view = self.view()?;
        let mut import_lines = None;

        // group consecutive authored imports
        for expression_id in self.file_roots(file_id)? {
            if !matches!(
                view.get::<dir::Expression>(*expression_id),
                dir::Expression::Import { .. }
            ) {
                Self::flush_import_folding_range(&mut import_lines, ranges);
                continue;
            }

            let span = self.node_span(view, (*expression_id).into())?;
            let (start_line, _) = file
                .get_position(span.start)
                .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;
            let (end_line, _) = file
                .get_position(span.end)
                .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;

            match &mut import_lines {
                Some((_, group_end)) => *group_end = end_line,
                None => import_lines = Some((start_line, end_line)),
            }
        }

        Self::flush_import_folding_range(&mut import_lines, ranges);

        Ok(())
    }

    /// Flush one pending import section.
    fn flush_import_folding_range(
        import_lines: &mut Option<(u32, u32)>,
        ranges: &mut Vec<FoldingRange>,
    ) {
        let Some((start_line, end_line)) = import_lines.take() else {
            return;
        };
        if end_line <= start_line {
            return;
        }

        let mut range = FoldingRange::new(start_line, end_line);
        range.kind = Some(FoldingRangeKind::Imports);
        ranges.push(range);
    }

    /// Collect node and named-container folding ranges.
    fn collect_node_folding_ranges(
        &self,
        file_id: FileId,
        ranges: &mut Vec<FoldingRange>,
    ) -> QueryResult<()> {
        let file = self.read_file(file_id)?;
        let view = self.view()?;

        // collect declaration extents
        for (declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
            if !Self::declaration_is_foldable(declaration) {
                continue;
            }

            let Some(span) = view.get_span_by_id(declaration_id.id) else {
                continue;
            };
            if span.file != file_id {
                continue;
            }

            Self::push_folding_range(span, None, None, &file, ranges)?;
        }

        // collect block extents
        for (block_id, _) in view.iter_nodes::<dir::Block>() {
            if Self::block_is_function_declaration_body(view, block_id) {
                continue;
            }

            let Some(span) = view.get_span_by_id(block_id.id) else {
                continue;
            };
            if span.file == file_id {
                Self::push_folding_range(span, None, None, &file, ranges)?;
            }
        }

        // collect expression containers
        for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
            if !matches!(
                expression,
                dir::Expression::ArrayExpression { .. }
                    | dir::Expression::FixedArrayExpression { .. }
                    | dir::Expression::TupleExpression { .. }
                    | dir::Expression::ObjectExpression { .. }
                    | dir::Expression::StructExpression { .. }
                    | dir::Expression::Call { .. }
                    | dir::Expression::New { .. }
            ) {
                continue;
            }

            let Some(span) = view.get_span_by_id(expression_id.id) else {
                continue;
            };
            if span.file == file_id {
                Self::push_folding_range(span, None, None, &file, ranges)?;
            }
        }

        // collect structural type containers
        for (type_id, type_expression) in view.iter_nodes::<dir::TypeExpression>() {
            if !matches!(
                type_expression,
                dir::TypeExpression::Tuple { .. } | dir::TypeExpression::Object { .. }
            ) {
                continue;
            }

            let Some(span) = view.get_span_by_id(type_id.id) else {
                continue;
            };
            if span.file == file_id {
                Self::push_folding_range(span, None, None, &file, ranges)?;
            }
        }

        // collect named source containers from every visible node
        let regions = [
            NodeSpanRegion::GenericParameters,
            NodeSpanRegion::Parameters,
            NodeSpanRegion::Attributes,
            NodeSpanRegion::Parentheses,
            NodeSpanRegion::TreeContainer,
        ];
        for node_id in view.iter_node_ids() {
            let source_id = view.get_source_any(node_id);
            for region in regions {
                let span_type = NodeSpanType::Region(region);
                let Some(span) = self.source_index()?.get_side(source_id, span_type) else {
                    continue;
                };
                if span.file == file_id {
                    Self::push_folding_range(span, None, None, &file, ranges)?;
                }
            }
        }

        Ok(())
    }

    /// Return whether a block duplicates its containing function declaration fold.
    fn block_is_function_declaration_body(
        view: dir::View<'_>,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> bool {
        let Some(expression_id) = view.get_parent_any(block_id.into()) else {
            return false;
        };
        if expression_id.ty != dir::NodeType::Expression {
            return false;
        }
        let Some(declaration_id) = view.get_parent_any(expression_id) else {
            return false;
        };
        if declaration_id.ty != dir::NodeType::Declaration {
            return false;
        }

        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(declaration_id.id);

        matches!(
            view.get(declaration_id),
            dir::Declaration::Function(function)
                if function.signature.form == dir::FunctionForm::Function
        )
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
                | dir::Declaration::Module { .. }
                | dir::Declaration::Extension { .. }
        )
    }

    /// Collect comment folding ranges.
    fn collect_comment_folding_ranges(
        &self,
        file_id: FileId,
        ranges: &mut Vec<FoldingRange>,
    ) -> QueryResult<()> {
        let file = self.read_file(file_id)?;
        let mut line_comment_blocks = LineCommentBlocks::default();

        // scan comments in source order
        for comment in self.comments(file_id)? {
            if Self::region_marker(comment, &file)?.is_some() {
                line_comment_blocks.flush(ranges);
                continue;
            }

            self.collect_comment_folding_range(ranges, &mut line_comment_blocks, comment, &file)?;
        }

        line_comment_blocks.flush(ranges);

        Ok(())
    }

    /// Collect the folding range for one comment.
    fn collect_comment_folding_range(
        &self,
        ranges: &mut Vec<FoldingRange>,
        line_comment_blocks: &mut LineCommentBlocks,
        comment: &dir::Comment,
        file: &File,
    ) -> QueryResult<()> {
        if comment.is_line() {
            let (start_line, _) =
                file.get_position(comment.span.start)
                    .ok_or(QueryError::invalid(format!(
                        "source span: {:?}",
                        comment.span
                    )))?;

            line_comment_blocks.insert(start_line, ranges);
        } else {
            line_comment_blocks.flush(ranges);
            self.collect_block_comment_folding_range(ranges, comment, file)?;
        }

        Ok(())
    }

    /// Collect explicitly marked source regions.
    fn collect_region_folding_ranges(
        &self,
        file_id: FileId,
        ranges: &mut Vec<FoldingRange>,
    ) -> QueryResult<()> {
        let file = self.read_file(file_id)?;
        let mut regions = Vec::new();

        // pair nested region markers in source order
        for comment in self.comments(file_id)? {
            let Some(marker) = Self::region_marker(comment, &file)? else {
                continue;
            };
            let (line, _) = file
                .get_position(comment.span.start)
                .ok_or(QueryError::invalid(format!(
                    "source span: {:?}",
                    comment.span
                )))?;

            match marker {
                RegionMarker::Start(collapsed_text) => regions.push((line, collapsed_text)),
                RegionMarker::End => {
                    let Some((start_line, collapsed_text)) = regions.pop() else {
                        continue;
                    };
                    if line > start_line {
                        ranges.push(FoldingRange {
                            start_line,
                            end_line: line,
                            start_character: None,
                            end_character: None,
                            kind: Some(FoldingRangeKind::Region),
                            collapsed_text,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse one line comment as a source region marker.
    fn region_marker(comment: &dir::Comment, file: &File) -> QueryResult<Option<RegionMarker>> {
        if !comment.is_line() {
            return Ok(None);
        }

        let text = file
            .get_span_str(comment.span)
            .ok_or(QueryError::invalid(format!(
                "source span: {:?}",
                comment.span
            )))?;
        let Some(text) = text.strip_prefix("//") else {
            return Ok(None);
        };
        let text = text.trim();
        let marker = if text == "#endregion" {
            Some(RegionMarker::End)
        } else if let Some(name) = text.strip_prefix("#region")
            && name.chars().next().is_none_or(char::is_whitespace)
        {
            let name = name.trim();
            let name = (!name.is_empty()).then(|| name.to_string());

            Some(RegionMarker::Start(name))
        } else {
            None
        };

        Ok(marker)
    }

    /// Append one multiline source span as a folding range.
    fn push_folding_range(
        span: Span,
        kind: Option<FoldingRangeKind>,
        collapsed_text: Option<String>,
        file: &File,
        ranges: &mut Vec<FoldingRange>,
    ) -> QueryResult<()> {
        let (start_line, _) = file
            .get_position(span.start)
            .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;
        let (end_line, _) = file
            .get_position(span.end)
            .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;
        if end_line <= start_line {
            return Ok(());
        }

        ranges.push(FoldingRange {
            start_line,
            end_line,
            start_character: None,
            end_character: None,
            kind,
            collapsed_text,
        });

        Ok(())
    }

    /// Collect the folding range for one block comment.
    fn collect_block_comment_folding_range(
        &self,
        ranges: &mut Vec<FoldingRange>,
        comment: &dir::Comment,
        file: &File,
    ) -> QueryResult<()> {
        let (start_line, _) = file
            .get_position(comment.span.start)
            .ok_or(QueryError::invalid(format!(
                "source span: {:?}",
                comment.span
            )))?;
        let (end_line, _) = file
            .get_position(comment.span.end)
            .ok_or(QueryError::invalid(format!(
                "source span: {:?}",
                comment.span
            )))?;

        if end_line > start_line {
            ranges.push(FoldingRange::comment(start_line, end_line));
        }

        Ok(())
    }
}
