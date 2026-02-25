use destack_ast::{
    Declaration, Expression, FunctionKind, IfKind, LocalNodeId, TokenSpan, TokenType, WhileKind,
};

use crate::DestackFormatContext;

/// One normalized semicolon guard seam classification for own-line comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemicolonGuardCommentSeam {
    /// No semicolon guard seam was detected.
    None,
    /// The comment is between one semicolon and one guard head token.
    BeforeComment {
        /// The guard head token after the comment.
        target_type: TokenType,
    },
    /// The comment is followed by one semicolon and one guard target token.
    AfterComment {
        /// The first non-trivia guard target token after the semicolon.
        target_type: TokenType,
    },
}

impl SemicolonGuardCommentSeam {
    /// Return whether this seam classification has one semicolon guard shape.
    #[inline]
    pub(crate) fn has_guard_shape(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Return whether this seam is the semicolon-after-comment guard shape.
    #[inline]
    pub(crate) fn is_after_comment(self) -> bool {
        matches!(self, Self::AfterComment { .. })
    }
}

/// Return whether one token type is one whitespace trivia token.
#[inline]
pub(crate) fn token_type_is_whitespace_trivia(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return whether one token type is one comment trivia token.
#[inline]
pub(crate) fn token_type_is_comment_trivia(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

/// Return whether one token type is trivia.
#[inline]
pub(crate) fn token_type_is_trivia(token_type: TokenType) -> bool {
    token_type_is_whitespace_trivia(token_type) || token_type_is_comment_trivia(token_type)
}

/// Return whether one token type can start one semicolon-guarded statement head.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_head(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenBracket | TokenType::OpenParenthesis
    )
}

/// Return whether one token type can be one guard target after semicolon.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_target(token_type: TokenType) -> bool {
    !matches!(token_type, TokenType::Semicolon) && !token_type_is_trivia(token_type)
}

/// Return the next non-trivia token type after one semantic token index.
pub(crate) fn next_non_trivia_token_type(
    semantic_tokens: &[TokenSpan],
    index: usize,
) -> Option<TokenType> {
    let mut cursor = index + 1;
    while cursor < semantic_tokens.len() {
        let token_type = semantic_tokens[cursor].token.ty;
        if !token_type_is_trivia(token_type) {
            return Some(token_type);
        }

        cursor += 1;
    }

    None
}

/// Classify one semicolon guard seam around one own-line comment.
pub(crate) fn classify_semicolon_guard_comment_seam(
    semantic_tokens: &[TokenSpan],
    token_before_type: Option<TokenType>,
    token_after_type: Option<TokenType>,
    token_after_index: Option<usize>,
) -> SemicolonGuardCommentSeam {
    if token_before_type == Some(TokenType::Semicolon)
        && token_after_type.is_some_and(token_type_is_semicolon_guard_head)
    {
        if let Some(target_type) = token_after_type {
            return SemicolonGuardCommentSeam::BeforeComment { target_type };
        }
    }

    if token_after_type == Some(TokenType::Semicolon)
        && let Some(token_after_index) = token_after_index
        && let Some(target_type) = next_non_trivia_token_type(semantic_tokens, token_after_index)
        && token_type_is_semicolon_guard_target(target_type)
    {
        return SemicolonGuardCommentSeam::AfterComment { target_type };
    }

    SemicolonGuardCommentSeam::None
}

/// Return whether one block expression needs a trailing statement terminator.
pub(crate) fn expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
    is_expression_context_tail: bool,
) -> bool {
    if is_expression_context_tail {
        return false;
    }

    matches!(
        expression,
        Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
    ) || matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    ) || matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function {
                    descriptor,
                    signature,
                    ..
                }
                if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
            )
    ) || (!matches!(expression, Expression::Statement(_))
        && !matches!(expression, Expression::Stub | Expression::Error)
        && !expression.ends_statement_on_newline())
}

/// Return whether one statement wrapper should keep its trailing semicolon.
pub(crate) fn statement_wrapper_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    if let Expression::Statement(inner_expression_id) = expression {
        return statement_wrapper_needs_semicolon(context, *inner_expression_id);
    }

    if matches!(
        expression,
        Expression::Declaration(_) | Expression::Block(_)
    ) {
        return false;
    }

    if matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While {
            kind: WhileKind::While,
            ..
        } | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Try { .. }
            | Expression::Match { .. }
            | Expression::Labelled { .. }
    ) {
        return false;
    }

    true
}
