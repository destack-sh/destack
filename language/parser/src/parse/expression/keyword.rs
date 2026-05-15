use crate::parse::expression::common::DeclarationHeader;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    Asynchrony, Declaration, EnumKind, ExportKind, Expression, Keyword, LocalNodeId,
    OperatorPrecedence, Path, ScalarLiteral, TokenType, TypeExpression, TypeKind, TypeLiteral,
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
            return self.lookahead(|parser| {
                parser.bump();
                let mut parenthesis_depth = 0u32;

                loop {
                    let token_type = parser.peek_token_type();
                    if token_type == TokenType::End {
                        return false;
                    }

                    if token_type == TokenType::OpenParenthesis {
                        parenthesis_depth += 1;
                    } else if token_type == TokenType::CloseParenthesis {
                        parenthesis_depth -= 1;
                        if parenthesis_depth == 0 {
                            parser.bump();
                            return matches!(
                                parser.peek_token_type(),
                                TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
                            );
                        }
                    }

                    parser.bump();
                }
            });
        }

        // async <T>(...) must be a real generic arrow head
        if next_token_type == TokenType::LessThan {
            return self.lookahead(|parser| {
                parser.bump();
                parser.peek_generic_arrow_after_type_parameters(false)
            });
        }

        // non parenthesized non identifier starts are signature candidates
        if matches!(next_token_type, TokenType::At | TokenType::Multiply) {
            return true;
        }

        // async function ...
        if next_token_type == TokenType::Identifier
            && self.lookahead(|parser| {
                parser.bump();
                parser.current_keyword()
            }) == Some(Keyword::Function)
        {
            return true;
        }

        // async x => ...
        if next_token_type == TokenType::Identifier {
            return self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                matches!(
                    parser.peek_token_type(),
                    TokenType::Arrow | TokenType::ArrowWide
                )
            });
        }

        false
    }

    /// Wrap a declaration node in one type expression with the current span.
    #[inline]
    pub(crate) fn insert_declaration_type_expression(
        &mut self,
        start: &ParserSpanStart,
        declaration_id: LocalNodeId<Declaration>,
    ) -> LocalNodeId<TypeExpression> {
        self.insert_node(
            TypeExpression::Declaration {
                declaration: declaration_id,
            },
            self.get_span_from(start),
        )
    }

    /// Wrap a declaration node in an expression with the current span.
    #[inline]
    pub(super) fn insert_declaration_expression(
        &mut self,
        start: &ParserSpanStart,
        declaration_id: LocalNodeId<Declaration>,
    ) -> LocalNodeId<Expression> {
        self.insert_node(
            Expression::Declaration(declaration_id),
            self.get_span_from(start),
        )
    }

    /// Lower one parsed `type` keyword result into value-space expression form.
    #[inline]
    fn insert_type_keyword_expression(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        // type alias declarations must remain declaration expressions in value space
        if let TypeExpression::Declaration { declaration } = self.tree.get(type_expression_id) {
            return self.insert_node(
                Expression::Declaration(*declaration),
                self.tree.get_span(type_expression_id),
            );
        }

        // plain type expressions stay wrapped
        self.wrap_type_expression(type_expression_id)
    }

    /// Return whether the current keyword is followed by `.<member>` for any expected member name.
    pub(super) fn keyword_member_access_is_any(
        &mut self,
        member_names: &[&str],
        allow_newlines: bool,
    ) -> ParseResult<bool> {
        self.lookahead(|parser| {
            // require dot member access
            parser.bump();
            let dot_token = parser.current_token();
            if dot_token.token.ty != TokenType::Dot
                || (!allow_newlines && dot_token.token.is_on_new_line)
            {
                return Ok(false);
            }

            // require identifier member
            parser.bump();
            let identifier_token = parser.current_token();
            if identifier_token.token.ty != TokenType::Identifier {
                return Err(ParseError::unexpected(identifier_token.span));
            }

            // compare directly against source text to avoid interning in hot lookahead
            let member_name = parser.current_token_str();
            let matches_member_name = member_names.contains(&member_name);

            Ok(matches_member_name)
        })
    }

    /// Return true when one `type` family keyword may start a type form.
    fn keyword_begins_type_form(
        &mut self,
        keyword: Keyword,
        next_token_type: TokenType,
        next_is_on_new_line: bool,
        next_keyword: Option<Keyword>,
        following_token_type: TokenType,
    ) -> bool {
        // `type` does not continue across a newline in value space
        if keyword == Keyword::Type && next_is_on_new_line {
            return false;
        }

        // `type as` and related operators should stay in value space
        let stays_in_value_space = keyword == Keyword::Type && {
            let is_type_relation = is_type_relation_keyword(next_keyword);
            let starts_alias_head = matches!(
                following_token_type,
                TokenType::Assign | TokenType::LessThan | TokenType::ShiftLeft
            );

            is_type_relation && !starts_alias_head
        };
        if stays_in_value_space {
            return false;
        }

        // base grammars keep `type` aliases identifier headed
        if !self.language.is_destack() {
            return next_token_type == TokenType::Identifier;
        }

        // extended grammars also admit direct structural alias heads
        matches!(
            next_token_type,
            TokenType::Identifier
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Literal
        )
    }

    /// Eat `import.meta` as one dedicated expression.
    fn eat_import_meta_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::Import)?;
        self.eat_token(TokenType::Dot)?;
        self.eat_identifier_str("meta")?;

        Ok(self.insert_node(Expression::ImportMeta, self.get_span_from(start)))
    }

    /// Eat `import.source` as one qualified reference expression.
    fn eat_import_source_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let import_span = self.peek_keyword(Keyword::Import)?.span;
        let import_name = self.strings.intern("import");

        self.eat_keyword(Keyword::Import)?;
        self.eat_token(TokenType::Dot)?;
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::New)?;
        self.eat_token(TokenType::Dot)?;
        self.eat_identifier_str("target")?;

        Ok(self.insert_node(Expression::NewTarget, self.get_span_from(start)))
    }

    /// Return true when `await` may begin an `await using` declaration.
    #[inline]
    fn can_start_await_using(&mut self) -> bool {
        self.using_keyword_is(Asynchrony::Async)
    }

    /// Try to parse common statement keywords without the full keyword dispatch table.
    pub(crate) fn try_eat_direct_statement_keyword_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let header = DeclarationHeader::default();
        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;
        let next_is_on_new_line = next_token.token.is_on_new_line;
        let next_keyword = self.next_keyword();
        let following_token_type = self.lookahead(|parser| {
            parser.bump();
            parser.bump();
            parser.peek_token_type()
        });
        let next_is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);

        match keyword {
            Keyword::Function => {
                if !Self::can_start_function_signature(next_token_type) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            Keyword::Class => {
                let allow_anonymous_class = !self.flags.is_in_statement_position()
                    || header.export == Some(ExportKind::Default);
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            Keyword::Struct
                if self.language.is_destack()
                    && (next_is_declaration_start || next_is_on_new_line) =>
            {
                let struct_id = self.eat_struct_or_class(start, header, false)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            Keyword::Enum if next_is_declaration_start && !next_is_on_new_line => {
                let enum_id = self.eat_enum(start, EnumKind::Enum, header)?;
                Ok(Some(self.insert_declaration_expression(start, enum_id)))
            }
            Keyword::Enum => Err(ParseError::unexpected(self.peek()?.span)),
            Keyword::Interface if next_is_declaration_start || next_is_on_new_line => {
                let interface_id = self.eat_interface(start, header, TypeKind::Structural)?;
                Ok(Some(
                    self.insert_declaration_expression(start, interface_id),
                ))
            }
            Keyword::Extension if self.language.is_destack() && next_is_declaration_start => {
                let extension_id = self.eat_extension(start, header)?;
                Ok(Some(
                    self.insert_declaration_expression(start, extension_id),
                ))
            }
            Keyword::Type if !self.flags.is_in_new_receiver() => {
                if !self.keyword_begins_type_form(
                    Keyword::Type,
                    next_token_type,
                    next_is_on_new_line,
                    next_keyword,
                    following_token_type,
                ) {
                    return Ok(None);
                }
                let type_expression_id = self.eat_type(start, header)?;

                Ok(Some(
                    self.insert_type_keyword_expression(type_expression_id),
                ))
            }
            Keyword::Import => {
                // special import member forms
                if self.keyword_member_access_is_any(&["meta"], true)? {
                    return Ok(Some(self.eat_import_meta_expression(start)?));
                }
                if self.keyword_member_access_is_any(&["source"], true)? {
                    return Ok(Some(self.eat_import_source_expression(start)?));
                }
                if self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Dot)
                }) {
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
                Ok(Some(self.eat_import()?))
            }
            Keyword::If => Ok(Some(self.eat_if()?)),
            Keyword::While => Ok(Some(self.eat_while()?)),
            Keyword::Do if self.is_do_while_statement(next_token_type) => {
                Ok(Some(self.eat_while()?))
            }
            Keyword::For => Ok(Some(self.eat_for()?)),
            Keyword::Loop if self.language.is_destack() && self.is_next_block_start() => {
                Ok(Some(self.eat_loop()?))
            }
            Keyword::Try => Ok(Some(self.eat_try()?)),
            Keyword::Switch => Ok(Some(self.eat_match()?)),
            Keyword::Match if self.language.is_destack() => Ok(Some(self.eat_match()?)),
            Keyword::Break => Ok(Some(self.eat_break()?)),
            Keyword::Continue => Ok(Some(self.eat_continue()?)),
            Keyword::Throw => Ok(Some(self.eat_throw()?)),
            Keyword::Return => Ok(Some(self.eat_return()?)),
            Keyword::Debugger => {
                self.bump();
                Ok(Some(
                    self.tree
                        .insert(Expression::Debugger, self.get_span_from(start)),
                ))
            }
            Keyword::Yield if self.flags.is_in_generator() => Ok(Some(self.eat_yield()?)),
            Keyword::Comptime if self.language.is_destack() => Ok(Some(self.eat_comptime()?)),
            Keyword::Let => Ok(Some(self.eat_let_from_keyword(start, header, keyword)?)),
            Keyword::Const => {
                if next_keyword == Some(Keyword::Enum) && !next_is_on_new_line {
                    self.eat_keyword(Keyword::Const)?;
                    let enum_id = self.eat_enum(start, EnumKind::Const, header)?;
                    Ok(Some(self.insert_declaration_expression(start, enum_id)))
                } else {
                    Ok(Some(self.eat_let_from_keyword(
                        start,
                        header,
                        Keyword::Const,
                    )?))
                }
            }
            Keyword::Using => {
                if self.can_parse_using_declaration(&header, Asynchrony::Sync) {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Sync)?))
                } else {
                    Ok(None)
                }
            }
            Keyword::Await => {
                if self.flags.is_forbid_await() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                if self.can_start_await_using()
                    && self.can_parse_using_declaration(&header, Asynchrony::Async)
                {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Async)?))
                } else {
                    Ok(Some(self.eat_await()?))
                }
            }
            Keyword::Async if next_token_type == TokenType::Identifier && !next_is_on_new_line => {
                if next_keyword != Some(Keyword::Function) {
                    return Ok(None);
                }
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
        next_keyword: Option<Keyword>,
    ) -> bool {
        // this path only applies inside type expressions
        if !self.flags.is_in_type() {
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

        next_keyword == Some(Keyword::Is)
    }

    /// Eat a keyword-led primary expression in strict type space when possible.
    pub(super) fn eat_type_keyword_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        keyword: Keyword,
        next_token_type: TokenType,
        next_has_line_break: bool,
        next_keyword: Option<Keyword>,
        after_next_token_type: TokenType,
        is_declaration_start: bool,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // parse contextual keyword subjects like `override is X` as identifiers
        if self.keyword_parses_as_type_predicate_subject_identifier(
            keyword,
            next_token_type,
            next_keyword,
        ) {
            return Ok(None);
        }

        match keyword {
            // struct declaration
            Keyword::Struct
                if self.language.is_destack() && (is_declaration_start || next_has_line_break) =>
            {
                let allow_anonymous_class = !self.flags.is_in_statement_position()
                    || header.export == Some(ExportKind::Default);
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, struct_id),
                ))
            }

            // class declaration
            Keyword::Class if is_declaration_start || next_has_line_break => {
                let allow_anonymous_class = header.export == Some(ExportKind::Default)
                    || !self.flags.is_in_statement_position();
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, struct_id),
                ))
            }

            // enum declaration
            Keyword::Enum if is_declaration_start && !next_has_line_break => {
                let enum_id = self.eat_enum(start, EnumKind::Enum, header)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, enum_id),
                ))
            }
            Keyword::Enum => Err(ParseError::unexpected(self.peek()?.span)),

            // const enum declaration
            Keyword::Const if next_keyword == Some(Keyword::Enum) => {
                self.eat_keyword(Keyword::Const)?;
                let enum_id = self.eat_enum(start, EnumKind::Const, header)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, enum_id),
                ))
            }

            // newtype interface or alias declaration
            Keyword::Newtype => {
                // parse newtype interface declaration
                if next_keyword == Some(Keyword::Interface) {
                    self.eat_keyword(Keyword::Newtype)?;
                    let interface_id = self.eat_interface(start, header, TypeKind::Nominal)?;

                    return Ok(Some(
                        self.insert_declaration_type_expression(start, interface_id),
                    ));
                }

                // otherwise parse newtype alias declaration
                if !self.keyword_begins_type_form(
                    Keyword::Newtype,
                    next_token_type,
                    next_has_line_break,
                    next_keyword,
                    after_next_token_type,
                ) {
                    return Ok(None);
                }
                let type_expression_id = self.eat_type(start, header)?;

                Ok(Some(type_expression_id))
            }

            // interface declaration
            Keyword::Interface if is_declaration_start || next_has_line_break => {
                let interface_id = self.eat_interface(start, header, TypeKind::Structural)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, interface_id),
                ))
            }

            // extension declaration
            Keyword::Extension
                if self.language.is_destack()
                    && self.flags.is_in_statement_position()
                    && is_declaration_start =>
            {
                let extension_id = self.eat_extension(start, header)?;

                Ok(Some(
                    self.insert_declaration_type_expression(start, extension_id),
                ))
            }

            // async declaration
            Keyword::Async => {
                if next_has_line_break {
                    return Ok(None);
                }

                // avoid async generic parses in stronger infix contexts
                let has_generic_head = self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::LessThan)
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::ShiftLeft)
                });
                let has_stronger_infix_context =
                    self.flags.left_precedence.is_some_and(|left_precedence| {
                        left_precedence > OperatorPrecedence::Assignment as u16
                    });
                if has_generic_head && has_stronger_infix_context {
                    return Ok(None);
                }

                // avoid async generic parses when tree literal disambiguation is active
                if self.language.supports_jsx()
                    && self.flags.is_disallow_ambiguous_tree_literal()
                    && self.flags.left_precedence.is_some()
                    && has_generic_head
                {
                    return Ok(None);
                }

                // require a valid function signature start
                if !self.can_start_async_function_signature(next_token_type) {
                    return Ok(None);
                }
                let type_expression_id =
                    self.eat_function_type_expression(start, header, false, false)?;

                Ok(Some(type_expression_id))
            }

            // function or method declaration
            Keyword::Function | Keyword::Abstract | Keyword::Override => {
                let is_multiline_abstract_construct_signature =
                    keyword == Keyword::Abstract && next_keyword == Some(Keyword::New);
                if matches!(keyword, Keyword::Abstract | Keyword::Override)
                    && next_has_line_break
                    && !is_multiline_abstract_construct_signature
                {
                    return Ok(None);
                }

                // require a valid function signature start
                if !Self::can_start_function_signature(next_token_type) {
                    return Ok(None);
                }
                let type_expression_id =
                    self.eat_function_type_expression(start, header, false, false)?;

                Ok(Some(type_expression_id))
            }

            // new signature declaration
            Keyword::New => {
                if !Self::can_start_function_signature(next_token_type) {
                    return Ok(None);
                }
                let type_expression_id =
                    self.eat_function_type_expression(start, header, false, false)?;

                Ok(Some(type_expression_id))
            }

            // variant method declaration
            Keyword::Get | Keyword::Set | Keyword::Constructor if self.flags.is_in_variant() => {
                if !Self::can_start_function_signature(next_token_type) {
                    return Ok(None);
                }
                let type_expression_id =
                    self.eat_function_type_expression(start, header, false, false)?;

                Ok(Some(type_expression_id))
            }

            // this type
            Keyword::This => {
                self.bump(); // eat this

                Ok(Some(self.insert_node(
                    TypeExpression::This,
                    self.get_span_from(start),
                )))
            }

            // null literal type
            Keyword::Null => {
                self.bump(); // eat null

                Ok(Some(self.insert_node(
                    TypeExpression::Literal {
                        value: TypeLiteral::Null,
                    },
                    self.get_span_from(start),
                )))
            }

            // infer type expression
            Keyword::Infer => {
                let type_expression_id = self.eat_type_infer_expression()?;

                Ok(Some(type_expression_id))
            }

            // asserts type predicate
            Keyword::Asserts => {
                if !self.can_start_type_predicate_asserts() {
                    return Ok(None);
                }

                let type_expression_id = self.eat_type_predicate_asserts()?;

                Ok(Some(type_expression_id))
            }

            // readonly stays a type operator in typed type contexts
            Keyword::Readonly if self.language.is_typescript() || self.language.is_javascript() => {
                if self.flags.is_in_new_receiver() {
                    return Ok(None);
                }
                let type_expression_id = self.eat_type(start, header)?;

                Ok(Some(type_expression_id))
            }

            // type alias declaration and readonly aliases
            Keyword::Type | Keyword::Readonly => {
                if self.flags.is_in_for_each() {
                    return Ok(None);
                }

                if self.flags.is_in_new_receiver() {
                    return Ok(None);
                }

                if !self.keyword_begins_type_form(
                    keyword,
                    next_token_type,
                    next_has_line_break,
                    next_keyword,
                    after_next_token_type,
                ) {
                    return Ok(None);
                }
                let type_expression_id = self.eat_type(start, header)?;

                Ok(Some(type_expression_id))
            }

            _ => Ok(None),
        }
    }

    /// Eat a keyword-led expression when possible.
    ///
    /// Examples:
    /// ```
    /// async () => value
    /// function named() {}
    /// import.meta
    /// export { a, b }
    /// class Box<T> {}
    /// ```
    pub(super) fn eat_keyword_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        keyword: Keyword,
        next_token_type: TokenType,
        next_has_line_break: bool,
        next_keyword: Option<Keyword>,
        after_next_token_type: TokenType,
        is_declaration_start: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // parse contextual keyword subjects like `override is X` as identifiers
        if self.keyword_parses_as_type_predicate_subject_identifier(
            keyword,
            next_token_type,
            next_keyword,
        ) {
            return Ok(None);
        }

        match keyword {
            // final class declaration
            Keyword::Final
                if self.language.is_destack()
                    && !next_has_line_break
                    && next_keyword == Some(Keyword::Class) =>
            {
                let mut header = header;
                header.is_final = true;
                self.eat_keyword(Keyword::Final)?;

                let allow_anonymous_class = header.export == Some(ExportKind::Default)
                    || !self.flags.is_in_statement_position();
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            // struct declaration
            Keyword::Struct
                if self.language.is_destack() && (is_declaration_start || next_has_line_break) =>
            {
                let allow_anonymous_class = !self.flags.is_in_statement_position()
                    || header.export == Some(ExportKind::Default);
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            // class declaration
            Keyword::Class if is_declaration_start || next_has_line_break => {
                let allow_anonymous_class = header.export == Some(ExportKind::Default)
                    || !self.flags.is_in_statement_position();
                let struct_id = self.eat_struct_or_class(start, header, allow_anonymous_class)?;
                Ok(Some(self.insert_declaration_expression(start, struct_id)))
            }
            // enum declaration
            Keyword::Enum if is_declaration_start && !next_has_line_break => {
                let enum_id = self.eat_enum(start, EnumKind::Enum, header)?;
                Ok(Some(self.insert_declaration_expression(start, enum_id)))
            }
            Keyword::Enum => Err(ParseError::unexpected(self.peek()?.span)),
            // const enum or binding declaration
            Keyword::Const => {
                // parse const enum declaration
                if next_keyword == Some(Keyword::Enum) {
                    self.eat_keyword(Keyword::Const)?;
                    let enum_id = self.eat_enum(start, EnumKind::Const, header)?;
                    Ok(Some(self.insert_declaration_expression(start, enum_id)))
                // otherwise parse binding declaration
                } else {
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
                    self.eat_keyword(Keyword::Newtype)?;
                    let interface_id = self.eat_interface(start, header, TypeKind::Nominal)?;
                    Ok(Some(
                        self.insert_declaration_expression(start, interface_id),
                    ))
                // otherwise parse newtype alias declaration
                } else {
                    // parse the alias when it can start
                    if self.keyword_begins_type_form(
                        Keyword::Newtype,
                        next_token_type,
                        next_has_line_break,
                        next_keyword,
                        after_next_token_type,
                    ) {
                        let type_expression_id = self.eat_type(start, header)?;

                        Ok(Some(
                            self.insert_type_keyword_expression(type_expression_id),
                        ))
                    // otherwise bail
                    } else {
                        Ok(None)
                    }
                }
            }
            // interface declaration
            Keyword::Interface if is_declaration_start || next_has_line_break => {
                let interface_id = self.eat_interface(start, header, TypeKind::Structural)?;
                Ok(Some(
                    self.insert_declaration_expression(start, interface_id),
                ))
            }
            // extension declaration
            Keyword::Extension
                if self.language.is_destack()
                    && self.flags.is_in_statement_position()
                    && is_declaration_start =>
            {
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
                let has_generic_head = self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::LessThan)
                }) || self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::ShiftLeft)
                });
                let has_stronger_infix_context =
                    self.flags.left_precedence.is_some_and(|left_precedence| {
                        left_precedence > OperatorPrecedence::Assignment as u16
                    });
                if has_generic_head && has_stronger_infix_context {
                    return Ok(None);
                }

                // avoid async generic parses when tree literal disambiguation is active
                if self.language.supports_jsx()
                    && self.flags.is_disallow_ambiguous_tree_literal()
                    && self.flags.left_precedence.is_some()
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
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // override is contextual in value expressions
            Keyword::Override => Ok(None),
            // abstract is contextual outside declaration positions
            Keyword::Abstract
                if !self.flags.is_in_statement_position() && header.export.is_none() =>
            {
                Ok(None)
            }
            // function or method declaration
            Keyword::Function | Keyword::Abstract => {
                // abstract stays contextual across line breaks in value space
                if keyword == Keyword::Abstract && next_has_line_break {
                    return Ok(None);
                }

                // require a valid function signature start
                let can_start_signature = Self::can_start_function_signature(next_token_type);
                if !can_start_signature {
                    return Ok(None);
                }

                // parse function declaration
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // variant method declaration
            Keyword::Get | Keyword::Set | Keyword::Constructor if self.flags.is_in_variant() => {
                // require a valid function signature start
                let can_start_signature = Self::can_start_function_signature(next_token_type);
                if !can_start_signature {
                    return Ok(None);
                }

                // parse variant method declaration
                let function_id = self.eat_function(start, header, false, false)?;
                Ok(Some(self.insert_declaration_expression(start, function_id)))
            }
            // this expression
            Keyword::This => {
                self.bump(); // eat this

                Ok(Some(
                    self.tree
                        .insert(Expression::This, self.get_span_from(start)),
                ))
            }
            // super expression
            Keyword::Super => {
                self.bump(); // eat super
                Ok(Some(
                    self.tree
                        .insert(Expression::Super, self.get_span_from(start)),
                ))
            }
            // null literal
            Keyword::Null => {
                self.bump(); // eat null

                Ok(Some(self.insert_node(
                    Expression::ScalarLiteral(ScalarLiteral::Null),
                    self.get_span_from(start),
                )))
            }
            // new expression
            Keyword::New => {
                // allow one committed or recoverable constructor slot
                let can_start_new_expression = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::OpenBrace
                        | TokenType::LessThan
                ) || next_has_line_break
                    || Self::is_expression_slot_boundary_token(next_token_type);
                if can_start_new_expression {
                    // parse new expression
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
            // import declaration or import meta
            Keyword::Import => {
                // special import member forms
                if self.keyword_member_access_is_any(&["meta"], true)? {
                    return Ok(Some(self.eat_import_meta_expression(start)?));
                }
                if self.keyword_member_access_is_any(&["source"], true)? {
                    return Ok(Some(self.eat_import_source_expression(start)?));
                }
                if self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Dot)
                }) {
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

                Ok(Some(self.eat_import()?))
            }
            // let binding declaration
            Keyword::Let => Ok(Some(self.eat_let_from_keyword(start, header, keyword)?)),
            // using declaration
            Keyword::Using => {
                if self.can_parse_using_declaration(&header, Asynchrony::Sync) {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Sync)?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // readonly stays a type operator in typed type contexts
            Keyword::Readonly if self.language.is_typescript() || self.language.is_javascript() => {
                // in new receiver context, readonly behaves like an identifier
                if self.flags.is_in_new_receiver() {
                    return Ok(None);
                }

                // value positions keep readonly contextual
                Ok(None)
            }
            // type alias declaration and readonly or newtype aliases
            Keyword::Type | Keyword::Readonly => {
                // for each bindings keep `type` and `readonly` as identifiers
                if self.flags.is_in_for_each() {
                    return Ok(None);
                }

                // in new receiver context, `type` and `readonly` behave like identifiers
                if self.flags.is_in_new_receiver() {
                    return Ok(None);
                }

                if keyword == Keyword::Type && next_has_line_break {
                    return Ok(None);
                }

                // parse type alias when it can start
                if self.keyword_begins_type_form(
                    Keyword::Type,
                    next_token_type,
                    next_has_line_break,
                    next_keyword,
                    after_next_token_type,
                ) {
                    let type_expression_id = self.eat_type(start, header)?;

                    Ok(Some(
                        self.insert_type_keyword_expression(type_expression_id),
                    ))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // if
            Keyword::If => Ok(Some(self.eat_if()?)),
            // while
            Keyword::While => Ok(Some(self.eat_while()?)),
            // do while
            Keyword::Do if self.is_do_while_statement(next_token_type) => {
                Ok(Some(self.eat_while()?))
            }
            // for
            Keyword::For => Ok(Some(self.eat_for()?)),
            // loop
            Keyword::Loop if self.language.is_destack() && self.is_next_block_start() => {
                Ok(Some(self.eat_loop()?))
            }
            // try
            Keyword::Try => Ok(Some(self.eat_try()?)),
            // switch
            Keyword::Switch => Ok(Some(self.eat_match()?)),
            // match
            Keyword::Match if self.language.is_destack() => Ok(Some(self.eat_match()?)),
            // break
            Keyword::Break => Ok(Some(self.eat_break()?)),
            // continue
            Keyword::Continue => Ok(Some(self.eat_continue()?)),
            // await expression or await using
            Keyword::Await => {
                // reject await in contexts that forbid it
                if self.flags.is_forbid_await() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // parse await using when allowed
                if self.can_start_await_using()
                    && self.can_parse_using_declaration(&header, Asynchrony::Async)
                {
                    Ok(Some(self.eat_using(start, header, Asynchrony::Async)?))
                // otherwise parse await expression
                } else {
                    Ok(Some(self.eat_await()?))
                }
            }
            // comptime expression
            Keyword::Comptime if self.language.is_destack() => Ok(Some(self.eat_comptime()?)),
            // yield statement
            Keyword::Yield if self.flags.is_in_generator() => Ok(Some(self.eat_yield()?)),
            // throw statement
            Keyword::Throw => Ok(Some(self.eat_throw()?)),
            // return statement
            Keyword::Return => Ok(Some(self.eat_return()?)),
            // debugger statement
            Keyword::Debugger => {
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
