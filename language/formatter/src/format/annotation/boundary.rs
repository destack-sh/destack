use ast::{AnnotationPosition, Keyword, NodeParentIndex, NodeTree, TokenSpan, TokenType};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::FxHashMap;

use super::ownership::find_smallest_owner_enclosing_range;

/// Return whether one token kind is an opening delimiter.
#[inline]
pub(crate) fn is_open_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
    )
}

/// Return whether one token kind is a closing delimiter.
#[inline]
pub(crate) fn is_close_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
    )
}

/// Return whether one open and close delimiter token pair matches.
#[inline]
pub(crate) fn delimiters_match(open: TokenType, close: TokenType) -> bool {
    matches!(
        (open, close),
        (TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            | (TokenType::OpenBrace, TokenType::CloseBrace)
            | (TokenType::OpenBracket, TokenType::CloseBracket)
    )
}

/// Return whether one token after a comment seam prefers preceding ownership.
#[inline]
pub(crate) fn token_after_prefers_preceding_ownership(token_type: TokenType) -> bool {
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
pub(crate) fn previous_non_newline_token_index(
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

/// One normalized identifier keyword used in comment seam rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentSeamKeyword {
    /// One non-keyword identifier.
    None,
    /// One `as` keyword.
    As,
    /// One `satisfies` keyword.
    Satisfies,
    /// One `export` keyword.
    Export,
    /// One `implements` keyword.
    Implements,
    /// One `else` keyword.
    Else,
    /// One `case` keyword.
    Case,
    /// One `default` keyword.
    Default,
    /// One `const` keyword.
    Const,
}

/// Classify one identifier token into one seam keyword family.
#[inline]
pub(crate) fn comment_seam_keyword(
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    token: Option<TokenSpan>,
) -> CommentSeamKeyword {
    let Some(token) = token else {
        return CommentSeamKeyword::None;
    };

    if token.token.ty != TokenType::Identifier {
        return CommentSeamKeyword::None;
    }

    let keyword = token_keyword_by_span.get(&token.span).copied().flatten();
    let Some(keyword) = keyword else {
        return CommentSeamKeyword::None;
    };

    match keyword {
        Keyword::As => CommentSeamKeyword::As,
        Keyword::Satisfies => CommentSeamKeyword::Satisfies,
        Keyword::Export => CommentSeamKeyword::Export,
        Keyword::Implements => CommentSeamKeyword::Implements,
        Keyword::Else => CommentSeamKeyword::Else,
        Keyword::Case => CommentSeamKeyword::Case,
        Keyword::Default => CommentSeamKeyword::Default,
        Keyword::Const => CommentSeamKeyword::Const,
        _ => CommentSeamKeyword::None,
    }
}

/// Immutable context for one comment seam attachment.
#[derive(Clone, Copy)]
pub(crate) struct CommentSeamContext<'a> {
    /// The source file.
    pub(crate) file: &'a File,
    /// The syntax tree.
    pub(crate) tree: &'a NodeTree,
    /// The semantic token stream.
    pub(crate) semantic_tokens: &'a [TokenSpan],
    /// Parsed identifier keywords by token span.
    pub(crate) token_keyword_by_span: &'a FxHashMap<Span, Option<Keyword>>,
    /// The comment trivia payload.
    pub(crate) trivia: destack_ast::CommentTrivia,
    /// Parent links for owner promotion.
    pub(crate) parents: &'a NodeParentIndex,
    /// Token index before the seam.
    pub(crate) token_before: Option<usize>,
    /// Token index after the seam.
    pub(crate) token_after: Option<usize>,
    /// Token span before the seam.
    pub(crate) token_before_span: Option<TokenSpan>,
    /// Token span after the seam.
    pub(crate) token_after_span: Option<TokenSpan>,
}

/// Compact seam signals derived once per comment seam.
#[derive(Clone, Copy)]
pub(crate) struct CommentSeamData {
    /// Whether trivia has at least one newline before comment text.
    pub(crate) has_leading_newline: bool,
    /// Whether trivia has at least one newline after comment text.
    pub(crate) has_trailing_newline: bool,
    /// Whether comment style is `//`.
    pub(crate) comment_is_line: bool,
    /// Whether comment style is `/* */`.
    pub(crate) comment_is_star: bool,
    /// Whether `/* */` comment text spans multiple lines.
    pub(crate) comment_is_multiline_star: bool,
    /// Token kind before seam.
    pub(crate) token_before_type: Option<TokenType>,
    /// Token kind after seam.
    pub(crate) token_after_type: Option<TokenType>,
    /// Keyword class for identifier before seam.
    pub(crate) token_before_keyword: CommentSeamKeyword,
    /// Keyword class for identifier after seam.
    pub(crate) token_after_keyword: CommentSeamKeyword,
    /// Whether token after seam structurally prefers preceding ownership.
    pub(crate) token_after_prefers_preceding: bool,
    /// Whether seam is one return type boundary after `):`.
    pub(crate) token_before_is_return_type_colon: bool,
    /// Whether default trailing behavior should prefer following binding.
    pub(crate) seam_binds_right: bool,
}

impl CommentSeamData {
    /// Build one seam fact snapshot.
    pub(crate) fn build(context: &CommentSeamContext<'_>) -> Self {
        let token_before_type = context.token_before_span.map(|token| token.token.ty);
        let token_after_type = context.token_after_span.map(|token| token.token.ty);
        let token_before_keyword =
            comment_seam_keyword(context.token_keyword_by_span, context.token_before_span);
        let token_after_keyword =
            comment_seam_keyword(context.token_keyword_by_span, context.token_after_span);
        let has_leading_newline = context.trivia.boundary.newlines.has_leading_newline();
        let has_trailing_newline = context.trivia.boundary.newlines.has_trailing_newline();
        let comment_style = context.tree.get(context.trivia.comment).style;
        let comment_is_line = comment_style == ast::CommentStyle::Slash;
        let comment_is_star = comment_style == ast::CommentStyle::Star;
        let comment_is_multiline_star = comment_is_star && {
            let comment_end = context.trivia.span.end.saturating_sub(1);
            !context
                .file
                .is_same_line(context.trivia.span.start, comment_end)
        };
        let token_after_prefers_preceding =
            token_after_type.is_some_and(token_after_prefers_preceding_ownership);
        let token_before_is_return_type_colon = context
            .token_before
            .and_then(|index| previous_non_newline_token_index(context.semantic_tokens, index))
            .is_some_and(|index| {
                context.semantic_tokens[index].token.ty == TokenType::CloseParenthesis
            })
            && token_before_type == Some(TokenType::Colon);
        let seam_binds_right = token_before_type.is_some_and(is_open_delimiter_token)
            || matches!(
                token_before_type,
                Some(TokenType::Arrow | TokenType::ArrowWide)
            )
            || token_before_type == Some(TokenType::Colon)
            || token_before_keyword == CommentSeamKeyword::Export
            || token_before_keyword == CommentSeamKeyword::Satisfies
            || token_before_keyword == CommentSeamKeyword::As
            || token_before_type == Some(TokenType::Assign);

        Self {
            has_leading_newline,
            has_trailing_newline,
            comment_is_line,
            comment_is_star,
            comment_is_multiline_star,
            token_before_type,
            token_after_type,
            token_before_keyword,
            token_after_keyword,
            token_after_prefers_preceding,
            token_before_is_return_type_colon,
            seam_binds_right,
        }
    }

    /// Return whether token before seam has one type.
    #[inline]
    pub(crate) fn token_before_is(self, token_type: TokenType) -> bool {
        self.token_before_type == Some(token_type)
    }

    /// Return whether token after seam has one type.
    #[inline]
    pub(crate) fn token_after_is(self, token_type: TokenType) -> bool {
        self.token_after_type == Some(token_type)
    }

    /// Return whether token before seam is one keyword.
    #[inline]
    pub(crate) fn token_before_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_before_keyword == keyword
    }

    /// Return whether token after seam is one keyword.
    #[inline]
    pub(crate) fn token_after_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_after_keyword == keyword
    }

    /// Return whether token after seam starts one switch label.
    #[inline]
    pub(crate) fn token_after_is_case_or_default(self) -> bool {
        self.token_after_keyword == CommentSeamKeyword::Case
            || self.token_after_keyword == CommentSeamKeyword::Default
    }
}

/// Return whether one seam appears directly after `${`.
pub(crate) fn seam_is_template_interpolation_open_brace(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    if !seam.token_before_is(TokenType::OpenBrace) {
        return false;
    }

    let Some(token_before_index) = context.token_before else {
        return false;
    };
    if token_before_index == 0 {
        return false;
    }

    let previous_token_type = context.semantic_tokens[token_before_index - 1].token.ty;
    matches!(
        previous_token_type,
        TokenType::TemplateString
            | TokenType::TemplateStringStart
            | TokenType::TemplateStringMiddle
            | TokenType::TemplateStringEnd
    )
}

/// Mutable caches for one seam attachment evaluation.
#[derive(Default)]
pub(crate) struct CommentEnclosingOwnerCache {
    /// Lazily resolved smallest owner that encloses seam token range.
    pub(crate) enclosing_owner: Option<u32>,
    /// Whether seam owner lookup was executed.
    pub(crate) enclosing_owner_resolved: bool,
}

/// One resolved attachment for one comment seam.
pub(crate) type CommentAttachment = (Option<u32>, AnnotationPosition);

/// Owner candidates adjacent to one comment seam.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CommentAttachmentNeighbors {
    /// The nearest preceding owner candidate.
    pub(crate) preceding: Option<u32>,
    /// The nearest following owner candidate.
    pub(crate) following: Option<u32>,
}

impl CommentAttachmentNeighbors {
    /// Build one owner candidate pair.
    #[inline]
    pub(crate) fn new(preceding: Option<u32>, following: Option<u32>) -> Self {
        Self {
            preceding,
            following,
        }
    }
}

/// Resolve one seam owner lazily from seam token range.
pub(crate) fn comment_enclosing_owner(
    context: &CommentSeamContext<'_>,
    cache: &mut CommentEnclosingOwnerCache,
) -> Option<u32> {
    if cache.enclosing_owner_resolved {
        return cache.enclosing_owner;
    }

    cache.enclosing_owner_resolved = true;
    cache.enclosing_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(context.tree, before.span.start, after.span.end)
        });
    cache.enclosing_owner
}
