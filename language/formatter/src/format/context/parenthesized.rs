use super::context::DestackFormatContext;
use destack_ast::{Comment, Expression, LocalNodeId, TokenSpan, TokenType};
use destack_source::Span;

/// One formatter-side normalized view of a preserved parenthesized expression.
#[derive(Debug, Copy, Clone)]
pub struct ParenthesizedExpressionView<'context, 'ast> {
    /// The shared formatter context.
    context: &'context DestackFormatContext<'ast>,
    /// The outer parenthesized expression node.
    node_id: LocalNodeId<Expression>,
    /// The inner wrapped expression node.
    inner_expression_id: LocalNodeId<Expression>,
    /// The explicit opening delimiter token.
    open_parenthesis: TokenSpan,
    /// The explicit closing delimiter token.
    close_parenthesis: TokenSpan,
}

impl<'context, 'ast> ParenthesizedExpressionView<'context, 'ast> {
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

        let open_parenthesis = context.previous_non_whitespace_token_before_span(inner_span)?;
        if open_parenthesis.token.ty != TokenType::OpenParenthesis
            || open_parenthesis.span.file != parenthesized_span.file
            || open_parenthesis.span.start < parenthesized_span.start
            || open_parenthesis.span.end > parenthesized_span.end
        {
            return None;
        }

        let close_parenthesis = context.next_non_whitespace_token_after_span(inner_span)?;
        if close_parenthesis.token.ty != TokenType::CloseParenthesis
            || close_parenthesis.span.file != parenthesized_span.file
            || close_parenthesis.span.start < parenthesized_span.start
            || close_parenthesis.span.end > parenthesized_span.end
        {
            return None;
        }

        Some(Self {
            context,
            node_id,
            inner_expression_id,
            open_parenthesis,
            close_parenthesis,
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

    /// Return the explicit opening parenthesis token.
    #[inline]
    pub const fn open_parenthesis(self) -> TokenSpan {
        self.open_parenthesis
    }

    /// Return the explicit closing parenthesis token.
    #[inline]
    pub const fn close_parenthesis(self) -> TokenSpan {
        self.close_parenthesis
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

    /// Return the first inner non-trivia token start.
    #[inline]
    pub fn inner_token_start(self) -> u32 {
        let inner_span = self.inner_span();

        self.context
            .first_non_trivia_token_in_span(inner_span)
            .map_or(inner_span.start, |token| token.span.start)
    }

    /// Return comments between `(` and the inner expression.
    pub fn leading_inner_comments(self) -> Vec<Comment> {
        let leading_start = self.open_parenthesis.span.end;
        let inner_start = self.inner_token_start();
        if leading_start >= inner_start {
            return Vec::new();
        }

        {
            let comments = self.context.comments();
            comments
                .comments_in_range(leading_start, inner_start)
                .to_vec()
        }
    }

    /// Return comments that belong immediately before this wrapper's closing `)`.
    pub fn trailing_inner_comments(self) -> Vec<Comment> {
        {
            let comments = self.context.comments();
            comments
                .comments_in_range(
                    self.open_parenthesis.span.end,
                    self.close_parenthesis.span.start,
                )
                .to_vec()
        }
        .into_iter()
        .filter(|comment| {
            self.context
                .next_non_whitespace_token_after_span(comment.span)
                .is_some_and(|token| {
                    token.token.ty == TokenType::CloseParenthesis
                        && token.span.start == self.close_parenthesis.span.start
                        && token.span.end == self.close_parenthesis.span.end
                })
        })
        .collect()
    }

    /// Return postfix comments between this wrapper's closing `)` and the next continuation token.
    pub fn postfix_comments(self) -> Vec<Comment> {
        let Some(next_token) = self
            .context
            .next_non_trivia_token_after_span(self.close_parenthesis.span)
        else {
            return Vec::new();
        };
        if next_token.span.file != self.close_parenthesis.span.file
            || self.close_parenthesis.span.end >= next_token.span.start
        {
            return Vec::new();
        }

        {
            let comments = self.context.comments();
            comments
                .comments_in_range(self.close_parenthesis.span.end, next_token.span.start)
                .to_vec()
        }
    }

    /// Return block comments owned at the wrapper boundary after the inner expression.
    pub fn boundary_comments(self) -> Vec<Comment> {
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
        let leading_start = self.open_parenthesis.span.end;
        let inner_span = self.inner_span();
        let inner_start = self.inner_token_start();
        if leading_start >= inner_start {
            return false;
        }

        self.context
            .has_newline(Span::new(inner_span.file, leading_start, inner_start))
    }

    /// Return whether source contains a leading line comment between `(` and the inner expression.
    pub fn has_leading_inner_line_comment(self) -> bool {
        self.leading_inner_comments()
            .iter()
            .any(|comment| comment.is_line())
    }

    /// Return whether source contains leading comments or newlines before the inner expression.
    fn has_leading_inner_pattern(self, include_newline: bool) -> bool {
        let leading_start = self.open_parenthesis.span.end;
        let inner_span = self.inner_span();
        let inner_start = self.inner_token_start();
        if leading_start >= inner_start {
            return false;
        }

        let leading_span = Span::new(inner_span.file, leading_start, inner_start);
        if include_newline && self.context.has_newline(leading_span) {
            return true;
        }

        {
            let comments = self.context.comments();
            !comments
                .comments_in_range(leading_span.start, leading_span.end)
                .is_empty()
        }
    }
}
