use crate::parse::{
    DeclarationNesting, ExpressionPosition, ExpressionStop, TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    Access, Asynchrony, BlockContext, Declarator, Expression, Keyword, LetKind, LocalNodeId,
    Mutability, NodeType, OperatorPrecedence, Pattern, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};

use super::DeclarationHeader;

/// Value requirements for one declarator.
#[derive(Debug, Copy, Clone)]
pub(crate) enum DeclaratorValue {
    /// Allow the declarator to omit its value.
    Optional,
    /// Require a value parsed above the given enclosing precedence.
    Required(OperatorPrecedence),
}

/// The declaration form represented by one let-like keyword.
#[derive(Debug, Copy, Clone)]
pub(super) struct LetHead {
    /// The declaration kind.
    pub(super) kind: LetKind,
    /// The binding mutability.
    pub(super) mutability: Mutability,
}

impl LetHead {
    /// Classify one let-like declaration keyword.
    pub(super) const fn from_keyword(keyword: Keyword) -> Option<Self> {
        match keyword {
            Keyword::Let => Some(Self {
                kind: LetKind::Let,
                mutability: Mutability::Mutable,
            }),
            Keyword::Const | Keyword::Readonly => Some(Self {
                kind: LetKind::Const,
                mutability: Mutability::Immutable,
            }),
            _ => None,
        }
    }
}

impl Parser {
    /// Parse a let or const binding.
    ///
    /// Examples:
    /// ```ds
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
    pub(crate) fn parse_let(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let keyword_range = self.peek_token_span().span.range();
        let head = self.parse_let_head()?;

        self.parse_let_declarators(start, header, head, keyword_range)
    }

    /// Parse a using binding (incl. `using` keyword and optional `await`).
    ///
    /// Examples:
    /// ```ds
    /// using file = openFile(path)
    /// await using conn = openConnection()
    /// using a = openA(), b = openB()
    /// ```
    pub(crate) fn parse_using(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        asynchrony: Asynchrony,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // optional await
        if asynchrony == Asynchrony::Async {
            self.eat_keyword(Keyword::Await)?;
        }

        // using keyword
        self.eat_keyword(Keyword::Using)?;

        // recover the missing binding at the declaration boundary
        if self.peek_declaration_boundary(DeclarationNesting::None) {
            return Ok(self.recover_missing_binding(start));
        }

        // parse declarators
        let mut declarators = Vec::new();
        loop {
            let declarator_id =
                self.parse_declarator(DeclaratorValue::Required(OperatorPrecedence::Lowest))?;
            declarators.push(declarator_id);

            if self.peek_is(TokenType::Comma) {
                self.bump();
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
            self.range_since(start),
        );

        Ok(using_id)
    }

    /// Return true when an async or sync using head starts at the current position.
    #[inline]
    pub(crate) fn peek_using(&self, asynchrony: Asynchrony) -> bool {
        if asynchrony == Asynchrony::Async {
            return self.peek_is_keyword(Keyword::Await)
                && self.peek_keyword_at(1) == Some(Keyword::Using)
                && !self.peek_token_at(1).is_on_new_line();
        }

        self.peek_is_keyword(Keyword::Using)
    }

    /// Return the offset of the first binding token after `using`.
    #[inline]
    pub(crate) fn peek_using_binding_head_offset(&self, asynchrony: Asynchrony) -> Option<usize> {
        if asynchrony == Asynchrony::Async {
            if !self.peek_is_keyword(Keyword::Await)
                || self.peek_keyword_at(1) != Some(Keyword::Using)
            {
                return None;
            }

            return Some(2);
        }

        self.peek_is_keyword(Keyword::Using).then_some(1)
    }

    /// Return the first declarator token after `using` when it stays on the same line.
    #[inline]
    pub(crate) fn peek_using_binding_head_token(
        &self,
        asynchrony: Asynchrony,
    ) -> Option<TokenType> {
        let offset = self.peek_using_binding_head_offset(asynchrony)?;
        let token = self.peek_token_at(offset);

        (!token.is_on_new_line()).then_some(token.ty())
    }

    /// Return true when a token can start a `using` binding pattern.
    #[inline]
    pub(crate) const fn can_start_using_binding_pattern(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Identifier
                | TokenType::OpenParenthesis
                | TokenType::OpenBrace
                | TokenType::OpenBracket
        )
    }

    /// Parse declarators for a consumed let-like keyword.
    fn parse_let_declarators(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        head: LetHead,
        keyword_range: ByteRange,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // recover the missing binding at the declaration boundary
        if self.peek_declaration_boundary(DeclarationNesting::None) {
            return Ok(self.recover_missing_binding(start));
        }

        let first_declarator = self.parse_declarator(DeclaratorValue::Optional)?;

        // let else
        if self.peek_is_keyword(Keyword::Else) {
            if header.export.is_some() || header.is_ambient {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let declarator = self.tree.get(first_declarator);
            if declarator.value.is_none() {
                return Err(ParserError::expected(
                    self.peek_token_span(),
                    TokenType::Assign,
                ));
            }

            let else_range = self.eat_keyword(Keyword::Else)?.range();

            // else { ... }
            if !self.peek_is(TokenType::OpenBrace) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let else_branch = {
                let branch_start = self.mark_parse_start();
                let else_block = self.parse_block(BlockContext::Statement)?;

                self.insert_node(
                    Expression::Block(else_block),
                    self.range_since(&branch_start),
                )
            };

            let let_else_id = self.insert_node(
                Expression::LetElse {
                    kind: head.kind,
                    mutability: head.mutability,
                    declarator: first_declarator,
                    else_branch,
                },
                self.range_since(start),
            );
            self.tree.set_main_range(let_else_id, keyword_range);
            self.tree.set_side_range(
                let_else_id,
                NodeSpanType::Region(NodeSpanRegion::Else),
                else_range,
            );

            return Ok(let_else_id);
        }

        // rest of declarators for regular let
        let mut declarators = vec![first_declarator];
        loop {
            if self.peek_is(TokenType::Comma) {
                self.bump();
                let declarator_id = self.parse_declarator(DeclaratorValue::Optional)?;
                declarators.push(declarator_id);
                continue;
            }

            if !self.peek_declarator_statement_boundary() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            break;
        }

        let let_id = self.insert_node(
            Expression::Let {
                kind: head.kind,
                export: header.export,
                is_ambient: header.is_ambient,
                place: header.place,
                mutability: head.mutability,
                declarators,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(let_id, keyword_range);

        Ok(let_id)
    }

    /// Recover an absent binding at the next declaration boundary.
    fn recover_missing_binding(&mut self, start: &ParseStart) -> LocalNodeId<Expression> {
        self.report_unexpected_here(NodeType::Declarator);

        self.insert_node(Expression::Error, self.range_since(start))
    }

    /// Parse a let or const head.
    pub(super) fn parse_let_head(&mut self) -> ParserResult<LetHead> {
        let keyword = self.peek_any_keyword()?;
        if let Some(head) = LetHead::from_keyword(keyword) {
            self.bump();
            Ok(head)
        } else {
            Err(ParserError::expected(
                self.require_token(TokenType::Identifier)?.range(),
                TokenType::Identifier,
            ))
        }
    }

    /// Parse a reference mutability modifier, defaulting to mutable.
    pub(crate) fn parse_reference_mutability(&mut self) -> Mutability {
        let Ok(keyword) = self.peek_any_keyword() else {
            return Mutability::Mutable;
        };

        // readonly
        if keyword == Keyword::Readonly || keyword == Keyword::Const {
            self.bump();
            Mutability::Immutable
        }
        // mutable by default
        else {
            Mutability::Mutable
        }
    }

    /// Parse one borrow access modifier, defaulting to mutable.
    pub(crate) fn parse_borrow_access(&mut self) -> ParserResult<Access> {
        let access = match self.peek_keyword() {
            Some(Keyword::Readonly | Keyword::Const) => Access::Readonly,
            Some(Keyword::Immutable) => Access::Immutable,
            Some(Keyword::Exclusive) => Access::Exclusive,
            _ => return Ok(Access::Mutable),
        };
        self.bump();

        // reject a second access modifier
        if matches!(
            self.peek_keyword(),
            Some(Keyword::Readonly | Keyword::Const | Keyword::Immutable | Keyword::Exclusive)
        ) {
            return Err(ParserError::unexpected(self.peek_token()));
        }

        Ok(access)
    }

    /// Parse a single declarator with an optional value unless `require_value` is set.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// value = 1
    /// { x, y }: Point = point
    /// [head, ...tail] = values
    /// readonly buffer: Buffer
    /// ```
    pub(super) fn parse_declarator(
        &mut self,
        value: DeclaratorValue,
    ) -> ParserResult<LocalNodeId<Declarator>> {
        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();

        // recognize identifier heads that cannot continue into richer patterns
        let parses_plain_binding = if self.peek_is(TokenType::Identifier) {
            let peek_next_token_type = self.peek_token_type_at(1);
            let has_binding_boundary = self.peek_token_at_is_on_new_line(1)
                || matches!(
                    peek_next_token_type,
                    TokenType::Colon
                        | TokenType::Assign
                        | TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseBrace
                        | TokenType::CloseParenthesis
                        | TokenType::CloseBracket
                        | TokenType::End
                );

            // reserve mutability markers and the Destack wildcard for pattern parsing
            let keyword = self.peek_keyword();
            let is_mutability_keyword = matches!(keyword, Some(Keyword::Const | Keyword::Let))
                || keyword == Some(Keyword::Readonly);
            let is_wildcard = self.peek_identifier_is("_");

            has_binding_boundary && !is_mutability_keyword && !is_wildcard
        } else {
            false
        };

        // parse the selected binding or pattern form
        let pattern_id = if parses_plain_binding {
            let (name, name_range) = self.eat_binding_identifier_with_range()?;
            let pattern_id = self.insert_node(
                Pattern::Binding {
                    name,
                    pattern: None,
                },
                self.range_since(&start),
            );
            self.tree.set_main_range(pattern_id, name_range);

            pattern_id
        } else {
            self.parse_pattern_before_type()?
        };
        let is_must_pattern = matches!(self.tree.get(pattern_id), Pattern::Must(_));

        // type
        let (ty, type_range) = if self.peek_is(TokenType::Colon) {
            let type_start = self.mark_parse_start();
            self.bump();
            let ty = self.parse_type_or_recover_missing(
                TypePosition::Type,
                TypeStop::default(),
                NodeType::Declarator,
            )?;
            (Some(ty), Some(self.range_since(&type_start)))
        } else {
            (None, None)
        };

        // value
        let (value, value_operator_range) = if self.peek_is(TokenType::Assign) {
            let operator_start = self.mark_parse_start();
            self.bump();
            let operator_range = self.range_since(&operator_start);

            let minimum_precedence = match value {
                DeclaratorValue::Optional => OperatorPrecedence::Lowest,
                DeclaratorValue::Required(precedence) => precedence,
            };
            let position = ExpressionPosition::NestedStatement;
            let stop = ExpressionStop::NEWLINE_CALL;
            let value = if self.peek_expression_slot_boundary() {
                self.recover_missing_expression_here(NodeType::Declarator)
            } else {
                self.parse_expression_at(position, stop, minimum_precedence)?
            };

            (Some(value), Some(operator_range))
        } else if is_must_pattern || matches!(value, DeclaratorValue::Required(_)) {
            return Err(ParserError::expected(
                self.peek_token_span(),
                TokenType::Assign,
            ));
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
            self.range_since(&start),
        );

        // set the type source range for the type annotation
        if let Some(range) = type_range {
            self.tree.set_side_range(
                declarator_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                range,
            );
        }

        // value operator
        if let Some(range) = value_operator_range {
            self.tree.set_main_range(declarator_id, range);
        }

        self.attach_documentation(declarator_id, documentation);

        Ok(declarator_id)
    }

    /// Return true when the current token can terminate a declaration statement.
    fn peek_declarator_statement_boundary(&self) -> bool {
        self.peek_statement_stop()
            || self.peek_is_on_new_line()
            || self.peek_is(TokenType::CloseBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || self.peek_is_keyword(Keyword::Else)
    }
}
