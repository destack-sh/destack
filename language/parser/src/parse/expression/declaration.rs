use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Ambientness, Asynchrony, DependencyMode, ExportMode, Expression, Keyword, LiteralType,
    TokenType,
};

use super::super::PendingDecorators;
use super::common::{
    DECLARATION_START_TOKENS, DeclarationHeader, DescriptorHead, is_declaration_keyword,
    is_type_relation_keyword,
};

impl Parser {
    /// Return true when declaration modifier parsing is needed in this context.
    #[inline]
    pub(super) fn should_parse_declaration_descriptor(&mut self) -> bool {
        if self.options.is_in_type()
            || self.options.is_in_variant()
            || self.options.is_in_declare_context()
            || self.language.is_declaration()
        {
            return true;
        }

        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        let keyword = self.keyword_for_index(self.pos_index());
        if matches!(
            keyword,
            Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static)
        ) {
            return true;
        }

        // contextual global and module declarations in statement position
        if !self.options.is_in_statement_position() {
            return false;
        }

        let next_token_type = self.peek_next_token_type();
        let can_start_global_or_module_declaration = matches!(
            next_token_type,
            TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal | TokenType::Newline
        );
        if !can_start_global_or_module_declaration {
            return false;
        }

        if self.is_global_identifier_at(self.pos_index()) {
            return true;
        }

        self.is_module_identifier_at(self.pos_index())
    }

    /// Check whether a `{` in statement position should be parsed as an object literal.
    /// NOTE #Cleanup: can_parse_object_literal_in_statement_position is ugly and might not be fixable.
    pub(super) fn can_parse_object_literal_in_statement_position(&mut self) -> bool {
        // only allow this in destack files
        if !self.language.is_destack() {
            return false;
        }

        // avoid object literals when a block is expected
        if self.options.is_in_before_block() {
            return false;
        }

        // skip newlines after the opening brace
        let open_pos = match self.skip_newlines(self.pos()) {
            Ok(pos) => pos,
            Err(_) => return false,
        };
        self.ensure_token(open_pos as usize + 1);
        let Some(next_token) = self.tokens().get(open_pos as usize + 1) else {
            return false;
        };

        // spread property start
        if next_token.token.ty == TokenType::Spread {
            return true;
        }

        // computed key start: require a clear property marker after the closing bracket
        if next_token.token.ty == TokenType::OpenBracket {
            let open_bracket_pos = open_pos + 1;
            let Some(close_bracket_pos) = self.matching_pair_or_lex(open_bracket_pos as usize)
            else {
                return false;
            };
            let close_bracket_pos = close_bracket_pos as u32;

            let after_close_pos = match self.skip_newlines(close_bracket_pos) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            self.ensure_token(after_close_pos as usize + 1);
            let Some(after_close) = self.tokens().get(after_close_pos as usize + 1) else {
                return false;
            };

            return matches!(after_close.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        // identifier or literal key with an explicit value marker
        if next_token.token.ty == TokenType::Identifier || next_token.token.ty == TokenType::Literal
        {
            // only allow string or number literal keys
            if next_token.token.ty == TokenType::Literal {
                let literal = next_token.token.literal;
                let is_key_literal = matches!(
                    literal,
                    Some(
                        LiteralType::String { .. }
                            | LiteralType::Int { .. }
                            | LiteralType::Float { .. }
                    )
                );
                if !is_key_literal {
                    return false;
                }
            }

            // check for a colon or optional marker after the key
            let key_pos = match self.skip_newlines(open_pos + 1) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            self.ensure_token(key_pos as usize + 1);
            let Some(after_key) = self.tokens().get(key_pos as usize + 1) else {
                return false;
            };

            return matches!(after_key.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        false
    }

    /// Return true when tokens can plausibly start a using declarator.
    fn can_start_using_declarator(&mut self, asynchrony: Asynchrony) -> bool {
        // resolve the using keyword at the current position
        let Some(using_index) = self.using_keyword_index(asynchrony) else {
            return false;
        };

        // keep declarators on the same line as `using`
        let Some(declarator_cursor) = self.using_binding_head_cursor(using_index) else {
            return false;
        };

        // using declarations require lexical binding heads
        self.token_can_start_using_binding_pattern(declarator_cursor.token_type)
    }

    /// Return true when a using declarator has a required initializer.
    fn using_declarator_has_required_initializer(&mut self, declarator_index: usize) -> bool {
        let mut index = declarator_index;

        loop {
            // read the next significant token in the declarator
            let token_type = self.token_type_at(index);

            // skip nested groups by jumping to their cached close token
            if matches!(
                token_type,
                TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
            ) && let Some(close_index) = self.matching_pair_or_lex(index)
                && close_index > index
            {
                let next_index = self.next_non_newline_index_from(close_index + 1);
                if next_index <= index {
                    return false;
                }
                index = next_index;
                continue;
            }

            // using declarators require a top level initializer
            if token_type == TokenType::Assign {
                return true;
            }

            // stop once the declarator ends before an initializer
            if matches!(
                token_type,
                TokenType::Comma
                    | TokenType::Semicolon
                    | TokenType::End
                    | TokenType::CloseBrace
                    | TokenType::CloseParenthesis
            ) {
                return false;
            }

            // continue scanning across newline trivia
            let next_index = self.next_non_newline_index_from(index.saturating_add(1));
            if next_index <= index {
                return false;
            }
            index = next_index;
        }
    }

    /// Check whether a using declaration can be parsed at the current position.
    pub(super) fn can_parse_using_declaration(
        &mut self,
        _header: &DeclarationHeader,
        asynchrony: Asynchrony,
    ) -> bool {
        // reject impossible using starts with scanner-level checks
        if !self.can_start_using_declarator(asynchrony) {
            return false;
        }

        let Some(using_index) = self.using_keyword_index(asynchrony) else {
            return false;
        };
        let Some(declarator_cursor) = self.using_binding_head_cursor(using_index) else {
            return false;
        };

        self.using_declarator_has_required_initializer(declarator_cursor.index)
    }

    /// Eat declaration modifiers and return a descriptor or a parsed expression.
    pub(super) fn eat_declaration_descriptor(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<DescriptorHead> {
        let mut header: DeclarationHeader = DeclarationHeader::default();
        let mut decorators = PendingDecorators::new();

        // decorators parse as expressions only
        if self.options.is_in_decorator() {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // declaration modifiers only start on identifiers
        if !self.peek_is(TokenType::Identifier) {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // check for a modifier keyword or a global or module identifier
        let pos = self.pos_index();
        let next_token_type = self.peek_next_token_type();
        let can_start_global_or_module_declaration = matches!(
            next_token_type,
            TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal | TokenType::Newline
        );
        let keyword = self.keyword_for_index(pos);
        let is_modifier_keyword = matches!(
            keyword,
            Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static)
        );
        let is_global_identifier = !is_modifier_keyword
            && can_start_global_or_module_declaration
            && self.is_global_identifier_at(pos);
        let is_module_identifier = !is_modifier_keyword
            && can_start_global_or_module_declaration
            && self.is_module_identifier_at(pos);

        if !is_modifier_keyword && !is_global_identifier && !is_module_identifier {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // export modifier
        if self.is_keyword(Keyword::Export) {
            self.bump(); // eat export
            let export_mode = if self.is_keyword(Keyword::Default) {
                self.bump(); // eat default
                Some(DependencyMode::Default)
            } else if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                Some(DependencyMode::Namespace)
            } else {
                Some(DependencyMode::Item)
            };

            // export namespace handled by export statement parsing
            let is_export_namespace =
                self.is_keyword(Keyword::As) && self.is_next_keyword(Keyword::Namespace);
            if is_export_namespace {
                self.rewind(start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            // export dependencies handled by export statement parsing
            let next_keyword = self.peek_any_keyword().ok();
            let next_non_newline_index = self.next_non_newline_index_from(self.pos_index());
            let next_non_newline_token_type = self.token_type_at(next_non_newline_index);
            let next_keyword_after_newlines =
                if next_non_newline_token_type == TokenType::Identifier {
                    self.keyword_for_index(next_non_newline_index)
                } else {
                    None
                };
            let has_module_identifier_declaration = self.is_module_identifier_at(self.pos_index());
            let has_decorator_declaration_head = next_non_newline_token_type == TokenType::At
                && export_mode != Some(DependencyMode::Default);
            let has_declaration_keyword = next_keyword.is_some_and(is_declaration_keyword)
                || next_keyword_after_newlines.is_some_and(is_declaration_keyword)
                || has_decorator_declaration_head
                || has_module_identifier_declaration;

            // reject export default enum declarations
            if export_mode == Some(DependencyMode::Default) && self.is_keyword(Keyword::Enum) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let is_export_type_binding = self.is_keyword(Keyword::Type)
                && (self.peek_next_is(TokenType::OpenBrace)
                    || self.peek_next_is(TokenType::Multiply)
                    || self.peek_next_is(TokenType::Semicolon)
                    || self.peek_next_is(TokenType::Newline)
                    || self.peek_next_is(TokenType::End));
            let is_invalid_export_form = !has_declaration_keyword
                && !self.peek_is(TokenType::At)
                && !self.peek_dependency_binding_is()
                && !self.is_keyword_after_newlines(Keyword::Import);
            let is_export_dependency = export_mode == Some(DependencyMode::Namespace)
                || is_export_type_binding
                || (!has_declaration_keyword && self.peek_dependency_binding_is())
                || (export_mode == Some(DependencyMode::Default) && !has_declaration_keyword)
                || is_invalid_export_form;
            if is_export_dependency {
                self.rewind(start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            header.export = match export_mode {
                Some(DependencyMode::Default) => Some(ExportMode::Default),
                Some(DependencyMode::Item) => Some(ExportMode::Named),
                Some(DependencyMode::Namespace) | None => None,
            };

            // parse decorators after export so descriptor modifiers still parse correctly
            if self.peek_is(TokenType::At) {
                let export_decorators = self.eat_decorators_maybe()?;
                decorators.extend(export_decorators);
            }
        }

        // skip newlines after export before declaration-style heads
        if header.export.is_some() && self.peek_is(TokenType::Newline) {
            let next_index = self.next_non_newline_index_from(self.pos_index());
            let next_token_type = self.token_type_at(next_index);
            let next_keyword = if next_token_type == TokenType::Identifier {
                self.keyword_for_index(next_index)
            } else {
                None
            };
            let is_after_export_import_equals_head = next_keyword == Some(Keyword::Import);
            let is_after_export_declaration_head = next_keyword.is_some_and(is_declaration_keyword)
                || next_token_type == TokenType::At
                || self.is_module_identifier_at(next_index)
                || self.is_global_identifier_at(next_index);
            if is_after_export_import_equals_head || is_after_export_declaration_head {
                self.eat_newlines_maybe()?;

                // parse decorators after export when they follow skipped newlines
                if self.peek_is(TokenType::At) {
                    let mut export_decorators = self.eat_decorators_maybe()?;
                    decorators.append(&mut export_decorators);
                }
            }
        }

        // declare modifier
        let is_declare = self.is_keyword(Keyword::Declare);
        let direct_index = self.pos_index() + 1;

        // locate a declare target
        let declare_target_index = if is_declare {
            if self.is_declare_target_at(direct_index) {
                Some(direct_index)
            } else if self.keyword_for_index(direct_index) == Some(Keyword::Abstract) {
                let after_abstract = direct_index + 1;
                let target_index = self.next_non_newline_index_from(after_abstract);
                if target_index == after_abstract && self.is_declare_target_at(target_index) {
                    Some(target_index)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // report newline errors for declare forms that must be contiguous
        let declare_newline_error_span =
            if is_declare && self.keyword_for_index(direct_index) == Some(Keyword::Abstract) {
                let after_abstract = direct_index + 1;
                let target_index = self.next_non_newline_index_from(after_abstract);
                if target_index > after_abstract && self.is_declare_target_at(target_index) {
                    self.ensure_token(after_abstract);
                    self.tokens().get(after_abstract).map(|token| token.span)
                } else {
                    None
                }
            } else if is_declare && self.keyword_for_index(direct_index) == Some(Keyword::Type) {
                let after_type = direct_index + 1;
                let name_index = self.next_non_newline_index_from(after_type);
                if name_index > after_type
                    && self
                        .tokens()
                        .get(name_index)
                        .is_some_and(|token| token.token.ty == TokenType::Identifier)
                {
                    self.ensure_token(after_type);
                    self.tokens().get(after_type).map(|token| token.span)
                } else {
                    None
                }
            } else {
                None
            };

        if let Some(span) = declare_newline_error_span {
            let error = ParseError::unexpected(span);
            self.error(&error);
        }

        let declare_has_target = declare_target_index.is_some();
        header.ambient = if is_declare && declare_has_target {
            self.bump(); // eat declare
            Ambientness::Ambient
        } else {
            Ambientness::Concrete
        };

        // abstraction modifier
        header.is_abstract = self.is_keyword(Keyword::Abstract)
            && !self.options.is_in_variant()
            && !self.peek_next_is(TokenType::Newline)
            && self
                .peek_next_any_keyword()
                .is_ok_and(is_declaration_keyword);
        if header.is_abstract {
            self.bump(); // eat abstract
        }

        // global declaration
        if (header.ambient == Ambientness::Ambient
            || self.language.is_declaration()
            || self.options.is_in_declare_context())
            && self.is_global_identifier_at(self.pos_index())
            && self.is_token_after_newlines(self.pos(), TokenType::OpenBrace)
        {
            let mut global_header = header;
            global_header.ambient = Ambientness::Ambient;
            let global_id = self.eat_global(start, global_header)?;
            let expression_id = self.insert_node(
                Expression::Declaration(global_id),
                self.get_span_from(start),
            );
            return Ok(DescriptorHead::Expression(expression_id));
        }

        Ok(DescriptorHead::Header { header, decorators })
    }

    /// Check whether a token index starts a declare target keyword.
    pub(super) fn is_declare_keyword_target_at(&mut self, index: usize) -> bool {
        let Some(keyword) = self.keyword_for_index(index) else {
            return false;
        };

        if keyword == Keyword::Declare || !is_declaration_keyword(keyword) {
            return false;
        }

        let next_cursor = self.scanner_cursor_from(index + 1);
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;
        let next_has_line_break = next_cursor.has_line_break_before;
        let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);

        match keyword {
            // ambient async heads only exist for `async function`
            Keyword::Async => {
                !next_has_line_break
                    && next_token_type == TokenType::Identifier
                    && self.keyword_for_index(next_token_index) == Some(Keyword::Function)
            }

            // ambient const, let, and var declarations commit immediately
            Keyword::Const | Keyword::Let | Keyword::Var => true,

            // ambient nominal and structural declarations keep their existing heads
            Keyword::Class
            | Keyword::Function
            | Keyword::Interface
            | Keyword::Struct
            | Keyword::Extension
            | Keyword::Union
            | Keyword::Newtype => true,

            // enum declarations must stay on the same line as the head keyword
            Keyword::Enum => is_declaration_start && !next_has_line_break,

            // namespace declarations only accept identifier names
            Keyword::Namespace => {
                !next_has_line_break
                    && next_token_type == TokenType::Identifier
                    && !is_type_relation_keyword(self.keyword_for_index(next_token_index))
            }

            // type aliases require a contiguous identifier name
            Keyword::Type => !next_has_line_break && next_token_type == TokenType::Identifier,

            // the remaining declaration keywords are contextual modifiers, not ambient heads
            _ => false,
        }
    }

    /// Check whether a token index starts a declare identifier target.
    pub(super) fn is_declare_identifier_at(&mut self, index: usize) -> bool {
        self.ensure_token(index);
        let Some(token) = self.tokens().get(index) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }
        self.is_global_identifier_at(index) || self.is_module_identifier_at(index)
    }

    /// Check whether a token index starts a declare await using target.
    pub(super) fn is_declare_await_using_at(&mut self, index: usize) -> bool {
        if self.keyword_for_index(index) != Some(Keyword::Await) {
            return false;
        }
        let mut after = index + 1;
        loop {
            self.ensure_token(after);
            let Some(token) = self.tokens().get(after) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            after += 1;
        }
        self.keyword_for_index(after) == Some(Keyword::Using)
    }

    /// Check whether a token index starts a declare target.
    pub(super) fn is_declare_target_at(&mut self, index: usize) -> bool {
        self.is_declare_keyword_target_at(index)
            || self.is_declare_identifier_at(index)
            || self.is_declare_await_using_at(index)
    }
}
