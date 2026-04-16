use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Keyword, LiteralType, Path, TokenType};

impl Parser {
    /// Return the end index after one tree literal starting at `start_index`.
    pub(crate) fn tree_literal_end_index_at(&mut self, start_index: usize) -> Option<usize> {
        if !self.language.supports_jsx() {
            return None;
        }

        let saved_pos = self.pos_index();
        self.advance_to(start_index);

        let ambient_context = self.options.with_tree_literal(true);
        let expression_context = self.options.not_in_position();
        let scan_options = self
            .options
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context);
        let old_options = self.swap_options(scan_options);
        let result = self
            .skip_tree_literal_structure()
            .ok()
            .map(|_| self.pos_index());
        self.restore_options(old_options);

        self.advance_to(saved_pos);

        result
    }

    /// Skip one tree literal without building AST nodes.
    fn skip_tree_literal_structure(&mut self) -> ParseResult<()> {
        self.skip_tree_literal_structure_with_child_context(false)
    }

    /// Skip one tree literal and optionally re-enter tree child lexing after the close.
    fn skip_tree_literal_structure_with_child_context(
        &mut self,
        in_tree_child: bool,
    ) -> ParseResult<()> {
        self.eat_tree_opening_angle()?;
        self.eat_newlines_maybe()?;

        // fragment
        if self.peek_starts_tree_tag_close() {
            return self.skip_tree_fragment(in_tree_child);
        }

        // element
        if self.peek_is(TokenType::Identifier) {
            return self.skip_tree_element(in_tree_child);
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Skip one tree fragment after `<`.
    fn skip_tree_fragment(&mut self, in_tree_child: bool) -> ParseResult<()> {
        self.eat_tree_tag_close(true)?;
        self.skip_tree_children_and_closing(None, in_tree_child)
    }

    /// Skip one tree element after `<`.
    fn skip_tree_element(&mut self, in_tree_child: bool) -> ParseResult<()> {
        let (path, is_self_closing) = self.skip_tree_opening_element(in_tree_child)?;
        if is_self_closing {
            return Ok(());
        }

        self.skip_tree_children_and_closing(Some(path), in_tree_child)
    }

    /// Skip one tree opening element after `<`.
    fn skip_tree_opening_element(&mut self, in_tree_child: bool) -> ParseResult<(Path, bool)> {
        let path = self.eat_tree_literal_path()?;
        if self.tree_literal_path_has_namespace_member(&path) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        self.eat_newlines_maybe()?;

        // generic arguments
        if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
            self.skip_tree_generic_arguments()?;
        }
        self.eat_newlines_maybe()?;

        // attributes
        while self.has_more_tokens() {
            self.skip_tree_whitespace()?;
            if self.peek_is(TokenType::Divide) || self.peek_starts_tree_tag_close() {
                break;
            }

            self.skip_tree_literal_header_argument()?;
            self.eat_newlines_maybe()?;
        }
        self.eat_newlines_maybe()?;

        // self closing
        let is_self_closing = self.peek_is(TokenType::Divide);
        if is_self_closing {
            self.bump();
        }

        // closing angle
        if !is_self_closing || in_tree_child {
            self.eat_tree_tag_close(true)?;
        } else {
            self.eat_tree_tag_close(false)?;
        }

        Ok((path, is_self_closing))
    }

    /// Skip tree children until one matching closing fragment or element.
    fn skip_tree_children_and_closing(
        &mut self,
        path: Option<Path>,
        in_tree_child: bool,
    ) -> ParseResult<()> {
        loop {
            self.skip_tree_whitespace_with_child_context(true)?;

            // closing inline
            if self.peek_is(TokenType::LessThan) {
                let closing_mark = self.mark_rewind();
                self.bump();
                self.eat_newlines_maybe()?;

                if self.peek_is(TokenType::Divide) {
                    self.bump();
                    return self.skip_tree_closing_inline(path.as_ref(), in_tree_child);
                }

                self.rewind(closing_mark);
            }

            // child item
            self.skip_tree_child_structure()?;
        }
    }

    /// Skip one inline closing fragment or element after `</`.
    fn skip_tree_closing_inline(
        &mut self,
        path: Option<&Path>,
        in_tree_child: bool,
    ) -> ParseResult<()> {
        self.eat_newlines_maybe()?;

        // closing fragment
        if self.peek_starts_tree_tag_close() {
            if path.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            self.eat_tree_tag_close(in_tree_child)?;
            return Ok(());
        }

        // closing element
        let Some(path) = path else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };

        let closing_path = self.eat_tree_literal_path()?;
        if self.tree_literal_path_has_namespace_member(&closing_path) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.skip_tree_whitespace()?;
        self.eat_tree_tag_close(in_tree_child)?;

        if &closing_path == path {
            return Ok(());
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Skip one tree child item.
    fn skip_tree_child_structure(&mut self) -> ParseResult<()> {
        let token = *self.peek()?;

        // expression container
        if token.token.ty == TokenType::OpenBrace {
            self.skip_tree_expression_container(true)?;
            return Ok(());
        }

        // nested tree literal
        if token.token.ty == TokenType::LessThan {
            self.skip_tree_literal_structure_with_child_context(true)?;
            return Ok(());
        }

        // tree text and entities
        if token.token.ty == TokenType::Literal
            && matches!(
                token.token.literal,
                Some(LiteralType::TreeString)
                    | Some(LiteralType::Character {
                        is_html_entity: true,
                        ..
                    })
            )
        {
            self.bump_tree_child();
            return Ok(());
        }

        Err(ParseError::unexpected(token.span))
    }

    /// Skip one tree header argument.
    fn skip_tree_literal_header_argument(&mut self) -> ParseResult<()> {
        // spread container
        if self.starts_tree_spread_attribute() {
            self.skip_tree_expression_container(false)?;
            return Ok(());
        }

        // named argument or flag
        if self.peek_is(TokenType::Identifier) {
            self.eat_tree_literal_identifier()?;

            let has_value_separator = self.tree_literal_argument_has_value_separator();

            if !has_value_separator {
                return Ok(());
            }

            self.set_tree_attribute_value(true);
            self.eat_newlines_maybe()?;
            self.bump();
            self.eat_newlines_maybe()?;

            // expression container value
            if self.peek_is(TokenType::OpenBrace) {
                self.skip_tree_expression_container(false)?;
                return Ok(());
            }

            // string literal value
            if self.peek_string_literal_is() {
                self.bump();
                return Ok(());
            }

            // array literal value
            if self.peek_is(TokenType::OpenBracket) {
                self.skip_expression_balanced_delimiter(
                    TokenType::OpenBracket,
                    TokenType::CloseBracket,
                )?;
                return Ok(());
            }

            // nested tree literal value
            if self.peek_is(TokenType::LessThan) {
                self.skip_tree_literal_structure()?;
                return Ok(());
            }

            return Err(ParseError::unexpected(self.peek()?.span));
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Skip a balanced expression delimiter in non-child mode.
    fn skip_expression_balanced_delimiter(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParseResult<()> {
        let open_pos = self.pos();
        let close_pos =
            self.find_matching_close_in_expression(open_pos, open_token, close_token)?;
        self.advance_to(close_pos as usize + 1);

        Ok(())
    }

    /// Skip one `{...}` expression container.
    fn skip_tree_expression_container(&mut self, in_tree_child: bool) -> ParseResult<()> {
        let open_pos = self.pos();
        self.bump();
        self.eat_newlines_maybe()?;

        // empty container
        if self.peek_is(TokenType::CloseBrace) {
            if in_tree_child {
                self.expect_tree_child(TokenType::CloseBrace)?;
            } else {
                self.eat_token(TokenType::CloseBrace)?;
            }

            return Ok(());
        }

        // inner expression
        let close_pos = self.find_matching_close_in_expression(
            open_pos,
            TokenType::OpenBrace,
            TokenType::CloseBrace,
        )?;
        self.advance_to(close_pos as usize);

        // closing brace
        if in_tree_child {
            self.expect_tree_child(TokenType::CloseBrace)?;
        } else {
            self.eat_token(TokenType::CloseBrace)?;
        }

        Ok(())
    }

    /// Skip one generic argument list on a tree head.
    fn skip_tree_generic_arguments(&mut self) -> ParseResult<()> {
        // opening angle
        if self.peek_is(TokenType::LessThan) {
            self.bump();
        } else if self.peek_is(TokenType::ShiftLeft) {
            if !self.re_lex_generic_l_angle() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            self.bump();
        } else {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        self.eat_newlines_maybe()?;

        // track nested type angles
        let mut angle_depth = 1usize;
        while self.has_more_tokens() {
            let token_type = self.peek_token_type();
            let close_width = Self::expression_r_angle_width(token_type);
            if close_width > 0 {
                if angle_depth <= close_width {
                    break;
                }

                angle_depth -= close_width;
                self.bump();
                continue;
            }

            let open_width = match token_type {
                TokenType::LessThan => 1,
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => 2,
                _ => 0,
            };
            if open_width > 0 {
                angle_depth += open_width;
            }

            self.bump();
        }

        if angle_depth == 0 || !self.peek_starts_expression_type_angle_close() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.eat_expression_type_angle_close()
    }

    /// Return the number of `>` closers at the start of one expression token.
    const fn expression_r_angle_width(token_type: TokenType) -> usize {
        match token_type {
            TokenType::GreaterThan => 1,
            TokenType::ShiftRight => 2,
            TokenType::UnsignedShiftRight => 3,
            _ => 0,
        }
    }

    /// Peek a tree literal with value-expression position rules.
    fn peek_tree_literal_in_value_position(&mut self) -> bool {
        self.with_options(self.options.not_in_position(), |parser| {
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
            self.options.is_disallow_ambiguous_tree_literal() && !self.options.is_in_type();
        self.peek_generic_arrow_after_type_parameters(require_disambiguator)
    }

    /// Return true if `<` starts a tree literal without committing tokens.
    pub(crate) fn can_start_tree_literal(&mut self) -> bool {
        if !self.language.supports_jsx() || self.options.is_in_type() {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.has_shift_left_tree_generic_arguments() {
            return self.peek_tree_literal_in_value_position();
        }

        let mark = self.mark_rewind();
        let require_disambiguator = self.options.is_disallow_ambiguous_tree_literal();
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
        let cursor = self.scanner_cursor_from(self.pos_index());
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
            let ambient_context = parser.options.with_type(false);
            let expression_context = parser.options.not_in_position();
            parser.with_options(
                parser
                    .options
                    .with_ambient_context(ambient_context)
                    .with_expression_context(expression_context),
                |parser| parser.can_start_tree_literal(),
            )
        })
    }

    /// Return true when `<Identifier <<` starts tree generic arguments.
    pub(super) fn has_shift_left_tree_generic_arguments(&mut self) -> bool {
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

        // locate the matching close brace
        let open_index = self.index_for_next();
        let matching = match self.find_matching_close_maybe(
            Some(open_index as u32),
            TokenType::OpenBrace,
            TokenType::CloseBrace,
        ) {
            Some(pos) => pos,
            None => return false,
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
