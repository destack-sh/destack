use destack_ast::{TokenSpan, TokenType};

/// Return whether one token type is whitespace trivia.
#[inline]
pub(crate) fn token_type_is_whitespace_trivia(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return whether one token type is comment trivia.
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

/// Return whether one token type is any trivia.
#[inline]
pub(crate) fn token_type_is_trivia(token_type: TokenType) -> bool {
    token_type_is_whitespace_trivia(token_type) || token_type_is_comment_trivia(token_type)
}

/// Return the previous semantic token index before one index that is not trivia.
pub(crate) fn previous_non_trivia_token_index(
    semantic_tokens: &[TokenSpan],
    index: usize,
) -> Option<usize> {
    if index == 0 {
        return None;
    }

    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let token_type = semantic_tokens[cursor].token.ty;
        if !token_type_is_trivia(token_type) {
            return Some(cursor);
        }
    }

    None
}

/// Return the next semantic token index after one index that is not trivia.
pub(crate) fn next_non_trivia_token_index(
    semantic_tokens: &[TokenSpan],
    index: usize,
) -> Option<usize> {
    let mut cursor = index + 1;
    while cursor < semantic_tokens.len() {
        let token_type = semantic_tokens[cursor].token.ty;
        if !token_type_is_trivia(token_type) {
            return Some(cursor);
        }

        cursor += 1;
    }

    None
}

/// Return the next token type after one seam while skipping newline tokens only.
pub(crate) fn next_non_newline_token_type_after_seam(
    semantic_tokens: &[TokenSpan],
    token_after_type: Option<TokenType>,
    token_after_index: Option<usize>,
) -> Option<TokenType> {
    if token_after_type.is_some_and(|token_type| token_type != TokenType::Newline) {
        return token_after_type;
    }

    let token_after_index = token_after_index?;
    for token in semantic_tokens.iter().skip(token_after_index) {
        if token.token.ty != TokenType::Newline {
            return Some(token.token.ty);
        }
    }

    None
}

/// Return whether one seam token snapshot is one JSX closing-tag head seam.
#[inline]
pub(crate) fn is_tree_closing_tag_head_seam(
    token_before_type: Option<TokenType>,
    token_before_predecessor_type: Option<TokenType>,
    token_after_type: Option<TokenType>,
    token_after_successor_type: Option<TokenType>,
) -> bool {
    let token_before_is_less_than = token_before_type == Some(TokenType::LessThan);
    let token_before_is_divide = token_before_type == Some(TokenType::Divide);
    let token_before_is_identifier = token_before_type == Some(TokenType::Identifier);
    let token_after_is_divide = token_after_type == Some(TokenType::Divide);
    let token_after_is_identifier = token_after_type == Some(TokenType::Identifier);
    let token_after_is_greater_than = token_after_type == Some(TokenType::GreaterThan);

    // before slash: `</*comment*/tag>`
    let seam_is_before_closing_slash = token_before_is_less_than
        && token_after_is_divide
        && matches!(
            token_after_successor_type,
            Some(TokenType::Identifier | TokenType::GreaterThan)
        );
    // after slash: `</*comment*/tag>` or `</*comment*/>`
    let seam_is_after_closing_slash = token_before_is_divide
        && token_before_predecessor_type == Some(TokenType::LessThan)
        && (token_after_is_identifier || token_after_is_greater_than);
    // after name: `</tag/*comment*/>`
    let seam_is_after_closing_tag_name = token_before_is_identifier
        && token_after_is_greater_than
        && token_before_predecessor_type == Some(TokenType::Divide);

    seam_is_before_closing_slash || seam_is_after_closing_slash || seam_is_after_closing_tag_name
}
