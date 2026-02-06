use std::str::FromStr;
use std::sync::Arc;

use super::html_entities::HTML_NAMED_ENTITIES;
use super::lexer::{Lexer, TreeExpressionEntry, TreeState};
use destack_ast::{
    Keyword, LiteralType, NumberBase, Token, TokenSpan, TokenType, UnaryOperator,
    is_identifier_continue, is_identifier_start, is_whitespace,
};

use destack_source::{File, LanguageType, Span};
use destack_unicode::UnicodeEmoji;

/// Result of lexing with additional flags.
#[derive(Debug)]
pub struct LexResult {
    /// The semantic tokens (identifiers, keywords, literals, operators).
    pub tokens: Vec<TokenSpan>,
    /// The non-semantic tokens (whitespace, comments).
    pub side_tokens: Vec<TokenSpan>,
    /// The end-of-file token.
    pub eof_token: TokenSpan,
    /// Whether an `@` token was seen.
    pub has_at: bool,
}

/// Result of parsing a single-quoted literal.
enum SingleQuotedLiteral {
    Character {
        is_terminated: bool,
        has_invalid_escape: bool,
    },
    String {
        is_terminated: bool,
        has_invalid_escape: bool,
    },
}

pub const TRIVIA_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Whitespace,
    TokenType::LineComment,
    TokenType::BlockComment,
    TokenType::DocLineComment,
    TokenType::DocBlockComment,
];

pub const EXPRESSION_START_TOKEN_TYPES: [TokenType; 18] = [
    TokenType::Assign,
    TokenType::Comma,
    TokenType::Colon,
    TokenType::Semicolon,
    TokenType::Equal,
    TokenType::NotEqual,
    TokenType::EqualWide,
    TokenType::NotEqualWide,
    TokenType::ElementwiseAnd,
    TokenType::LogicalAnd,
    TokenType::LogicalOr,
    TokenType::LogicalAndAssign,
    TokenType::LogicalOrAssign,
    TokenType::Maybe,
    TokenType::Coalesce,
    TokenType::OpenParenthesis,
    TokenType::OpenBracket,
    TokenType::OpenBrace,
];

/// Check if a token is semantic (not whitespace or comment).
/// Optimized for fast inline checking.
#[inline]
pub fn is_semantic(token_type: TokenType) -> bool {
    !matches!(
        token_type,
        TokenType::Whitespace
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

impl Lexer {
    /// Lex the input string into semantic tokens, side tokens, and the end-of-sequence Token.
    /// Semantic tokens are identifiers, keywords, literals, operators.
    /// Side tokens are whitespace and comments.
    pub fn lex(
        file: Arc<File>,
        language: LanguageType,
    ) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
        let result = Self::lex_with_flags(file, language);
        (result.tokens, result.side_tokens, result.eof_token)
    }

    /// Lex the input string and return extra flags.
    pub fn lex_with_flags(file: Arc<File>, language: LanguageType) -> LexResult {
        let mut lexer = Lexer::new(file, language);
        let eof_token = lexer.run();
        LexResult {
            tokens: lexer.tokens,
            side_tokens: lexer.side_tokens,
            eof_token,
            has_at: lexer.has_at,
        }
    }

    /// Runs the lexer until the end of the input string.
    /// Returns the end-of-sequence Token.
    fn run(&mut self) -> TokenSpan {
        // tokenize with spans, classifying into semantic vs side tokens
        loop {
            let start = self.pos as u32;
            let token = self.advance();
            let token_span = TokenSpan {
                token,
                span: Span {
                    file: self.file_id,
                    start,
                    end: (start + token.len),
                },
            };

            // push to appropriate vec based on token type
            if is_semantic(token.ty) {
                self.tokens.push(token_span);
                if token.ty == TokenType::At {
                    self.has_at = true;
                }
            } else {
                self.side_tokens.push(token_span);
            }

            // track last tokens for O(1) context lookups
            if is_semantic(token.ty) {
                self.track_semantic_token(token_span);
            }
            if token.ty == TokenType::End {
                break;
            }
        }

        // eof token (always in semantic tokens)
        *self.tokens.last().unwrap_or(&TokenSpan {
            span: Span {
                file: self.file_id,
                start: 0,
                end: 0,
            },
            token: Token::end(),
        })
    }

    /// Track semantic token context for regex and tree disambiguation.
    pub(super) fn track_semantic_token(&mut self, token_span: TokenSpan) {
        // update the last non-whitespace token (includes newlines)
        self.options.last_non_whitespace_token = Some(token_span);

        // update semantic token history (excludes newlines)
        if token_span.token.ty != TokenType::Newline {
            self.options.prev_prev_semantic_token = self.options.prev_semantic_token;
            self.options.prev_semantic_token = self.options.last_semantic_token;
            self.options.last_semantic_token = Some(token_span);
        }
    }

    /// Parses a token from the input string.
    pub(super) fn advance(&mut self) -> Token {
        // if we're in tree content mode, try to eat tree text
        if self.in_tree_content()
            && let Some(token) = self.try_eat_tree_text()
        {
            return token;
        }

        // eat first character until nothing is left (=EOF)
        let Some(first_char) = self.eat() else {
            return Token::new(TokenType::End, 0, None);
        };

        // parse token
        let (token_type, literal) = match first_char {
            // whitespace
            c if is_whitespace(c) => {
                if c == '\n' {
                    (TokenType::Newline, None)
                } else {
                    (self.eat_whitespace(), None)
                }
            }

            // slash, comments, regex, divide ops, or tree self-closing
            '/' => {
                let bytes = self.as_str().as_bytes();
                let next = bytes.first().copied();
                match next {
                    // //
                    Some(b'/') => {
                        // doc line comment if exactly three slashes and the fourth is not '/'
                        let third_is_slash = bytes.get(1).copied() == Some(b'/');
                        let fourth_is_slash = bytes.get(2).copied() == Some(b'/');
                        let is_doc_line = third_is_slash && !fourth_is_slash;
                        self.eat_until(b'\n');
                        if is_doc_line {
                            (TokenType::DocLineComment, None)
                        } else {
                            (TokenType::LineComment, None)
                        }
                    }
                    // /*
                    // block comments starting with '/*'
                    Some(b'*') => {
                        // detect doc block comment for exactly '/**' (not '/***')
                        let third_is_star = bytes.get(1).copied() == Some(b'*');
                        let fourth_is_star = bytes.get(2).copied() == Some(b'*');
                        let is_doc_block = third_is_star && !fourth_is_star;
                        // consume the initial '*'
                        self.eat();
                        // doc block comments do not nest
                        let is_terminated = if is_doc_block {
                            self.eat_doc_block_comment()
                        } else {
                            self.eat_block_comment()
                        };
                        // unterminated comment is an error
                        if !is_terminated {
                            (TokenType::Unknown, None)
                        } else if is_doc_block {
                            (TokenType::DocBlockComment, None)
                        } else {
                            (TokenType::BlockComment, None)
                        }
                    }
                    _ => {
                        // in tree opening tag mode, / is part of self-closing tag
                        if self.tree_state() == TreeState::OpeningTag && self.peek() == '>'
                            || self.tree_state() == TreeState::ClosingTag
                        {
                            (TokenType::Divide, None)
                        }
                        // regex or divide
                        else {
                            // /regex/ if we're in a start context
                            let is_expression_start = self.is_expression_start_for_regex();
                            // check it's not a closing tag
                            // (`/>` without a closing `/` on the same line)
                            let is_regex_start = {
                                // not an expression start, not a regex
                                if !is_expression_start {
                                    false
                                }
                                // not a closing tag, definitely a regex
                                else if self.peek() != '>' {
                                    true
                                }
                                // might be a regex iff we find a closing `/` on the line
                                else {
                                    let mut found_closing_slash_on_line = false;
                                    for c in self.as_str().chars() {
                                        if c == '/' {
                                            found_closing_slash_on_line = true;
                                            break;
                                        } else if c == '\n' {
                                            break;
                                        }
                                    }
                                    found_closing_slash_on_line
                                }
                            };

                            // regex
                            if is_regex_start {
                                let has_flags = self.eat_regex_string();
                                (
                                    TokenType::Literal,
                                    Some(LiteralType::RegexString { has_flags }),
                                )
                            }
                            // /=
                            else if self.peek() == '=' {
                                self.eat();
                                (TokenType::DivideAssign, None)
                            }
                            // /
                            else {
                                (TokenType::Divide, None)
                            }
                        }
                    }
                }
            }

            // other identifier
            c if is_identifier_start(c) => self.eat_identifier_or_such(c),

            // numeric literal
            c @ '0'..='9' => {
                let literal = self.eat_number_literal(c);
                (TokenType::Literal, Some(literal))
            }

            // symbols
            ':' => (TokenType::Colon, None),
            ';' => (TokenType::Semicolon, None),
            ',' => (TokenType::Comma, None),
            '.' => {
                // ..
                if self.peek() == '.' {
                    // ...
                    if self.peek_next() == '.' {
                        self.eat();
                        self.eat();
                        (TokenType::Spread, None)
                    }
                    // ..
                    else {
                        self.eat();
                        (TokenType::Range, None)
                    }
                }
                // decimal literal starting with .
                else if self.peek().is_ascii_digit() {
                    let literal = self.eat_leading_dot_number_literal();
                    (TokenType::Literal, Some(literal))
                }
                // .
                else {
                    (TokenType::Dot, None)
                }
            }
            '@' => (TokenType::At, None),
            '#' => (TokenType::Hash, None),
            '~' => (TokenType::ElementwiseNot, None),
            '?' => {
                // ??
                if self.peek() == '?' {
                    self.eat();
                    // ??=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::CoalesceAssign, None)
                    }
                    // ??
                    else {
                        (TokenType::Coalesce, None)
                    }
                }
                // ?
                else {
                    (TokenType::Maybe, None)
                }
            }

            // brackets
            '(' => {
                self.options.parentheses_depth += 1;
                (TokenType::OpenParenthesis, None)
            }
            ')' => {
                self.options.parentheses_depth -= 1;
                (TokenType::CloseParenthesis, None)
            }
            '[' => {
                self.options.parentheses_depth += 1;
                (TokenType::OpenBracket, None)
            }
            ']' => {
                self.options.parentheses_depth -= 1;
                (TokenType::CloseBracket, None)
            }
            '{' => {
                // in tree content mode, { starts an expression container
                if self.in_tree_content() {
                    // leave content mode (will return when } is matched)
                    self.pop_tree_state();
                    // track the depth and tree level so we know when to return to content mode
                    self.options
                        .tree_expression_stack
                        .push(TreeExpressionEntry {
                            parentheses_depth: self.options.parentheses_depth,
                            tree_depth: self.options.tree_state_stack.len(),
                            from_content: true,
                        });
                }
                // in tree opening tag mode, { starts an attribute expression container
                // (e.g., <Component attr={<NestedJSX />} />)
                else if self.tree_state() == TreeState::OpeningTag {
                    // don't pop OpeningTag - we're still parsing attributes
                    // but track the expression so nested JSX is recognized
                    self.options
                        .tree_expression_stack
                        .push(TreeExpressionEntry {
                            parentheses_depth: self.options.parentheses_depth,
                            tree_depth: self.options.tree_state_stack.len(),
                            from_content: false,
                        });
                }
                self.options.parentheses_depth += 1;
                (TokenType::OpenBrace, None)
            }
            // closing brace or maybe start of template middle
            '}' => {
                self.options.parentheses_depth -= 1;

                // we're at the end of a template string interpolation
                if self.options.template_string_stack.last()
                    == Some(&self.options.parentheses_depth)
                {
                    self.options.template_string_stack.pop();
                    let (is_complete, has_invalid_escape) = self.eat_template_string();
                    // invalid escape in template literal is an error
                    if has_invalid_escape {
                        (TokenType::Unknown, None)
                    } else if is_complete {
                        (TokenType::TemplateStringEnd, None)
                    } else {
                        // continue eating the template (after `${`, again)
                        self.options
                            .template_string_stack
                            .push(self.options.parentheses_depth);
                        self.options.parentheses_depth += 1; // for the opening `{` (again)
                        (TokenType::TemplateStringMiddle, None)
                    }
                }
                // we're at the end of a tree expression container
                else if let Some(entry) = self.options.tree_expression_stack.last()
                    && entry.parentheses_depth == self.options.parentheses_depth
                    && entry.tree_depth == self.options.tree_state_stack.len()
                {
                    let from_content = entry.from_content;
                    self.options.tree_expression_stack.pop();
                    // only return to content mode if we came from content mode
                    // (not for attribute expression containers in OpeningTag mode)
                    if from_content {
                        self.push_tree_state(TreeState::Content);
                    }
                    (TokenType::CloseBrace, None)
                } else {
                    (TokenType::CloseBrace, None)
                }
            }

            // bang
            '!' => {
                // !=
                if self.peek() == '=' {
                    self.eat();
                    // !==
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::NotEqualWide, None)
                    }
                    // !=
                    else {
                        (TokenType::NotEqual, None)
                    }
                }
                // !
                else {
                    (TokenType::Not, None)
                }
            }

            // subtract or arrow
            '-' => {
                // ->
                if self.peek() == '>' {
                    self.eat();
                    (TokenType::Arrow, None)
                }
                // -%
                else if self.peek() == '%' {
                    self.eat();
                    // -%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingSubtractAssign, None)
                    }
                    // -%
                    else {
                        (TokenType::WrappingSubtract, None)
                    }
                }
                // -|
                else if self.peek() == '|' {
                    self.eat();
                    // -|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingSubtractAssign, None)
                    }
                    // -|
                    else {
                        (TokenType::SaturatingSubtract, None)
                    }
                }
                // -=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::SubtractAssign, None)
                }
                // --
                else if self.peek() == '-' {
                    self.eat();
                    (TokenType::Decrement, None)
                }
                // -
                else {
                    (TokenType::Subtract, None)
                }
            }

            // elementwise and, logical and and their assignments
            '&' => {
                // only decode html entities inside tree content
                if self.in_tree_content()
                    && let Some(html_entity_token) = self.try_eat_html_entity()
                {
                    html_entity_token
                }
                // treat invalid html entities as text content
                else if self.in_tree_content() {
                    self.eat_tree_text_after_ampersand();
                    (TokenType::Literal, Some(LiteralType::TreeString))
                }
                // &&
                else if self.peek() == '&' {
                    self.eat();
                    // &&=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::LogicalAndAssign, None)
                    }
                    // &&
                    else {
                        (TokenType::LogicalAnd, None)
                    }
                }
                // &=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseAndAssign, None)
                }
                // &
                else {
                    (TokenType::ElementwiseAnd, None)
                }
            }

            // elementwise or, logical or and their assignments
            '|' => {
                // ||
                if self.peek() == '|' {
                    self.eat();
                    // ||=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::LogicalOrAssign, None)
                    }
                    // ||
                    else {
                        (TokenType::LogicalOr, None)
                    }
                }
                // |=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseOrAssign, None)
                }
                // |
                else {
                    (TokenType::ElementwiseOr, None)
                }
            }

            // equal or assign
            '=' => {
                // =>
                if self.peek() == '>' {
                    self.eat();
                    (TokenType::ArrowWide, None)
                }
                // ==
                else if self.peek() == '=' {
                    self.eat();
                    // ===
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::EqualWide, None)
                    }
                    // ==
                    else {
                        (TokenType::Equal, None)
                    }
                }
                // =
                else {
                    (TokenType::Assign, None)
                }
            }

            // less than, shift left, or tree literal opening
            '<' => {
                let in_tree_opening_tag = self.tree_state() == TreeState::OpeningTag
                    && !self.in_tree_attribute_expression();

                // static arguments inside opening tag
                if in_tree_opening_tag {
                    // <<
                    if self.peek() == '<' {
                        self.eat();
                        self.options.tree_tag_angle_depth += 2;
                        // <<|
                        if self.peek() == '|' {
                            self.eat();
                            // <<|=
                            if self.peek() == '=' {
                                self.eat();
                                (TokenType::SaturatingShiftLeftAssign, None)
                            }
                            // <<|
                            else {
                                (TokenType::SaturatingShiftLeft, None)
                            }
                        }
                        // <<=
                        else if self.peek() == '=' {
                            self.eat();
                            (TokenType::ShiftLeftAssign, None)
                        }
                        // <<
                        else {
                            (TokenType::ShiftLeft, None)
                        }
                    }
                    // <=
                    else if self.peek() == '=' {
                        self.eat();
                        self.options.tree_tag_angle_depth += 1;
                        (TokenType::LessThanOrEqual, None)
                    }
                    // <
                    else {
                        self.options.tree_tag_angle_depth += 1;
                        (TokenType::LessThan, None)
                    }
                }
                // <<
                else if self.peek() == '<' {
                    self.eat();
                    // <<|
                    if self.peek() == '|' {
                        self.eat();
                        // <<|=
                        if self.peek() == '=' {
                            self.eat();
                            (TokenType::SaturatingShiftLeftAssign, None)
                        }
                        // <<|
                        else {
                            (TokenType::SaturatingShiftLeft, None)
                        }
                    }
                    // <<=
                    else if self.peek() == '=' {
                        self.eat();
                        (TokenType::ShiftLeftAssign, None)
                    }
                    // <<
                    else {
                        (TokenType::ShiftLeft, None)
                    }
                }
                // <=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::LessThanOrEqual, None)
                }
                // </ - tree closing tag (when tree state is Content)
                // NOTE #Robustness: we check tree_state() directly, not in_tree_content()
                // (because in_tree_content() returns false inside expression containers
                // but we still need to recognize </tag> for nested tree literals)
                else if self.tree_state() == TreeState::Content
                    && self.peek_tree_closing_after_trivia()
                {
                    // pop from content mode (closing tag will finish with >)
                    self.pop_tree_state();
                    // push closing tag mode
                    self.push_tree_state(TreeState::ClosingTag);
                    (TokenType::LessThan, None)
                }
                // < - tree opening tag inside tree content
                else if self.tree_state() == TreeState::Content {
                    self.push_tree_state(TreeState::OpeningTag);
                    (TokenType::LessThan, None)
                }
                // < - plain less than
                else {
                    (TokenType::LessThan, None)
                }
            }

            // greater than, shift right, or tree tag close
            '>' => {
                // in tree opening tag mode, > ends the tag
                if self.tree_state() == TreeState::OpeningTag
                    && !self.in_tree_attribute_expression()
                {
                    if self.options.tree_tag_angle_depth > 0 {
                        self.options.tree_tag_angle_depth -= 1;
                        (TokenType::GreaterThan, None)
                    } else {
                        // check if previous token was / (self-closing tag)
                        let prev_is_divide = self
                            .options
                            .last_semantic_token
                            .map(|t| t.token.ty == TokenType::Divide)
                            .unwrap_or(false);

                        if prev_is_divide {
                            // self-closing: just pop, no content mode
                            self.pop_tree_state();
                        } else {
                            // regular opening: transition to content mode
                            self.pop_tree_state();
                            self.push_tree_state(TreeState::Content);
                        }
                        (TokenType::GreaterThan, None)
                    }
                }
                // in tree closing tag mode, > ends the closing tag
                else if self.tree_state() == TreeState::ClosingTag {
                    // just pop closing tag mode, return to parent context
                    self.pop_tree_state();
                    (TokenType::GreaterThan, None)
                }
                // >>=
                else if self.peek() == '>' && self.peek_next() == '=' {
                    self.eat(); // >
                    self.eat(); // =
                    (TokenType::ShiftRightAssign, None)
                }
                // >>>=
                else if self.peek() == '>'
                    && self.peek_next() == '>'
                    && self.peek_next_next() == '='
                {
                    self.eat(); // >
                    self.eat(); // >
                    self.eat(); // =
                    (TokenType::UnsignedShiftRightAssign, None)
                }
                // >=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::GreaterThanOrEqual, None)
                }
                // >
                else {
                    (TokenType::GreaterThan, None)
                }
            }

            // xor
            '^' => {
                // ^=
                if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseXorAssign, None)
                }
                // ^
                else {
                    (TokenType::ElementwiseXor, None)
                }
            }

            // add
            '+' => {
                // +%
                if self.peek() == '%' {
                    self.eat();
                    // +%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingAddAssign, None)
                    }
                    // +%
                    else {
                        (TokenType::WrappingAdd, None)
                    }
                }
                // +|
                else if self.peek() == '|' {
                    self.eat();
                    // +|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingAddAssign, None)
                    }
                    // +|
                    else {
                        (TokenType::SaturatingAdd, None)
                    }
                }
                // +=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::AddAssign, None)
                }
                // ++
                else if self.peek() == '+' {
                    self.eat();
                    (TokenType::Increment, None)
                }
                // +
                else {
                    (TokenType::Add, None)
                }
            }

            // multiply
            '*' => {
                // ** (exponent, exponent assign, wrapping/saturating exponent, etc)
                if self.peek() == '*' {
                    self.eat();
                    // **%
                    if self.peek() == '%' {
                        self.eat();
                        // **%=
                        if self.peek() == '=' {
                            self.eat();
                            (TokenType::WrappingExponentAssign, None)
                        }
                        // **%
                        else {
                            (TokenType::WrappingExponent, None)
                        }
                    }
                    // **|
                    else if self.peek() == '|' {
                        self.eat();
                        // **|=
                        if self.peek() == '=' {
                            self.eat();
                            (TokenType::SaturatingExponentAssign, None)
                        }
                        // **|
                        else {
                            (TokenType::SaturatingExponent, None)
                        }
                    }
                    // **=
                    else if self.peek() == '=' {
                        self.eat();
                        (TokenType::ExponentAssign, None)
                    }
                    // **
                    else {
                        (TokenType::Exponent, None)
                    }
                }
                // *%
                else if self.peek() == '%' {
                    self.eat();
                    // *%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingMultiplyAssign, None)
                    }
                    // *%
                    else {
                        (TokenType::WrappingMultiply, None)
                    }
                }
                // *|
                else if self.peek() == '|' {
                    self.eat();
                    // *|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingMultiplyAssign, None)
                    }
                    // *|
                    else {
                        (TokenType::SaturatingMultiply, None)
                    }
                }
                // *=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::MultiplyAssign, None)
                }
                // *
                else {
                    (TokenType::Multiply, None)
                }
            }

            // remainder
            '%' => {
                // %=
                if self.peek() == '=' {
                    self.eat();
                    (TokenType::RemainderAssign, None)
                }
                // %
                else {
                    (TokenType::Remainder, None)
                }
            }

            // character literal (with fallback to string literal for #Compatibility)
            '\'' => match self.eat_single_quoted_string() {
                SingleQuotedLiteral::Character {
                    is_terminated,
                    has_invalid_escape,
                } => {
                    // Treat invalid escape as unterminated to trigger parse error
                    let kind = LiteralType::Character {
                        is_terminated: is_terminated && !has_invalid_escape,
                        is_html_entity: false,
                    };
                    (TokenType::Literal, Some(kind))
                }
                SingleQuotedLiteral::String {
                    is_terminated,
                    has_invalid_escape,
                } => {
                    let kind = LiteralType::String {
                        is_terminated,
                        has_invalid_escape,
                    };
                    (TokenType::Literal, Some(kind))
                }
            },

            // string literal
            '"' => {
                let (terminated, has_invalid_escape) = self.eat_double_quoted_string();
                let kind = LiteralType::String {
                    is_terminated: terminated,
                    has_invalid_escape,
                };
                (TokenType::Literal, Some(kind))
            }

            // template string literal
            '`' => {
                let (is_complete, has_invalid_escape) = self.eat_template_string();
                // invalid escape in template literal is an error
                if has_invalid_escape {
                    (TokenType::Unknown, None)
                } else if is_complete {
                    (TokenType::TemplateString, None)
                } else {
                    self.options
                        .template_string_stack
                        .push(self.options.parentheses_depth);
                    self.options.parentheses_depth += 1; // for the opening `${`
                    (TokenType::TemplateStringStart, None)
                }
            }

            // identifier starting with an emoji (for graceful error recovery)
            c if !c.is_ascii() && c.is_emoji_char() => (self.eat_invalid_identifier(), None),

            // backslash - could be unicode escape starting an identifier (\uXXXX or \u{...})
            '\\' => {
                if let Some(token) = self.try_eat_unicode_escape_identifier() {
                    token
                } else {
                    (TokenType::Unknown, None)
                }
            }

            _ => (TokenType::Unknown, None),
        };

        let token = Token::new(token_type, self.get_pos_within_token(), literal);
        self.reset_pos_within_token();
        token
    }

    fn try_eat_html_entity(&mut self) -> Option<(TokenType, Option<LiteralType>)> {
        let rest = self.as_str();
        let semicolon_idx = rest.find(';')?;
        if semicolon_idx == 0 {
            return None;
        }

        let entity_data = &rest.as_bytes()[..semicolon_idx];
        if entity_data.is_empty()
            || entity_data.iter().any(|byte| {
                !matches!(
                    byte,
                    b'#' | b'x' | b'X' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'
                )
            })
        {
            return None;
        }

        let start = self.pos.saturating_sub(1);
        let end = self.pos + semicolon_idx + 1;
        let source = self.file.text();
        if end > source.len() {
            return None;
        }

        let entity_slice = &source[start..end];
        let _ = decode_html_entity(entity_slice)?;

        for _ in 0..=semicolon_idx {
            self.eat();
        }

        Some((
            TokenType::Literal,
            Some(LiteralType::Character {
                is_terminated: true,
                is_html_entity: true,
            }),
        ))
    }

    /// Parses a whitespace sequence (excluding first character).
    fn eat_whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenType::Whitespace
    }

    /// Parses an identifier, unknown prefix or some literal string (excluding first character).
    /// Returns the token type and the literal type if it's a hardcoded literal.
    fn eat_identifier_or_such(&mut self, first_char: char) -> (TokenType, Option<LiteralType>) {
        debug_assert!(is_identifier_start(first_char));
        let start_pos = self.pos;
        // consume continuation characters until an unknown character is met
        self.eat_while(is_identifier_continue);
        // check for unicode escapes mid-identifier (e.g., `AB\u{43}`)
        // Only consume if it's a valid escape
        if self.peek() == '\\' && self.peek_next() == 'u' && self.is_valid_unicode_escape_ahead() {
            self.eat_identifier_with_unicode_escapes();
            return (TokenType::Identifier, None);
        }
        // known prefixes must have been handled earlier
        match self.peek() {
            '#' => return (TokenType::UnknownLiteralPrefix, None),
            c if !c.is_ascii() && c.is_emoji_char() => {
                return (self.eat_invalid_identifier(), None);
            }
            _ => {}
        }
        // boolean
        let source = self.file.text();
        if first_char == 't' && source[start_pos - 1..self.pos].eq("true") {
            (
                TokenType::Literal,
                Some(LiteralType::Boolean { value: true }),
            )
        }
        // false
        else if first_char == 'f' && source[start_pos - 1..self.pos].eq("false") {
            (
                TokenType::Literal,
                Some(LiteralType::Boolean { value: false }),
            )
        }
        // just an identifier
        else {
            (TokenType::Identifier, None)
        }
    }

    /// Parses an invalid identifier (excluding first character).
    fn eat_invalid_identifier(&mut self) -> TokenType {
        // start is already eaten, eat the rest of identifier
        self.eat_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_identifier_continue(c)
                || (!c.is_ascii() && c.is_emoji_char())
                || c == ZERO_WIDTH_JOINER
        });
        TokenType::InvalidIdentifier
    }

    /// Try to parse a unicode escape sequence that starts an identifier.
    /// Called after `\` has been eaten.
    /// Returns Some((TokenType::Identifier, None)) if successful, None otherwise.
    fn try_eat_unicode_escape_identifier(&mut self) -> Option<(TokenType, Option<LiteralType>)> {
        // check for \u
        if self.peek() != 'u' {
            return None;
        }
        self.eat(); // eat 'u'

        // parse the unicode escape value
        let code_point = if self.peek() == '{' {
            // \u{XXXX} form (ES6)
            self.eat(); // eat '{'
            let mut value: u32 = 0;
            let mut count = 0;
            while self.peek() != '}' && !self.is_end() {
                let c = self.peek();
                let digit = match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => return None, // invalid hex digit
                };
                value = value.checked_mul(16)?.checked_add(digit)?;
                self.eat();
                count += 1;
                if count > 6 {
                    return None; // too many digits
                }
            }
            if count == 0 || self.peek() != '}' {
                return None; // empty or unterminated
            }
            self.eat(); // eat '}'
            value
        } else {
            // \uXXXX form (ES5) - exactly 4 hex digits
            let mut value: u32 = 0;
            for _ in 0..4 {
                let c = self.peek();
                let digit = match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => return None, // invalid hex digit
                };
                value = value * 16 + digit;
                self.eat();
            }
            value
        };

        // convert to char and check if valid identifier start
        let ch = char::from_u32(code_point)?;
        if !is_identifier_start(ch) {
            return None;
        }

        // continue eating identifier (including more unicode escapes or regular chars)
        self.eat_identifier_with_unicode_escapes();

        Some((TokenType::Identifier, None))
    }

    /// Continue eating an identifier that may contain unicode escapes.
    fn eat_identifier_with_unicode_escapes(&mut self) {
        loop {
            let c = self.peek();
            if is_identifier_continue(c) {
                self.eat();
            } else if c == '\\' && self.peek_next() == 'u' {
                // Validate the unicode escape before consuming it
                if !self.is_valid_unicode_escape_ahead() {
                    break; // invalid escape, stop here
                }
                // Now consume the validated escape
                self.eat(); // eat '\'
                self.eat(); // eat 'u'
                if self.peek() == '{' {
                    // \u{XXXX} form
                    self.eat(); // eat '{'
                    while self.peek() != '}' && !self.is_end() {
                        self.eat();
                    }
                    if self.peek() == '}' {
                        self.eat();
                    }
                } else {
                    // \uXXXX form - exactly 4 hex digits
                    for _ in 0..4 {
                        self.eat();
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Check if there's a valid unicode escape sequence starting at current position.
    /// Does NOT consume any characters, just peeks ahead.
    fn is_valid_unicode_escape_ahead(&self) -> bool {
        let bytes = self.as_str().as_bytes();
        if bytes.len() < 2 || bytes[0] != b'\\' || bytes[1] != b'u' {
            return false;
        }

        if bytes.len() > 2 && bytes[2] == b'{' {
            // \u{XXXX} form - at least one hex digit and closing brace
            let mut i = 3;
            let mut count = 0;
            while i < bytes.len() && bytes[i] != b'}' {
                let c = bytes[i];
                if !c.is_ascii_hexdigit() {
                    return false;
                }
                count += 1;
                i += 1;
                if count > 6 {
                    return false;
                }
            }
            count > 0 && i < bytes.len() && bytes[i] == b'}'
        } else {
            // \uXXXX form - exactly 4 hex digits
            if bytes.len() < 6 {
                return false;
            }
            if !(bytes[2..6].iter().all(|&c| c.is_ascii_hexdigit())) {
                return false;
            }
            true
        }
    }

    /// Parses a number literal (excluding first digit).
    /// Returns the number literal.
    fn eat_number_literal(&mut self, first_digit: char) -> LiteralType {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // parse encoding base
            match self.peek() {
                // binary literal
                'b' | 'B' => {
                    base = NumberBase::Binary;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return LiteralType::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // octal literal
                'o' | 'O' => {
                    base = NumberBase::Octal;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return LiteralType::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // hexadecimal literal
                'x' | 'X' => {
                    base = NumberBase::Hexadecimal;
                    self.eat();
                    if !self.eat_hexadecimal_digits() {
                        return LiteralType::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // not a base prefix; consume additional digits
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }

                // also not a base prefix; nothing more to do here
                '.' | 'e' | 'E' | 'n' => {}

                // just a 0
                _ => {
                    return LiteralType::Int {
                        base,
                        is_empty: false,
                        is_bigint: false,
                    };
                }
            }
        } else {
            // no base prefix, parse number in the usual way
            self.eat_decimal_digits();
        }

        match self.peek() {
            // don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.peek_next() != '.' && !is_identifier_start(self.peek_next()) => {
                // might have stuff after the ., and if it does, it starts with a number
                self.eat();
                let mut is_empty_exponent = false;
                if self.peek().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.peek() {
                        'e' | 'E' => {
                            self.eat();
                            is_empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                LiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'e' | 'E' => {
                self.eat();
                let is_empty_exponent = !self.eat_float_exponent();
                LiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'n' => {
                self.eat();
                LiteralType::Int {
                    base,
                    is_empty: false,
                    is_bigint: true,
                }
            }
            _ => LiteralType::Int {
                base,
                is_empty: false,
                is_bigint: false,
            },
        }
    }

    /// Parse a decimal literal starting with a leading dot.
    fn eat_leading_dot_number_literal(&mut self) -> LiteralType {
        let base = NumberBase::Decimal;
        let is_empty_exponent = {
            self.eat_decimal_digits();
            if matches!(self.peek(), 'e' | 'E') {
                self.eat();
                !self.eat_float_exponent()
            } else {
                false
            }
        };
        LiteralType::Float {
            base,
            is_empty_exponent,
        }
    }

    /// Return true when quoted strings can span lines in tree opening tag attributes.
    #[inline]
    fn allow_line_terminator_in_tree_attribute_string(&self) -> bool {
        self.tree_state() == TreeState::OpeningTag && !self.in_tree_attribute_expression()
    }

    /// Parse a single-quoted literal (excluding the initial `'`).
    /// Might be a character if single-quoted length is 1 or a string otherwise.
    fn eat_single_quoted_string(&mut self) -> SingleQuotedLiteral {
        debug_assert!(self.prev() == '\'');

        let mut logical_len = 0_u32;
        let mut has_invalid_escape = false;

        // parse until either quotes are terminated or EOF is reached
        loop {
            // check for EOF first to avoid infinite loop on unterminated strings
            if self.is_end() {
                return Self::finish_single_quoted_literal(logical_len, false, has_invalid_escape);
            }

            match self.peek() {
                // quotes are terminated, finish parsing
                '\'' => {
                    self.eat();
                    return Self::finish_single_quoted_literal(
                        logical_len,
                        true,
                        has_invalid_escape,
                    );
                }
                // line terminators are not allowed in single quoted strings
                '\n' | '\r' => {
                    // allow multiline quoted values inside tree opening tag attributes
                    if self.allow_line_terminator_in_tree_attribute_string() {
                        self.eat();
                        logical_len = logical_len.saturating_add(1);
                        continue;
                    }
                    return Self::finish_single_quoted_literal(
                        logical_len,
                        false,
                        has_invalid_escape,
                    );
                }
                // escaped character is considered one logical character
                '\\' => {
                    self.eat(); // eat '\'
                    if self.is_end() {
                        return Self::finish_single_quoted_literal(
                            logical_len,
                            false,
                            has_invalid_escape,
                        );
                    }

                    // consume escape sequence and track invalid escapes
                    if self.eat_string_escape_sequence() {
                        has_invalid_escape = true;
                    }
                    logical_len = logical_len.saturating_add(1);
                }
                // regular character
                _ => {
                    self.eat();
                    logical_len = logical_len.saturating_add(1);
                }
            }
        }
    }

    #[inline]
    fn finish_single_quoted_literal(
        logical_len: u32,
        is_terminated: bool,
        has_invalid_escape: bool,
    ) -> SingleQuotedLiteral {
        if logical_len == 1 {
            SingleQuotedLiteral::Character {
                is_terminated,
                has_invalid_escape,
            }
        } else {
            SingleQuotedLiteral::String {
                is_terminated,
                has_invalid_escape,
            }
        }
    }

    /// Parses a double-quoted string (excluding first `"`).
    /// Returns (is_terminated, has_invalid_escape).
    fn eat_double_quoted_string(&mut self) -> (bool, bool) {
        debug_assert!(self.prev() == '"');
        let mut has_invalid_escape = false;
        while !self.is_end() {
            match self.peek() {
                '"' => {
                    self.eat();
                    return (true, has_invalid_escape);
                }
                '\\' => {
                    self.eat();
                    // consume escape sequence and track invalid escapes
                    if self.eat_string_escape_sequence() {
                        has_invalid_escape = true;
                    }
                }
                // line terminators are not allowed in double quoted strings
                '\n' | '\r' => {
                    // allow multiline quoted values inside tree opening tag attributes
                    if self.allow_line_terminator_in_tree_attribute_string() {
                        self.eat();
                        continue;
                    }
                    return (false, has_invalid_escape);
                }
                _ => {
                    self.eat();
                }
            }
        }
        // end of file reached
        (false, has_invalid_escape)
    }

    /// Consume a string escape sequence after `\`.
    /// Returns true when the escape sequence is invalid.
    fn eat_string_escape_sequence(&mut self) -> bool {
        if self.is_end() {
            return true;
        }

        let escaped = self.peek();
        match escaped {
            // \8 and \9 are always invalid
            '8' | '9' => {
                self.eat();
                true
            }
            // \0 through \7 are legacy octal in js and ts
            '0'..='7' => {
                self.eat();
                if self.language.is_javascript() || self.language.is_typescript() {
                    if escaped != '0' {
                        return true;
                    }

                    if self.peek().is_ascii_digit() {
                        return true;
                    }
                }
                false
            }
            // \uXXXX and \u{...}
            'u' => {
                self.eat(); // eat `u`
                self.eat_unicode_escape_after_u()
            }
            // \xXX
            'x' => {
                self.eat(); // eat `x`
                self.eat_fixed_hex_escape(2)
            }
            // regular escaped character
            _ => {
                self.eat();
                false
            }
        }
    }

    /// Consume a unicode escape sequence body after `\u`.
    /// Returns true when the sequence is invalid.
    fn eat_unicode_escape_after_u(&mut self) -> bool {
        if self.peek() == '{' {
            self.eat(); // eat `{`
            let mut digits = 0usize;
            let mut value: u32 = 0;
            let mut overflowed = false;
            while self.peek().is_ascii_hexdigit() {
                if !overflowed {
                    let digit = self.peek().to_digit(16).unwrap_or(0);
                    if let Some(next) = value.checked_mul(16).and_then(|v| v.checked_add(digit)) {
                        value = next;
                    } else {
                        overflowed = true;
                    }
                }
                self.eat();
                digits += 1;
            }

            if digits == 0 || self.peek() != '}' {
                return true;
            }
            self.eat(); // eat `}`
            overflowed || value > 0x10FFFF
        } else {
            self.eat_fixed_hex_escape(4)
        }
    }

    /// Consume an exact number of hexadecimal digits.
    /// Returns true when the sequence is invalid.
    fn eat_fixed_hex_escape(&mut self, width: usize) -> bool {
        for _ in 0..width {
            if !self.peek().is_ascii_hexdigit() {
                return true;
            }
            self.eat();
        }
        false
    }

    /// Parses a regex string (excluding first `/`, including any flags after `/`).
    /// Works exactly like JS/TS regex literals.
    fn eat_regex_string(&mut self) -> bool {
        debug_assert!(self.prev() == '/');
        let mut escaped = false;
        let mut in_character_class = false;

        // match until next '/' outside character classes
        while let Some(c) = self.eat() {
            if escaped {
                escaped = false;
                continue;
            }

            if c == '\\' {
                escaped = true;
                continue;
            }

            if in_character_class {
                if c == ']' {
                    in_character_class = false;
                }
                continue;
            }

            if c == '[' {
                in_character_class = true;
                continue;
            }

            if c == '/' {
                break;
            }
        }
        // flags are are any alpha characters immediately after the last '/'
        let mut has_flags = false;
        loop {
            if self.peek().is_ascii_alphabetic() {
                has_flags = true;
                self.eat();
            } else {
                break;
            }
        }
        has_flags
    }

    /// Parses a template string (excluding first backtick).
    /// Returns (is_complete, has_invalid_escape).
    fn eat_template_string(&mut self) -> (bool, bool) {
        let mut has_invalid_escape = false;
        while let Some(c) = self.eat() {
            match c {
                '`' => {
                    return (true, has_invalid_escape);
                }
                '$' if self.peek() == '{' => {
                    self.eat();
                    return (false, has_invalid_escape);
                }
                '\\' => {
                    let escaped = self.peek();
                    // octal escapes are forbidden in template literals
                    // this includes \0 followed by another digit, and \1 through \9
                    if escaped.is_ascii_digit() && escaped != '0' {
                        // \1 through \9 are always invalid in templates
                        has_invalid_escape = true;
                    } else if escaped == '0' {
                        // \0 followed by another digit is invalid (legacy octal)
                        self.eat();
                        if self.peek().is_ascii_digit() {
                            has_invalid_escape = true;
                        }
                        continue;
                    }
                    // skip escaped backslash or backtick
                    if escaped == '\\' || escaped == '`' {
                        self.eat();
                    }
                }
                _ => (),
            }
        }
        (false, has_invalid_escape)
    }

    /// Parses decimal digits.
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
                '_' => {
                    self.eat();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.eat();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Parses hexadecimal digits.
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
                '_' => {
                    self.eat();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.eat();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Parses the float exponent (excluding `e` or `E`).
    /// Returns whether the exponent is non-empty.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.peek() == '-' || self.peek() == '+' {
            self.eat();
        }
        self.eat_decimal_digits()
    }

    /// Parse a doc block comment body without nesting support.
    /// Assume the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    /// Return true if the comment was properly terminated, false if EOF was reached.
    pub(crate) fn eat_doc_block_comment(&mut self) -> bool {
        // scan until the first closing delimiter
        while !self.is_end() {
            let bytes = self.as_str().as_bytes();
            if bytes.len() >= 2 && bytes[0] == b'*' && bytes[1] == b'/' {
                // consume '*/'
                self.eat();
                self.eat();
                return true;
            }
            // consume a single character and continue
            let _ = self.eat();
        }

        // reached EOF without closing comment
        false
    }

    /// Parses a block comment body with nesting support.
    /// Assumes the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    /// Returns true if the comment was properly terminated, false if EOF was reached.
    pub(crate) fn eat_block_comment(&mut self) -> bool {
        let mut depth: u32 = 1;
        while !self.is_end() {
            let bytes = self.as_str().as_bytes();
            if bytes.len() >= 2 {
                // start of nested block comment
                if bytes[0] == b'/' && bytes[1] == b'*' {
                    // consume '/*'
                    self.eat();
                    self.eat();
                    depth = depth.saturating_add(1);
                    continue;
                }
                // end of current block comment level
                if bytes[0] == b'*' && bytes[1] == b'/' {
                    // consume '*/'
                    self.eat();
                    self.eat();
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return true; // properly terminated
                    }
                    continue;
                }
            }
            // consume a single character and continue
            let _ = self.eat();
        }
        // reached EOF without closing comment
        false
    }

    /// Return true when a tree closing tag starts after trivia.
    fn peek_tree_closing_after_trivia(&self) -> bool {
        // scan forward until we hit a non trivia character
        let mut iter = self.as_str().char_indices().peekable();

        while let Some((_offset, c)) = iter.next() {
            // skip whitespace
            if is_whitespace(c) {
                continue;
            }

            // handle comment prefixes and closing tags
            if c == '/' {
                let next = iter.peek().map(|(_, c)| *c);

                // skip line comments
                if next == Some('/') {
                    iter.next();
                    for (_offset, c) in iter.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }

                // skip block comments
                if next == Some('*') {
                    iter.next();
                    let mut prev = '\0';
                    for (_offset, c) in iter.by_ref() {
                        if prev == '*' && c == '/' {
                            break;
                        }
                        prev = c;
                    }
                    continue;
                }

                return true;
            }

            return false;
        }

        false
    }

    /// Tries to eat tree literal text content (TSX-compatible).
    /// Returns a Literal token with TreeString type if there's text content.
    /// Text content ends at `<`, `{`, or `&` (for HTML entities).
    fn try_eat_tree_text(&mut self) -> Option<Token> {
        // peek at what's coming - don't eat yet
        let first = self.peek();

        // these characters start other tokens, not text
        // `&` may start an HTML entity, so let advance() handle it
        if matches!(first, '<' | '>' | '{' | '}' | '&' | '\0') {
            return None;
        }

        // eat characters until we hit a boundary
        self.eat(); // consume first character

        while !self.is_end() {
            let c = self.peek();
            match c {
                // boundaries: start of tag, expression container, or potential html entity
                '<' | '>' | '{' | '}' | '&' => break,
                _ => {
                    self.eat();
                }
            }
        }

        // always produce a token for consumed text
        // (the parser will normalize/trim whitespace as needed per TSX rules)
        let token = Token::new(
            TokenType::Literal,
            self.get_pos_within_token(),
            Some(LiteralType::TreeString),
        );
        self.reset_pos_within_token();
        Some(token)
    }

    /// Eat tree literal text starting from an already consumed `&`.
    fn eat_tree_text_after_ampersand(&mut self) {
        while !self.is_end() {
            let c = self.peek();
            match c {
                '<' | '>' | '{' | '&' => break,
                _ => {
                    self.eat();
                }
            }
        }
    }

    /// Check if `/` can start a regex literal.
    fn is_expression_start_for_regex(&self) -> bool {
        let last_non_whitespace = self.options.last_non_whitespace_token.as_ref();
        let Some(last_non_whitespace) = last_non_whitespace else {
            return true;
        };

        // handle newline boundaries via statement termination rules
        if last_non_whitespace.token.ty == TokenType::Newline {
            let Some(last_semantic) = self.options.last_semantic_token.as_ref() else {
                return true;
            };
            return self.is_statement_boundary_after_newline(
                Some(last_non_whitespace),
                last_semantic,
                self.options.prev_semantic_token.as_ref(),
            );
        }

        // control statement headers can be followed by expression statements
        if last_non_whitespace.token.ty == TokenType::CloseParenthesis
            && self.close_parenthesis_ends_control_header()
        {
            return true;
        }

        if EXPRESSION_START_TOKEN_TYPES.contains(&last_non_whitespace.token.ty) {
            return true;
        }

        if last_non_whitespace.token.ty == TokenType::Identifier {
            return Keyword::from_str(self.get_span_str(last_non_whitespace.span))
                .map(|keyword| {
                    keyword.is_control()
                        || keyword == Keyword::Delete
                        || UnaryOperator::from_prefix_keyword(keyword).is_some()
                        || keyword == Keyword::Default && self.default_follows_export()
                })
                .unwrap_or(false);
        }

        false
    }

    // detect `export default /regex/` context
    fn default_follows_export(&self) -> bool {
        let Some(previous) = self.options.prev_semantic_token.as_ref() else {
            return false;
        };
        if previous.token.ty != TokenType::Identifier {
            return false;
        }
        Keyword::from_str(self.get_span_str(previous.span)) == Ok(Keyword::Export)
    }

    // detect control headers ending in `)` where the body can start with `/regex/`
    fn close_parenthesis_ends_control_header(&self) -> bool {
        let Some(last_token) = self.options.last_semantic_token.as_ref() else {
            return false;
        };
        if last_token.token.ty != TokenType::CloseParenthesis {
            return false;
        }

        let mut depth = 0i32;
        let mut matching_open_index: Option<usize> = None;

        // find the matching opening parenthesis for the trailing `)`
        for index in (0..self.tokens.len()).rev() {
            let token = self.tokens[index];
            if token.token.ty == TokenType::Newline {
                continue;
            }

            if token.token.ty == TokenType::CloseParenthesis {
                depth += 1;
                continue;
            }

            if token.token.ty == TokenType::OpenParenthesis {
                depth -= 1;
                if depth == 0 {
                    matching_open_index = Some(index);
                    break;
                }
            }
        }

        let Some(matching_open_index) = matching_open_index else {
            return false;
        };

        // look at the token before the matched `(`
        let mut previous_keyword_token: Option<TokenSpan> = None;
        for index in (0..matching_open_index).rev() {
            let token = self.tokens[index];
            if token.token.ty == TokenType::Newline {
                continue;
            }
            previous_keyword_token = Some(token);
            break;
        }

        let Some(previous_keyword_token) = previous_keyword_token else {
            return false;
        };
        if previous_keyword_token.token.ty != TokenType::Identifier {
            return false;
        }

        let Ok(keyword) = Keyword::from_str(self.get_span_str(previous_keyword_token.span)) else {
            return false;
        };
        matches!(
            keyword,
            Keyword::If | Keyword::While | Keyword::For | Keyword::With
        )
    }

    /// Check whether a newline can terminate a statement before a tree literal.
    fn is_statement_boundary_after_newline(
        &self,
        last_non_whitespace: Option<&TokenSpan>,
        last_semantic: &TokenSpan,
        prev_semantic: Option<&TokenSpan>,
    ) -> bool {
        // only treat newlines as boundaries
        let is_newline = matches!(
            last_non_whitespace.map(|token| token.token.ty),
            Some(TokenType::Newline)
        );
        if !is_newline {
            return false;
        }

        // semicolons always terminate statements
        if last_semantic.token.ty == TokenType::Semicolon {
            return true;
        }

        // block end followed by newline starts a new statement
        if last_semantic.token.ty == TokenType::CloseBrace {
            return true;
        }

        // regex literals can terminate a statement before another regex literal
        if last_semantic.token.ty == TokenType::Literal
            && matches!(
                last_semantic.token.literal,
                Some(LiteralType::RegexString { .. })
            )
        {
            return true;
        }

        // allow after statement-terminating keywords
        if last_semantic.token.ty == TokenType::Identifier
            && let Ok(keyword) = Keyword::from_str(self.get_span_str(last_semantic.span))
            && matches!(
                keyword,
                Keyword::Return
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::Debugger
                    | Keyword::Yield
            )
        {
            return true;
        }

        // allow after let/var declaration with newline terminator
        if let Some(prev_token) = prev_semantic
            && last_semantic.token.ty == TokenType::Identifier
            && prev_token.token.ty == TokenType::Identifier
            && let Ok(keyword) = Keyword::from_str(self.get_span_str(prev_token.span))
            && matches!(keyword, Keyword::Let | Keyword::Var)
        {
            return true;
        }

        false
    }
}

/// Decodes an HTML entity into a character.
pub fn decode_html_entity(entity: &str) -> Option<char> {
    if !entity.starts_with('&') || !entity.ends_with(';') {
        return None;
    }

    let body = &entity[1..entity.len() - 1];
    if body.is_empty() {
        return None;
    }

    if let Some(codepoint) = body.strip_prefix('#') {
        return decode_numeric_entity(codepoint);
    }

    decode_named_entity(body)
}

/// Decodes a numeric entity into a character (like `&#x1234;` or `&#1234;`).
#[inline]
fn decode_numeric_entity(codepoint: &str) -> Option<char> {
    let (radix, digits) = if let Some(hex_digits) = codepoint.strip_prefix(['x', 'X']) {
        (16, hex_digits)
    } else {
        (10, codepoint)
    };

    if digits.is_empty() {
        return None;
    }

    let is_valid_digits = if radix == 16 {
        digits.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        digits.chars().all(|c| c.is_ascii_digit())
    };

    if !is_valid_digits {
        return None;
    }

    let value = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(value)
}

/// Decodes a named entity into a character (like `&lt;` or `&amp;`).
#[inline]
fn decode_named_entity(name: &str) -> Option<char> {
    HTML_NAMED_ENTITIES
        .binary_search_by(|(entity_name, _)| (*entity_name).cmp(name))
        .ok()
        .map(|idx| HTML_NAMED_ENTITIES[idx].1)
}
