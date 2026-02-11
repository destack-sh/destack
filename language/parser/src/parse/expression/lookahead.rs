use crate::{ParseResult, Parser};

use destack_ast::TokenType;

/// Shape metadata for a parenthesized group lookahead.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ParenthesizedGroupShape {
    /// Whether the group has a top-level comma.
    pub has_top_level_comma: bool,
    /// Whether an arrow follows the closing parenthesis.
    pub has_arrow_follow: bool,
    /// Whether a colon follows the closing parenthesis.
    pub has_colon_follow: bool,
    /// Whether top-level `|` or `&` appears inside the group.
    pub has_top_level_type_union_or_intersection: bool,
    /// Whether a top-level parameter colon appears inside the group.
    pub has_top_level_parameter_colon: bool,
    /// Whether the group is empty aside from newlines.
    pub is_empty: bool,
}

impl Parser {
    /// Try to look ahead at a parenthesized group shape without committing parser state.
    pub(super) fn try_lookahead_parenthesized_group_shape(
        &mut self,
    ) -> ParseResult<ParenthesizedGroupShape> {
        // snapshot parser state before speculative lookahead
        let lookahead_mark = self.mark();
        let lookahead_result = self.lookahead_parenthesized_group_shape_inner();
        self.rewind(lookahead_mark);

        // lookahead disambiguation should never surface parse errors directly
        if let Ok(group_shape) = lookahead_result {
            Ok(group_shape)
        } else {
            Ok(ParenthesizedGroupShape::default())
        }
    }

    /// Compute group-shape metadata for the current `(` lookahead.
    fn lookahead_parenthesized_group_shape_inner(
        &mut self,
    ) -> ParseResult<ParenthesizedGroupShape> {
        // locate the closing parenthesis and follow token
        let open_pos = self.pos();
        let close_pos = self.find_matching_close(
            Some(open_pos),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        )?;
        let close_pos_for_follow = self.skip_newlines(close_pos)?;

        // compute top-level separators and operators inside the group
        let has_top_level_comma = self.language.is_destack()
            && self.has_top_level_token_in_parenthesized(open_pos, close_pos, TokenType::Comma)?;
        let has_top_level_or = self.has_top_level_token_in_parenthesized(
            open_pos,
            close_pos,
            TokenType::ElementwiseOr,
        )?;
        let has_top_level_and = self.has_top_level_token_in_parenthesized(
            open_pos,
            close_pos,
            TokenType::ElementwiseAnd,
        )?;
        let has_top_level_parameter_colon =
            self.has_top_level_token_in_parenthesized(open_pos, close_pos, TokenType::Colon)?;

        // compute follow token shape
        let follow_token_type = self
            .token_ref_at(close_pos_for_follow as usize + 1)
            .map(|token| token.token.ty);
        let has_arrow_follow = matches!(
            follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide)
        );
        let has_colon_follow = matches!(follow_token_type, Some(TokenType::Colon));

        // compute whether the group contains only newlines
        let is_empty = self.is_parenthesized_group_empty(open_pos, close_pos);

        Ok(ParenthesizedGroupShape {
            has_top_level_comma,
            has_arrow_follow,
            has_colon_follow,
            has_top_level_type_union_or_intersection: has_top_level_or || has_top_level_and,
            has_top_level_parameter_colon,
            is_empty,
        })
    }

    /// Check whether a token appears at top-level inside the given parenthesized range.
    fn has_top_level_token_in_parenthesized(
        &mut self,
        open_pos: u32,
        close_pos: u32,
        token_type: TokenType,
    ) -> ParseResult<bool> {
        self.with_options(self.options.in_type(), |parser| {
            parser.has_token_before_matching_close(open_pos, close_pos, token_type, true)
        })
    }

    /// Return true when the parenthesized range contains only newline trivia.
    fn is_parenthesized_group_empty(&mut self, open_pos: u32, close_pos: u32) -> bool {
        // scan all tokens between `(` and `)`
        let mut token_index = open_pos as usize + 1;
        while token_index < close_pos as usize {
            self.token_stream.ensure_token(token_index);
            let Some(token) = self.tokens().get(token_index) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                return false;
            }
            token_index += 1;
        }

        true
    }
}
