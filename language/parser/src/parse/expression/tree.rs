use crate::Parser;

use destack_ast::{Keyword, TokenType};

impl Parser {
    /// Peek a tree literal with value-expression position rules.
    fn peek_tree_literal_in_value_position(&mut self) -> bool {
        self.with_flags(self.flags.not_in_position(), |parser| {
            parser.peek_tree_literal().is_ok()
        })
    }

    /// Return true when `<` starts a generic arrow expression.
    pub(super) fn can_start_generic_arrow_expression(&mut self) -> bool {
        if !self.language.is_typescript() && !self.language.is_destack() {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.language.supports_jsx() && self.has_shift_left_tree_generic_arguments() {
            return false;
        }
        if self.language.supports_jsx() && self.can_start_tree_literal() {
            return false;
        }

        // require disambiguators only when explicitly requested by settings
        let require_disambiguator =
            self.flags.is_disallow_ambiguous_tree_literal() && !self.flags.is_in_type();
        self.peek_generic_arrow_after_type_parameters(require_disambiguator)
    }

    /// Return true if `<` starts a tree literal without committing tokens.
    pub(crate) fn can_start_tree_literal(&mut self) -> bool {
        if !self.language.supports_jsx() || self.flags.is_in_type() {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.has_shift_left_tree_generic_arguments() {
            return self.peek_tree_literal_in_value_position();
        }

        let mark = self.cursor_checkpoint();
        let require_disambiguator = self.flags.is_disallow_ambiguous_tree_literal();
        let is_disambiguated_generic =
            self.peek_generic_arrow_after_type_parameters(require_disambiguator);
        self.rewind(mark);
        if is_disambiguated_generic {
            return false;
        }

        self.peek_tree_literal_in_value_position()
    }

    /// Return true when a line break is followed by a tree literal start.
    pub(super) fn can_start_tree_literal_after_line_break(&mut self) -> bool {
        // require jsx support
        if !self.language.supports_jsx() {
            return false;
        }

        // require a line break before the next semantic token
        if !self.current_token_is_on_new_line() {
            return false;
        }

        // probe current token with in_type disabled
        self.lookahead(|parser| {
            let ambient_context = parser.flags.with_type(false);
            let expression_context = parser.flags.not_in_position();
            parser.with_flags(
                parser
                    .flags
                    .with_ambient_context(ambient_context)
                    .with_expression_context(expression_context),
                |parser| parser.can_start_tree_literal(),
            )
        })
    }

    /// Return true when `<Identifier <<` starts tree generic arguments.
    pub(super) fn has_shift_left_tree_generic_arguments(&mut self) -> bool {
        self.lookahead(|parser| {
            parser.bump();
            if !parser.peek_is(TokenType::Identifier) {
                return false;
            }

            parser.bump();
            parser.peek_is(TokenType::ShiftLeft)
        })
    }

    /// Return true if the current `do` token starts a do-while statement.
    /// Block-required modes only accept `do { ... } while ...`, while semicolon statement forms accept any statement body.
    pub(super) fn is_do_while_statement(&mut self, next_token_type: TokenType) -> bool {
        // semicolon statement forms allow any statement body
        if !self.language.is_destack() {
            return true;
        }

        // block-required modes require a block body
        if next_token_type != TokenType::OpenBrace {
            return false;
        }

        self.lookahead(|parser| {
            parser.bump();
            let matching = match parser
                .find_matching_close_maybe(TokenType::OpenBrace, TokenType::CloseBrace)
            {
                Some(span) => span,
                None => return false,
            };

            while parser.current_token().span.start <= matching.start {
                parser.bump();
            }

            parser.current_keyword() == Some(Keyword::While)
        })
    }
}
