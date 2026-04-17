use super::context::DestackFormatContext;
use destack_ast::{Comment, Expression, LocalNodeId};
use destack_source::{NodeSpanType, SourcePartKey, Span};

/// One formatter-side normalized view of a preserved parenthesized expression.
#[derive(Debug, Copy, Clone)]
pub struct ParenthesizedExpressionView<'context, 'ast> {
    /// The shared formatter context.
    context: &'context DestackFormatContext<'ast>,
    /// The outer parenthesized expression node.
    node_id: LocalNodeId<Expression>,
    /// The inner wrapped expression node.
    inner_expression_id: LocalNodeId<Expression>,
}

impl<'context, 'ast> ParenthesizedExpressionView<'context, 'ast> {
    /// Return one explicit side span on the wrapped inner expression.
    fn inner_side_span(self, span_type: NodeSpanType) -> Option<Span> {
        self.context
            .tree
            .get_side_span(self.inner_expression_id, span_type)
    }

    /// Return structurally attached comments on one explicit inner side span.
    fn structural_comments_on_inner_side(self, span_type: NodeSpanType) -> Vec<Comment> {
        self.context
            .tree
            .comments()
            .iter()
            .copied()
            .filter(|comment| {
                comment.attached_part == SourcePartKey::new(self.inner_expression_id.id, span_type)
            })
            .collect()
    }

    /// Return structurally attached comments inside one byte range.
    fn structural_comments_in_range(self, start: u32, end: u32) -> Vec<Comment> {
        if start >= end {
            return Vec::new();
        }

        self.context
            .tree
            .comments()
            .iter()
            .copied()
            .filter(|comment| comment.span.end > start && comment.span.end <= end)
            .collect()
    }

    /// Create one normalized parenthesized-expression view.
    pub fn from_node(
        context: &'context DestackFormatContext<'ast>,
        node_id: LocalNodeId<Expression>,
    ) -> Option<Self> {
        let Expression::Parenthesized { expression } = context.tree.get(node_id) else {
            return None;
        };

        let inner_expression_id = *expression;
        let parenthesized_span = context.span(node_id);
        let inner_span = context.span(inner_expression_id);
        if parenthesized_span.file != inner_span.file
            || parenthesized_span.start >= inner_span.start
            || inner_span.end >= parenthesized_span.end
        {
            return None;
        }

        Some(Self {
            context,
            node_id,
            inner_expression_id,
        })
    }

    /// Return the parenthesized wrapper node id.
    #[inline]
    pub const fn node_id(self) -> LocalNodeId<Expression> {
        self.node_id
    }

    /// Return the wrapped inner expression id.
    #[inline]
    pub const fn inner_expression_id(self) -> LocalNodeId<Expression> {
        self.inner_expression_id
    }

    /// Return the full wrapper span.
    #[inline]
    pub fn span(self) -> Span {
        self.context.span(self.node_id)
    }

    /// Return the inner expression span.
    #[inline]
    pub fn inner_span(self) -> Span {
        self.context.span(self.inner_expression_id)
    }

    /// Return the explicit leading span between `(` and the inner head.
    #[inline]
    pub fn leading_inner_span(self) -> Option<Span> {
        self.inner_side_span(NodeSpanType::Leading)
    }

    /// Return the explicit trailing span between the inner tail and `)`.
    #[inline]
    pub fn trailing_inner_span(self) -> Option<Span> {
        self.inner_side_span(NodeSpanType::Trailing)
    }

    /// Return comments between `(` and the inner expression.
    pub fn leading_inner_comments(self) -> Vec<Comment> {
        self.structural_comments_on_inner_side(NodeSpanType::Leading)
    }

    /// Return comments that belong immediately before this wrapper's closing `)`.
    pub fn trailing_inner_comments(self) -> Vec<Comment> {
        self.structural_comments_on_inner_side(NodeSpanType::Trailing)
    }

    /// Return postfix comments between this wrapper's closing `)` and the next continuation token.
    pub fn postfix_comments(self) -> Vec<Comment> {
        let Some(next_token) = self.context.next_non_trivia_token_after_span(self.span()) else {
            return Vec::new();
        };
        let parenthesized_span = self.span();
        if next_token.span.file != parenthesized_span.file
            || parenthesized_span.end >= next_token.span.start
        {
            return Vec::new();
        }

        self.structural_comments_in_range(parenthesized_span.end, next_token.span.start)
    }

    /// Return block comments immediately before this wrapper's closing `)`.
    pub fn trailing_inner_block_comments(self) -> Vec<Comment> {
        self.trailing_inner_comments()
            .into_iter()
            .filter(|comment| comment.is_block())
            .collect()
    }

    /// Return whether source contains any trivia between `(` and the inner expression.
    #[inline]
    pub fn has_leading_inner_trivia(self) -> bool {
        self.has_leading_inner_pattern(true)
    }

    /// Return whether source contains comments between `(` and the inner expression.
    #[inline]
    pub fn has_leading_inner_comments(self) -> bool {
        self.has_leading_inner_pattern(false)
    }

    /// Return whether source contains a newline between `(` and the inner expression.
    pub fn has_leading_inner_newline(self) -> bool {
        self.inner_side_span(NodeSpanType::Leading)
            .is_some_and(|leading_span| self.context.has_newline(leading_span))
    }

    /// Return whether source contains a leading line comment between `(` and the inner expression.
    pub fn has_leading_inner_line_comment(self) -> bool {
        self.leading_inner_comments()
            .iter()
            .any(|comment| comment.is_line())
    }

    /// Return whether source contains leading comments or newlines before the inner expression.
    fn has_leading_inner_pattern(self, include_newline: bool) -> bool {
        let Some(leading_span) = self.inner_side_span(NodeSpanType::Leading) else {
            return false;
        };
        if include_newline && self.context.has_newline(leading_span) {
            return true;
        }

        !self.leading_inner_comments().is_empty()
    }
}
