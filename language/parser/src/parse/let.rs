use crate::parse::timing::tags;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Asynchrony, DeclarationDescriptor, Declarator, Expression, Keyword, LetKind, LocalNodeId,
    Mutability, Pattern, TokenType,
};
use destack_source::NodeSpanType;

impl Parser {
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
        match keyword {
            Keyword::Let => {
                self.bump();
                Ok((LetKind::Let, Mutability::Mutable))
            }
            Keyword::Var => {
                self.bump();
                Ok((LetKind::Var, Mutability::Mutable))
            }
            Keyword::Const | Keyword::Readonly => {
                self.bump();
                Ok((LetKind::Const, Mutability::Immutable))
            }
            _ => Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            )),
        }
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
    pub fn eat_let(
        &mut self,
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_LET);
        // kind and mutability
        let (kind, mutability) = self.eat_let_kind()?;
        self.eat_newlines_maybe()?;

        // parse declarators (comma-separated list)
        let mut declarators = Vec::new();
        loop {
            let declarator_id = self.eat_declarator(false, false)?;
            declarators.push(declarator_id);

            // continue when a comma follows, even after line terminators
            if self.eat_declarator_separator_maybe()? {
                continue;
            }

            // declarations require statement boundaries after declarators
            if !self.declarator_has_statement_boundary() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            break;
        }

        // let
        let let_id = self.tree.insert(
            Expression::Let {
                kind,
                descriptor,
                mutability,
                declarators,
            },
            self.get_span_from(start),
        );
        Ok(let_id)
    }

    /// Eat a using binding (incl. `using` keyword and optional `await`).
    ///
    /// Examples:
    /// ```
    /// using file = openFile(path)
    /// await using conn = openConnection()
    /// using a = openA(), b = openB()
    /// ```
    pub fn eat_using(
        &mut self,
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
        asynchrony: Asynchrony,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_USING);
        // optional await
        if asynchrony == Asynchrony::Async {
            self.eat_keyword(Keyword::Await)?;
        }

        // using keyword
        self.eat_newlines_maybe()?;
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

        let using_id = self.tree.insert(
            Expression::Using {
                asynchrony,
                descriptor,
                declarators,
            },
            self.get_span_from(start),
        );
        Ok(using_id)
    }

    /// Eat a single declarator with an optional value unless `require_value` is set.
    pub(super) fn eat_declarator(
        &mut self,
        require_value: bool,
        allow_match_pattern: bool,
    ) -> ParseResult<LocalNodeId<Declarator>> {
        let _timing = self.timing_scope(tags::PARSE_DECLARATOR);
        let start = self.mark_span();
        let pattern_options = self
            .options
            .not_in_position()
            .in_before_type()
            .not_in_before_block();

        // pattern
        let pattern_id = if self.peek_is(TokenType::Identifier) {
            // fast path for simple binding patterns
            let next_token_type = self.peek_next_token_type();
            let can_fast_path = matches!(
                next_token_type,
                TokenType::Colon
                    | TokenType::Assign
                    | TokenType::Comma
                    | TokenType::Semicolon
                    | TokenType::CloseBrace
                    | TokenType::CloseParenthesis
                    | TokenType::CloseBracket
                    | TokenType::End
                    | TokenType::Newline
            );
            if can_fast_path {
                let has_active_split = self.has_active_split();
                let keyword = if has_active_split {
                    None
                } else {
                    self.keyword_for_index(self.pos_index())
                };
                let is_mutability_keyword =
                    matches!(keyword, Some(Keyword::Var | Keyword::Const | Keyword::Let))
                        || self.language.is_destack() && keyword == Some(Keyword::Readonly);
                let is_underscore_identifier = self.identifier_equals_at(self.pos_index(), "_");
                let allow_underscore_binding =
                    self.language.is_javascript() || self.language.is_typescript();
                if !is_mutability_keyword
                    && !has_active_split
                    && (!is_underscore_identifier || allow_underscore_binding)
                {
                    let (name, name_span) = self.eat_binding_identifier_with_span()?;
                    let pattern_id = self.tree.insert(
                        Pattern::Binding {
                            mutability: None,
                            name,
                            pattern: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(pattern_id, name_span);
                    pattern_id
                } else {
                    self.with_options(pattern_options, |parser| parser.eat_pattern())?
                }
            } else {
                self.with_options(pattern_options, |parser| parser.eat_pattern())?
            }
        } else {
            self.with_options(pattern_options, |parser| parser.eat_pattern())?
        };

        // declaration declarators must use binding patterns
        if !allow_match_pattern && !self.declarator_pattern_is_valid_binding(pattern_id) {
            return Err(ParseError::unexpected(self.tree.get_span(pattern_id)));
        }

        // type
        let (ty, ty_span) = if self.peek_colon_is() {
            let type_start = self.mark_span();
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            let type_options = self.options.not_in_position().in_type();
            let ty = self.eat_expression(type_options)?;
            (Some(ty), Some(self.get_span_from(&type_start)))
        } else {
            (None, None)
        };

        // value
        let value = if self.peek_is(TokenType::Assign)
            || self.is_token_after_newlines(self.pos(), TokenType::Assign)
        {
            self.eat_newlines_maybe()?;
            self.bump(); // eat assign
            self.eat_newlines_maybe()?;
            let value_options = self.options.not_in_position().not_in_sequence_expression();
            Some(self.eat_expression(value_options)?)
        } else if require_value {
            return Err(ParseError::expected(self.peek()?.span, TokenType::Assign));
        } else {
            None
        };

        // declarator
        let declarator_id = self.tree.insert(
            Declarator {
                pattern: pattern_id,
                ty,
                value,
            },
            self.get_span_from(&start),
        );

        // set type span for the type annotation
        if let Some(span) = ty_span {
            self.tree
                .set_side_span(declarator_id, NodeSpanType::Type, span);
        }

        Ok(declarator_id)
    }

    /// Eat one declarator separator comma maybe.
    fn eat_declarator_separator_maybe(&mut self) -> ParseResult<bool> {
        if !self.declarator_has_separator() {
            return Ok(false);
        }

        self.eat_newlines_maybe()?;
        self.bump(); // eat comma
        self.eat_newlines_maybe()?;

        Ok(true)
    }

    /// Return true when the next token sequence continues a declarator list.
    fn declarator_has_separator(&mut self) -> bool {
        self.peek_is(TokenType::Comma) || self.is_token_after_newlines(self.pos(), TokenType::Comma)
    }

    /// Return true when the current token can terminate a declaration statement.
    fn declarator_has_statement_boundary(&mut self) -> bool {
        self.is_statement_stop()
            || self.has_line_terminator_before_current_token()
            || self.peek_is(TokenType::CloseBrace)
            || self.peek_is(TokenType::CloseParenthesis)
    }

    /// Return true when a declarator pattern is a valid binding.
    fn declarator_pattern_is_valid_binding(&self, pattern_id: LocalNodeId<Pattern>) -> bool {
        match self.tree.get(pattern_id) {
            Pattern::Expression { value } => self.declarator_expression_is_valid_binding(*value),
            _ => true,
        }
    }

    /// Return true when an expression is a valid declarator binding.
    fn declarator_expression_is_valid_binding(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match self.tree.get(expression_id) {
            Expression::Path {
                path,
                static_arguments: None,
            } => path.segments.len() == 1,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Asynchrony, BinaryOperator, Declaration, DeclarationDescriptor, Declarator,
        Expression, FunctionKind, IntType, Key, LetKind, Mutability, Name, Parameter, Pattern,
        PatternField, Property, ScalarLiteral, TypeBinaryOperator, TypeLiteral,
    };
    use destack_source::LanguageType;

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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });

                // int32
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));

                // 1
                let value_id = value.expect("expected value");
                assert_node!(parser.tree, value_id, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_let_type_annotation_newline() {
        let mut test = TestParser::new_with_options(
            r###"
const constants:
    & typeof Foo
    & typeof Bar
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { ty, value, .. } => {
                assert!(value.is_none());
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                });
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let using_id = parser
            .eat_using(&start, DeclarationDescriptor::default(), Asynchrony::Sync)
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
        let mut test = TestParser::new_with_options(
            "const foo: Tmp = <T,>(str: T): T => { return str; }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();

        // const foo: Tmp = <T,>(str: T): T => { return str; }
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "foo");
                });
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Tmp");
                });
                let value_id = value.expect("expected value");
                assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        let generics = signature.generics.as_ref().expect("expected generics");
                        let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                        assert_eq!(static_parameters.len(), 1);
                        assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                            assert_string!(parser, *name, "T");
                        });
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "str");
                            assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "T");
                            });
                        });
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "T");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_readonly_identifier_with_type_annotation_typescript() {
        let mut test = TestParser::new_with_options(
            "const readonly: <A>(value: A) => Readonly<A> = identity",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
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
                assert_node!(parser.tree, value_id, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "identity");
                });
            });
        });
    }

    #[test]
    fn test_parse_let_array_pattern_readonly_identifier_typescript() {
        let mut test = TestParser::new_with_options(
            "const [readonly, setReadonly] = useState(false)",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Array { fields } => {
                    assert_eq!(fields.len(), 2);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                        assert_name!(parser, *name, "readonly");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let using_id = parser
            .eat_using(&start, DeclarationDescriptor::default(), Asynchrony::Async)
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let using_id = parser
            .eat_using(&start, DeclarationDescriptor::default(), Asynchrony::Sync)
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            // var (mutable)
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, .. } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });

                // float64[3]
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, Expression::TypeIndex { left, index } => {
                    assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::Float(float_ty)) => {
                        assert_eq!(float_ty.width, Some(64));
                    });
                    assert_node!(parser.tree, *index, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            // let (immutable)
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

                // no explicit type
                assert!(ty.is_none());

                // foo()
                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_let_definite_assignment_pattern() {
        let mut test = TestParser::new_with_options("let {}! = {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
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

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
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
                // int32
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
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
                assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, static_arguments: _, dynamic_arguments: _ } => {
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "foo.parse");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_multiline_with_static_arguments() {
        let mut test = TestParser::new(
            r###"
const registry: Map<
  string,
  Set<{count: number}>
> = new Map()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_expression(parser.options).unwrap();

        // const renderCounter
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
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, static_arguments } => {
                    // Map
                    assert_path!(parser, *path, "Map");
                    // <string, Set<{count: number}>>
                    // string
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    // Set<{count: number}>
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            // Set
                            assert_path!(parser, *path, "Set");
                            // <{count: number}>
                            assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::ObjectExpression { ty: None, properties, .. } => {
                                    assert_eq!(properties.len(), 1);
                                    assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), .. } => {
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
        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();
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
    fn test_parse_var_declarators_with_leading_comma_newline_javascript() {
        let mut test = TestParser::new_with_options(
            r#"var args = new Array(arguments.length - 1)
  , callbacks = this._callbacks['$' + event]"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        // parse both declarators split by a newline before the comma
        assert_node!(parser.tree, let_id, Expression::Let { kind, mutability, declarators, .. } => {
            assert_eq!(*kind, LetKind::Var);
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 2);

            // first declarator name
            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "args");
                });
            });

            // second declarator name
            assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "callbacks");
                });
            });
        });
    }

    #[test]
    fn test_parse_const_declarators_with_newline_after_keyword_javascript() {
        let mut test = TestParser::new_with_options(
            r#"const
  first = 1,
  second = 2"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        // parse two declarators split across newlines after const
        assert_node!(parser.tree, let_id, Expression::Let { kind, mutability, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 2);

            // first declarator name
            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "first");
                });
            });

            // second declarator name
            assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "second");
                });
            });
        });
    }

    #[test]
    fn test_parse_const_declarator_boundary_with_line_terminator_trivia_typescript() {
        let mut test = TestParser::new_with_options(
            r#"const result = CreateRecord(IntegerKey, value) /*
*/ return result as never"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let start = parser.mark();
        let let_id = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap();

        // const declarator stops before return after line terminator trivia
        assert_node!(parser.tree, let_id, Expression::Let { kind, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(declarators.len(), 1);
        });

        // return expression is parsed separately as a cast
        let return_id = parser.eat_return().unwrap();
        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::TypeBinary { operator, .. } => {
                assert_eq!(*operator, TypeBinaryOperator::Cast);
            });
        });
    }

    #[test]
    fn test_reject_js_indexed_declarator_target() {
        // source: var a[0]=0;
        let mut test = TestParser::new_with_options("var a[0]=0;", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let start = parser.mark();
        let error = parser
            .eat_let(&start, DeclarationDescriptor::default())
            .unwrap_err();

        // [
        assert_eq!(parser.get_span_str(error.leaf_span()), "[");
    }
}
