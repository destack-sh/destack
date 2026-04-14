use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Asynchrony, Declaration, EnumKind, ExportMode, Expression, Keyword, LocalNodeId,
    OperatorPrecedence, Path, ScalarLiteral, TokenType, TypeExpression, TypeKind,
};
use smallvec::smallvec;

use super::common::{DECLARATION_START_TOKENS, is_type_relation_keyword};

#[allow(clippy::too_many_arguments)]
impl Parser {
    /// Return true when a token can start a function signature head.
    #[inline]
    fn can_start_function_signature(next_token_type: TokenType) -> bool {
        matches!(
            next_token_type,
            TokenType::Identifier
                | TokenType::OpenParenthesis
                | TokenType::LessThan
                | TokenType::At
                | TokenType::Multiply
        )
    }

    /// Return true when `async` can plausibly start a function form.
    #[inline]
    fn can_start_async_function_signature(&mut self, next_token_type: TokenType) -> bool {
        // async (...) must be followed by an arrow or return type to form a lambda
        if next_token_type == TokenType::OpenParenthesis {
            let open_index = self.index_for_next() as u32;
            let Some(close_pos) = self.find_matching_close_maybe(
                Some(open_index),
                TokenType::OpenParenthesis,
                TokenType::CloseParenthesis,
            ) else {
                return false;
            };
            let follow_index = self.next_non_newline_index_from(close_pos as usize + 1);
            let follow_token_type = self.token_ref_at(follow_index).map(|token| token.token.ty);
            return matches!(
                follow_token_type,
                Some(TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon)
            );
        }

        // async <T>(...) must be a real generic arrow head
        if next_token_type == TokenType::LessThan {
            let next_index = self.index_for_next();
            return self.with_pos(next_index, |parser| {
                parser.peek_generic_arrow_after_type_parameters(false)
            });
        }

        // non parenthesized non identifier starts are signature candidates
        if matches!(next_token_type, TokenType::At | TokenType::Multiply) {
            return true;
        }

        // async function ...
        if next_token_type == TokenType::Identifier
            && self.keyword_for_index(self.index_for_next()) == Some(Keyword::Function)
        {
            return true;
        }

        // async x => ...
        if next_token_type == TokenType::Identifier {
            let next_index = self.index_for_next();
            let after_next_index = self.next_non_newline_index_from(next_index + 1);
            let after_next_token_type = self.token_type_at(after_next_index);
            return after_next_token_type == TokenType::Arrow
                || after_next_token_type == TokenType::ArrowWide;
        }

        false
    }

    /// Wrap a declaration node in an expression with the current span.
    #[inline]
    pub(super) fn insert_declaration_expression(
        &mut self,
        start: &ParserMark,
        declaration_id: LocalNodeId<Declaration>,
    ) -> LocalNodeId<Expression> {
        if self.options.is_in_type() {
            let type_expression_id = self.insert_node(
                TypeExpression::Declaration {
                    declaration: declaration_id,
                },
                self.get_span_from(start),
            );
            self.insert_type_expression_value(type_expression_id)
        } else {
            self.insert_node(
                Expression::Declaration(declaration_id),
                self.get_span_from(start),
            )
        }
    }

    /// Return whether the current keyword is followed by `.<member>` for any expected member name.
    pub(super) fn keyword_member_access_is_any(
        &mut self,
        member_names: &[&str],
        allow_newlines: bool,
    ) -> ParseResult<bool> {
        // require dot member access
        let dot_index = if allow_newlines {
            if !self.is_token_after_newlines(self.pos(), TokenType::Dot) {
                return Ok(false);
            }
            self.next_non_newline_index_from(self.pos_index() + 1)
        } else {
            if !self.peek_next_is(TokenType::Dot) {
                return Ok(false);
            }
            self.index_for_next()
        };

        // require identifier member
        let identifier_index = if allow_newlines {
            self.next_non_newline_index_from(dot_index + 1)
        } else {
            dot_index + 1
        };
        self.ensure_token(identifier_index);
        let Some(identifier_token) = self.tokens().get(identifier_index).copied() else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };
        if identifier_token.token.ty != TokenType::Identifier {
            return Err(ParseError::unexpected(identifier_token.span));
        }

        // compare directly against source text to avoid interning in hot lookahead
        let matches_member_name = member_names
            .iter()
            .any(|member_name| self.identifier_equals_at(identifier_index, member_name));
        Ok(matches_member_name)
    }

    /// Eat `import.meta` as one dedicated expression.
    fn eat_import_meta_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::Import)?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::Dot)?;
        self.eat_newlines_maybe()?;
        self.eat_identifier_str("meta")?;

        Ok(self.insert_node(Expression::ImportMeta, self.get_span_from(start)))
    }

    /// Eat `import.source` as one qualified reference expression.
    fn eat_import_source_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let import_span = self.peek_keyword(Keyword::Import)?.span;
        let import_name = self.strings.intern("import");

        self.eat_keyword(Keyword::Import)?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::Dot)?;
        self.eat_newlines_maybe()?;
        let (source_name, source_span) = self.eat_identifier_with_span()?;

        let path = Path {
            segments: smallvec![import_name, source_name],
        };
        let expression = Expression::QualifiedReference {
            path,
            generic_arguments: vec![],
        };
        let expression_id = self.insert_node(expression, self.get_span_from(start));

        self.set_path_expression_spans(expression_id, &[import_span, source_span]);

        Ok(expression_id)
    }

    /// Eat `new.target` as one dedicated expression.
    fn eat_new_target_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::New)?;
        self.eat_token(TokenType::Dot)?;
        self.eat_identifier_str("target")?;

        Ok(self.insert_node(Expression::NewTarget, self.get_span_from(start)))
    }

    /// Return true when `await` may begin an `await using` declaration.
    #[inline]
    fn can_start_await_using(&mut self) -> bool {
        self.using_keyword_index(Asynchrony::Async).is_some()
    }

    /// Try to parse common statement keywords without the full keyword dispatch table.
    pub(crate) fn try_eat_direct_statement_keyword_expression(
        &mut self,
        start: &ParserMark,
        keyword: Keyword,
        next_raw_token_type: TokenType,
        next_cursor: NonNewlineTokenCursor,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let header = DeclarationHeader::default();
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;
        let next_has_line_break = next_cursor.has_line_break_before;
        let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
        let next_keyword = if next_token_type == TokenType::Identifier {
            self.keyword_for_index(next_token_index)
        } else {
            None
        };
        let next_raw_keyword = if next_raw_token_type == TokenType::Identifier {
            self.keyword_for_index(self.index_for_next())
        } else {
            None
        };

        match keyword {
            Keyword::Function => {
                if !Self::can_start_function_signature(next_token_type) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            Keyword::Class => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let allow_anonymous_class = !self.options.is_in_statement_position()
                    || header.export == Some(ExportMode::Default);
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            Keyword::Struct
                if self.language.is_destack() && (is_declaration_start || next_has_line_break) =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let struct_id = self.eat_struct_or_class(start, header, false)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            Keyword::Enum if is_declaration_start && !next_has_line_break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let enum_id = self.eat_enum(start, EnumKind::Enum, header)?;
                Ok(Some(self.insert_declaration_expression(start, enum_id)))
            }
            Keyword::Interface if is_declaration_start || next_has_line_break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let interface_id = self.eat_interface(start, header, TypeKind::Structural)?;
                Ok(Some(
                    self.insert_declaration_expression(start, interface_id),
                ))
            }
            Keyword::Namespace
                if is_declaration_start
                    && !next_has_line_break
                    && next_token_type == TokenType::Identifier
                    && !is_type_relation_keyword(next_keyword) =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let namespace_id = self.eat_namespace(start, header)?;
                Ok(Some(
                    self.insert_declaration_expression(start, namespace_id),
                ))
            }
            Keyword::Extension if self.language.is_destack() && is_declaration_start => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let extension_id = self.eat_extension(start, header)?;
                Ok(Some(
                    self.insert_declaration_expression(start, extension_id),
                ))
            }
            Keyword::Type if !self.options.is_in_new_receiver() => {
                if next_has_line_break {
                    return Ok(None);
                }
                let next_index = next_token_index;
                let after_next_index = self.next_non_newline_index_from(next_index + 1);
                let after_next_token_type = self.token_type_at(after_next_index);
                let starts_type_operator = is_type_relation_keyword(next_keyword)
                    && !matches!(
                        after_next_token_type,
                        TokenType::Assign
                            | TokenType::LessThan
                            | TokenType::ShiftLeft
                            | TokenType::SaturatingShiftLeft
                    );

                let can_start_type_alias =
                    if self.language.is_typescript() || self.language.is_javascript() {
                        next_token_type == TokenType::Identifier && !starts_type_operator
                    } else {
                        matches!(
                            next_token_type,
                            TokenType::Identifier
                                | TokenType::OpenBrace
                                | TokenType::OpenParenthesis
                                | TokenType::OpenBracket
                                | TokenType::Literal
                        ) && !starts_type_operator
                    };
                if !can_start_type_alias {
                    return Ok(None);
                }

                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                Ok(Some(self.eat_type(start, header)?))
            }
            Keyword::Import if next_raw_token_type == TokenType::OpenParenthesis => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_import_call_expression(start)?))
            }
            Keyword::Import => {
                // special import member forms
                if self.is_token_after_newlines(self.pos(), TokenType::Dot) {
                    if self.keyword_member_access_is_any(&["meta"], true)? {
                        return Ok(Some(self.eat_import_meta_expression(start)?));
                    }
                    if self.keyword_member_access_is_any(&["source"], true)? {
                        return Ok(Some(self.eat_import_source_expression(start)?));
                    }
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // require a valid import statement shape
                let can_start_import = matches!(
                    next_token_type,
                    TokenType::Multiply
                        | TokenType::Identifier
                        | TokenType::OpenBrace
                        | TokenType::Literal
                ) || self.can_start_import_statement();
                if !can_start_import {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_import()?))
            }
            Keyword::If => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_if()?))
            }
            Keyword::While => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            Keyword::Do if self.is_do_while_statement(next_token_type) => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            Keyword::For => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_for()?))
            }
            Keyword::Loop if self.language.is_destack() && self.is_next_block_start() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_loop()?))
            }
            Keyword::Try => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_try()?))
            }
            Keyword::Switch => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            Keyword::Match if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            Keyword::Break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_break()?))
            }
            Keyword::Continue => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_continue()?))
            }
            Keyword::Throw => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_throw()?))
            }
            Keyword::Return => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_return()?))
            }
            Keyword::Debugger => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                self.bump();
                Ok(Some(
                    self.tree
                        .insert(Expression::Debugger, self.get_span_from(start)),
                ))
            }
            Keyword::Yield if self.options.is_in_generator() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_yield()?))
            }
            Keyword::Comptime if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                Ok(Some(self.eat_comptime()?))
            }
            Keyword::Let | Keyword::Var => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                Ok(Some(self.eat_let_from_keyword(start, header, keyword)?))
            }
            Keyword::Const => {
                if next_raw_keyword == Some(Keyword::Enum) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    self.eat_keyword(Keyword::Const)?;
                    let enum_id = self.eat_enum(start, EnumKind::Const, header)?;
                    Ok(Some(self.insert_declaration_expression(start, enum_id)))
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_let_from_keyword(
                        start,
                        header,
                        Keyword::Const,
                    )?))
                }
            }
            Keyword::Using => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                if self.can_parse_using_declaration(&header, Asynchrony::Sync) {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Sync)?))
                } else {
                    Ok(None)
                }
            }
            Keyword::Await => {
                if self.options.is_forbid_await() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                if self.can_start_await_using()
                    && self.can_parse_using_declaration(&header, Asynchrony::Async)
                {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_using(start, header, Asynchrony::Async)?))
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                    Ok(Some(self.eat_await()?))
                }
            }
            Keyword::Async if next_raw_token_type == TokenType::Identifier => {
                if next_keyword != Some(Keyword::Function) {
                    return Ok(None);
                }

                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            _ => Ok(None),
        }
    }

    /// Return true when a keyword token should parse as a type predicate subject identifier.
    fn keyword_parses_as_type_predicate_subject_identifier(
        &mut self,
        keyword: Keyword,
        next_token_type: TokenType,
        next_token_index: usize,
    ) -> bool {
        // this path only applies inside type expressions
        if !self.options.is_in_type() {
            return false;
        }

        // `this is T` is a dedicated type predicate subject form
        if keyword == Keyword::This {
            return false;
        }

        // predicates require `identifier is Type`
        if next_token_type != TokenType::Identifier {
            return false;
        }

        self.keyword_for_index(next_token_index) == Some(Keyword::Is)
    }

    /// Eat a keyword-led expression when possible.
    ///
    /// Examples:
    /// ```
    /// async () => value
    /// function named() {}
    /// import.meta
    /// import("pkg")
    /// export { a, b }
    /// class Box<T> {}
    /// ```
    pub(super) fn eat_keyword_expression(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        keyword: Keyword,
        next_token_type: TokenType,
        next_token_index: usize,
        next_has_line_break: bool,
        next_raw_token_type: TokenType,
        is_declaration_start: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // parse contextual keyword subjects like `override is X` as identifiers
        if self.keyword_parses_as_type_predicate_subject_identifier(
            keyword,
            next_token_type,
            next_token_index,
        ) {
            return Ok(None);
        }

        let next_keyword = if next_token_type == TokenType::Identifier {
            self.keyword_for_index(next_token_index)
        } else {
            None
        };

        match keyword {
            // namespace declaration
            Keyword::Namespace
                if is_declaration_start
                    && !next_has_line_break
                    && next_token_type == TokenType::Identifier
                    && !is_type_relation_keyword(next_keyword) =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let namespace_id = self.eat_namespace(start, header)?;
                Ok(Some(
                    self.insert_declaration_expression(start, namespace_id),
                ))
            }
            // struct declaration
            Keyword::Struct
                if self.language.is_destack() && (is_declaration_start || next_has_line_break) =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let allow_anonymous_class = !self.options.is_in_statement_position()
                    || header.export == Some(ExportMode::Default);
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            // class declaration
            Keyword::Class if is_declaration_start || next_has_line_break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let allow_anonymous_class = header.export == Some(ExportMode::Default)
                    || !self.options.is_in_statement_position();
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            // enum declaration
            Keyword::Enum if is_declaration_start && !next_has_line_break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let enum_id = self.eat_enum(start, EnumKind::Enum, header)?;
                Ok(Some(self.insert_declaration_expression(start, enum_id)))
            }
            // const enum or binding declaration
            Keyword::Const => {
                // parse const enum declaration
                if next_keyword == Some(Keyword::Enum) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    self.eat_keyword(Keyword::Const)?;
                    let enum_id = self.eat_enum(start, EnumKind::Const, header)?;
                    Ok(Some(self.insert_declaration_expression(start, enum_id)))
                // otherwise parse binding declaration
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_let_from_keyword(
                        start,
                        header,
                        Keyword::Const,
                    )?))
                }
            }
            // newtype interface or alias declaration
            Keyword::Newtype => {
                // parse newtype interface declaration
                if next_keyword == Some(Keyword::Interface) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    self.eat_keyword(Keyword::Newtype)?;
                    let interface_id = self.eat_interface(start, header, TypeKind::Nominal)?;
                    Ok(Some(
                        self.insert_declaration_expression(start, interface_id),
                    ))
                // otherwise parse newtype alias declaration
                } else {
                    // require a valid type alias start
                    let can_start_type_alias = matches!(
                        next_token_type,
                        TokenType::Identifier
                            | TokenType::OpenBrace
                            | TokenType::OpenParenthesis
                            | TokenType::OpenBracket
                            | TokenType::Literal
                    );

                    // parse the alias when it can start
                    if can_start_type_alias {
                        let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                        Ok(Some(self.eat_type(start, header)?))
                    // otherwise bail
                    } else {
                        Ok(None)
                    }
                }
            }
            // interface declaration
            Keyword::Interface if is_declaration_start || next_has_line_break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let interface_id = self.eat_interface(start, header, TypeKind::Structural)?;
                Ok(Some(
                    self.insert_declaration_expression(start, interface_id),
                ))
            }
            // extension declaration
            Keyword::Extension
                if self.language.is_destack()
                    && self.options.is_in_statement_position()
                    && is_declaration_start =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let extension_id = self.eat_extension(start, header)?;
                Ok(Some(
                    self.insert_declaration_expression(start, extension_id),
                ))
            }
            // async declaration or async path
            Keyword::Async => {
                if next_has_line_break {
                    return Ok(None);
                }

                // avoid async generic parses in stronger infix contexts
                let has_generic_head = self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::ShiftLeft);
                let has_stronger_infix_context =
                    self.options.left_precedence.is_some_and(|left_precedence| {
                        left_precedence > OperatorPrecedence::Assignment as u16
                    });
                if has_generic_head && has_stronger_infix_context {
                    return Ok(None);
                }

                // avoid async generic parses when tree literal disambiguation is active
                if self.language.supports_jsx()
                    && self.options.is_disallow_ambiguous_tree_literal()
                    && self.options.left_precedence.is_some()
                    && has_generic_head
                {
                    return Ok(None);
                }

                // require a valid function signature start
                let can_start_signature = self.can_start_async_function_signature(next_token_type);
                if !can_start_signature {
                    return Ok(None);
                }

                // parse async declaration directly after scanner disambiguation
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // override is contextual in value expressions
            Keyword::Override if !self.options.is_in_type() => Ok(None),
            // abstract is contextual outside declaration positions
            Keyword::Abstract
                if !self.options.is_in_type()
                    && !self.options.is_in_statement_position()
                    && header.export.is_none() =>
            {
                Ok(None)
            }
            // function or method declaration
            Keyword::Function | Keyword::Abstract | Keyword::Override => {
                // keep multiline `abstract new (...) => ...` construct signatures valid in type context
                let is_multiline_abstract_construct_signature = keyword == Keyword::Abstract
                    && self.options.is_in_type()
                    && next_keyword == Some(Keyword::New);
                if matches!(keyword, Keyword::Abstract | Keyword::Override)
                    && next_has_line_break
                    && !is_multiline_abstract_construct_signature
                {
                    return Ok(None);
                }

                // require a valid function signature start
                let can_start_signature = Self::can_start_function_signature(next_token_type);
                if !can_start_signature {
                    return Ok(None);
                }

                // parse function declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // new signature declaration in type positions
            Keyword::New if self.options.is_in_type() => {
                // require a valid function signature start
                let can_start_signature = Self::can_start_function_signature(next_token_type);
                if can_start_signature {
                    // parse constructor signature
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    let function_id = self.eat_function(start, header, false, false)?;
                    Ok(Some(self.insert_declaration_expression(start, function_id)))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // variant method declaration
            Keyword::Get | Keyword::Set | Keyword::Constructor if self.options.is_in_variant() => {
                // require a valid function signature start
                let can_start_signature = Self::can_start_function_signature(next_token_type);
                if !can_start_signature {
                    return Ok(None);
                }

                // parse variant method declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // this expression
            Keyword::This => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                self.bump(); // eat this

                // type positions use the dedicated type node directly
                if self.options.is_in_type() {
                    let type_expression_id =
                        self.insert_node(TypeExpression::This, self.get_span_from(start));
                    Ok(Some(self.insert_type_expression_value(type_expression_id)))
                } else {
                    Ok(Some(
                        self.tree
                            .insert(Expression::This, self.get_span_from(start)),
                    ))
                }
            }
            // super expression
            Keyword::Super => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                self.bump(); // eat super
                Ok(Some(
                    self.tree
                        .insert(Expression::Super, self.get_span_from(start)),
                ))
            }
            // null literal
            Keyword::Null => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                self.bump(); // eat null

                // type positions keep `null` in type space
                if self.options.is_in_type() {
                    let type_expression_id = self.insert_node(
                        TypeExpression::ScalarLiteral {
                            value: ScalarLiteral::Null,
                        },
                        self.get_span_from(start),
                    );

                    Ok(Some(self.insert_type_expression_value(type_expression_id)))
                } else {
                    Ok(Some(self.insert_node(
                        Expression::ScalarLiteral(ScalarLiteral::Null),
                        self.get_span_from(start),
                    )))
                }
            }
            // new expression
            Keyword::New if !self.options.is_in_type() => {
                // allow one committed or recoverable constructor slot
                let can_start_new_expression =
                    matches!(
                        next_token_type,
                        TokenType::Identifier
                            | TokenType::OpenParenthesis
                            | TokenType::OpenBrace
                            | TokenType::LessThan
                            | TokenType::Newline
                    ) || Self::is_expression_slot_boundary_token(next_token_type);
                if can_start_new_expression {
                    // parse new expression
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                    Ok(Some(self.eat_new()?))
                }
                // keep `new.target` explicit
                else if next_token_type == TokenType::Dot {
                    if self.keyword_member_access_is_any(&["target"], false)? {
                        Ok(Some(self.eat_new_target_expression(start)?))
                    } else {
                        Err(ParseError::unexpected(self.peek()?.span))
                    }
                }
                // otherwise reject `new` in expression position
                else {
                    Err(ParseError::unexpected(self.peek()?.span))
                }
            }
            // delete expression
            Keyword::Delete if next_token_type != TokenType::Colon => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                Ok(Some(self.eat_delete()?))
            }
            // type only import expression
            Keyword::Import
                if self.options.is_in_type()
                    && next_raw_token_type == TokenType::OpenParenthesis =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_type_import_expression()?))
            }
            // import call expression
            Keyword::Import if next_raw_token_type == TokenType::OpenParenthesis => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_import_call_expression(start)?))
            }
            // import declaration or import meta
            Keyword::Import => {
                // special import member forms
                if self.is_token_after_newlines(self.pos(), TokenType::Dot) {
                    if self.keyword_member_access_is_any(&["meta"], true)? {
                        return Ok(Some(self.eat_import_meta_expression(start)?));
                    }
                    if self.keyword_member_access_is_any(&["source"], true)? {
                        return Ok(Some(self.eat_import_source_expression(start)?));
                    }
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // require a valid import start
                let can_start_import = matches!(
                    next_token_type,
                    TokenType::Multiply
                        | TokenType::Identifier
                        | TokenType::OpenBrace
                        | TokenType::Literal
                ) || self.can_start_import_statement();
                if !can_start_import {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // parse import or export import equals declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                if header.export.is_some() && self.peek_import_equals_after_import() {
                    Ok(Some(self.eat_export_import_equals(start, header)?))
                } else {
                    Ok(Some(self.eat_import()?))
                }
            }
            // infer type expression
            Keyword::Infer if self.options.is_in_type() => {
                Ok(Some(self.eat_type_infer_expression()?))
            }
            // asserts type predicate
            Keyword::Asserts if self.options.is_in_type() => {
                // allow asserts predicate only when grammar supports it
                if self.can_start_type_predicate_asserts() {
                    Ok(Some(self.eat_type_predicate_asserts()?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // let or var binding declaration
            Keyword::Let | Keyword::Var => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                Ok(Some(self.eat_let_from_keyword(start, header, keyword)?))
            }
            // using declaration
            Keyword::Using => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                if self.can_parse_using_declaration(&header, Asynchrony::Sync) {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Sync)?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // readonly type operator in ts and js type contexts
            Keyword::Readonly if self.language.is_typescript() || self.language.is_javascript() => {
                // in new receiver context, readonly behaves like an identifier
                if self.options.is_in_new_receiver() {
                    return Ok(None);
                }

                // ts and js parse readonly as a type unary in type positions
                if self.options.is_in_type() {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    return Ok(Some(self.eat_type(start, header)?));
                }

                // value positions keep readonly contextual
                Ok(None)
            }
            // type alias declaration and destack readonly/newtype aliases
            Keyword::Type | Keyword::Readonly => {
                // for each bindings keep `type` and `readonly` as identifiers
                if self.options.is_in_for_each() && !self.options.is_in_type() {
                    return Ok(None);
                }

                // in new receiver context, `type` and `readonly` behave like identifiers
                if self.options.is_in_new_receiver() {
                    return Ok(None);
                }

                if keyword == Keyword::Type && next_has_line_break {
                    return Ok(None);
                }

                let next_index = next_token_index;
                let after_next_index = self.next_non_newline_index_from(next_index + 1);
                let after_next_token_type = self.token_type_at(after_next_index);
                let starts_type_operator = is_type_relation_keyword(next_keyword)
                    && !matches!(
                        after_next_token_type,
                        TokenType::Assign
                            | TokenType::LessThan
                            | TokenType::ShiftLeft
                            | TokenType::SaturatingShiftLeft
                    );

                // typescript and javascript only allow identifier names in type alias declarations
                let can_start_type_alias =
                    if self.language.is_typescript() || self.language.is_javascript() {
                        next_token_type == TokenType::Identifier && !starts_type_operator
                    }
                    // destack keeps broader alias starts
                    else {
                        matches!(
                            next_token_type,
                            TokenType::Identifier
                                | TokenType::OpenBrace
                                | TokenType::OpenParenthesis
                                | TokenType::OpenBracket
                                | TokenType::Literal
                        ) && !starts_type_operator
                    };

                // parse type alias when it can start
                if can_start_type_alias {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    Ok(Some(self.eat_type(start, header)?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // if
            Keyword::If => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_if()?))
            }
            // while
            Keyword::While => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            // do while
            Keyword::Do if self.is_do_while_statement(next_token_type) => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            // for
            Keyword::For => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_for()?))
            }
            // loop
            Keyword::Loop if self.language.is_destack() && self.is_next_block_start() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_loop()?))
            }
            // try
            Keyword::Try => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_try()?))
            }
            // switch
            Keyword::Switch => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            // match
            Keyword::Match if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            // break
            Keyword::Break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_break()?))
            }
            // continue
            Keyword::Continue => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_continue()?))
            }
            // await expression or await using
            Keyword::Await => {
                // reject await in contexts that forbid it
                if self.options.is_forbid_await() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // parse await using when allowed
                if self.can_start_await_using()
                    && self.can_parse_using_declaration(&header, Asynchrony::Async)
                {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_using(start, header, Asynchrony::Async)?))
                // otherwise parse await expression
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                    Ok(Some(self.eat_await()?))
                }
            }
            // comptime expression
            Keyword::Comptime if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                Ok(Some(self.eat_comptime()?))
            }
            // yield statement
            Keyword::Yield if self.options.is_in_generator() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_yield()?))
            }
            // throw statement
            Keyword::Throw => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_throw()?))
            }
            // return statement
            Keyword::Return => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_return()?))
            }
            // debugger statement
            Keyword::Debugger => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                self.bump(); // eat `debugger`
                Ok(Some(
                    self.tree
                        .insert(Expression::Debugger, self.get_span_from(start)),
                ))
            }
            // fall through when not a keyword expression
            _ => Ok(None),
        }
    }
}
