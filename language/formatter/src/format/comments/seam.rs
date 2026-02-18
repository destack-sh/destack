use ast::{NodeParentIndex, NodeTree, TokenSpan, TokenType};
use destack_ast as ast;
use destack_source::File;

use super::owner::find_smallest_owner_enclosing_range;
use super::token::{
    is_open_delimiter_token, previous_non_newline_token_index, token_after_prefers_left_ownership,
    token_is_control_head_close_paren,
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
fn classify_comment_seam_keyword(file: &File, token: Option<TokenSpan>) -> CommentSeamKeyword {
    let Some(token) = token else {
        return CommentSeamKeyword::None;
    };

    if token.token.ty != TokenType::Identifier {
        return CommentSeamKeyword::None;
    }

    match file.span_str(token.span) {
        "as" => CommentSeamKeyword::As,
        "satisfies" => CommentSeamKeyword::Satisfies,
        "export" => CommentSeamKeyword::Export,
        "implements" => CommentSeamKeyword::Implements,
        "else" => CommentSeamKeyword::Else,
        "case" => CommentSeamKeyword::Case,
        "default" => CommentSeamKeyword::Default,
        "const" => CommentSeamKeyword::Const,
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
    /// Whether token before seam closes one control-flow head.
    pub(super) token_before_is_control_head_close_paren: bool,
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
            classify_comment_seam_keyword(context.file, context.token_before_span);
        let token_after_keyword =
            classify_comment_seam_keyword(context.file, context.token_after_span);
        let has_leading_newline = context.trivia.boundary.newlines.has_leading_newline();
        let has_trailing_newline = context.trivia.boundary.newlines.has_trailing_newline();
        let comment_style = context.tree.get(context.trivia.comment).style;
        let comment_is_line = comment_style == ast::CommentStyle::Slash;
        let comment_is_star = comment_style == ast::CommentStyle::Star;
        let comment_is_multiline_star =
            comment_is_star && context.file.span_str(context.trivia.span).contains('\n');
        let token_after_prefers_left =
            token_after_type.is_some_and(token_after_prefers_left_ownership);
        let token_before_is_control_head_close_paren = context.token_before.is_some_and(|index| {
            token_is_control_head_close_paren(context.file, context.semantic_tokens, index)
        });
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
            token_before_is_control_head_close_paren,
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

/// Mutable caches for one seam rule evaluation.
#[derive(Default)]
pub(super) struct CommentSeamRuleState {
    /// Lazily resolved smallest owner that encloses seam token range.
    pub(super) seam_owner: Option<u32>,
    /// Whether seam owner lookup was executed.
    pub(super) seam_owner_resolved: bool,
}

/// Resolve one seam owner lazily from seam token range.
pub(super) fn resolve_comment_seam_owner(
    context: &CommentSeamContext<'_>,
    state: &mut CommentSeamRuleState,
) -> Option<u32> {
    if state.seam_owner_resolved {
        return state.seam_owner;
    }

    state.seam_owner_resolved = true;
    state.seam_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(context.tree, before.span.start, after.span.end)
        });
    state.seam_owner
}
