use ast::{AnnotationPosition, Keyword, NodeParentIndex, NodeTree, TokenSpan, TokenType};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::FxHashMap;

use super::owner::find_smallest_owner_enclosing_range;
use super::token::{
    is_open_delimiter_token, previous_non_newline_token_index, token_after_prefers_left_ownership,
};

/// One normalized identifier keyword used in comment seam rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommentSeamKeyword {
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
pub(super) fn classify_comment_seam_keyword(
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

/// Immutable context for one comment seam attachment decision.
#[derive(Clone, Copy)]
pub(super) struct CommentSeamContext<'a> {
    /// The source file.
    pub(super) file: &'a File,
    /// The syntax tree.
    pub(super) tree: &'a NodeTree,
    /// The semantic token stream.
    pub(super) semantic_tokens: &'a [TokenSpan],
    /// Parsed identifier keywords by token span.
    pub(super) token_keyword_by_span: &'a FxHashMap<Span, Option<Keyword>>,
    /// The comment trivia payload.
    pub(super) trivia: destack_ast::CommentTrivia,
    /// Parent links for owner promotion.
    pub(super) parents: &'a NodeParentIndex,
    /// Token index before the seam.
    pub(super) token_before: Option<usize>,
    /// Token index after the seam.
    pub(super) token_after: Option<usize>,
    /// Token span before the seam.
    pub(super) token_before_span: Option<TokenSpan>,
    /// Token span after the seam.
    pub(super) token_after_span: Option<TokenSpan>,
}

/// Compact seam facts derived once per comment seam.
#[derive(Clone, Copy)]
pub(super) struct CommentSeamFacts {
    /// Whether trivia has at least one newline before comment text.
    pub(super) has_leading_newline: bool,
    /// Whether trivia has at least one newline after comment text.
    pub(super) has_trailing_newline: bool,
    /// Whether comment style is `//`.
    pub(super) comment_is_line: bool,
    /// Whether comment style is `/* */`.
    pub(super) comment_is_star: bool,
    /// Whether `/* */` comment text spans multiple lines.
    pub(super) comment_is_multiline_star: bool,
    /// Token kind before seam.
    pub(super) token_before_type: Option<TokenType>,
    /// Token kind after seam.
    pub(super) token_after_type: Option<TokenType>,
    /// Keyword class for identifier before seam.
    pub(super) token_before_keyword: CommentSeamKeyword,
    /// Keyword class for identifier after seam.
    pub(super) token_after_keyword: CommentSeamKeyword,
    /// Whether token after seam structurally prefers left ownership.
    pub(super) token_after_prefers_left: bool,
    /// Whether seam is one return type boundary after `):`.
    pub(super) token_before_is_return_type_colon: bool,
    /// Whether default trailing behavior should prefer right binding.
    pub(super) seam_binds_right: bool,
}

impl CommentSeamFacts {
    /// Build one seam fact snapshot.
    pub(super) fn build(context: &CommentSeamContext<'_>) -> Self {
        let token_before_type = context.token_before_span.map(|token| token.token.ty);
        let token_after_type = context.token_after_span.map(|token| token.token.ty);
        let token_before_keyword =
            classify_comment_seam_keyword(context.token_keyword_by_span, context.token_before_span);
        let token_after_keyword =
            classify_comment_seam_keyword(context.token_keyword_by_span, context.token_after_span);
        let has_leading_newline = context.trivia.boundary.newlines.has_leading_newline();
        let has_trailing_newline = context.trivia.boundary.newlines.has_trailing_newline();
        let comment_style = context.tree.get(context.trivia.comment).style;
        let comment_is_line = comment_style == ast::CommentStyle::Slash;
        let comment_is_star = comment_style == ast::CommentStyle::Star;
        let comment_is_multiline_star = if comment_is_star {
            let comment_end = context.trivia.span.end.saturating_sub(1);
            !context
                .file
                .is_same_line(context.trivia.span.start, comment_end)
        } else {
            false
        };
        let token_after_prefers_left =
            token_after_type.is_some_and(token_after_prefers_left_ownership);
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
            token_after_prefers_left,
            token_before_is_return_type_colon,
            seam_binds_right,
        }
    }

    /// Return whether token before seam has one type.
    #[inline]
    pub(super) fn token_before_is(self, token_type: TokenType) -> bool {
        self.token_before_type == Some(token_type)
    }

    /// Return whether token after seam has one type.
    #[inline]
    pub(super) fn token_after_is(self, token_type: TokenType) -> bool {
        self.token_after_type == Some(token_type)
    }

    /// Return whether token before seam is one keyword.
    #[inline]
    pub(super) fn token_before_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_before_keyword == keyword
    }

    /// Return whether token after seam is one keyword.
    #[inline]
    pub(super) fn token_after_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_after_keyword == keyword
    }

    /// Return whether token after seam starts one switch label.
    #[inline]
    pub(super) fn token_after_is_case_or_default(self) -> bool {
        self.token_after_keyword == CommentSeamKeyword::Case
            || self.token_after_keyword == CommentSeamKeyword::Default
    }
}

/// Mutable caches for one seam attachment evaluation.
#[derive(Default)]
pub(super) struct CommentSeamOwnerCache {
    /// Lazily resolved smallest owner that encloses seam token range.
    pub(super) seam_owner: Option<u32>,
    /// Whether seam owner lookup was executed.
    pub(super) seam_owner_resolved: bool,
}

/// One resolved attachment decision for one comment seam.
pub(super) type CommentAttachmentDecision = (Option<u32>, AnnotationPosition);

/// Owner candidates adjacent to one comment seam.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CommentAttachmentOwners {
    /// The nearest left owner candidate.
    pub(super) left: Option<u32>,
    /// The nearest right owner candidate.
    pub(super) right: Option<u32>,
}

impl CommentAttachmentOwners {
    /// Build one owner candidate pair.
    #[inline]
    pub(super) fn new(left: Option<u32>, right: Option<u32>) -> Self {
        Self { left, right }
    }
}

/// Resolve one seam owner lazily from seam token range.
pub(super) fn resolve_comment_seam_owner(
    context: &CommentSeamContext<'_>,
    cache: &mut CommentSeamOwnerCache,
) -> Option<u32> {
    if cache.seam_owner_resolved {
        return cache.seam_owner;
    }

    cache.seam_owner_resolved = true;
    cache.seam_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(context.tree, before.span.start, after.span.end)
        });
    cache.seam_owner
}
