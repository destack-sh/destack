use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Asynchrony, BindingAnchor, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    DependencyMode, Expression, Keyword, LiteralType, TokenType,
};

use super::common::{DECLARATION_KEYWORDS, DescriptorHead};

impl Parser {
    /// Check whether a `{` in statement position should be parsed as an object literal.
    /// NOTE #Cleanup: can_parse_object_literal_in_statement_position is ugly and might not be fixable.
    pub(super) fn can_parse_object_literal_in_statement_position(&mut self) -> bool {
        // only allow this in destack files
        if !self.language.is_destack() {
            return false;
        }

        // avoid object literals when a block is expected
        if self.options.in_before_block {
            return false;
        }

        // skip newlines after the opening brace
        let open_pos = match self.skip_newlines(self.pos()) {
            Ok(pos) => pos,
            Err(_) => return false,
        };
        self.token_stream.ensure_token(open_pos as usize + 1);
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
            let close_bracket_pos = match self.find_matching_close(
                Some(open_bracket_pos),
                TokenType::OpenBracket,
                TokenType::CloseBracket,
            ) {
                Ok(pos) => pos,
                Err(_) => return false,
            };

            let after_close_pos = match self.skip_newlines(close_bracket_pos) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            self.token_stream.ensure_token(after_close_pos as usize + 1);
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
            self.token_stream.ensure_token(key_pos as usize + 1);
            let Some(after_key) = self.tokens().get(key_pos as usize + 1) else {
                return false;
            };

            return matches!(after_key.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        false
    }

    /// Check whether a using declaration can be parsed at the current position.
    pub(super) fn can_parse_using_declaration(
        &mut self,
        descriptor: &DeclarationDescriptor,
        asynchrony: Asynchrony,
    ) -> bool {
        // speculatively parse a using declaration
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();
        let result = self
            .eat_using(&speculative_start, descriptor.clone(), asynchrony)
            .is_ok();
        self.restore(speculative_start, speculative_start_idx);
        result
    }

    /// Eat declaration modifiers and return a descriptor or a parsed expression.
    pub(super) fn eat_declaration_descriptor(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<DescriptorHead> {
        let mut descriptor: DeclarationDescriptor = DeclarationDescriptor::default();

        // decorators parse as expressions only
        if self.options.in_decorator {
            return Ok(DescriptorHead::Descriptor(descriptor));
        }

        // declaration modifiers only start on identifiers
        if !self.peek_is(TokenType::Identifier) {
            return Ok(DescriptorHead::Descriptor(descriptor));
        }

        // check for a modifier keyword or a global or module identifier
        let pos = self.pos_index();
        let keyword = if self.has_active_split() {
            self.peek_any_keyword().ok()
        } else {
            self.keyword_for_index(pos)
        };
        let is_modifier_keyword = matches!(
            keyword,
            Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static)
        );
        let next_token_type = self.peek_next_token_type();
        let can_start_global_or_module_declaration = matches!(
            next_token_type,
            TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal | TokenType::Newline
        );
        let is_global_identifier = !is_modifier_keyword
            && can_start_global_or_module_declaration
            && self.identifier_equals_at(pos, "global");
        let is_module_identifier = !is_modifier_keyword
            && can_start_global_or_module_declaration
            && self.language.supports_module_declaration()
            && self.identifier_equals_at(pos, "module");
        if !is_modifier_keyword && !is_global_identifier && !is_module_identifier {
            return Ok(DescriptorHead::Descriptor(descriptor));
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
            let has_module_identifier_declaration = self.language.supports_module_declaration()
                && self.peek_identifier_str("module").is_ok();
            let has_declaration_keyword = next_keyword
                .is_some_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
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
                && self.peek_dependency_binding().is_err()
                && !self.is_keyword_after_newlines(Keyword::Import);
            let is_export_dependency = export_mode == Some(DependencyMode::Namespace)
                || is_export_type_binding
                || (!has_declaration_keyword && self.peek_dependency_binding().is_ok())
                || (export_mode == Some(DependencyMode::Default) && !has_declaration_keyword)
                || is_invalid_export_form;
            if is_export_dependency {
                self.rewind(start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            descriptor.export = export_mode;

            // allow decorators after export modifier
            if self.peek_is(TokenType::At) {
                self.eat_decorators_prefix_maybe()?;
            }
        }

        // skip newlines before export import equals
        if descriptor.export.is_some()
            && self.peek_is(TokenType::Newline)
            && self.is_keyword_after_newlines(Keyword::Import)
        {
            self.eat_newlines_maybe()?;
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
                    self.token_stream.ensure_token(after_abstract);
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
                    self.token_stream.ensure_token(after_type);
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
        descriptor.kind = if is_declare && declare_has_target {
            self.bump(); // eat declare
            DeclarationKind::Declaration
        } else {
            DeclarationKind::Definition
        };

        // abstraction modifier
        descriptor.abstraction = if self.is_keyword(Keyword::Abstract)
            && !self.options.in_variant
            && !self.peek_next_is(TokenType::Newline)
            && self
                .peek_next_any_keyword()
                .is_ok_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
        {
            self.bump(); // eat abstract
            DeclarationAbstraction::Abstract
        } else {
            DeclarationAbstraction::Concrete
        };

        // anchor modifier
        descriptor.anchor = if self.is_keyword(Keyword::Static) {
            self.bump(); // eat static
            BindingAnchor::Static
        } else {
            BindingAnchor::Instance
        };

        // global declaration
        if (descriptor.kind == DeclarationKind::Declaration
            || self.language.is_declaration()
            || self.options.in_declare_context)
            && self.peek_identifier_str("global").is_ok()
            && self
                .peek_token_after_newlines(self.pos(), TokenType::OpenBrace)
                .is_ok()
        {
            let mut global_descriptor = descriptor;
            if global_descriptor.kind == DeclarationKind::Definition {
                // global declarations are always declarations
                global_descriptor.kind = DeclarationKind::Declaration;
            }
            let global_id = self.eat_global(start, global_descriptor)?;
            let expression_id = self.tree.insert(
                Expression::Declaration(global_id),
                self.get_span_from(start),
            );
            return Ok(DescriptorHead::Expression(expression_id));
        }

        Ok(DescriptorHead::Descriptor(descriptor))
    }

    /// Check whether a token index starts a declare target keyword.
    pub(super) fn is_declare_keyword_target_at(&mut self, index: usize) -> bool {
        let keyword = self.keyword_for_index(index);
        keyword.is_some_and(|kw| kw != Keyword::Declare && DECLARATION_KEYWORDS.contains(&kw))
    }

    /// Check whether a token index starts a declare identifier target.
    pub(super) fn is_declare_identifier_at(&mut self, index: usize) -> bool {
        self.token_stream.ensure_token(index);
        let Some(token) = self.tokens().get(index) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }
        let token_str = self.get_span_str(token.span);
        token_str == "global"
            || self.language.supports_module_declaration() && token_str == "module"
    }

    /// Check whether a token index starts a declare await using target.
    pub(super) fn is_declare_await_using_at(&mut self, index: usize) -> bool {
        if self.keyword_for_index(index) != Some(Keyword::Await) {
            return false;
        }
        let mut after = index + 1;
        loop {
            self.token_stream.ensure_token(after);
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
