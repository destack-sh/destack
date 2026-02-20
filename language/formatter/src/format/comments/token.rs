use ast::{TokenSpan, TokenType};
use destack_ast as ast;

/// Return whether one token kind is an opening delimiter.
#[inline]
pub(super) fn is_open_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
    )
}

/// Return whether one token kind is a closing delimiter.
#[inline]
pub(super) fn is_close_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
    )
}

/// Return whether one open and close delimiter token pair matches.
#[inline]
pub(super) fn delimiters_match(open: TokenType, close: TokenType) -> bool {
    matches!(
        (open, close),
        (TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            | (TokenType::OpenBrace, TokenType::CloseBrace)
            | (TokenType::OpenBracket, TokenType::CloseBracket)
    )
}

/// Return whether one token after a comment seam prefers left ownership.
#[inline]
pub(super) fn token_after_prefers_left_ownership(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Semicolon
            | TokenType::Comma
            | TokenType::CloseParenthesis
            | TokenType::CloseBrace
            | TokenType::CloseBracket
            | TokenType::ElementwiseAnd
            | TokenType::ElementwiseOr
            | TokenType::ElementwiseXor
            | TokenType::LogicalAnd
            | TokenType::LogicalOr
            | TokenType::Coalesce
            | TokenType::Equal
            | TokenType::EqualWide
            | TokenType::NotEqual
            | TokenType::NotEqualWide
            | TokenType::LessThan
            | TokenType::LessThanOrEqual
            | TokenType::GreaterThan
            | TokenType::GreaterThanOrEqual
            | TokenType::Add
            | TokenType::WrappingAdd
            | TokenType::SaturatingAdd
            | TokenType::Subtract
            | TokenType::WrappingSubtract
            | TokenType::SaturatingSubtract
            | TokenType::Multiply
            | TokenType::WrappingMultiply
            | TokenType::SaturatingMultiply
            | TokenType::Exponent
            | TokenType::WrappingExponent
            | TokenType::SaturatingExponent
            | TokenType::Divide
            | TokenType::Remainder
            | TokenType::ShiftLeft
            | TokenType::SaturatingShiftLeft
            | TokenType::Assign
    )
}

/// Return the previous non-newline semantic token index before one index.
pub(super) fn previous_non_newline_token_index(
    semantic_tokens: &[TokenSpan],
    index: usize,
) -> Option<usize> {
    if index == 0 {
        return None;
    }

    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        if semantic_tokens[cursor].token.ty != TokenType::Newline {
            return Some(cursor);
        }
    }

    None
}
