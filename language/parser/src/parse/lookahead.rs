use crate::parse::ExpressionStop;
use crate::{Parser, TokenProbe};
use tspp_dir::{Keyword, Token, TokenType};

/// Nested delimiters tracked during classification and recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DelimiterDepth {
    /// The nested parenthesis depth.
    parenthesis: u32,
    /// The nested bracket depth.
    bracket: u32,
    /// The nested brace depth.
    brace: u32,
    /// The nested angle depth.
    angle: u32,
    /// Whether angle brackets affect depth.
    tracks_angle: bool,
}

/// Whether a generic parameter head is distinct from a tree opening tag.
#[derive(Clone, Copy, Eq, PartialEq)]
enum GenericDisambiguation {
    /// The head remains ambiguous in value space.
    Ambiguous,
    /// A top-level comma or constraint distinguishes the head from a tree tag.
    Distinct,
}

impl DelimiterDepth {
    /// Create delimiter depth for type expression scans.
    pub(crate) const fn type_expression() -> Self {
        Self {
            parenthesis: 0,
            bracket: 0,
            brace: 0,
            angle: 0,
            tracks_angle: true,
        }
    }

    /// Create delimiter depth for value expression scans.
    pub(crate) const fn value() -> Self {
        Self {
            tracks_angle: false,
            ..Self::type_expression()
        }
    }

    /// Create delimiter depth for one list terminator.
    pub(crate) const fn list(terminator: TokenType) -> Self {
        Self {
            tracks_angle: matches!(terminator, TokenType::GreaterThan),
            ..Self::type_expression()
        }
    }

    /// Return whether no nested delimiter is open.
    #[inline]
    pub(crate) const fn is_top_level(self) -> bool {
        self.parenthesis == 0 && self.bracket == 0 && self.brace == 0 && self.angle == 0
    }

    /// Return whether the cursor is directly inside one outer ordinary delimiter.
    pub(crate) const fn is_directly_inside(self, open: TokenType) -> bool {
        match open {
            TokenType::OpenParenthesis => {
                self.parenthesis == 1 && self.bracket == 0 && self.brace == 0
            }
            TokenType::OpenBracket => self.parenthesis == 0 && self.bracket == 1 && self.brace == 0,
            TokenType::OpenBrace => self.parenthesis == 0 && self.bracket == 0 && self.brace == 1,
            _ => false,
        }
    }

    /// Return whether one compound close continues into an enclosing angle group.
    pub(crate) const fn closes_enclosing_angle(self, token_type: TokenType) -> bool {
        if self.parenthesis != 0 || self.bracket != 0 || self.brace != 0 {
            return false;
        }

        Self::angle_close_width(token_type) > self.angle
    }

    /// Advance delimiter depth for one token.
    #[inline]
    pub(crate) fn advance(&mut self, token_type: TokenType) -> bool {
        self.advance_delimiter(token_type, self.tracks_angle)
    }

    /// Advance delimiter depth without interpreting angle tokens.
    #[inline]
    fn advance_without_angle(&mut self, token_type: TokenType) -> bool {
        self.advance_delimiter(token_type, false)
    }

    /// Apply one delimiter token under the requested angle policy.
    fn advance_delimiter(&mut self, token_type: TokenType, tracks_angle: bool) -> bool {
        match token_type {
            TokenType::OpenParenthesis => {
                self.parenthesis += 1;
            }
            TokenType::CloseParenthesis => return Self::close(&mut self.parenthesis),
            TokenType::OpenBracket => {
                self.bracket += 1;
            }
            TokenType::CloseBracket => return Self::close(&mut self.bracket),
            TokenType::OpenBrace => {
                self.brace += 1;
            }
            TokenType::CloseBrace => return Self::close(&mut self.brace),
            TokenType::LessThan if tracks_angle => self.angle += 1,
            TokenType::ShiftLeft if tracks_angle => self.angle += 2,
            TokenType::GreaterThan if tracks_angle => return self.close_angle(1),
            TokenType::ShiftRight if tracks_angle => return self.close_angle(2),
            TokenType::UnsignedShiftRight if tracks_angle => return self.close_angle(3),
            _ => {}
        }

        true
    }

    /// Close one ordinary delimiter level.
    fn close(depth: &mut u32) -> bool {
        if *depth == 0 {
            return false;
        }

        *depth -= 1;

        true
    }

    /// Close one or more angle levels.
    fn close_angle(&mut self, width: u32) -> bool {
        if self.angle < width {
            return false;
        }

        self.angle -= width;

        true
    }

    /// Return how many angle levels one opening token adds.
    pub(crate) const fn angle_open_width(token_type: TokenType) -> u32 {
        match token_type {
            TokenType::LessThan => 1,
            TokenType::ShiftLeft => 2,
            _ => 0,
        }
    }

    /// Return how many angle levels one closing token consumes.
    pub(crate) const fn angle_close_width(token_type: TokenType) -> u32 {
        match token_type {
            TokenType::GreaterThan => 1,
            TokenType::ShiftRight => 2,
            TokenType::UnsignedShiftRight => 3,
            _ => 0,
        }
    }
}

impl Parser {
    /// Return whether one delimited assignment pattern starts here.
    pub(crate) fn peek_destructuring_assignment(&self) -> bool {
        let (open, close) = match self.peek_token_type() {
            TokenType::OpenBrace => (TokenType::OpenBrace, TokenType::CloseBrace),
            TokenType::OpenBracket => (TokenType::OpenBracket, TokenType::CloseBracket),
            TokenType::OpenParenthesis => (TokenType::OpenParenthesis, TokenType::CloseParenthesis),
            _ => return false,
        };
        let mut probe = self.cursor.probe(&self.file);

        probe.scan_group(open, close) && probe.peek_token_type() == TokenType::Assign
    }

    /// Return whether one parenthesized assignment target is tuple shaped.
    pub(crate) fn peek_tuple_assignment_pattern(&self) -> bool {
        let mut probe = self.cursor.probe(&self.file);
        let mut has_comma = false;
        let is_group = probe.scan_group_direct(
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
            |token| has_comma |= token == TokenType::Comma,
        );

        is_group && has_comma
    }

    /// Return whether the current generic argument is type-shaped.
    pub(crate) fn peek_generic_argument_type(&self) -> bool {
        let argument_start = self.peek_token().start();
        let mut probe = self.cursor.probe(&self.file);

        // skip a generic spread marker
        if probe.peek_token_type() == TokenType::Spread {
            probe.bump();
        }

        // classify top-level operators while balancing nested groups
        let mut delimiters = DelimiterDepth::type_expression();
        let mut saw_extends = false;
        loop {
            let token_type = probe.peek_token_type();

            // treat damaged input through EOF as type-shaped for recovery
            if token_type == TokenType::End {
                return true;
            }

            // stop at the generic item terminator
            if delimiters.is_top_level()
                && (token_type == TokenType::Comma
                    || matches!(
                        token_type,
                        TokenType::GreaterThan
                            | TokenType::ShiftRight
                            | TokenType::UnsignedShiftRight
                            | TokenType::GreaterThanOrEqual
                            | TokenType::ShiftRightAssign
                            | TokenType::UnsignedShiftRightAssign
                    ))
            {
                return true;
            }
            if delimiters.closes_enclosing_angle(token_type) {
                return true;
            }

            // classify operators that cannot occur in a type argument
            if delimiters.is_top_level() {
                if matches!(
                    token_type,
                    TokenType::Assign
                        | TokenType::AddAssign
                        | TokenType::SubtractAssign
                        | TokenType::MultiplyAssign
                        | TokenType::DivideAssign
                        | TokenType::RemainderAssign
                        | TokenType::ExponentAssign
                        | TokenType::ShiftLeftAssign
                        | TokenType::ElementwiseAndAssign
                        | TokenType::ElementwiseOrAssign
                        | TokenType::ElementwiseXorAssign
                        | TokenType::LogicalAnd
                        | TokenType::LogicalOr
                        | TokenType::Coalesce
                        | TokenType::Equal
                        | TokenType::EqualWide
                        | TokenType::NotEqual
                        | TokenType::NotEqualWide
                        | TokenType::Divide
                        | TokenType::Remainder
                        | TokenType::Exponent
                ) {
                    return false;
                }

                if matches!(token_type, TokenType::Add | TokenType::Subtract)
                    && probe.peek_token().start() != argument_start
                {
                    return false;
                }

                if token_type == TokenType::Maybe && !saw_extends {
                    return false;
                }

                if token_type == TokenType::Identifier {
                    match probe.peek_keyword() {
                        Some(Keyword::Extends) => saw_extends = true,
                        Some(
                            Keyword::As
                            | Keyword::In
                            | Keyword::InstanceOf
                            | Keyword::Is
                            | Keyword::Satisfies,
                        ) => return false,
                        _ => {}
                    }
                }
            }

            if !delimiters.advance(token_type) {
                return false;
            }

            probe.bump();
        }
    }

    /// Return whether the current parenthesized group contains one top-level token.
    pub(crate) fn peek_parenthesized_group_contains(&self, target: TokenType) -> Option<bool> {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return None;
        }

        // scan the group while observing its direct children
        let mut probe = self.cursor.probe(&self.file);
        let mut depth = DelimiterDepth::value();
        let mut is_first = true;
        let mut contains = false;
        loop {
            let token_type = probe.peek_token_type();

            // reject an unclosed group
            if token_type == TokenType::End {
                return None;
            }

            // record a direct occurrence of the target
            if !is_first && depth.parenthesis == 1 && token_type == target {
                contains = true;
            }

            // reject mismatched delimiters
            if !depth.advance(token_type) {
                return None;
            }

            probe.bump();

            // finish after consuming the outer close
            if !is_first && depth.is_top_level() {
                return Some(contains);
            }

            is_first = false;
        }
    }

    /// Return whether the current parenthesized group is unmistakably a parameter list.
    pub(crate) fn peek_parenthesized_parameter_list(&self) -> bool {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return false;
        }

        // accept unmistakable first-token forms
        let mut probe = self.cursor.probe(&self.file);
        probe.bump();
        if probe.peek_token_type() == TokenType::CloseParenthesis {
            return true;
        }

        if matches!(
            probe.peek_token_type(),
            TokenType::Spread | TokenType::OpenBrace | TokenType::OpenBracket | TokenType::At
        ) {
            return true;
        }

        // scan direct group children for parameter-only separators
        let mut delimiters = DelimiterDepth::value();
        if !delimiters.advance(TokenType::OpenParenthesis) {
            return false;
        }

        loop {
            let token_type = probe.peek_token_type();

            // reject an unclosed group
            if token_type == TokenType::End {
                return false;
            }

            // accept parameter punctuation at the outer group level
            if delimiters.is_directly_inside(TokenType::OpenParenthesis)
                && matches!(
                    token_type,
                    TokenType::Colon | TokenType::Comma | TokenType::Maybe | TokenType::Assign
                )
            {
                return true;
            }

            // reject mismatched delimiters
            if !delimiters.advance(token_type) {
                return false;
            }

            probe.bump();

            // a plain closed group remains expression-shaped
            if delimiters.is_top_level() {
                return false;
            }
        }
    }

    /// Return the token after one balanced delimiter group.
    pub(crate) fn peek_token_after_group(
        &self,
        start_offset: usize,
        open: TokenType,
        close: TokenType,
    ) -> Option<Token> {
        // advance to the requested opening token
        let mut probe = self.cursor.probe(&self.file);
        for _ in 0..start_offset {
            probe.bump();
        }

        if !probe.scan_delimiter_group(open, close) {
            return None;
        }

        Some(probe.peek_token())
    }

    /// Return the token after one balanced angle group.
    pub(crate) fn peek_token_after_angle_group(
        &self,
        start_offset: usize,
        mut angle_depth: u32,
    ) -> Option<Token> {
        // advance to the first token inside the angle group
        let mut probe = self.cursor.probe(&self.file);
        for _ in 0..start_offset {
            probe.bump();
        }

        // balance angles separately from ordinary nested groups
        let mut delimiters = DelimiterDepth::type_expression();
        loop {
            let token_type = probe.peek_token_type();

            // stop at damaged or statement-terminated input
            if token_type == TokenType::End
                || token_type == TokenType::Semicolon && delimiters.is_top_level()
            {
                return None;
            }

            // ignore angle shaped tokens inside ordinary delimiter groups
            let is_nested =
                delimiters.parenthesis > 0 || delimiters.bracket > 0 || delimiters.brace > 0;
            if !is_nested {
                let close_width = DelimiterDepth::angle_close_width(token_type);

                // return after the exact outer angle close
                if close_width == angle_depth {
                    probe.bump();

                    return Some(probe.peek_token());
                }

                // treat the remainder of a wider compound close as the follow operator
                if close_width > angle_depth {
                    return Some(probe.peek_token());
                }

                // update angle depth for compound angle tokens
                angle_depth += DelimiterDepth::angle_open_width(token_type);
                angle_depth -= close_width;
            }

            // reject mismatched ordinary delimiters
            if !delimiters.advance_without_angle(token_type) {
                return None;
            }

            probe.bump();
        }
    }

    /// Return whether the current angle group can continue one value expression as generics.
    pub(crate) fn peek_angle_group_expression_postfix(&self) -> bool {
        let Some(follow) = self.peek_token_after_angle_group(1, 1) else {
            return false;
        };
        let token_type = follow.ty();

        // declaration heads on a new line follow a completed instantiation statement
        if follow.is_on_new_line()
            && matches!(
                self.token_keyword(follow),
                Some(
                    Keyword::Let
                        | Keyword::Const
                        | Keyword::Using
                        | Keyword::Function
                        | Keyword::Struct
                        | Keyword::Class
                        | Keyword::Enum
                        | Keyword::Interface
                        | Keyword::Extension
                        | Keyword::Type
                        | Keyword::Newtype
                        | Keyword::Export
                )
            )
        {
            return true;
        }

        // these tokens can instead be the right operand of a relational expression
        !matches!(
            token_type,
            TokenType::Identifier | TokenType::Literal | TokenType::Add | TokenType::Subtract
        )
    }

    /// Return whether the current `async` token starts a lambda expression.
    pub(crate) fn peek_async_lambda(&self) -> bool {
        if self.peek_keyword() != Some(Keyword::Async) {
            return false;
        }

        let mut probe = self.cursor.probe(&self.file);
        probe.bump();
        if probe.peek_token().is_on_new_line() {
            return false;
        }

        match probe.peek_token_type() {
            TokenType::Identifier => {
                probe.bump();

                probe.peek_token_type() == TokenType::ArrowWide
            }
            TokenType::OpenParenthesis => probe.scan_function(),
            TokenType::LessThan | TokenType::ShiftLeft => {
                probe.scan_generic_function() == Some(GenericDisambiguation::Distinct)
            }
            _ => false,
        }
    }

    /// Return whether the current angle group starts a generic lambda expression.
    pub(crate) fn peek_generic_lambda(&self) -> bool {
        if !matches!(
            self.peek_token_type(),
            TokenType::LessThan | TokenType::ShiftLeft
        ) {
            return false;
        }

        let mut probe = self.cursor.probe(&self.file);

        probe.scan_generic_function() == Some(GenericDisambiguation::Distinct)
    }

    /// Return whether the current angle group starts a generic function type.
    pub(crate) fn peek_generic_function_type(&self) -> bool {
        let mut probe = self.cursor.probe(&self.file);

        probe.scan_generic_function().is_some()
    }

    /// Return whether one parenthesized group starts a lambda expression.
    pub(crate) fn peek_parenthesized_lambda(&self, stop: ExpressionStop) -> bool {
        let is_parameter_list = self.peek_parenthesized_parameter_list();
        let mut probe = self.cursor.probe(&self.file);
        if !probe.scan_group(TokenType::OpenParenthesis, TokenType::CloseParenthesis) {
            return false;
        }
        if stop.has(ExpressionStop::CONDITIONAL_COLON)
            && probe.peek_token_type() == TokenType::Colon
            && !is_parameter_list
        {
            return false;
        }

        probe.scan_function_arrow()
    }

    /// Return whether `<<` starts generic arguments with a generic function type.
    pub(crate) fn peek_shift_left_generic_function_argument(&self) -> bool {
        if !self.peek_is(TokenType::ShiftLeft) {
            return false;
        }

        let mut probe = self.cursor.probe(&self.file);
        probe.bump();

        probe.scan_open_generic_function(1).is_some()
    }
}

impl TokenProbe<'_> {
    /// Advance past one balanced delimiter group, including statement bodies.
    pub(in crate::parse) fn scan_delimiter_group(
        &mut self,
        open: TokenType,
        close: TokenType,
    ) -> bool {
        if self.peek_token_type() != open {
            return false;
        }

        // balance the complete group
        let mut delimiters = DelimiterDepth::value();
        if !delimiters.advance(open) {
            return false;
        }

        self.bump();
        loop {
            let token_type = self.peek_token_type();

            // reject an unclosed group
            if token_type == TokenType::End {
                return false;
            }

            // reject mismatched delimiters
            if !delimiters.advance(token_type) {
                return false;
            }

            self.bump();

            // stop on the token after the outer close
            if delimiters.is_top_level() {
                debug_assert_eq!(token_type, close);

                return true;
            }
        }
    }

    /// Return whether one consumed parameter head has a function arrow.
    fn scan_function_arrow(&mut self) -> bool {
        if self.peek_token_type() == TokenType::ArrowWide {
            return true;
        }
        if self.peek_token_type() != TokenType::Colon {
            return false;
        }

        self.bump();

        self.scan_function_return()
    }

    /// Advance through one parenthesized parameter list and its function arrow.
    fn scan_function(&mut self) -> bool {
        self.scan_group(TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            && self.scan_function_arrow()
    }

    /// Advance through one generic function head.
    fn scan_generic_function(&mut self) -> Option<GenericDisambiguation> {
        let disambiguation = self.scan_generic_parameter_group()?;
        self.scan_function().then_some(disambiguation)
    }

    /// Advance through one generic function head whose opening token was consumed.
    fn scan_open_generic_function(&mut self, depth: u32) -> Option<GenericDisambiguation> {
        let disambiguation = self.scan_open_generic_parameter_group(depth)?;
        self.scan_function().then_some(disambiguation)
    }

    /// Advance through one generic parameter group.
    fn scan_generic_parameter_group(&mut self) -> Option<GenericDisambiguation> {
        let depth = match self.peek_token_type() {
            TokenType::LessThan => 1u32,
            TokenType::ShiftLeft => 2u32,
            _ => return None,
        };
        self.bump();

        self.scan_open_generic_parameter_group(depth)
    }

    /// Advance through a generic parameter group whose opening token was consumed.
    fn scan_open_generic_parameter_group(
        &mut self,
        mut depth: u32,
    ) -> Option<GenericDisambiguation> {
        let outer_depth = depth;
        let mut disambiguation = GenericDisambiguation::Ambiguous;
        let mut delimiters = DelimiterDepth::value();

        loop {
            let token_type = self.peek_token_type();
            let is_top_level = delimiters.is_top_level();
            if token_type == TokenType::End || token_type == TokenType::Semicolon && is_top_level {
                return None;
            }

            // record TSX-safe generic parameter syntax at the outer angle level
            if is_top_level
                && depth == outer_depth
                && (matches!(token_type, TokenType::Colon | TokenType::Comma)
                    || self.peek_keyword() == Some(Keyword::Extends))
            {
                disambiguation = GenericDisambiguation::Distinct;
            }

            // balance angle tokens outside ordinary delimiter groups
            if is_top_level {
                let close_width = DelimiterDepth::angle_close_width(token_type);
                if close_width > depth {
                    return None;
                }
                depth += DelimiterDepth::angle_open_width(token_type);
                depth -= close_width;
                if depth == 0 {
                    self.bump();

                    return Some(disambiguation);
                }
            }

            if !delimiters.advance(token_type) {
                return None;
            }

            self.bump();
        }
    }

    /// Advance through one balanced ordinary delimiter group.
    fn scan_group(&mut self, open: TokenType, close: TokenType) -> bool {
        self.scan_group_direct(open, close, |_| {})
    }

    /// Advance through one balanced group and visit its directly nested tokens.
    fn scan_group_direct(
        &mut self,
        open: TokenType,
        close: TokenType,
        mut visit: impl FnMut(TokenType),
    ) -> bool {
        if self.peek_token_type() != open {
            return false;
        }

        let mut delimiters = DelimiterDepth::value();
        if !delimiters.advance(open) {
            return false;
        }
        self.bump();

        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End
                || token_type == TokenType::Semicolon && delimiters.is_directly_inside(open)
            {
                return false;
            }
            if delimiters.is_directly_inside(open) {
                visit(token_type);
            }
            if !delimiters.advance(token_type) {
                return false;
            }

            self.bump();
            if delimiters.is_top_level() {
                return token_type == close;
            }
        }
    }

    /// Return whether a function return scan reaches an arrow at top level.
    fn scan_function_return(&mut self) -> bool {
        let mut delimiters = DelimiterDepth::value();
        let mut angle_depth = 0u32;

        loop {
            let token_type = self.peek_token_type();
            let is_top_level = delimiters.is_top_level() && angle_depth == 0;

            // reject an unclosed return type at end of source
            if token_type == TokenType::End {
                return false;
            }

            // accept the lambda arrow after a complete return type
            if is_top_level && token_type == TokenType::ArrowWide {
                return true;
            }

            // stop at top-level expression boundaries
            if is_top_level
                && matches!(
                    token_type,
                    TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseParenthesis
                        | TokenType::CloseBracket
                        | TokenType::CloseBrace
                )
            {
                return false;
            }

            // track type argument angles outside ordinary delimiters
            if delimiters.is_top_level() {
                let close_width = DelimiterDepth::angle_close_width(token_type);
                if close_width > angle_depth {
                    return false;
                }
                angle_depth += DelimiterDepth::angle_open_width(token_type);
                angle_depth -= close_width;
            }
            if !delimiters.advance(token_type) {
                return false;
            }

            self.bump();
        }
    }
}
