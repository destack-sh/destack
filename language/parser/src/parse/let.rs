use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Asynchrony, BlockContext, Declarator, Expression, Keyword, LetKind, LocalNodeId, Mutability,
    NodeType, Pattern, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::DeclarationHeader;

impl Parser {
    /// Eat a let or const binding.
    ///
    /// Examples:
    /// ```
    /// const x = 1
    /// const x: int32 = 1
    /// let a: T1 = v1, b: T2  // multiple declarators
    ///
    /// const Some(x) = someFunction()
    /// const t = foo() ?? return;
    ///
    /// if (const Some(x) = someFunction()) {
    ///     ...
    /// }
    /// ```
    pub(crate) fn eat_let(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let (kind, mutability) = self.eat_let_kind()?;
        self.eat_let_after_keyword(start, header, kind, mutability)
    }

    /// Eat a using binding (incl. `using` keyword and optional `await`).
    ///
    /// Examples:
    /// ```
    /// using file = openFile(path)
    /// await using conn = openConnection()
    /// using a = openA(), b = openB()
    /// ```
    pub(crate) fn eat_using(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        asynchrony: Asynchrony,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // optional await
        if asynchrony == Asynchrony::Async {
            self.eat_keyword(Keyword::Await)?;
        }

        // using keyword
        self.eat_keyword(Keyword::Using)?;

        // parse declarators
        let mut declarators = Vec::new();
        loop {
            let declarator_id = self.eat_declarator(true, false)?;
            declarators.push(declarator_id);

            if self.peek_is(TokenType::Comma) || self.next_token_type() == TokenType::Comma {
                self.bump(); // eat comma
                continue;
            }

            break;
        }

        let using_id = self.insert_node(
            Expression::Using {
                asynchrony,
                export: header.export,
                is_ambient: header.is_ambient,
                declarators,
            },
            self.get_span_from(start),
        );

        Ok(using_id)
    }

    /// Return true when an async or sync using head starts at the current position.
    #[inline]
    pub(crate) fn using_keyword_is(&mut self, asynchrony: Asynchrony) -> bool {
        if asynchrony == Asynchrony::Async {
            return self.is_keyword(Keyword::Await)
                && self.keyword_at_offset(1) == Some(Keyword::Using)
                && !self.token_at_offset(1).token.is_on_new_line();
        }

        self.is_keyword(Keyword::Using)
    }

    /// Return the offset of the first binding token after `using`.
    #[inline]
    pub(crate) fn using_binding_head_offset(&mut self, asynchrony: Asynchrony) -> Option<usize> {
        if asynchrony == Asynchrony::Async {
            if !self.is_keyword(Keyword::Await) || self.keyword_at_offset(1) != Some(Keyword::Using)
            {
                return None;
            }

            return Some(2);
        }

        self.is_keyword(Keyword::Using).then_some(1)
    }

    /// Return the first declarator token after `using` when it stays on the same line.
    #[inline]
    pub(crate) fn using_binding_head_token(&mut self, asynchrony: Asynchrony) -> Option<TokenType> {
        let offset = self.using_binding_head_offset(asynchrony)?;
        let token = self.token_at_offset(offset);

        (!token.token.is_on_new_line()).then_some(token.token.ty())
    }

    /// Return true when a token can start a `using` binding pattern.
    #[inline]
    pub(crate) fn token_can_start_using_binding_pattern(&self, token_type: TokenType) -> bool {
        if self.language.is_destack() {
            return matches!(
                token_type,
                TokenType::Identifier
                    | TokenType::OpenParenthesis
                    | TokenType::OpenBrace
                    | TokenType::OpenBracket
            );
        }

        matches!(
            token_type,
            TokenType::Identifier | TokenType::OpenBrace | TokenType::OpenBracket
        )
    }

    /// Return let kind and mutability for a declaration keyword.
    #[inline]
    fn let_kind_and_mutability_for_keyword(keyword: Keyword) -> Option<(LetKind, Mutability)> {
        match keyword {
            Keyword::Let => Some((LetKind::Let, Mutability::Mutable)),
            Keyword::Const | Keyword::Readonly => Some((LetKind::Const, Mutability::Immutable)),
            _ => None,
        }
    }

    /// Parse declarators for a consumed let-like keyword.
    fn eat_let_after_keyword(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        kind: LetKind,
        mutability: Mutability,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let first_declarator = self.eat_declarator(false, true)?;

        // let else
        if self.is_keyword(Keyword::Else) {
            if header.export.is_some() || header.is_ambient {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            let declarator = self.tree.get(first_declarator);
            if declarator.value.is_none() {
                return Err(ParserError::expected(self.peek()?.span, TokenType::Assign));
            }

            let else_span = self.eat_keyword(Keyword::Else)?.span;

            // else { ... }
            if !self.peek_is(TokenType::OpenBrace) {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            let else_branch = {
                let branch_start = self.span_start();
                let else_block = self.eat_block(BlockContext::Statement)?;

                self.insert_node(
                    Expression::Block(else_block),
                    self.get_span_from(&branch_start),
                )
            };

            let let_else_id = self.insert_node(
                Expression::LetElse {
                    kind,
                    mutability,
                    declarator: first_declarator,
                    else_branch,
                },
                self.get_span_from(start),
            );
            self.tree.set_side_span(
                let_else_id,
                NodeSpanType::Region(NodeSpanRegion::Clause),
                else_span,
            );

            return Ok(let_else_id);
        }

        // rest of declarators for regular let
        let mut declarators = vec![first_declarator];
        loop {
            if self.peek_is(TokenType::Comma) || self.next_token_type() == TokenType::Comma {
                self.bump(); // eat comma
                let declarator_id = self.eat_declarator(false, false)?;
                declarators.push(declarator_id);
                continue;
            }

            if !self.declarator_has_statement_boundary() {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            break;
        }

        let let_id = self.insert_node(
            Expression::Let {
                kind,
                export: header.export,
                is_ambient: header.is_ambient,
                is_shared: header.is_shared,
                mutability,
                declarators,
            },
            self.get_span_from(start),
        );

        Ok(let_id)
    }

    /// Eat a let or const keyword and return the kind and mutability.
    pub fn eat_let_kind(&mut self) -> ParserResult<(LetKind, Mutability)> {
        let keyword = self.peek_any_keyword()?;
        if let Some((kind, mutability)) = Self::let_kind_and_mutability_for_keyword(keyword) {
            self.bump();
            Ok((kind, mutability))
        } else {
            Err(ParserError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a let-like binding when the caller already resolved the keyword.
    pub(crate) fn eat_let_from_keyword(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        keyword: Keyword,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if Self::let_kind_and_mutability_for_keyword(keyword).is_none() {
            return Err(ParserError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ));
        }

        self.eat_let(start, header)
    }

    /// Eat a reference mutability modifier, defaulting to mutable.
    pub fn eat_reference_mutability_maybe(&mut self) -> ParserResult<Option<Mutability>> {
        let Ok(keyword) = self.peek_any_keyword() else {
            return Ok(Some(Mutability::Mutable));
        };

        // readonly
        if keyword == Keyword::Readonly || keyword == Keyword::Const {
            self.bump(); // eat readonly
            Ok(Some(Mutability::Immutable))
        }
        // exclusive
        else if keyword == Keyword::Exclusive {
            self.bump(); // eat exclusive
            Ok(Some(Mutability::Exclusive))
        }
        // mutable by default
        else {
            Ok(Some(Mutability::Mutable))
        }
    }

    /// Eat a single declarator with an optional value unless `require_value` is set.
    ///
    /// Examples:
    /// ```
    /// value
    /// value = 1
    /// { x, y }: Point = point
    /// [head, ...tail] = values
    /// readonly buffer: Buffer
    /// ```
    pub(super) fn eat_declarator(
        &mut self,
        require_value: bool,
        allow_match_pattern: bool,
    ) -> ParserResult<LocalNodeId<Declarator>> {
        let start = self.span_start();
        let pattern_flags = self
            .flags
            .not_in_position()
            .in_before_type()
            .not_in_before_block();

        // pattern
        let pattern_id = if self.peek_is(TokenType::Identifier) {
            // simple binding heads are decided by the next visible token
            let next_token_type = self.token_type_at_offset(1);
            let can_use_simple_let_path = self.token_at_offset_has_leading_line_break(1)
                || matches!(
                    next_token_type,
                    TokenType::Colon
                        | TokenType::Assign
                        | TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseBrace
                        | TokenType::CloseParenthesis
                        | TokenType::CloseBracket
                        | TokenType::End
                );
            if can_use_simple_let_path {
                let keyword = self.current_keyword();
                let is_mutability_keyword = matches!(keyword, Some(Keyword::Const | Keyword::Let))
                    || self.language.is_destack() && keyword == Some(Keyword::Readonly);
                let allow_underscore_binding =
                    self.language.is_javascript() || self.language.is_typescript();
                let is_underscore_identifier = if allow_underscore_binding {
                    false
                } else {
                    self.current_identifier_str_is("_")
                };
                if !is_mutability_keyword && (!is_underscore_identifier || allow_underscore_binding)
                {
                    let (name, name_span) = self.eat_binding_identifier_with_span()?;
                    let pattern_id = self.insert_node(
                        Pattern::Binding {
                            name,
                            pattern: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(pattern_id, name_span);
                    pattern_id
                } else if self.flags == pattern_flags {
                    self.eat_pattern()?
                } else {
                    let old_flags = self.swap_flags(pattern_flags);
                    let pattern_result = self.eat_pattern();
                    self.restore_flags(old_flags);
                    pattern_result?
                }
            } else if self.flags == pattern_flags {
                self.eat_pattern()?
            } else {
                let old_flags = self.swap_flags(pattern_flags);
                let pattern_result = self.eat_pattern();
                self.restore_flags(old_flags);
                pattern_result?
            }
        } else if self.flags == pattern_flags {
            self.eat_pattern()?
        } else {
            let old_flags = self.swap_flags(pattern_flags);
            let pattern_result = self.eat_pattern();
            self.restore_flags(old_flags);
            pattern_result?
        };

        // non Destack declaration declarators must use plain binding patterns
        if !allow_match_pattern
            && !self.language.is_destack()
            && !self.declarator_pattern_is_valid_binding(pattern_id)
        {
            return Err(ParserError::unexpected(self.tree.get_span(pattern_id)));
        }

        // type
        let (ty, ty_span) = if self.peek_colon_is() {
            let type_start = self.span_start();
            self.bump(); // eat colon
            let type_flags = self.flags.not_in_position().in_type();
            let ty =
                self.eat_type_expression_or_recover_missing(type_flags, NodeType::Declarator)?;
            (Some(ty), Some(self.get_span_from(&type_start)))
        } else {
            (None, None)
        };

        // value
        let (value, value_operator_span) =
            if self.peek_is(TokenType::Assign) || self.next_token_type() == TokenType::Assign {
                let operator_start = self.span_start();
                self.bump(); // eat assign
                let operator_span = self.get_span_from(&operator_start);

                let value_flags = self.flags.not_in_position().not_in_sequence_expression();
                let value =
                    self.eat_expression_or_recover_missing(value_flags, NodeType::Declarator)?;

                (Some(value), Some(operator_span))
            } else if require_value {
                return Err(ParserError::expected(self.peek()?.span, TokenType::Assign));
            } else {
                (None, None)
            };

        // declarator
        let declarator_id = self.insert_node(
            Declarator {
                pattern: pattern_id,
                ty,
                value,
            },
            self.get_span_from(&start),
        );

        // set type span for the type annotation
        if let Some(span) = ty_span {
            self.tree.set_side_span(
                declarator_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
        }

        // value operator
        if let Some(span) = value_operator_span {
            self.tree.set_main_span(declarator_id, span);
        }

        Ok(declarator_id)
    }

    /// Return true when the current token can terminate a declaration statement.
    fn declarator_has_statement_boundary(&mut self) -> bool {
        self.is_statement_stop()
            || self.current_token_is_on_new_line()
            || self.peek_is(TokenType::CloseBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || self.is_keyword(Keyword::Else)
    }

    /// Return true when a declarator pattern is a valid binding.
    fn declarator_pattern_is_valid_binding(&self, pattern_id: LocalNodeId<Pattern>) -> bool {
        match self.tree.get(pattern_id) {
            Pattern::Expression { value } => self.declarator_expression_is_valid_binding(*value),
            Pattern::TypeExpression { .. } => false,
            _ => true,
        }
    }

    /// Return true when an expression is a valid declarator binding.
    fn declarator_expression_is_valid_binding(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(self.tree.get(expression_id), Expression::Identifier { .. })
    }
}
