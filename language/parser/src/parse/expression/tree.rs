use crate::Parser;

use destack_ast::{Keyword, TokenType};

impl Parser {
    /// Return true when `<` starts a generic arrow expression.
    pub(super) fn can_start_generic_arrow_expression(&mut self) -> bool {
        if !self.language.is_typescript() && !self.language.is_destack() {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.language.supports_jsx() && self.has_shift_left_tree_static_arguments() {
            return false;
        }
        if self.language.supports_jsx() && self.can_start_tree_literal() {
            return false;
        }

        // require disambiguators only when explicitly requested by settings
        let require_disambiguator =
            self.options.disallow_ambiguous_tree_literal && !self.options.in_type;
        self.peek_generic_arrow_after_type_parameters(require_disambiguator)
    }

    /// Return true if `<` starts a tree literal without committing tokens.
    pub(crate) fn can_start_tree_literal(&mut self) -> bool {
        if !self.language.supports_jsx() || self.options.in_type {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.has_shift_left_tree_static_arguments() {
            return self.peek_tree_literal().is_ok();
        }

        let mark = self.mark_rewind();
        let require_disambiguator = self.options.disallow_ambiguous_tree_literal;
        let is_disambiguated_generic =
            self.peek_generic_arrow_after_type_parameters(require_disambiguator);
        self.rewind(mark);
        if is_disambiguated_generic {
            return false;
        }

        self.peek_tree_literal().is_ok()
    }

    /// Return true when a line break is followed by a tree literal start.
    pub(super) fn can_start_tree_literal_after_line_break(&mut self) -> bool {
        // require jsx support
        if !self.language.supports_jsx() {
            return false;
        }

        // require a line break before the next semantic token
        let cursor = self.peek_cursor();
        if !cursor.has_line_break_before {
            return false;
        }

        // ensure the token exists for probing
        let next = cursor.index;
        self.ensure_token(next);
        if next >= self.tokens().len() {
            return false;
        }

        // probe from the next token with in_type disabled
        self.with_pos(next, |parser| {
            let mut options = parser.options;
            options.in_type = false;
            parser.with_options(options, |inner| inner.can_start_tree_literal())
        })
    }

    /// Return true when `<Identifier <<` starts tree static arguments.
    pub(super) fn has_shift_left_tree_static_arguments(&mut self) -> bool {
        // snapshot parser state for lookahead
        let mark = self.mark_rewind();

        // probe for `<Identifier <<` without consuming tokens
        let has_shift_left = (|| {
            // find the identifier after the current position
            let identifier_index = self.next_non_newline_index_from(self.pos_index() + 1);
            let identifier_token = self.token_ref_at(identifier_index)?;
            if identifier_token.token.ty != TokenType::Identifier {
                return Some(false);
            }

            // find the operator after the identifier
            let after_identifier = self.next_non_newline_index_from(identifier_index + 1);
            let after_token = self.token_ref_at(after_identifier)?;
            Some(matches!(
                after_token.token.ty,
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft
            ))
        })()
        .unwrap_or(false);

        // restore parser state
        self.rewind(mark);

        has_shift_left
    }

    /// Return true if the current `do` token starts a do-while statement.
    /// Destack requires `do { ... } while ...`, while JS/TS allow `do` with any statement.
    pub(super) fn is_do_while_statement(&mut self, next_token_type: TokenType) -> bool {
        // allow JS/TS do while forms
        if !self.language.is_destack() {
            return true;
        }

        // destack requires a block after do
        if next_token_type != TokenType::OpenBrace {
            return false;
        }

        // locate the matching close brace
        let open_index = self.index_for_next();
        let matching = match self.find_matching_close(
            Some(open_index as u32),
            TokenType::OpenBrace,
            TokenType::CloseBrace,
        ) {
            Ok(pos) => pos,
            Err(_) => return false,
        };

        // skip newlines after the block
        let after_close = match self.skip_newlines(matching) {
            Ok(pos) => pos,
            Err(_) => return false,
        };

        // check for a trailing while keyword
        let after_close_index = after_close as usize + 1;
        self.ensure_token(after_close_index);
        let Some(after_token) = self.tokens().get(after_close_index) else {
            return false;
        };
        if after_token.token.ty != TokenType::Identifier {
            return false;
        }

        self.keyword_for_index(after_close_index) == Some(Keyword::While)
    }
}
