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
        match lookahead_result {
            Ok(group_shape) => Ok(group_shape),
            Err(_) => Ok(ParenthesizedGroupShape::default()),
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
        let mut group_shape = self.scan_parenthesized_group_shape(open_pos, close_pos);

        // compute follow token shape
        let follow_token_type = self
            .token_ref_at(close_pos_for_follow as usize + 1)
            .map(|token| token.token.ty);
        group_shape.has_arrow_follow = matches!(
            follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide)
        );
        group_shape.has_colon_follow = matches!(follow_token_type, Some(TokenType::Colon));

        Ok(group_shape)
    }

    /// Scan parenthesized contents once and collect top-level shape metadata.
    fn scan_parenthesized_group_shape(
        &mut self,
        open_pos: u32,
        close_pos: u32,
    ) -> ParenthesizedGroupShape {
        let mut shape = ParenthesizedGroupShape {
            has_top_level_comma: false,
            has_arrow_follow: false,
            has_colon_follow: false,
            has_top_level_type_union_or_intersection: false,
            has_top_level_parameter_colon: false,
            is_empty: true,
        };
        let mut paren_depth = 0u32;
        let mut brace_depth = 0u32;
        let mut bracket_depth = 0u32;
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

            // track nested delimiters
            match token_type {
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => paren_depth = paren_depth.saturating_sub(1),
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::LessThan => angle_depth += 1,
                TokenType::GreaterThan => angle_depth = angle_depth.saturating_sub(1),
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                _ => {}
            }

            let is_top_level =
                paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 && angle_depth == 0;
            if is_top_level {
                if self.language.is_destack() && token_type == TokenType::Comma {
                    shape.has_top_level_comma = true;
                } else if token_type == TokenType::Colon {
                    shape.has_top_level_parameter_colon = true;
                } else if matches!(
                    token_type,
                    TokenType::ElementwiseOr | TokenType::ElementwiseAnd
                ) {
                    shape.has_top_level_type_union_or_intersection = true;
                }
            }

            token_index += 1;
        }

        shape
    }
}
