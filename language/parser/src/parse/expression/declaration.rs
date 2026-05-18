use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    Asynchrony, DependencyBinding, ExportKind, Expression, Keyword, TokenLiteral, TokenType,
};

use super::super::PendingDecorators;
use super::common::{
    DECLARATION_START_TOKENS, DeclarationHeader, DescriptorHead, is_declaration_keyword,
};

impl Parser {
    /// Return true when the current identifier can start a global declaration.
    fn can_start_global_declaration(&mut self) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        if !self.is_global_identifier() {
            return false;
        }

        let next_token = self.next_token();
        if next_token.token.is_on_new_line {
            return false;
        }

        let can_parse_global = self.language.is_destack()
            || self.language.is_declaration()
            || self.flags.is_in_declare_context();

        can_parse_global && next_token.token.ty == TokenType::OpenBrace
    }

    /// Return true when the current identifier can start a module declaration.
    fn can_start_module_declaration(&mut self) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        if !self.language.is_destack() || !self.is_module_identifier() {
            return false;
        }

        let next_token = self.next_token();
        if next_token.token.is_on_new_line {
            return false;
        }

        next_token.token.ty == TokenType::OpenBrace
    }

    /// Return true when declaration modifier parsing is needed in this context.
    #[inline]
    pub(crate) fn should_parse_declaration_descriptor(&mut self) -> bool {
        if self.flags.is_in_type()
            || self.flags.is_in_variant()
            || self.flags.is_in_declare_context()
            || self.language.is_declaration()
        {
            return true;
        }

        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        let keyword = self.current_keyword();
        if matches!(
            keyword,
            Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static,)
        ) {
            return true;
        }
        if self.language.is_destack() && keyword == Some(Keyword::Final) {
            return true;
        }
        if self.language.is_destack() && keyword == Some(Keyword::Shared) {
            return true;
        }

        // recognize contextual declaration heads
        self.can_start_global_declaration() || self.can_start_module_declaration()
    }

    /// Decide whether one `{` in statement position starts an object literal.
    ///
    /// Examples:
    /// ```
    /// { value: 1 }
    /// { [key]: value }
    /// { ...spread }
    /// ```
    pub(crate) fn can_parse_object_literal_in_statement_position(&mut self) -> bool {
        // only allow this when statement-position object literals are enabled
        if !self.language.is_destack() {
            return false;
        }

        // avoid object literals when a block is expected
        if self.flags.is_in_before_block() {
            return false;
        }

        let next_token = self.next_token();

        // spread property start
        if next_token.token.ty == TokenType::Spread {
            return true;
        }

        // computed key start: require a clear property marker after the closing bracket
        if next_token.token.ty == TokenType::OpenBracket {
            return self.lookahead(|parser| {
                parser.bump();
                let Some(close_bracket_span) = parser
                    .find_matching_close_maybe(TokenType::OpenBracket, TokenType::CloseBracket)
                else {
                    return false;
                };

                while parser.current_token().span.start <= close_bracket_span.start {
                    parser.bump();
                }

                matches!(
                    parser.peek_token_type(),
                    TokenType::Colon | TokenType::Maybe
                )
            });
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
                        TokenLiteral::String { .. }
                            | TokenLiteral::Int { .. }
                            | TokenLiteral::Float { .. }
                    )
                );
                if !is_key_literal {
                    return false;
                }
            }

            let after_key_token_type = self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.peek_token_type()
            });
            return matches!(after_key_token_type, TokenType::Colon | TokenType::Maybe);
        }

        false
    }

    /// Return true when tokens can plausibly start a using declarator.
    fn can_start_using_declarator(&mut self, asynchrony: Asynchrony) -> bool {
        let Some(declarator_token_type) = self.using_binding_head_token(asynchrony) else {
            return false;
        };

        // using declarations require lexical binding heads
        self.token_can_start_using_binding_pattern(declarator_token_type)
    }

    /// Return true when a using declarator has a required initializer.
    fn using_declarator_has_required_initializer(&mut self, asynchrony: Asynchrony) -> bool {
        self.lookahead(|parser| {
            if asynchrony == Asynchrony::Async {
                parser.bump();
            }
            if !parser.is_keyword(Keyword::Using) {
                return false;
            }

            parser.bump();
            if parser.current_token().token.is_on_new_line {
                return false;
            }

            let mut depth = 0_u32;
            loop {
                let token_type = parser.peek_token_type();

                if depth == 0 && token_type == TokenType::Assign {
                    return true;
                }

                if depth == 0
                    && matches!(
                        token_type,
                        TokenType::Comma
                            | TokenType::Semicolon
                            | TokenType::End
                            | TokenType::CloseBrace
                            | TokenType::CloseParenthesis
                    )
                {
                    return false;
                }

                match token_type {
                    TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket => {
                        depth += 1;
                    }
                    TokenType::CloseParenthesis
                    | TokenType::CloseBrace
                    | TokenType::CloseBracket => {
                        depth = depth.saturating_sub(1);
                    }
                    TokenType::End => return false,
                    _ => {}
                }

                parser.bump();
            }
        })
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

        self.using_declarator_has_required_initializer(asynchrony)
    }

    /// Eat declaration modifiers and return a descriptor or a parsed expression.
    pub(super) fn eat_declaration_descriptor(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<DescriptorHead> {
        let descriptor_start = self.cursor_checkpoint();
        let mut header: DeclarationHeader = DeclarationHeader::default();
        let mut decorators = PendingDecorators::new();

        // decorators parse as expressions only
        if self.flags.is_in_decorator() {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // declaration modifiers only start on identifiers
        if !self.peek_is(TokenType::Identifier) {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // check for a modifier keyword or contextual declaration identifier
        let keyword = self.current_keyword();
        let is_modifier_keyword = matches!(
            keyword,
            Some(
                Keyword::Export
                    | Keyword::Declare
                    | Keyword::Abstract
                    | Keyword::Shared
                    | Keyword::Static,
            )
        );
        let is_contextual_declaration_identifier = !is_modifier_keyword
            && (self.can_start_global_declaration() || self.can_start_module_declaration());
        let is_global_identifier =
            is_contextual_declaration_identifier && self.is_global_identifier();
        let is_module_identifier =
            is_contextual_declaration_identifier && self.is_module_identifier();

        if !is_modifier_keyword && !is_global_identifier && !is_module_identifier {
            return Ok(DescriptorHead::Header { header, decorators });
        }

        // export modifier
        if self.is_keyword(Keyword::Export) {
            self.bump(); // eat export
            let export_mode = if self.is_keyword(Keyword::Default) {
                self.bump(); // eat default
                Some(DependencyBinding::Default)
            } else if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                Some(DependencyBinding::Namespace)
            } else {
                Some(DependencyBinding::Named)
            };

            // export dependencies handled by export statement parsing
            let current_keyword = self.current_keyword();
            let current_token_type = self.peek_token_type();
            let has_decorator_declaration_head = current_token_type == TokenType::At
                && export_mode != Some(DependencyBinding::Default);
            let has_declaration_keyword = current_keyword.is_some_and(is_declaration_keyword)
                || has_decorator_declaration_head;

            // reject export default enum declarations
            if export_mode == Some(DependencyBinding::Default) && self.is_keyword(Keyword::Enum) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let is_export_type_binding = self.is_keyword(Keyword::Type)
                && (self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::OpenBrace)
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Multiply)
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Semicolon)
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.current_token_is_on_new_line()
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::End)
                }));
            let is_invalid_export_form = !has_declaration_keyword
                && !self.peek_is(TokenType::At)
                && !self.peek_dependency_binding_is()
                && !self.is_keyword(Keyword::Import);
            let is_export_dependency = export_mode == Some(DependencyBinding::Namespace)
                || is_export_type_binding
                || (!has_declaration_keyword && self.peek_dependency_binding_is())
                || (export_mode == Some(DependencyBinding::Default) && !has_declaration_keyword)
                || is_invalid_export_form;
            if is_export_dependency {
                self.rewind(descriptor_start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            header.export = match export_mode {
                Some(DependencyBinding::Default) => Some(ExportKind::Default),
                Some(DependencyBinding::Named) => Some(ExportKind::Named),
                Some(DependencyBinding::Namespace) | None => None,
            };

            // parse decorators after export so descriptor modifiers still parse correctly
            if self.peek_is(TokenType::At) {
                let export_decorators = self.eat_decorators_maybe()?;
                decorators.extend(export_decorators);
            }
        }

        // skip newlines after export before declaration-style heads
        if header.export.is_some() && self.current_token_is_on_new_line() {
            let next_token_type = self.peek_token_type();
            let next_keyword = self.current_keyword();
            let is_after_export_declaration_head = next_keyword.is_some_and(is_declaration_keyword)
                || next_token_type == TokenType::At
                || self.is_global_identifier();
            if is_after_export_declaration_head {
                // parse decorators after export when they follow skipped newlines
                if self.peek_is(TokenType::At) {
                    let mut export_decorators = self.eat_decorators_maybe()?;
                    decorators.append(&mut export_decorators);
                }
            }
        }

        // declare modifier
        let is_declare = self.is_keyword(Keyword::Declare);

        // locate a declare target
        let declare_has_target = is_declare
            && self.lookahead(|parser| {
                parser.bump();
                if parser.current_token_starts_declare_target() {
                    true
                } else if parser.current_keyword() == Some(Keyword::Abstract) {
                    parser.bump();
                    !parser.current_token_is_on_new_line()
                        && parser.current_token_starts_declare_target()
                } else {
                    false
                }
            });

        // report newline errors for declare forms that must be contiguous
        let declare_newline_error_span = is_declare
            .then(|| {
                self.lookahead(|parser| {
                    parser.bump();
                    if parser.current_keyword() == Some(Keyword::Abstract) {
                        parser.bump();
                        if parser.current_token_is_on_new_line()
                            && parser.current_token_starts_declare_target()
                        {
                            return Some(parser.current_token().span);
                        }
                    } else if parser.current_keyword() == Some(Keyword::Type) {
                        parser.bump();
                        if parser.current_token_is_on_new_line()
                            && parser.peek_is(TokenType::Identifier)
                        {
                            return Some(parser.current_token().span);
                        }
                    }

                    None
                })
            })
            .flatten();

        if let Some(span) = declare_newline_error_span {
            let error = ParseError::unexpected(span);
            self.error(&error);
        }

        (header.is_ambient, header.declare_span) = if is_declare && declare_has_target {
            let declare_span = self.peek()?.span;
            self.bump(); // eat declare
            (true, Some(declare_span))
        } else {
            (false, None)
        };

        // abstraction modifier
        header.is_abstract = self.is_keyword(Keyword::Abstract)
            && !self.flags.is_in_variant()
            && !self.lookahead(|parser| {
                parser.bump();
                parser.current_token_is_on_new_line()
            })
            && self
                .peek_next_any_keyword()
                .is_ok_and(is_declaration_keyword);
        if header.is_abstract {
            self.bump(); // eat abstract
        }

        // final modifier
        header.is_final = self.language.is_destack()
            && self.is_keyword(Keyword::Final)
            && !self.flags.is_in_variant()
            && !self.lookahead(|parser| {
                parser.bump();
                parser.current_token_is_on_new_line()
            })
            && self
                .peek_next_any_keyword()
                .is_ok_and(|keyword| keyword == Keyword::Class);
        if header.is_final {
            self.bump(); // eat final
        }

        // shared placement modifier
        header.is_shared = self.language.is_destack()
            && self.is_keyword(Keyword::Shared)
            && self.lookahead(|parser| {
                parser.bump();
                !parser.current_token_is_on_new_line()
                    && matches!(
                        parser.current_keyword(),
                        Some(Keyword::Const | Keyword::Let)
                    )
            });
        if header.is_shared {
            self.bump(); // eat shared
        }

        // module declaration
        let can_parse_module = self.language.is_destack()
            && self.flags.is_in_statement_position()
            && header.export.is_none()
            && !header.is_ambient
            && self.is_module_identifier()
            && self.next_token_type() == TokenType::OpenBrace;
        if can_parse_module {
            let module_id = self.eat_module(start)?;
            let expression_id = self.insert_node(
                Expression::Declaration(module_id),
                self.get_span_from(start),
            );
            return Ok(DescriptorHead::Expression(expression_id));
        }

        // global declaration
        let can_parse_destack_global =
            self.language.is_destack() && self.flags.is_in_statement_position();
        let can_parse_ambient_global = header.is_ambient
            || self.language.is_declaration()
            || self.flags.is_in_declare_context();
        if (can_parse_destack_global || can_parse_ambient_global)
            && self.is_global_identifier()
            && self.next_token_type() == TokenType::OpenBrace
        {
            let mut global_header = header;
            global_header.is_ambient = !self.language.is_destack() || header.is_ambient;
            let global_id = self.eat_global(start, global_header)?;
            let expression_id = self.insert_node(
                Expression::Declaration(global_id),
                self.get_span_from(start),
            );
            return Ok(DescriptorHead::Expression(expression_id));
        }

        Ok(DescriptorHead::Header { header, decorators })
    }

    /// Check whether the current token starts a declare target keyword.
    pub(super) fn current_token_starts_declare_keyword_target(&mut self) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };

        if keyword == Keyword::Declare || !is_declaration_keyword(keyword) {
            return false;
        }

        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;
        let next_has_line_break = next_token.token.is_on_new_line;
        let next_keyword = self.next_keyword();
        let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);

        match keyword {
            // is_ambient async heads only exist for `async function`
            Keyword::Async => {
                !next_has_line_break
                    && next_token_type == TokenType::Identifier
                    && next_keyword == Some(Keyword::Function)
            }

            // ambient const and let declarations commit immediately
            Keyword::Const | Keyword::Let => true,

            // is_ambient nominal and structural declarations keep their existing heads
            Keyword::Class
            | Keyword::Function
            | Keyword::Interface
            | Keyword::Struct
            | Keyword::Extension
            | Keyword::Union
            | Keyword::Newtype => true,

            // enum declarations must stay on the same line as the head keyword
            Keyword::Enum => is_declaration_start && !next_has_line_break,

            // type aliases require a contiguous identifier name
            Keyword::Type => !next_has_line_break && next_token_type == TokenType::Identifier,

            // the remaining declaration keywords are contextual modifiers, not is_ambient heads
            _ => false,
        }
    }

    /// Check whether the current token starts a declare identifier target.
    pub(super) fn current_token_starts_declare_identifier(&mut self) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }
        self.is_global_identifier()
    }

    /// Check whether the current token starts a declare await using target.
    pub(super) fn current_token_starts_declare_await_using(&mut self) -> bool {
        if self.current_keyword() != Some(Keyword::Await) {
            return false;
        }
        self.next_keyword() == Some(Keyword::Using)
    }

    /// Check whether the current token starts a declare target.
    pub(super) fn current_token_starts_declare_target(&mut self) -> bool {
        self.current_token_starts_declare_keyword_target()
            || self.current_token_starts_declare_identifier()
            || self.current_token_starts_declare_await_using()
    }
}
