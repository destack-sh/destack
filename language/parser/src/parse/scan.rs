use crate::Parser;
use crate::parse::RecoveryPoint;
use destack_dir::TokenType;

/// Nesting depth for syntax that scans across balanced delimiters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DelimiterDepth {
    /// The nested parenthesis depth.
    parenthesis: usize,
    /// The nested bracket depth.
    bracket: usize,
    /// The nested brace depth.
    brace: usize,
    /// The nested angle depth.
    angle: usize,
}

impl DelimiterDepth {
    /// Build delimiter depth after one outer delimiter has been consumed.
    fn from_outer(open: TokenType, close: TokenType) -> Option<Self> {
        match (open, close) {
            (TokenType::OpenParenthesis, TokenType::CloseParenthesis) => Some(Self {
                parenthesis: 1,
                ..Self::default()
            }),
            (TokenType::OpenBracket, TokenType::CloseBracket) => Some(Self {
                bracket: 1,
                ..Self::default()
            }),
            (TokenType::OpenBrace, TokenType::CloseBrace) => Some(Self {
                brace: 1,
                ..Self::default()
            }),
            _ => None,
        }
    }

    /// Return whether no nested delimiter is open.
    pub(crate) fn is_top_level(self) -> bool {
        self.parenthesis == 0 && self.bracket == 0 && self.brace == 0 && self.angle == 0
    }

    /// Return whether the cursor is directly inside one outer delimiter.
    fn is_directly_inside(self, open: TokenType) -> bool {
        match open {
            TokenType::OpenParenthesis => {
                self.parenthesis == 1 && self.bracket == 0 && self.brace == 0 && self.angle == 0
            }
            TokenType::OpenBracket => {
                self.parenthesis == 0 && self.bracket == 1 && self.brace == 0 && self.angle == 0
            }
            TokenType::OpenBrace => {
                self.parenthesis == 0 && self.bracket == 0 && self.brace == 1 && self.angle == 0
            }
            _ => false,
        }
    }

    /// Advance delimiter depth for one token.
    pub(crate) fn advance(&mut self, token_type: TokenType) -> bool {
        match token_type {
            TokenType::OpenParenthesis => self.parenthesis += 1,
            TokenType::CloseParenthesis => return Self::close(&mut self.parenthesis),
            TokenType::OpenBracket => self.bracket += 1,
            TokenType::CloseBracket => return Self::close(&mut self.bracket),
            TokenType::OpenBrace => self.brace += 1,
            TokenType::CloseBrace => return Self::close(&mut self.brace),
            TokenType::LessThan => self.angle += 1,
            TokenType::ShiftLeft => self.angle += 2,
            TokenType::GreaterThan => return self.close_angle(1),
            TokenType::ShiftRight => return self.close_angle(2),
            TokenType::UnsignedShiftRight => return self.close_angle(3),
            _ => {}
        }

        true
    }

    /// Close one ordinary delimiter level.
    fn close(depth: &mut usize) -> bool {
        if *depth == 0 {
            return false;
        }

        *depth -= 1;

        true
    }

    /// Close one or more angle levels.
    fn close_angle(&mut self, width: usize) -> bool {
        if self.angle < width {
            return false;
        }

        self.angle -= width;

        true
    }
}

impl Parser {
    /// Scan the token after a balanced parenthesized group at an offset.
    pub(crate) fn scan_parenthesized_follow_token_at_offset(
        &mut self,
        start_offset: usize,
    ) -> Option<TokenType> {
        self.scan_balanced_follow_token_at_offset(
            start_offset,
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
            true,
        )
    }

    /// Scan the token after a balanced parenthesized group whose open token was consumed.
    pub(crate) fn scan_parenthesized_follow_token_after_open(&mut self) -> Option<TokenType> {
        self.scan_balanced_follow_token_after_open(
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
            true,
        )
    }

    /// Scan the token after a balanced bracket group at an offset.
    pub(crate) fn scan_bracket_follow_token_at_offset(
        &mut self,
        start_offset: usize,
    ) -> Option<TokenType> {
        self.scan_balanced_follow_token_at_offset(
            start_offset,
            TokenType::OpenBracket,
            TokenType::CloseBracket,
            true,
        )
    }

    /// Scan the token after a balanced brace group at an offset.
    pub(crate) fn scan_brace_follow_token_at_offset(
        &mut self,
        start_offset: usize,
    ) -> Option<TokenType> {
        self.scan_balanced_follow_token_at_offset(
            start_offset,
            TokenType::OpenBrace,
            TokenType::CloseBrace,
            false,
        )
    }

    /// Skip one balanced delimiter group at the current token.
    pub(crate) fn skip_balanced_delimiter(&mut self, open: TokenType, close: TokenType) -> bool {
        if !self.peek_is(open) {
            return false;
        }

        self.bump();

        self.scan_balanced_follow_token_after_open(open, close, true)
            .is_some()
    }

    /// Scan the token after an angle group whose open token was consumed.
    pub(crate) fn scan_angle_follow_token_after_open(
        &mut self,
        mut angle_depth: usize,
    ) -> Option<TokenType> {
        while !self.peek_is(TokenType::End) {
            let token_type = self.peek_token_type();
            let close_width = Self::angle_close_width(token_type);

            if self.current_semicolon_precedes_recovery_point(token_type, RecoveryPoint::Statement)
            {
                return None;
            }

            // close this angle group
            if close_width == angle_depth {
                self.bump();

                return Some(self.peek_token_type());
            }

            // reject malformed angle closes
            if close_width > angle_depth {
                return None;
            }

            // advance angle depth
            angle_depth += Self::angle_open_width(token_type);
            angle_depth -= close_width;
            self.bump();
        }

        None
    }

    /// Scan the token after a balanced delimiter group at an offset.
    fn scan_balanced_follow_token_at_offset(
        &mut self,
        start_offset: usize,
        open: TokenType,
        close: TokenType,
        stops_at_semicolon: bool,
    ) -> Option<TokenType> {
        // skip to the owned delimiter
        for _ in 0..start_offset {
            if self.peek_is(TokenType::End) {
                return None;
            }

            self.bump();
        }

        // require the expected opener
        if !self.peek_is(open) {
            return None;
        }

        self.bump();
        self.scan_balanced_follow_token_after_open(open, close, stops_at_semicolon)
    }

    /// Scan the token after a balanced delimiter group whose open token was consumed.
    fn scan_balanced_follow_token_after_open(
        &mut self,
        open: TokenType,
        close: TokenType,
        stops_at_semicolon: bool,
    ) -> Option<TokenType> {
        let mut delimiter_depth = DelimiterDepth::from_outer(open, close)?;

        loop {
            let token_type = self.peek_token_type();

            // stop at hard boundaries
            if token_type == TokenType::End {
                return None;
            }
            if stops_at_semicolon
                && token_type == TokenType::Semicolon
                && (delimiter_depth.is_directly_inside(open)
                    || self.current_semicolon_precedes_recovery_point(
                        token_type,
                        RecoveryPoint::Statement,
                    ))
            {
                return None;
            }

            // advance all nested delimiters
            if !delimiter_depth.advance(token_type) {
                return None;
            }
            self.bump();

            if delimiter_depth.is_top_level() {
                return Some(self.peek_token_type());
            }
        }
    }

    /// Return how many angle levels one opening token adds.
    fn angle_open_width(token_type: TokenType) -> usize {
        match token_type {
            TokenType::LessThan => 1,
            TokenType::ShiftLeft => 2,
            _ => 0,
        }
    }

    /// Return how many angle levels one closing token consumes.
    fn angle_close_width(token_type: TokenType) -> usize {
        match token_type {
            TokenType::GreaterThan => 1,
            TokenType::ShiftRight => 2,
            TokenType::UnsignedShiftRight => 3,
            _ => 0,
        }
    }
}
