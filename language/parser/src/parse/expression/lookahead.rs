use crate::{ParseResult, Parser};

use destack_ast::TokenType;
use destack_source::Span;

/// Parenthesized group analysis metadata.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParenthesizedGroupShape {
    /// The matching close parenthesis span when known.
    pub close_span: Option<Span>,
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
        // tree literal starts like `(<div>...)` do not need delimiter-shape lookahead
        let has_parenthesized_tree_literal = self.language.supports_jsx()
            && !self.flags.is_in_type()
            && !self.flags.is_in_arrow_return_type()
            && self.parenthesized_group_starts_with_tree_literal();
        if has_parenthesized_tree_literal {
            return Ok(ParenthesizedGroupShape::default());
        }

        // base grouped expressions branch on the token after `)`
        let can_use_follow_token = !self.language.is_destack()
            && !self.flags.is_in_type()
            && !self.flags.is_in_arrow_return_type();
        if can_use_follow_token {
            self.stats.record_parenthesized_follow_token_call();

            if let Some(close_span) = self.find_matching_close_for_parenthesized_group() {
                let follow_token_type = self.lookahead(|parser| {
                    while parser.current_token().span.start <= close_span.start {
                        parser.bump();
                    }

                    parser.peek_token_type()
                });
                if follow_token_type != TokenType::End {
                    self.stats.record_parenthesized_follow_token_hit();

                    if matches!(follow_token_type, TokenType::Arrow | TokenType::ArrowWide) {
                        return Ok(ParenthesizedGroupShape {
                            close_span: Some(close_span),
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
        let needs_snapshot = self.allow_tree_literals() && !self.flags.is_in_type();
        if needs_snapshot {
            self.stats.record_delimiter_analysis_snapshot_lookup();
        }

        // compute the full grouped shape with snapshotting when needed
        let group_shape = if needs_snapshot {
            let rewind_mark = self.cursor_checkpoint();
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
        if !self.peek_is(TokenType::OpenParenthesis) {
            return Ok(ParenthesizedGroupShape::default());
        }

        // find the matching close parenthesis first
        let Some(close_span) = self.find_matching_close_for_parenthesized_group() else {
            return Ok(ParenthesizedGroupShape::default());
        };

        // inspect the follow token and surrounding context
        let needs_group_contents_shape = self.language.is_destack();
        let follow_token_type = self.lookahead(|parser| {
            while parser.current_token().span.start <= close_span.start {
                parser.bump();
            }

            match parser.peek_token_type() {
                TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon => {
                    Some(parser.peek_token_type())
                }
                _ => None,
            }
        });
        let follow_token_type = match follow_token_type {
            Some(TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon) => follow_token_type,
            _ => None,
        };
        let has_colon_follow = follow_token_type == Some(TokenType::Colon);
        let needs_parameter_shape_for_arrow_return = self.flags.is_in_arrow_return_type();
        let needs_parameter_shape_for_typed_colon = self.flags.is_in_type() && has_colon_follow;
        let needs_parameter_shape_for_ternary_colon =
            self.flags.is_in_ternary_condition() && has_colon_follow;

        // plain group parsing only needs the token after `)` unless an
        // enclosing context also needs the inner parameter shape
        if !needs_group_contents_shape
            && !needs_parameter_shape_for_arrow_return
            && !needs_parameter_shape_for_typed_colon
            && !needs_parameter_shape_for_ternary_colon
        {
            return Ok(ParenthesizedGroupShape {
                close_span: Some(close_span),
                follow_token_type,
                is_empty: false,
                ..ParenthesizedGroupShape::default()
            });
        }

        // compute top level separators and operators inside the group
        let mut group_shape = self.scan_parenthesized_group_shape(close_span);
        group_shape.close_span = Some(close_span);
        group_shape.follow_token_type = follow_token_type;

        Ok(group_shape)
    }

    /// Find the matching close token for the current parenthesized group.
    fn find_matching_close_for_parenthesized_group(&mut self) -> Option<Span> {
        // scan forward for the matching close
        self.find_matching_close_maybe(TokenType::OpenParenthesis, TokenType::CloseParenthesis)
    }

    /// Return true when the immediate parenthesized payload starts with a tree literal.
    fn parenthesized_group_starts_with_tree_literal(&mut self) -> bool {
        // require tree literal support first
        if !self.language.supports_jsx() {
            return false;
        }

        self.lookahead(|parser| {
            parser.bump();
            parser.can_start_tree_literal()
        })
    }

    /// Scan parenthesized contents once and collect top-level shape metadata.
    fn scan_parenthesized_group_shape(&mut self, close_span: Span) -> ParenthesizedGroupShape {
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

        // scan the grouped contents once
        self.lookahead(|parser| {
            parser.bump();
            while parser.current_token().span.start < close_span.start {
                let token_type = parser.peek_token_type();

                let is_in_nested_delimiter =
                    parenthesis_depth > 0 || brace_depth > 0 || bracket_depth > 0;
                let is_top_level = !is_in_nested_delimiter && angle_depth == 0;

                // remember wrapped expression heads like `(() => x)`
                if analysis.is_empty && is_top_level && token_type == TokenType::OpenParenthesis {
                    analysis.starts_with_nested_parenthesis = true;
                }

                analysis.is_empty = false;

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

                // track nested non angle delimiters inline
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
                if !needs_group_contents_shape && analysis.has_top_level_arrow && !analysis.is_empty
                {
                    break;
                }

                parser.bump();
            }
        });

        analysis
    }
}
