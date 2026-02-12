use crate::{ParseResult, Parser};

use destack_ast::TokenType;

/// Shape metadata for a parenthesized group lookahead.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParenthesizedGroupShape {
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
        // skip cache when split-token state can change current-token semantics
        if !self.has_active_split() {
            let lookahead_index = self.pos_index();
            self.token_stream.ensure_token(lookahead_index);

            if self
                .parenthesized_group_shapes_cached
                .get(lookahead_index)
                .copied()
                .unwrap_or(false)
            {
                return Ok(self.parenthesized_group_shapes[lookahead_index]);
            }
        }

        // tree literal lexing can mutate token stream state during lookahead
        let needs_snapshot = self.token_stream.allow_tree_literals() && !self.options.in_type;
        let lookahead_result = if needs_snapshot {
            let lookahead_mark = self.mark();
            let lookahead_result = self.lookahead_parenthesized_group_shape_inner();
            self.rewind(lookahead_mark);
            lookahead_result
        } else {
            self.lookahead_parenthesized_group_shape_inner()
        };

        // lookahead disambiguation should never surface parse errors directly
        let group_shape = match lookahead_result {
            Ok(group_shape) => Ok(group_shape),
            Err(_) => Ok(ParenthesizedGroupShape::default()),
        }?;

        // store cache for the current token index when split tokens are inactive
        if !self.has_active_split() {
            let lookahead_index = self.pos_index();
            if self.parenthesized_group_shapes.len() <= lookahead_index {
                self.parenthesized_group_shapes
                    .resize(lookahead_index + 1, ParenthesizedGroupShape::default());
                self.parenthesized_group_shapes_cached
                    .resize(lookahead_index + 1, false);
            }
            self.parenthesized_group_shapes[lookahead_index] = group_shape;
            self.parenthesized_group_shapes_cached[lookahead_index] = true;
        }

        Ok(group_shape)
    }

    /// Compute group-shape metadata for the current `(` lookahead.
    fn lookahead_parenthesized_group_shape_inner(
        &mut self,
    ) -> ParseResult<ParenthesizedGroupShape> {
        let tracks_tuple_commas = self.language.is_destack();

        // locate the closing parenthesis and follow token
        let open_pos = self.pos();
        let close_pos = if !self.has_active_split()
            && self
                .token_ref_at(open_pos as usize)
                .is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
        {
            if let Some(close_index) = self.token_stream.matching_pair(open_pos as usize) {
                close_index as u32
            } else {
                self.find_matching_close(
                    Some(open_pos),
                    TokenType::OpenParenthesis,
                    TokenType::CloseParenthesis,
                )?
            }
        } else {
            self.find_matching_close(
                Some(open_pos),
                TokenType::OpenParenthesis,
                TokenType::CloseParenthesis,
            )?
        };
        let follow_start_index = close_pos as usize + 1;
        let follow_index = if self.token_type_at(follow_start_index) == TokenType::Newline {
            self.next_non_newline_index_from(follow_start_index + 1)
        } else {
            follow_start_index
        };
        let follow_token_type = self.token_ref_at(follow_index).map(|token| token.token.ty);
        let has_arrow_follow = matches!(
            follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide)
        );
        let has_colon_follow = matches!(follow_token_type, Some(TokenType::Colon));
        let needs_parameter_shape_for_arrow_return = self.options.in_arrow_return_type;
        let needs_parameter_shape_for_typed_colon = self.options.in_type && has_colon_follow;

        // js and ts can usually decide lambda eligibility from the token after ')'
        // skip deep shape scanning unless parameter shape data is required
        if !tracks_tuple_commas
            && !needs_parameter_shape_for_arrow_return
            && !needs_parameter_shape_for_typed_colon
        {
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
