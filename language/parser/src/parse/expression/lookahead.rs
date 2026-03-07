use crate::{ParseResult, Parser};

use destack_ast::TokenType;

/// Cached delimiter analysis metadata keyed by opening token index.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DelimiterAnalysis {
    /// The matching close delimiter index when known.
    pub close_index: Option<usize>,
    /// Whether the group has a top level comma.
    pub has_top_level_comma: bool,
    /// Whether an arrow follows the closing parenthesis.
    pub has_arrow_follow: bool,
    /// Whether a colon follows the closing parenthesis.
    pub has_colon_follow: bool,
    /// Whether a top level parameter colon appears inside the group.
    pub has_top_level_parameter_colon: bool,
    /// Whether the group is empty aside from newlines.
    pub is_empty: bool,
}

/// Parenthesized group shape alias used by expression dispatch.
pub(crate) type ParenthesizedGroupShape = DelimiterAnalysis;

impl Parser {
    /// Return the shape for the current open parenthesis.
    pub(super) fn parenthesized_group_shape(&mut self) -> ParseResult<ParenthesizedGroupShape> {
        let ambient_context = self.current_ambient_context();
        let expression_context = self.current_expression_context();
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
            && !self.has_active_split()
            && !has_parenthesized_tree_literal;
        if can_use_plain_group_follow {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_follow_token_calls += 1;
            }

            if let Some(close_index) = self.matching_pair_or_lex(open_index) {
                let follow_token_type = self.token_type_at(close_index + 1);
                if follow_token_type != TokenType::End {
                    if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                        speculation_stats.parenthesized_follow_token_hits += 1;
                    }

                    if matches!(follow_token_type, TokenType::Arrow | TokenType::ArrowWide) {
                        return Ok(ParenthesizedGroupShape {
                            close_index: Some(close_index),
                            has_arrow_follow: true,
                            ..ParenthesizedGroupShape::default()
                        });
                    }

                    if follow_token_type != TokenType::Colon {
                        return Ok(ParenthesizedGroupShape::default());
                    }
                }
            }
        }

        if has_parenthesized_tree_literal {
            Ok(ParenthesizedGroupShape::default())
        } else {
            self.try_lookahead_parenthesized_group_shape()
        }
    }

    /// Try to look ahead at a parenthesized group shape without committing parser state.
    pub(super) fn try_lookahead_parenthesized_group_shape(
        &mut self,
    ) -> ParseResult<ParenthesizedGroupShape> {
        let ambient = self.current_ambient_context();

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_lookups += 1;
        }

        // skip cache when split token state can change current token semantics
        if !self.has_active_split() {
            let lookahead_index = self.pos_index();
            self.ensure_token(lookahead_index);

            if self
                .delimiter_analyses_cached
                .get(lookahead_index)
                .copied()
                .unwrap_or(false)
            {
                if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                    speculation_stats.delimiter_analysis_cache_hits += 1;
                }
                return Ok(self.delimiter_analyses[lookahead_index]);
            }
        }

        // tree literal lexing can mutate token stream state during lookahead
        let needs_snapshot = self.allow_tree_literals() && !ambient.is_in_type();
        if needs_snapshot && let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_snapshot_lookups += 1;
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

        // store cache for the current token index when split tokens are inactive
        if !self.has_active_split() {
            let lookahead_index = self.pos_index();
            self.ensure_delimiter_analysis_cache_capacity(lookahead_index);
            self.delimiter_analyses[lookahead_index] = delimiter_analysis;
            self.delimiter_analyses_cached[lookahead_index] = true;
        }

        Ok(delimiter_analysis)
    }

    /// Compute delimiter analysis metadata for the current opening token.
    fn lookahead_delimiter_analysis_inner(&mut self) -> ParseResult<DelimiterAnalysis> {
        let ambient = self.current_ambient_context();
        let expression = self.current_expression_context();

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.delimiter_analysis_scans += 1;
        }

        let open_index = self.pos_index();
        self.ensure_token(open_index);
        let Some(open_token) = self.tokens().get(open_index).copied() else {
            return Ok(DelimiterAnalysis::default());
        };
        let open_token_type = open_token.token.ty;

        // only opening delimiters participate in delimiter analysis
        if !matches!(
            open_token_type,
            TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
        ) {
            return Ok(DelimiterAnalysis::default());
        }

        let close_index =
            self.find_matching_close_for_open_delimiter(open_index as u32, open_token_type)?;
        let Some(close_index) = close_index else {
            return Ok(DelimiterAnalysis::default());
        };

        // braces and brackets only need close pair metadata for now
        if open_token_type != TokenType::OpenParenthesis {
            return Ok(DelimiterAnalysis {
                close_index: Some(close_index),
                ..DelimiterAnalysis::default()
            });
        }

        let tracks_tuple_commas = self.language.is_destack();
        let follow_cursor = self.scanner_cursor_from(close_index + 1);
        let follow_token_type = if follow_cursor.token_type == TokenType::End {
            None
        } else {
            Some(follow_cursor.token_type)
        };
        let has_arrow_follow = matches!(
            follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide)
        );
        let has_colon_follow = matches!(follow_token_type, Some(TokenType::Colon));
        let needs_parameter_shape_for_arrow_return = expression.is_in_arrow_return_type();
        let needs_parameter_shape_for_typed_colon = ambient.is_in_type() && has_colon_follow;

        // js and ts can usually decide lambda eligibility from the token after ')'
        // skip deep shape scanning unless parameter shape data is required
        if !tracks_tuple_commas
            && !needs_parameter_shape_for_arrow_return
            && !needs_parameter_shape_for_typed_colon
        {
            return Ok(DelimiterAnalysis {
                close_index: Some(close_index),
                has_arrow_follow,
                has_colon_follow,
                is_empty: false,
                ..DelimiterAnalysis::default()
            });
        }

        // compute top level separators and operators inside the group
        let mut delimiter_analysis =
            self.scan_parenthesized_delimiter_analysis(open_index as u32, close_index as u32);
        delimiter_analysis.close_index = Some(close_index);
        delimiter_analysis.has_arrow_follow = has_arrow_follow;
        delimiter_analysis.has_colon_follow = has_colon_follow;
        Ok(delimiter_analysis)
    }

    /// Find the matching close token for an opening delimiter token.
    fn find_matching_close_for_open_delimiter(
        &mut self,
        open_pos: u32,
        open_token_type: TokenType,
    ) -> ParseResult<Option<usize>> {
        let open_index = open_pos as usize;

        // tree literals can contain raw `)` text, so groups that start as tree literals use expression matching
        let tree_literals_allowed = self.expression_tree_literals_allowed();
        let needs_tree_aware_parenthesis_matching = open_token_type == TokenType::OpenParenthesis
            && tree_literals_allowed
            && (self.current_ambient_context().is_in_tree_literal()
                || self.parenthesized_group_starts_with_tree_literal(open_index));
        if needs_tree_aware_parenthesis_matching {
            let close_pos = self.find_matching_close_in_expression(
                open_pos,
                TokenType::OpenParenthesis,
                TokenType::CloseParenthesis,
            )?;
            return Ok(Some(close_pos as usize));
        }

        if !self.has_active_split()
            && self
                .token_ref_at(open_index)
                .is_some_and(|token| token.token.ty == open_token_type)
            && let Some(close_index) = self.matching_pair_or_lex(open_index)
        {
            return Ok(Some(close_index));
        }

        let close_pos = match open_token_type {
            TokenType::OpenParenthesis => self.find_matching_close(
                Some(open_pos),
                TokenType::OpenParenthesis,
                TokenType::CloseParenthesis,
            )?,
            TokenType::OpenBrace => self.find_matching_close(
                Some(open_pos),
                TokenType::OpenBrace,
                TokenType::CloseBrace,
            )?,
            TokenType::OpenBracket => self.find_matching_close(
                Some(open_pos),
                TokenType::OpenBracket,
                TokenType::CloseBracket,
            )?,
            _ => return Ok(None),
        };
        Ok(Some(close_pos as usize))
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
        let mut token_index = open_pos as usize + 1;
        let close_index = close_pos as usize;

        while token_index < close_index {
            self.ensure_token(token_index);
            let Some(token) = self.tokens().get(token_index) else {
                break;
            };
            let token_type = token.token.ty;

            // detect non-empty content
            if token_type != TokenType::Newline {
                analysis.is_empty = false;
            }

            let is_top_level = angle_depth == 0;

            // skip nested delimiters using cached pair indexes
            if is_top_level
                && matches!(
                    token_type,
                    TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
                )
                && let Some(close_index_for_token) = self.matching_pair_or_lex(token_index)
                && close_index_for_token > token_index
                && close_index_for_token < close_index
            {
                token_index = close_index_for_token + 1;
                continue;
            }

            if is_top_level {
                if tracks_tuple_commas && token_type == TokenType::Comma {
                    analysis.has_top_level_comma = true;
                } else if token_type == TokenType::Colon {
                    analysis.has_top_level_parameter_colon = true;
                }
            }

            // track top level angle depth for type parameter forms
            match token_type {
                TokenType::LessThan => angle_depth += 1,
                TokenType::GreaterThan => angle_depth = angle_depth.saturating_sub(1),
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                _ => {}
            }

            // in JS and TS typed lambda heads, once we see a top level parameter colon
            // and know the group is non empty, the remaining scan cannot change lambda gating
            if !tracks_tuple_commas && analysis.has_top_level_parameter_colon && !analysis.is_empty
            {
                break;
            }

            token_index += 1;
        }

        analysis
    }
}
