use crate::{ParseResult, Parser, is_semantic};

use destack_ast::TokenType;

/// Parenthesized group analysis metadata.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DelimiterAnalysis {
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
    pub(super) fn parenthesized_group_shape(&mut self) -> ParseResult<DelimiterAnalysis> {
        let ambient_context = self.options;
        let expression_context = self.options;
        let open_index = self.pos_index();

        // tree literal starts like `(<div>...)` do not need delimiter-shape lookahead
        let has_parenthesized_tree_literal = self.language.supports_jsx()
            && !ambient_context.is_in_type()
            && !expression_context.is_in_arrow_return_type()
            && self.parenthesized_group_starts_with_tree_literal(open_index);

        // plain path: in JS/TS value contexts, branch on the token after ')'
        let can_use_plain_group_follow = !self.language.is_destack()
            && !ambient_context.is_in_type()
            && !expression_context.is_in_arrow_return_type()
            && !has_parenthesized_tree_literal;
        if can_use_plain_group_follow {
            self.stats.record_parenthesized_follow_token_call();

            if let Some(close_index) = self.matching_pair_or_lex(open_index) {
                let follow_token_type = self.token_type_at(close_index + 1);
                if follow_token_type != TokenType::End {
                    self.stats.record_parenthesized_follow_token_hit();

                    if matches!(follow_token_type, TokenType::Arrow | TokenType::ArrowWide) {
                        return Ok(DelimiterAnalysis {
                            close_index: Some(close_index),
                            follow_token_type: Some(follow_token_type),
                            ..DelimiterAnalysis::default()
                        });
                    }

                    if follow_token_type != TokenType::Colon {
                        return Ok(DelimiterAnalysis::default());
                    }
                }
            }
        }

        if has_parenthesized_tree_literal {
            Ok(DelimiterAnalysis::default())
        } else {
            self.lookahead_parenthesized_group_shape()
        }
    }

    /// Look ahead at a parenthesized group shape without committing parser state.
    fn lookahead_parenthesized_group_shape(&mut self) -> ParseResult<DelimiterAnalysis> {
        let ambient = self.options;

        self.stats.record_delimiter_analysis_lookup();

        // tree literal lexing can mutate lexer state during lookahead
        let needs_snapshot = self.allow_tree_literals() && !ambient.is_in_type();
        if needs_snapshot {
            self.stats.record_delimiter_analysis_snapshot_lookup();
        }
        let lookahead_result = if needs_snapshot {
            let lookahead_mark = self.mark_rewind();
            let lookahead_result = self.lookahead_delimiter_analysis_inner();
            self.rewind(lookahead_mark);
            lookahead_result
        } else {
            self.lookahead_delimiter_analysis_inner()
        };

        // lookahead disambiguation should never surface parse errors directly
        let delimiter_analysis = match lookahead_result {
            Ok(delimiter_analysis) => Ok(delimiter_analysis),
            Err(_) => Ok(DelimiterAnalysis::default()),
        }?;

        Ok(delimiter_analysis)
    }

    /// Compute delimiter analysis metadata for the current opening token.
    fn lookahead_delimiter_analysis_inner(&mut self) -> ParseResult<DelimiterAnalysis> {
        let ambient = self.options;
        let expression = self.options;

        self.stats.record_delimiter_analysis_scan();

        let open_index = self.pos_index();
        if self.token_type_at(open_index) != TokenType::OpenParenthesis {
            return Ok(DelimiterAnalysis::default());
        }

        let Some(close_index) = self.find_matching_close_for_parenthesized_group(open_index as u32)
        else {
            return Ok(DelimiterAnalysis::default());
        };

        let tracks_tuple_commas = self.language.is_destack();
        let follow_cursor = self.scanner_cursor_from(close_index + 1);
        let follow_token_type = match follow_cursor.token_type {
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon => {
                Some(follow_cursor.token_type)
            }
            _ => None,
        };
        let has_colon_follow = follow_token_type == Some(TokenType::Colon);
        let needs_parameter_shape_for_arrow_return = expression.is_in_arrow_return_type();
        let needs_parameter_shape_for_typed_colon = ambient.is_in_type() && has_colon_follow;
        let needs_parameter_shape_for_ternary_colon =
            expression.is_in_ternary_condition() && has_colon_follow;

        // js and ts can usually decide lambda eligibility from the token after ')'
        // skip deep shape scanning unless parameter shape data is required
        if !tracks_tuple_commas
            && !needs_parameter_shape_for_arrow_return
            && !needs_parameter_shape_for_typed_colon
            && !needs_parameter_shape_for_ternary_colon
        {
            return Ok(DelimiterAnalysis {
                close_index: Some(close_index),
                follow_token_type,
                is_empty: false,
                ..DelimiterAnalysis::default()
            });
        }

        // compute top level separators and operators inside the group
        let mut delimiter_analysis =
            self.scan_parenthesized_delimiter_analysis(open_index as u32, close_index as u32);
        delimiter_analysis.close_index = Some(close_index);
        delimiter_analysis.follow_token_type = follow_token_type;
        Ok(delimiter_analysis)
    }

    /// Find the matching close token for the current parenthesized group.
    fn find_matching_close_for_parenthesized_group(&mut self, open_pos: u32) -> Option<usize> {
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

        if self
            .token_ref_at(open_index)
            .is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
            && let Some(close_index) = self.matching_pair_or_lex(open_index)
        {
            return Some(close_index);
        }

        let close_pos = self.find_matching_close_maybe(
            Some(open_pos),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        )?;
        Some(close_pos as usize)
    }

    /// Return true when the immediate parenthesized payload starts with a tree literal.
    fn parenthesized_group_starts_with_tree_literal(&mut self, open_index: usize) -> bool {
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
    fn scan_parenthesized_delimiter_analysis(
        &mut self,
        open_pos: u32,
        close_pos: u32,
    ) -> DelimiterAnalysis {
        let mut analysis = DelimiterAnalysis {
            is_empty: true,
            ..DelimiterAnalysis::default()
        };
        let tracks_tuple_commas = self.language.is_destack();
        let mut angle_depth = 0u32;
        let mut parenthesis_depth = 0u32;
        let mut brace_depth = 0u32;
        let mut bracket_depth = 0u32;
        let mut token_index = open_pos as usize + 1;
        let close_index = close_pos as usize;

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
                if tracks_tuple_commas && token_type == TokenType::Comma {
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

            // in JS and TS typed lambda heads, once we see one top level arrow
            // the remaining scan cannot change lambda gating
            if !tracks_tuple_commas && analysis.has_top_level_arrow && !analysis.is_empty {
                break;
            }

            token_index += 1;
        }

        analysis
    }
}
