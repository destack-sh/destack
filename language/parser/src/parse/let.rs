use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_ast::{
    Asynchrony, BlockContext, Declarator, Expression, Keyword, LetKind, LocalNodeId, Mutability,
    NodeType, Pattern, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::expression::common::DeclarationHeader;

impl Parser {
    /// Return true when an async or sync using head starts at the current position.
    #[inline]
    pub(crate) fn using_keyword_is(&mut self, asynchrony: Asynchrony) -> bool {
        if asynchrony == Asynchrony::Async {
            if !self.is_keyword(Keyword::Await) {
                return false;
            }

            return self.lookahead(|parser| {
                parser.bump();
                !parser.current_token().token.is_on_new_line && parser.is_keyword(Keyword::Using)
            });
        }

        self.is_keyword(Keyword::Using)
    }

    /// Return the first declarator token after `using` when it stays on the same line.
    #[inline]
    pub(crate) fn using_binding_head_token(&mut self, asynchrony: Asynchrony) -> Option<TokenType> {
        self.lookahead(|parser| {
            if asynchrony == Asynchrony::Async {
                parser.bump();
            }
            if !parser.is_keyword(Keyword::Using) {
                return None;
            }

            parser.bump();
            if parser.current_token().token.is_on_new_line {
                return None;
            }

            Some(parser.peek_token_type())
        })
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
            Keyword::Var => Some((LetKind::Var, Mutability::Mutable)),
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
    ) -> ParseResult<LocalNodeId<Expression>> {
        let first_declarator = self.eat_declarator(false, false)?;

        // let else
        if self.is_keyword(Keyword::Else) {
            if header.export.is_some() || header.is_ambient {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let declarator = self.tree.get(first_declarator);
            if declarator.value.is_none() {
                return Err(ParseError::expected(self.peek()?.span, TokenType::Assign));
            }

            let else_span = self.eat_keyword(Keyword::Else)?.span;

            // else { ... }
            if !self.peek_is(TokenType::OpenBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
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
            if self.eat_declarator_separator_maybe()? {
                let declarator_id = self.eat_declarator(false, false)?;
                declarators.push(declarator_id);
                continue;
            }

            if !self.declarator_has_statement_boundary() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            break;
        }

        let let_id = self.insert_node(
            Expression::Let {
                kind,
                export: header.export,
                is_ambient: header.is_ambient,
                mutability,
                declarators,
            },
            self.get_span_from(start),
        );

        Ok(let_id)
    }

    /// Peek a mutability modifier.
    pub fn peek_mutability(&mut self) -> ParseResult<()> {
        if self.peek_mutability_is() {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Return true when the next token is a mutability modifier.
    #[inline]
    pub fn peek_mutability_is(&mut self) -> bool {
        let Ok(keyword) = self.peek_any_keyword() else {
            return false;
        };

        keyword == Keyword::Var || keyword == Keyword::Const || keyword == Keyword::Readonly
    }

    /// Eat a let/var/const keyword and return the kind and mutability.
    pub fn eat_let_kind(&mut self) -> ParseResult<(LetKind, Mutability)> {
        let keyword = self.peek_any_keyword()?;
        if let Some((kind, mutability)) = Self::let_kind_and_mutability_for_keyword(keyword) {
            self.bump();
            Ok((kind, mutability))
        } else {
            Err(ParseError::expected(
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
    ) -> ParseResult<LocalNodeId<Expression>> {
        let Some((kind, mutability)) = Self::let_kind_and_mutability_for_keyword(keyword) else {
            return Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ));
        };

        self.bump();
        self.eat_let_after_keyword(start, header, kind, mutability)
    }

    /// Eat a mutability modifier.
    pub fn eat_mutability(&mut self) -> ParseResult<Mutability> {
        let (_, mutability) = self.eat_let_kind()?;
        Ok(mutability)
    }

    /// Eat a mutability modifier maybe.
    pub fn eat_mutability_maybe(&mut self) -> ParseResult<Option<Mutability>> {
        let Ok(keyword) = self.peek_any_keyword() else {
            return Ok(None);
        };
        // mutable
        if keyword == Keyword::Var {
            self.bump(); // eat mutability
            Ok(Some(Mutability::Mutable))
        }
        // immutable
        else if keyword == Keyword::Let || keyword == Keyword::Const {
            self.bump(); // eat mutability
            Ok(Some(Mutability::Immutable))
        }
        // nothing
        else {
            Ok(None)
        }
    }

    /// Eat a reference mutability modifier, defaulting to mutable.
    pub fn eat_reference_mutability_maybe(&mut self) -> ParseResult<Option<Mutability>> {
        let Ok(keyword) = self.peek_any_keyword() else {
            return Ok(Some(Mutability::Mutable));
        };

        // readonly
        if keyword == Keyword::Readonly || keyword == Keyword::Const {
            self.bump(); // eat readonly
            Ok(Some(Mutability::Immutable))
        }
        // mutable by default
        else {
            Ok(Some(Mutability::Mutable))
        }
    }

    /// Eat a let or var binding (incl. `let` or `var` keyword).
    ///
    /// Examples:
    /// ```
    /// const x = 1
    /// const x: int32 = 1
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// let a: T1 = v1, b: T2  // multiple declarators
    ///
    /// const Some(x) = someFunction()
    /// var Point { x, .. } = someFunction()
    /// const t = foo() ?? return;
    ///
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// ```
    #[cfg(test)]
    pub(crate) fn eat_let(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Expression>> {
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
    ) -> ParseResult<LocalNodeId<Expression>> {
        // optional await
        if asynchrony == Asynchrony::Async {
            self.eat_keyword(Keyword::Await)?;
        }

        // using keyword
        self.eat_keyword(Keyword::Using)?;

        // parse declarators (comma-separated list)
        let mut declarators = Vec::new();
        loop {
            let declarator_id = self.eat_declarator(true, false)?;
            declarators.push(declarator_id);

            // continue when a comma follows, even after line terminators
            if self.eat_declarator_separator_maybe()? {
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
    ) -> ParseResult<LocalNodeId<Declarator>> {
        let start = self.span_start();
        let pattern_flags = self
            .flags
            .not_in_position()
            .in_before_type()
            .not_in_before_block();

        // pattern
        let pattern_id = if self.peek_is(TokenType::Identifier) {
            // simple path for simple binding patterns
            let can_use_simple_let_path = self.lookahead(|parser| {
                parser.bump();
                parser.current_token_is_on_new_line()
                    || matches!(
                        parser.peek_token_type(),
                        TokenType::Colon
                            | TokenType::Assign
                            | TokenType::Comma
                            | TokenType::Semicolon
                            | TokenType::CloseBrace
                            | TokenType::CloseParenthesis
                            | TokenType::CloseBracket
                            | TokenType::End
                    )
            });
            if can_use_simple_let_path {
                let keyword = self.current_keyword();
                let is_mutability_keyword =
                    matches!(keyword, Some(Keyword::Var | Keyword::Const | Keyword::Let))
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
                            mutability: None,
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

        // declaration declarators must use binding patterns
        if !allow_match_pattern && !self.declarator_pattern_is_valid_binding(pattern_id) {
            return Err(ParseError::unexpected(self.tree.get_span(pattern_id)));
        }

        // type
        let (ty, ty_span) = if self.peek_colon_is() {
            let type_start = self.span_start();
            self.bump(); // eat colon
            let type_flags = self.flags.not_in_position().in_type();
            let ty =
                self.eat_type_expression_node_or_recover_missing(type_flags, NodeType::Declarator)?;
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
                return Err(ParseError::expected(self.peek()?.span, TokenType::Assign));
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

    /// Eat one declarator separator comma maybe.
    fn eat_declarator_separator_maybe(&mut self) -> ParseResult<bool> {
        if !self.declarator_has_separator() {
            return Ok(false);
        }

        self.bump(); // eat comma

        Ok(true)
    }

    /// Return true when the next token sequence continues a declarator list.
    fn declarator_has_separator(&mut self) -> bool {
        self.peek_is(TokenType::Comma) || self.next_token_type() == TokenType::Comma
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

#[cfg(test)]
mod tests {
    use destack_ast::{
        Asynchrony, Declaration, Declarator, Expression, FloatType, FunctionDeclaration,
        FunctionForm, GenericArgument, GenericParameter, IntegerType, Key, LetKind, Mutability,
        Name, Parameter, Pattern, PatternField, ScalarLiteral, TypeExpression, TypeLiteral,
        TypeMember,
    };
    use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_let_scalar() {
        let mut test = TestParser::new(
            r###"
const x: int32 = 1
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                let operator_span = parser
                    .tree
                    .get_main_span(declarators[0])
                    .expect("expected declarator operator span");
                assert_eq!(parser.get_span_str(operator_span), "=");

                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });

                // int32
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }) });

                // 1
                let value_id = value.expect("expected value");
                assert_node!(parser.tree, value_id, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_let_type_annotation_newline() {
        let mut test = TestParser::new_with_language(
            r###"
const constants:
    & typeof Foo
    & typeof Bar
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { ty, value, .. } => {
                assert!(value.is_none());
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                });
            });
        });
    }

    #[test]
    fn test_parse_let_recovers_missing_type_annotation_value() {
        let mut test = TestParser::new("const value: ");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // const value:
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });

                let ty = ty.expect("expected recovered type");
                assert_node!(parser.tree, ty, TypeExpression::Missing);
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_let_recovers_missing_type_before_initializer() {
        let mut test = TestParser::new("const value: = 1");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // const value: = 1
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });

                let ty = ty.expect("expected recovered type");
                assert_node!(parser.tree, ty, TypeExpression::Missing);

                let value = value.expect("expected initializer");
                assert_node!(parser.tree, value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_let_recovers_missing_initializer_value() {
        let mut test = TestParser::new("const value = ");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // const value =
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });

                assert!(ty.is_none());
                let value = value.expect("expected recovered value");
                assert_node!(parser.tree, value, Expression::Missing);
            });
        });
    }

    #[test]
    fn test_parse_using_scalar() {
        let mut test = TestParser::new(
            r###"
using x = open()
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let using_id = parser
            .eat_using(&start, DeclarationHeader::default(), Asynchrony::Sync)
            .unwrap();

        assert_node!(parser.tree, using_id, Expression::Using { asynchrony, declarators, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_let_generic_arrow_initializer() {
        let mut test = TestParser::new_with_language(
            "const foo: Tmp = <T,>(str: T): T => { return str; }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // const foo: Tmp = <T,>(str: T): T => { return str; }
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "foo");
                });
                assert_node!(parser.tree, ty.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Tmp");
                });
                let value_id = value.expect("expected value");
                assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.form, FunctionForm::Lambda);
                        assert_eq!(signature.generic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.generic_parameters[0], GenericParameter::Type { name, .. } => {
                            assert_string!(parser, *name, "T");
                        });
                        assert_eq!(signature.parameters.len(), 1);
                        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                            assert_string!(parser, *name, "str");
                            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                                assert_path!(parser, *path, "T");
                            });
                        });
                        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "T");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_readonly_identifier_with_type_annotation() {
        let mut test = TestParser::new_with_language(
            "const readonly: <A>(value: A) => Readonly<A> = identity",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "readonly");
                });
                assert!(ty.is_some());
                let value_id = value.expect("expected initializer");
                assert_node!(parser.tree, value_id, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "identity");
                });
            });
        });
    }

    #[test]
    fn test_parse_let_array_pattern_readonly_identifier() {
        let mut test = TestParser::new_with_language(
            "const [readonly, setReadonly] = useState(false)",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                    assert_eq!(fields.len(), 2);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, is_shorthand: true, pattern: None } => {
                        assert_name!(parser, *name, "readonly");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, is_shorthand: true, pattern: None } => {
                        assert_name!(parser, *name, "setReadonly");
                    });
                });
                assert_node!(parser.tree, value.expect("expected initializer"), Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "useState");
                });
            });
        });
    }

    #[test]
    fn test_parse_await_using_scalar() {
        let mut test = TestParser::new(
            r###"
await using conn = openConnection()
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let using_id = parser
            .eat_using(&start, DeclarationHeader::default(), Asynchrony::Async)
            .unwrap();

        assert_node!(parser.tree, using_id, Expression::Using { asynchrony, declarators, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(declarators.len(), 1);
        });
    }

    #[test]
    fn test_parse_using_multiple_declarators() {
        let mut test = TestParser::new(
            r###"
using a = openA(), b = openB()
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let using_id = parser
            .eat_using(&start, DeclarationHeader::default(), Asynchrony::Sync)
            .unwrap();

        assert_node!(parser.tree, using_id, Expression::Using { declarators, .. } => {
            assert_eq!(declarators.len(), 2);
        });
    }

    #[test]
    fn test_parse_var_array_undefined() {
        let mut test = TestParser::new(
            r###"
var x: float64[3] = undefined
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // var x: float64[3] = undefined
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, .. } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });

                // float64[3]
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, TypeExpression::Index { left, index } => {
                    assert_node!(parser.tree, *left, TypeExpression::Literal { value: TypeLiteral::Float(float_ty) } => {
                        assert_eq!(*float_ty, FloatType::Float64);
                    });
                    assert_node!(parser.tree, *index, TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(3) });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_tuple_pattern() {
        let mut test = TestParser::new(
            r###"
const (x, y) = foo()
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // const (x, y) = foo()
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                // (x, y)
                assert_node!(parser.tree, *pattern, Pattern::Tuple { fields, .. } => {
                    assert_eq!(fields.len(), 2);
                    // x
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                        assert_name!(parser, *name, "x");
                    });
                    // y
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, .. } => {
                        assert_name!(parser, *name, "y");
                    });
                });

                assert!(ty.is_none());

                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_let_definite_assignment_pattern() {
        let mut test = TestParser::new_with_language("let {}! = {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Must(inner) => {
                    assert_node!(parser.tree, *inner, Pattern::Object { .. } => {});
                });
            });
        });
    }

    #[test]
    fn test_parse_let_implicit_undefined() {
        let mut test = TestParser::new("const x: int32");
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // let x: int32
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                assert!(ty.is_some());
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_let_multiline_value() {
        let mut test = TestParser::new(
            r###"
const x =
    foo.parse()
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // const x = foo.parse()
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                // foo.parse()
                assert!(value.is_some());
                assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, generic_arguments: _, arguments: _ } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo.parse");
                });
            });
        });
    }

    #[test]
    fn test_parse_let_multiline_with_generic_arguments() {
        let mut test = TestParser::new(
            r###"
const registry: Map<
  string,
  Set<{count: number}>
> = new Map()
"###,
        );
        let mut parser = test.prepare();

        let let_id = parser.eat_expression(parser.flags).unwrap();

        // const registry: Map<..., ...> = new Map()
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert!(ty.is_some());
                assert!(value.is_some());

                // registry
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "registry");
                });

                // Map<string, Set<{count: number}>>
                assert_node!(parser.tree, ty.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
                    // Map
                    assert_path!(parser, *path, "Map");

                    // <string, Set<{count: number}>>
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        // string
                            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::String });
                    });

                    assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                        // Set<{count: number}>
                            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                // Set
                                assert_path!(parser, *path, "Set");

                                // <{count: number}>
                                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                        assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                                            assert_eq!(properties.len(), 1);
                                            assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), .. } => {
                                                assert_string!(parser, *name, "count");
                                            });
                                        });
                                });
                            });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_multiple_declarators() {
        let mut test = TestParser::new("let a: int32 = 1, b: string = \"hello\"");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();
        // let a: int32 = 1, b: string = "hello"
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 2);

            // a: int32 = 1
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "a");
                });
                assert!(ty.is_some());
                assert!(value.is_some());
            });
            // b: string = "hello"
            assert_node!(parser.tree, declarators[1], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "b");
                });
                assert!(ty.is_some());
                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_var_declarators_with_leading_comma_newline() {
        let mut test = TestParser::new_with_language(
            r#"var args = new Array(arguments.length - 1)
  , callbacks = this._callbacks['$' + event]"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // var args = new Array(arguments.length - 1)
        //   , callbacks = this._callbacks['$' + event]
        assert_node!(parser.tree, let_id, Expression::Let { kind, mutability, declarators, .. } => {
            assert_eq!(*kind, LetKind::Var);
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 2);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "args");
                });
            });

            assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "callbacks");
                });
            });
        });
    }

    #[test]
    fn test_parse_const_declarators_with_newline_after_keyword() {
        let mut test = TestParser::new_with_language(
            r#"const
  first = 1,
  second = 2"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // const
        //   first = 1,
        //   second = 2
        assert_node!(parser.tree, let_id, Expression::Let { kind, mutability, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 2);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "first");
                });
            });

            assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "second");
                });
            });
        });
    }

    #[test]
    fn test_parse_const_declarator_boundary_with_line_terminator_trivia() {
        let mut test = TestParser::new_with_language(
            r#"const result = CreateRecord(IntegerKey, value) /*
*/ return result as never"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let start = parser.span_start();
        let let_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // const result = CreateRecord(IntegerKey, value)
        assert_node!(parser.tree, let_id, Expression::Let { kind, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(declarators.len(), 1);
        });

        // return result as never
        let return_id = parser.eat_return().unwrap();
        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::As { .. } => {
            });
        });
    }

    #[test]
    fn test_parse_let_else_with_block_branch() {
        let mut test = TestParser::new("let { x } = value else { return }");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let expression_id = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap();

        // let { x } = value else { return }
        assert_node!(parser.tree, expression_id, Expression::LetElse { kind, mutability, declarator, else_branch } => {
            assert_eq!(*kind, LetKind::Let);
            assert_eq!(*mutability, Mutability::Mutable);

            let else_span = parser
                .tree
                .get_side_span(expression_id, NodeSpanType::Region(NodeSpanRegion::Clause))
                .expect("expected else clause span");
            assert_eq!(parser.get_span_str(else_span), "else");

            // let { x } = value
            assert_node!(parser.tree, *declarator, Declarator { pattern, value, .. } => {
                assert!(value.is_some());
                assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                    assert_eq!(fields.len(), 1);
                });
            });

            // else { return }
            assert_node!(parser.tree, *else_branch, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                let branch_expression = block
                    .leading_expressions
                    .first()
                    .copied()
                    .or(block.tail_expression)
                    .expect("expected else branch expression");

                assert_node!(parser.tree, branch_expression, Expression::Return { .. } => {
                });
            });
        });
    }

    #[test]
    fn test_reject_let_else_without_initializer() {
        let mut test = TestParser::new("let x else { return }");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let error = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap_err();

        // let x else { return }
        assert_eq!(parser.get_span_str(error.leaf_span()), "else");
    }

    #[test]
    fn test_reject_let_else_without_block_branch() {
        let mut test = TestParser::new("let x = value else return");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let error = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap_err();

        // let x = value else return
        assert_eq!(parser.get_span_str(error.leaf_span()), "return");
    }

    #[test]
    fn test_reject_indexed_declarator_target_in_untyped_source() {
        // var a[0] = 0
        let mut test = TestParser::new_with_language("var a[0]=0;", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let start = parser.span_start();
        let error = parser
            .eat_let(&start, DeclarationHeader::default())
            .unwrap_err();

        assert_eq!(parser.get_span_str(error.leaf_span()), "[");
    }

    #[test]
    fn test_parse_let_lambda_initializer_before_next_line_expression() {
        let mut test = TestParser::new_with_language(
            "let f1 = (/* ... */) => {}\n(function (/* ... */) {})(/* ... */)\n",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 2);

        assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "f1");
                });
                assert_node!(parser.tree, value.expect("expected initializer"), Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.form, FunctionForm::Lambda);
                    });
                });
            });
        });

        assert_node!(parser.tree, expressions[1], Expression::Call { left, arguments, .. } => {
                assert!(arguments.is_empty());
                assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                            assert_eq!(signature.form, FunctionForm::Function);
                        });
                    });
                });
        });
    }
}
