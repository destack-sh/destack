use crate::{ParseResult, Parser, is_semantic};

use destack_ast::TokenType;

/// Parenthesized group analysis metadata.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParenthesizedGroupShape {
    /// The matching close parenthesis index when known.
    pub close_index: Option<usize>,
    /// Whether the group has a top level comma.
    pub has_top_level_comma: bool,
    /// The significant token after the closing parenthesis when relevant.
    pub follow_token_type: Option<TokenType>,
    /// Whether a top level parameter colon appears inside the group.
    pub has_top_level_parameter_colon: bool,
    /// Whether a top level arrow appears inside the group.
    pub has_top_level_arrow: bool,
    /// Whether the first top level token opens a nested parenthesized expression.
    pub starts_with_nested_parenthesis: bool,
    /// Whether the group is empty aside from newlines.
    pub is_empty: bool,
}

impl Parser {
    /// Return the shape for the current open parenthesis.
    pub(super) fn parenthesized_group_shape(&mut self) -> ParseResult<ParenthesizedGroupShape> {
        // inspect the current opening parenthesis
        let open_index = self.pos_index();

        // tree literal starts like `(<div>...)` do not need delimiter-shape lookahead
        let has_parenthesized_tree_literal = self.language.supports_jsx()
            && !self.options.is_in_type()
            && !self.options.is_in_arrow_return_type()
            && self.parenthesized_group_starts_with_tree_literal(open_index);
        if has_parenthesized_tree_literal {
            return Ok(ParenthesizedGroupShape::default());
        }

        // base grouped expressions branch on the token after `)`
        let can_use_follow_token = !self.language.is_destack()
            && !self.options.is_in_type()
            && !self.options.is_in_arrow_return_type();
        if can_use_follow_token {
            self.stats.record_parenthesized_follow_token_call();

            if let Some(close_index) = self.matching_pair_or_lex(open_index) {
                let follow_token_type = self.token_type_at(close_index + 1);
                if follow_token_type != TokenType::End {
                    self.stats.record_parenthesized_follow_token_hit();

                    if matches!(follow_token_type, TokenType::Arrow | TokenType::ArrowWide) {
                        return Ok(ParenthesizedGroupShape {
                            close_index: Some(close_index),
                            follow_token_type: Some(follow_token_type),
                            ..ParenthesizedGroupShape::default()
                        });
                    }

                    if follow_token_type != TokenType::Colon {
                        return Ok(ParenthesizedGroupShape::default());
                    }
                }
            }
        }

        // try the cheap follow-token fast path first
        self.stats.record_delimiter_analysis_lookup();

        // tree literal lexing can mutate lexer state during lookahead
        let needs_snapshot = self.allow_tree_literals() && !self.options.is_in_type();
        if needs_snapshot {
            self.stats.record_delimiter_analysis_snapshot_lookup();
        }

        // compute the full grouped shape with snapshotting when needed
        let group_shape = if needs_snapshot {
            let rewind_mark = self.mark_rewind();
            let group_shape = self.compute_parenthesized_group_shape().unwrap_or_default();
            self.rewind(rewind_mark);
            group_shape
        } else {
            self.compute_parenthesized_group_shape().unwrap_or_default()
        };

        Ok(group_shape)
    }

    /// Compute parenthesized group shape metadata for the current opening token.
    fn compute_parenthesized_group_shape(&mut self) -> ParseResult<ParenthesizedGroupShape> {
        // record the full grouped scan path
        self.stats.record_delimiter_analysis_scan();

        // require one opening parenthesis at the current cursor
        let open_index = self.pos_index();
        if self.token_type_at(open_index) != TokenType::OpenParenthesis {
            return Ok(ParenthesizedGroupShape::default());
        }

        // find the matching close parenthesis first
        let Some(close_index) = self.find_matching_close_for_parenthesized_group(open_index as u32)
        else {
            return Ok(ParenthesizedGroupShape::default());
        };

        // inspect the follow token and surrounding context
        let needs_group_contents_shape = self.language.is_destack();
        let follow_cursor = self.scanner_cursor_from(close_index + 1);
        let follow_token_type = match follow_cursor.token_type {
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon => {
                Some(follow_cursor.token_type)
            }
            _ => None,
        };
        let has_colon_follow = follow_token_type == Some(TokenType::Colon);
        let needs_parameter_shape_for_arrow_return = self.options.is_in_arrow_return_type();
        let needs_parameter_shape_for_typed_colon = self.options.is_in_type() && has_colon_follow;
        let needs_parameter_shape_for_ternary_colon =
            self.options.is_in_ternary_condition() && has_colon_follow;

        // plain group parsing only needs the token after `)` unless an
        // enclosing context also needs the inner parameter shape
        if !needs_group_contents_shape
            && !needs_parameter_shape_for_arrow_return
            && !needs_parameter_shape_for_typed_colon
            && !needs_parameter_shape_for_ternary_colon
        {
            return Ok(ParenthesizedGroupShape {
                close_index: Some(close_index),
                follow_token_type,
                is_empty: false,
                ..ParenthesizedGroupShape::default()
            });
        }

        // compute top level separators and operators inside the group
        let mut group_shape =
            self.scan_parenthesized_group_shape(open_index as u32, close_index as u32);
        group_shape.close_index = Some(close_index);
        group_shape.follow_token_type = follow_token_type;

        Ok(group_shape)
    }

    /// Find the matching close token for the current parenthesized group.
    fn find_matching_close_for_parenthesized_group(&mut self, open_pos: u32) -> Option<usize> {
        // normalize the open position
        let open_index = open_pos as usize;

        // tree literals can contain raw `)` text, so groups that start as tree literals use expression matching
        let tree_literals_allowed = self.expression_tree_literals_allowed();
        let needs_tree_aware_parenthesis_matching = tree_literals_allowed
            && (self.options.is_in_tree_literal()
                || self.parenthesized_group_starts_with_tree_literal(open_index));
        if needs_tree_aware_parenthesis_matching {
            let close_pos = self.find_matching_close_in_expression_maybe(
                open_pos,
                TokenType::OpenParenthesis,
                TokenType::CloseParenthesis,
            )?;
            return Some(close_pos as usize);
        }

        // prefer cached delimiter pairs when they exist
        if self
            .token_ref_at(open_index)
            .is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
            && let Some(close_index) = self.matching_pair_or_lex(open_index)
        {
            return Some(close_index);
        }

        // otherwise scan forward for the matching close
        let close_pos = self.find_matching_close_maybe(
            Some(open_pos),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        )?;
        Some(close_pos as usize)
    }

    /// Return true when the immediate parenthesized payload starts with a tree literal.
    fn parenthesized_group_starts_with_tree_literal(&mut self, open_index: usize) -> bool {
        // require tree literal support first
        if !self.language.supports_jsx() {
            return false;
        }

        // skip non semantic newlines before testing tree literal starts
        let next_index = self.next_non_newline_index_from(open_index + 1);
        if self.token_type_at(next_index) != TokenType::LessThan {
            return false;
        }

        self.with_pos(next_index, |parser| parser.can_start_tree_literal())
    }

    /// Scan parenthesized contents once and collect top-level shape metadata.
    fn scan_parenthesized_group_shape(
        &mut self,
        open_pos: u32,
        close_pos: u32,
    ) -> ParenthesizedGroupShape {
        // initialize the scan state
        let mut analysis = ParenthesizedGroupShape {
            is_empty: true,
            ..ParenthesizedGroupShape::default()
        };
        let needs_group_contents_shape = self.language.is_destack();
        let mut angle_depth = 0u32;
        let mut parenthesis_depth = 0u32;
        let mut brace_depth = 0u32;
        let mut bracket_depth = 0u32;
        let mut token_index = open_pos as usize + 1;
        let close_index = close_pos as usize;

        // scan the grouped contents once
        while token_index < close_index {
            self.ensure_token(token_index);
            let Some(token) = self.tokens().get(token_index) else {
                break;
            };
            let token_type = token.token.ty;

            let is_in_nested_delimiter =
                parenthesis_depth > 0 || brace_depth > 0 || bracket_depth > 0;
            let is_top_level = !is_in_nested_delimiter && angle_depth == 0;

            // remember wrapped expression heads like `(() => x)`
            if analysis.is_empty && is_top_level && token_type == TokenType::OpenParenthesis {
                analysis.starts_with_nested_parenthesis = true;
            }

            // detect non-empty semantic content
            if is_semantic(token_type) && token_type != TokenType::Newline {
                analysis.is_empty = false;
            }

            if is_top_level {
                if needs_group_contents_shape && token_type == TokenType::Comma {
                    analysis.has_top_level_comma = true;
                } else if token_type == TokenType::Colon {
                    analysis.has_top_level_parameter_colon = true;
                } else if matches!(token_type, TokenType::Arrow | TokenType::ArrowWide) {
                    analysis.has_top_level_arrow = true;
                }
            }

            // track top level angle depth for type parameter forms
            if !is_in_nested_delimiter {
                match token_type {
                    TokenType::LessThan => angle_depth += 1,
                    TokenType::GreaterThan => angle_depth = angle_depth.saturating_sub(1),
                    TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                    TokenType::ShiftRight => angle_depth = angle_depth.saturating_sub(2),
                    TokenType::UnsignedShiftRight => {
                        angle_depth = angle_depth.saturating_sub(3);
                    }
                    _ => {}
                }
            }

            // track nested non angle delimiters inline instead of consulting cached pairs
            match token_type {
                TokenType::OpenParenthesis => parenthesis_depth += 1,
                TokenType::CloseParenthesis => {
                    parenthesis_depth = parenthesis_depth.saturating_sub(1);
                }
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => {
                    brace_depth = brace_depth.saturating_sub(1);
                }
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                _ => {}
            }

            // once typed lambda heads see one top level arrow
            // the remaining scan cannot change lambda gating
            if !needs_group_contents_shape && analysis.has_top_level_arrow && !analysis.is_empty {
                break;
            }

            token_index += 1;
        }

        analysis
    }
}
