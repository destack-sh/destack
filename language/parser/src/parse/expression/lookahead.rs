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
        match lookahead_result {
            Ok(group_shape) => Ok(group_shape),
            Err(_) => Ok(ParenthesizedGroupShape::default()),
        }
    }

    /// Compute group-shape metadata for the current `(` lookahead.
    fn lookahead_parenthesized_group_shape_inner(
        &mut self,
    ) -> ParseResult<ParenthesizedGroupShape> {
        let tracks_tuple_commas = self.language.is_destack();

        // locate the closing parenthesis and follow token
        let open_pos = self.pos();
        let close_pos = self.find_matching_close(
            Some(open_pos),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        )?;
        let follow_index = self.next_non_newline_index_from(close_pos as usize + 1);
        let follow_token_type = self.token_ref_at(follow_index).map(|token| token.token.ty);
        let has_arrow_follow = matches!(
            follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide)
        );
        let has_colon_follow = matches!(follow_token_type, Some(TokenType::Colon));

        // most js and ts parenthesized expressions are not lambda heads
        // skip deep shape scanning when no follow token can start a lambda form
        if !tracks_tuple_commas && !has_arrow_follow && !has_colon_follow {
            return Ok(ParenthesizedGroupShape {
                has_top_level_comma: false,
                has_arrow_follow,
                has_colon_follow,
                has_top_level_parameter_colon: false,
                is_empty: false,
            });
        }

        // lambda heads with `=>` in value contexts do not need interior shape scanning
        if has_arrow_follow && !self.options.in_arrow_return_type {
            return Ok(ParenthesizedGroupShape {
                has_top_level_comma: false,
                has_arrow_follow,
                has_colon_follow,
                has_top_level_parameter_colon: false,
                is_empty: false,
            });
        }

        // compute top-level separators and operators inside the group
        let mut group_shape =
            self.scan_parenthesized_group_shape(open_pos, close_pos, tracks_tuple_commas);

        // compute follow token shape
        group_shape.has_arrow_follow = has_arrow_follow;
        group_shape.has_colon_follow = has_colon_follow;

        Ok(group_shape)
    }

    /// Scan parenthesized contents once and collect top-level shape metadata.
    fn scan_parenthesized_group_shape(
        &mut self,
        open_pos: u32,
        close_pos: u32,
        tracks_tuple_commas: bool,
    ) -> ParenthesizedGroupShape {
        let mut shape = ParenthesizedGroupShape {
            has_top_level_comma: false,
            has_arrow_follow: false,
            has_colon_follow: false,
            has_top_level_parameter_colon: false,
            is_empty: true,
        };
        let mut angle_depth = 0u32;
        let mut token_index = open_pos as usize + 1;
        let close_index = close_pos as usize;

        while token_index < close_index {
            self.token_stream.ensure_token(token_index);
            let Some(token) = self.tokens().get(token_index) else {
                break;
            };
            let token_type = token.token.ty;

            // detect non-empty content
            if token_type != TokenType::Newline {
                shape.is_empty = false;
            }

            let is_top_level = angle_depth == 0;

            // skip nested delimiters using cached pair indexes
            if is_top_level
                && matches!(
                    token_type,
                    TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
                )
                && let Some(close_index_for_token) = self.token_stream.matching_pair(token_index)
                && close_index_for_token > token_index
                && close_index_for_token < close_index
            {
                token_index = close_index_for_token + 1;
                continue;
            }

            if is_top_level {
                if tracks_tuple_commas && token_type == TokenType::Comma {
                    shape.has_top_level_comma = true;
                } else if token_type == TokenType::Colon {
                    shape.has_top_level_parameter_colon = true;
                }
            }

            // track top level angle depth for type parameter forms
            match token_type {
                TokenType::LessThan => angle_depth += 1,
                TokenType::GreaterThan => angle_depth = angle_depth.saturating_sub(1),
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                _ => {}
            }

            // in JS/TS typed lambda heads, once we see a top level parameter colon
            // and know the group is non empty, the remaining scan cannot change lambda gating
            if !tracks_tuple_commas && shape.has_top_level_parameter_colon && !shape.is_empty {
                break;
            }

            token_index += 1;
        }

        shape
    }
}
