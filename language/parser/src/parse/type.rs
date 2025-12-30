use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, DeclarationDescriptor, DeclarationType, Expression, FloatType, IntType, Keyword,
    LocalNodeId, Mutability, Name, TokenType, TypeBinaryOperator, TypeIntrinsic, TypeKind,
    TypeLiteral, TypeMappedModifiers, TypeMappedParameter, TypeModifier, TypePredicateSubject,
    TypeUnaryOperator, UnaryOperator, VarianceBound,
};

impl Parser {
    /// Eat a variance bound maybe.
    #[inline]
    pub fn eat_variance_bound_maybe(&mut self) -> ParseResult<Option<VarianceBound>> {
        // implements
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            Ok(Some(VarianceBound::Implements))
        }
        // extends
        else if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            Ok(Some(VarianceBound::Extends))
        }
        // super
        else if self.peek_keyword(Keyword::Super).is_ok() {
            self.bump(); // eat super
            Ok(Some(VarianceBound::Super))
        }
        // none
        else {
            Ok(None)
        }
    }

    /// Whether the token type can start an expression.
    #[inline]
    fn is_start_of_expression(&self, token_str: &str, token_type: TokenType) -> bool {
        // in type context, prefix operators can start a type expression too
        token_type == TokenType::OpenParenthesis
            || token_type == TokenType::Identifier
            || token_type == TokenType::Literal
            // (if we're before a block then { is a terminator, not the start of a block)
            || (token_type == TokenType::OpenBrace && !self.options.in_before_block)
            || UnaryOperator::from_prefix_token(token_type).is_some()
            || TypeUnaryOperator::from_prefix_token(token_str, token_type).is_some()
    }

    /// Whether the token string encodes a type literal with an explicit width.
    #[inline]
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Eat a composite / declaration type literal.
    pub fn eat_composite_type_literal(&mut self) -> ParseResult<TypeLiteral> {
        let next = self.eat_keyword_any()?;
        match next {
            Keyword::Type => Ok(TypeLiteral::Composite(DeclarationType::Type)),
            Keyword::Namespace => Ok(TypeLiteral::Composite(DeclarationType::Namespace)),
            Keyword::Struct => Ok(TypeLiteral::Composite(DeclarationType::Struct)),
            Keyword::Class => Ok(TypeLiteral::Composite(DeclarationType::Class)),
            Keyword::Enum => Ok(TypeLiteral::Composite(DeclarationType::Enum)),
            Keyword::Union => Ok(TypeLiteral::Composite(DeclarationType::Union)),
            Keyword::Interface => Ok(TypeLiteral::Composite(DeclarationType::Interface)),
            Keyword::Extension => Ok(TypeLiteral::Composite(DeclarationType::Extension)),
            Keyword::Function => Ok(TypeLiteral::Composite(DeclarationType::Function)),
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Peek a type literal (except composite types).
    /// Certain type literals are only parsed at the AST-level in static or type contexts.
    /// (This prevents shadowing in case we have a variable or parameter named `int` or `number`.)
    pub fn peek_type_literal(&self) -> ParseResult<TypeLiteral> {
        let next = self.peek()?;
        let next_str = self.get_span_str(next.span);

        // always available type literals
        let literal = match next_str {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(TypeLiteral::Unknown),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            // any
            "any" => Some(TypeLiteral::Any),
            // never
            "never" => Some(TypeLiteral::Never),
            _ => None,
        };
        if let Some(literal) = literal {
            return Ok(literal);
        }

        // bail if not inside static or type context
        if !self.options.in_type && !self.options.in_static {
            return Err(ParseError::unexpected(next.span));
        }

        let next_type = next.token.ty;
        let next_next = self.peek_next();
        let next_next_type = next_next.as_ref().map(|next| next.token.ty).ok();
        let next_next_str = next_next
            .as_ref()
            .map(|next| self.get_span_str(next.span))
            .ok();

        // !
        if next_type == TokenType::Not
            // if next token doesn't start a related expression
            && (next_next_type.is_none()
                || !self.is_start_of_expression(
                    self.get_span_str(next_next.unwrap().span),
                    next_next_type.unwrap(),
                ))
        {
            return Ok(TypeLiteral::Never);
        }

        // regular single-token type literals (also only inside static/type context)
        match next_str {
            // boolean
            "boolean" => Ok(TypeLiteral::Boolean),
            // character
            "character" => Ok(TypeLiteral::Character),
            // string
            "string" => Ok(TypeLiteral::String),
            // bigint
            "bigint" => Ok(TypeLiteral::Bigint),
            // number
            "number" => Ok(TypeLiteral::Number),
            // int (followed by number or nothing)
            "int" => Ok(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: true,
            })),
            "intp" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: true })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Ok(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: false,
            })),
            "uintp" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: false })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            // float (followed by number or nothing)
            "float" => Ok(TypeLiteral::Float(FloatType { width: None })),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Ok(TypeLiteral::Float(FloatType { width: Some(width) }))
            }
            // symbol
            "symbol" => Ok(TypeLiteral::Symbol),
            // unique symbol
            "unique" if next_next_str == Some("symbol") => Ok(TypeLiteral::UniqueSymbol),
            // composite type
            _ => Err(ParseError::unexpected(next.span)),
        }
    }

    /// Eat a type literal (except composite types).
    pub fn eat_type_literal(&mut self, literal: Option<TypeLiteral>) -> ParseResult<TypeLiteral> {
        let literal = match literal {
            Some(literal) => literal,
            None => self.peek_type_literal()?,
        };
        self.bump();
        if let TypeLiteral::UniqueSymbol = literal {
            self.bump(); // eat second token
        }
        Ok(literal)
    }

    /// Eat a type alias or expression.
    ///
    /// Examples:
    /// ```
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    /// type 1 | 2 | 3
    /// readonly T
    /// newtype T = int32
    /// newtype Foo<T> = Baz<T> | null
    /// newtype T = { a: int32, b: boolean } | true
    /// ```
    pub fn eat_type(
        &mut self,
        start: ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let keyword: Keyword =
            self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;

        // kind
        let kind = match keyword {
            Keyword::Type => TypeKind::Structural,
            Keyword::Readonly => TypeKind::Structural,
            Keyword::Newtype => TypeKind::Nominal,
            _ => unreachable!(),
        };

        // mutability
        let mutability = if keyword == Keyword::Readonly {
            Some(Mutability::Immutable)
        } else {
            None
        };

        // alias (or expression with static parameters)
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::LessThan).is_ok())
        {
            // identifier
            // (speculative because we don't know yet if we'll have a `=` afterwards)
            let speculative_start = (self.mark(), self.tree.next_id());
            let (name, name_span) = if let Some((n, s)) = self.eat_name_maybe_with_span()? {
                (Some(n), Some(s))
            } else {
                (None, None)
            };
            descriptor.name = name;

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // if followed by =, then it's a type alias
            if self.peek_token(TokenType::Assign).is_ok() {
                // =
                self.eat_token(TokenType::Assign)?;
                self.eat_newlines_maybe()?;
                // value
                let value_id = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })?;
                if let Some(name) = descriptor.name.as_ref() {
                    self.apply_intrinsic_type_literal(name, value_id);
                }
                // type declaration wrapped in expression
                let declaration = Declaration::Type {
                    kind,
                    mutability,
                    descriptor,
                    static_parameters,
                    value: value_id,
                };
                let declaration_id = self.tree.insert(declaration, self.get_span_from(start));

                // set main span to the name identifier
                if let Some(span) = name_span {
                    self.tree.set_main_span(declaration_id, span);
                }

                let expression = Expression::Declaration(declaration_id);
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
            // otherwise it's a type expression with static arguments
            else {
                // re-parse from before the static parameters to get them as a arguments
                self.restore(speculative_start.0, speculative_start.1);
                let right = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })?;
                let operator = if mutability == Some(Mutability::Immutable) {
                    TypeUnaryOperator::Readonly
                } else if kind == TypeKind::Nominal {
                    TypeUnaryOperator::Newtype
                } else {
                    TypeUnaryOperator::Type
                };
                let expression = Expression::TypeUnary { operator, right };
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
        }
        // type expression
        else {
            let right = self.with_options(self.options.not_in_position().in_type(), |parser| {
                parser.eat_expression()
            })?;
            let operator = if mutability == Some(Mutability::Immutable) {
                TypeUnaryOperator::Readonly
            } else if kind == TypeKind::Nominal {
                TypeUnaryOperator::Newtype
            } else {
                TypeUnaryOperator::Type
            };
            let expression = Expression::TypeUnary { operator, right };
            Ok(self.tree.insert(expression, self.get_span_from(start)))
        }
    }

    /// Apply an intrinsic type literal to a value expression.
    fn apply_intrinsic_type_literal(&mut self, name: &Name, value_id: LocalNodeId<Expression>) {
        // intrinsics are only valid for specific standard alias names
        let Some(intrinsic) = self.type_intrinsic_for_name(name) else {
            return;
        };
        let Expression::Path {
            path,
            static_arguments,
        } = self.tree.get(value_id)
        else {
            return;
        };

        // intrinsic must be the exact literal `intrinsic` without static args
        if static_arguments.is_some() || path.segments.len() != 1 {
            return;
        }
        let segment = path.segments[0];
        if self.strings.get(segment) != "intrinsic" {
            return;
        }
        *self.tree.get_mut(value_id) = Expression::TypeLiteral(TypeLiteral::Intrinsic(intrinsic));
    }

    /// Get the type intrinsic for a name.
    fn type_intrinsic_for_name(&self, name: &Name) -> Option<TypeIntrinsic> {
        // map known intrinsic aliases to intrinsic markers
        let Name::Identifier(name_id) = name else {
            return None;
        };
        match self.strings.get(*name_id).as_ref() {
            "Uppercase" => Some(TypeIntrinsic::Uppercase),
            "Lowercase" => Some(TypeIntrinsic::Lowercase),
            "Capitalize" => Some(TypeIntrinsic::Capitalize),
            "Uncapitalize" => Some(TypeIntrinsic::Uncapitalize),
            "NoInfer" => Some(TypeIntrinsic::NoInfer),
            "BuiltinIteratorReturn" => Some(TypeIntrinsic::BuiltinIteratorReturn),
            _ => None,
        }
    }

    /// Eat a type infer expression.
    pub fn eat_type_infer_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Infer)?;
        let name = self.eat_identifier()?;
        // optional constraint: infer T extends U
        let constraint = if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            self.eat_newlines_maybe()?;
            Some(
                self.with_options(self.options.not_in_position().in_type(), |parser| {
                    parser.eat_expression()
                })?,
            )
        } else {
            None
        };
        Ok(self.tree.insert(
            Expression::TypeInfer { name, constraint },
            self.get_span_from(start),
        ))
    }

    /// Eat a type import expression.
    pub fn eat_type_import_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Import)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        let (target, _span) = self.eat_string_literal_with_span()?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        // optional qualifier: import("mod").Type
        let qualifier = if self.peek_token(TokenType::Dot).is_ok() {
            self.bump(); // eat dot
            Some(self.eat_path()?)
        } else {
            None
        };
        Ok(self.tree.insert(
            Expression::TypeImport { target, qualifier },
            self.get_span_from(start),
        ))
    }

    pub fn eat_type_predicate_asserts(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Asserts)?;
        self.eat_newlines_maybe()?;

        // asserts this | asserts param
        let subject = if self.peek_keyword(Keyword::This).is_ok() {
            self.bump(); // eat this
            TypePredicateSubject::This
        } else {
            let name = self.eat_identifier()?;
            TypePredicateSubject::Identifier(name)
        };

        // optional target: asserts x is T
        let target = if self.peek_keyword(Keyword::Is).is_ok() {
            self.bump(); // eat is
            self.eat_newlines_maybe()?;
            Some(
                self.with_options(self.options.not_in_position().in_type(), |parser| {
                    parser.eat_expression()
                })?,
            )
        } else {
            None
        };

        Ok(self.tree.insert(
            Expression::TypePredicate {
                asserts: true,
                subject,
                target,
            },
            self.get_span_from(start),
        ))
    }

    /// Get the type predicate subject from an expression.
    pub fn type_predicate_subject_from_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<TypePredicateSubject> {
        match self.tree.get(expression_id) {
            // this predicate
            Expression::This => Some(TypePredicateSubject::This),
            // identifier predicate
            Expression::Path {
                path,
                static_arguments,
            } if static_arguments.is_none() && path.segments.len() == 1 => {
                Some(TypePredicateSubject::Identifier(path.segments[0]))
            }
            _ => None,
        }
    }

    /// Eat a type mapped expression.
    pub fn eat_type_mapped_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;

        // readonly
        let readonly = self.eat_type_mapped_readonly_modifier()?;

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;

        // [K in keyof T]
        let name = self.eat_identifier()?;
        self.eat_newlines_maybe()?;
        self.eat_keyword(Keyword::In)?;
        self.eat_newlines_maybe()?;
        let constraint = self.with_options(self.options.not_in_position().in_type(), |parser| {
            parser.eat_expression()
        })?;

        // map key remaps can parse as a type cast in the constraint
        let (constraint, mut key_remap) = match self.tree.get(constraint) {
            Expression::TypeBinary {
                left,
                operator: TypeBinaryOperator::Cast,
                right,
            } => (*left, Some(*right)),
            _ => (constraint, None),
        };

        // optional key remap: [K in T as ...]
        if self.peek_keyword(Keyword::As).is_ok() {
            self.bump(); // eat as
            self.eat_newlines_maybe()?;
            key_remap = Some(
                self.with_options(self.options.not_in_position().in_type(), |parser| {
                    parser.eat_expression()
                })?,
            );
        }

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBracket)?;

        // optional modifier: ?, +?, -?
        let optional = self.eat_type_mapped_optional_modifier()?;
        let modifiers = TypeMappedModifiers { readonly, optional };

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::Colon)?;
        self.eat_newlines_maybe()?;

        // value type
        let value = self.with_options(self.options.not_in_position().in_type(), |parser| {
            parser.eat_expression()
        })?;
        self.eat_newlines_maybe()?;
        if self.peek_token(TokenType::Semicolon).is_ok()
            || self.peek_token(TokenType::Comma).is_ok()
        {
            self.bump();
            self.eat_newlines_maybe()?;
        }
        self.eat_token(TokenType::CloseBrace)?;

        let parameter = TypeMappedParameter {
            name,
            constraint,
            key_remap,
        };
        Ok(self.tree.insert(
            Expression::TypeMapped {
                parameter,
                modifiers,
                value,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat a type mapped readonly modifier.
    fn eat_type_mapped_readonly_modifier(&mut self) -> ParseResult<TypeModifier> {
        // readonly
        if self.peek_keyword(Keyword::Readonly).is_ok() {
            self.bump(); // eat readonly
            return Ok(TypeModifier::Add);
        }
        // -readonly
        else if self.peek_token(TokenType::Subtract).is_ok()
            && self.peek_next_keyword(Keyword::Readonly).is_ok()
        {
            self.bump(); // eat -
            self.bump(); // eat readonly
            return Ok(TypeModifier::Remove);
        }
        // +readonly
        else if self.peek_token(TokenType::Add).is_ok()
            && self.peek_next_keyword(Keyword::Readonly).is_ok()
        {
            self.bump(); // eat +
            self.bump(); // eat readonly
            return Ok(TypeModifier::Add);
        }
        Ok(TypeModifier::None)
    }

    /// Eat a type mapped optional modifier.
    fn eat_type_mapped_optional_modifier(&mut self) -> ParseResult<TypeModifier> {
        // ?
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat ?
            return Ok(TypeModifier::Add);
        }
        // -?
        else if self.peek_token(TokenType::Subtract).is_ok()
            && self.peek_next_token(TokenType::Maybe).is_ok()
        {
            self.bump(); // eat -
            self.bump(); // eat ?
            return Ok(TypeModifier::Remove);
        }
        // +?
        else if self.peek_token(TokenType::Add).is_ok()
            && self.peek_next_token(TokenType::Maybe).is_ok()
        {
            self.bump(); // eat +
            self.bump(); // eat ?
            return Ok(TypeModifier::Add);
        }
        Ok(TypeModifier::None)
    }

    /// Eat extends types (without the leading keyword).
    pub fn eat_extends_types_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // check for extends keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            self.eat_super_types_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat implements types maybe.
    #[inline]
    pub fn eat_implements_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // check for implements keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            self.eat_super_types_maybe(&[Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat a super type clause maybe.
    #[inline]
    fn eat_super_types_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        let is_parenthesized = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump();
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };
        let types = self.with_options(self.options.in_super_type(), |parser| {
            parser.eat_super_types(terminators)
        })?;
        if is_parenthesized {
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
        }
        Ok(Some(types))
    }

    /// Eat super types (without the leading keyword).
    fn eat_super_types(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let mut types: Vec<LocalNodeId<Expression>> = Vec::new();
        while self.peek().is_ok() {
            // eat until open brace or close parenthesis
            if self.peek_token(TokenType::OpenBrace).is_ok()
                || self.peek_token(TokenType::CloseParenthesis).is_ok()
                || terminators
                    .iter()
                    .any(|terminator| self.peek_keyword(*terminator).is_ok())
            {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
            // keep eating super types
            else {
                let ty = self.with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })?;
                types.push(ty);
            }
        }
        Ok(types)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, BinaryOperator, BindingKind, BindingModifier, Declaration, Expression,
        FunctionAbstraction, FunctionKind, FunctionMode, IntType, Key, Mutability, Parameter,
        Property, ScalarLiteral, TypeBinaryOperator, TypeIntrinsic, TypeLiteral,
        TypeMappedModifiers, TypeModifier, TypePredicateSubject, TypeUnaryOperator, UnaryOperator,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_type_alias() {
        let mut test = TestParser::new("type T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    /// Parse `this` in a type alias.
    #[test]
    fn test_parse_this_type_alias() {
        let mut test = TestParser::new("type Builder = this");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type Builder = this
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Builder");
                assert_node!(parser.tree, *value, Expression::This);
            });
        });
    }

    #[test]
    fn test_parse_type_alias_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B> = intp");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B> = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, static_parameters, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Pointer { is_signed: true })));
                assert!(static_parameters.is_some());
                assert_eq!(static_parameters.as_ref().unwrap().len(), 2);
            });
        });
    }

    #[test]
    fn test_parse_type_parameter_function_constraint() {
        let mut test = TestParser::new("type Parameters<T extends (a: any) => any> = T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type Parameters<T extends (a: any) => any> = T
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { static_parameters: Some(static_parameters), .. } => {
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { ty: Some(ty), .. } => {
                    assert_node!(parser.tree, *ty, Expression::Declaration(func_id) => {
                        assert_node!(parser.tree, *func_id, Declaration::Function { .. });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_expression_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B>
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "T");
                assert_eq!(static_arguments.as_ref().unwrap().len(), 2);
            })
        });
    }

    #[test]
    fn test_parse_type_expression() {
        let mut test = TestParser::new("type 1 | 2 | 3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type 1 | 2 |3
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    }

    #[test]
    fn test_parse_optional_type_rejected() {
        let mut test = TestParser::new("type T = Foo?");
        let mut parser = test.prepare();
        assert!(parser.eat_expression().is_err());
    }

    #[test]
    fn test_parse_readonly_type_expression() {
        let mut test = TestParser::new("readonly T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // readonly T
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Readonly);
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
    }

    #[test]
    fn test_parse_newtype_type_expression() {
        let mut test = TestParser::new("newtype T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // newtype T = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_inline_object() {
        let mut test = TestParser::new("type T = A extends B ? {} : { a: string | undefined }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B ? {} : { a: string | undefined }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { then_type, else_type, .. } => {
                    assert_node!(parser.tree, *then_type, Expression::ObjectExpression { properties, .. } => {
                        assert!(properties.is_empty());
                    });
                    assert_node!(parser.tree, *else_type, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_union_right() {
        let mut test = TestParser::new("type T = A extends B | C ? D : E");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B | C ? D : E
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert_node!(parser.tree, *right, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "D");
                    assert_expression_path!(parser, parser.tree.get(*else_type), "E");
                });
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_intersection_right() {
        let mut test = TestParser::new("type T = A extends B & C ? D : E");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B & C ? D : E
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert_node!(parser.tree, *right, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "D");
                    assert_expression_path!(parser, parser.tree.get(*else_type), "E");
                });
            });
        });
    }

    #[test]
    fn test_parse_type_extends_with_union_right() {
        let mut test = TestParser::new("type T = A extends B | C");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B | C
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Extends);
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert_node!(parser.tree, *right, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    });
                });
            });
        });
    }

    /// Parse conditional types with function right-hand sides.
    #[test]
    fn test_parse_conditional_type_with_function_right() {
        let mut test = TestParser::new("type T = A extends (x: number) => any ? C : D");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends (x: number) => any ? C : D
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { right, then_type, else_type, .. } => {
                    assert_node!(parser.tree, *right, Expression::Declaration(function_id) => {
                        assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "C");
                    assert_expression_path!(parser, parser.tree.get(*else_type), "D");
                });
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_semicolon_terminated_properties() {
        let mut test =
            TestParser::new("type T = X extends Y ? {} : { a: string | undefined; b: number; }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = X extends Y ? {} : { a: string | undefined; b: number; }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { else_type, .. } => {
                    assert_node!(parser.tree, *else_type, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 2, "expected 2 properties but got {}", properties.len());
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_conditional_multiline_nested() {
        let mut test = TestParser::new("type T = A extends B ?\n    C extends D ? E : F\n    : G");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B ? C extends D ? E : F : G
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { then_type, .. } => {
                    assert_node!(parser.tree, *then_type, Expression::TypeConditional { .. });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_intersection_with_inline_object() {
        let mut test = TestParser::new("type T = Z & { a: string | undefined }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = Z & { a: string | undefined }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { right, .. } => {
                    assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_array_tuple_type() {
        let mut test = TestParser::new("type T = [string, number]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [string, number]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // first element: string (positional)
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    // second element: number (positional)
                    assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tuple_type() {
        let mut test = TestParser::new("type T = (string, number)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = (string, number)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TupleExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // string
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "string");
                    });
                    // number
                    assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "number");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_labeled_tuple_type() {
        let mut test = TestParser::new("type T = [start: number, end: number]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [start: number, end: number]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // start: number
                    assert_node!(parser.tree, elements[0], Argument::Labeled { modifiers: _, label, value } => {
                        assert_string!(parser, *label, "start");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                    // end: number
                    assert_node!(parser.tree, elements[1], Argument::Labeled { modifiers: _, label, value } => {
                        assert_string!(parser, *label, "end");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_labeled_tuple_type_complex() {
        let mut test =
            TestParser::new("type T = [importCode: string, nameMap: Record<string, string>]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [importCode: string, nameMap: Record<string, string>]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // importCode: string
                    assert_node!(parser.tree, elements[0], Argument::Labeled { label, .. } => {
                        assert_string!(parser, *label, "importCode");
                    });
                    // nameMap: Record<string, string>
                    assert_node!(parser.tree, elements[1], Argument::Labeled { label, .. } => {
                        assert_string!(parser, *label, "nameMap");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_import_expression() {
        let mut test = TestParser::new("type T = import(\"mod\").Type");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = import("mod").Type
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeImport { target, qualifier } => {
                    assert_string!(parser, *target, "mod");
                    assert_path!(parser, qualifier.as_ref().unwrap(), "Type");
                });
            });
        });
    }

    #[test]
    fn test_parse_type_infer_with_constraint() {
        let mut test = TestParser::new("type T = infer U extends V");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = infer U extends V
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeInfer { name, constraint } => {
                    assert_string!(parser, *name, "U");
                    assert_expression_path!(parser, parser.tree.get(constraint.unwrap()), "V");
                });
            });
        });
    }

    #[test]
    fn test_parse_type_infer_with_wildcard() {
        let mut test = TestParser::new("type T = infer _");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = infer _
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeInfer { name, constraint } => {
                    assert_string!(parser, *name, "_");
                    assert!(constraint.is_none());
                });
            });
        });
    }

    #[test]
    fn test_parse_type_template_literal() {
        let mut test = TestParser::new("type T = `foo-${Bar}`");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = `foo-${Bar}`
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeTemplateLiteral { strings, spans } => {
                    assert_eq!(strings.len(), 2);
                    assert_eq!(spans.len(), 1);
                    assert_string!(parser, strings[0], "foo-");
                    assert_string!(parser, strings[1], "");
                    assert_expression_path!(parser, parser.tree.get(spans[0]), "Bar");
                });
            });
        });
    }

    #[test]
    fn test_parse_type_mapped_expression() {
        let mut test =
            TestParser::new("type T = { readonly [K in keyof T as `foo-${K}`]-?: T[K] }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { readonly [K in keyof T as `foo-${K}`]-?: T[K] }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeMapped { parameter, modifiers, value } => {
                    assert_eq!(*modifiers, TypeMappedModifiers {
                        readonly: TypeModifier::Add,
                        optional: TypeModifier::Remove,
                    });
                    assert_string!(parser, parameter.name, "K");
                    assert_node!(parser.tree, parameter.constraint, Expression::TypeUnary { operator, right } => {
                        assert_eq!(*operator, TypeUnaryOperator::Keyof);
                        assert_expression_path!(parser, parser.tree.get(*right), "T");
                    });
                    assert_node!(parser.tree, parameter.key_remap.unwrap(), Expression::TypeTemplateLiteral { strings, spans } => {
                        assert_eq!(strings.len(), 2);
                        assert_eq!(spans.len(), 1);
                        assert_string!(parser, strings[0], "foo-");
                        assert_string!(parser, strings[1], "");
                        assert_expression_path!(parser, parser.tree.get(spans[0]), "K");
                    });
                    assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                        assert_expression_path!(parser, parser.tree.get(*index), "K");
                    });
                });
            });
        });
    }

    /// Parse mapped types in static type arguments with readonly removal.
    #[test]
    fn test_parse_type_mapped_expression_in_static_arguments() {
        let mut test = TestParser::new("type T = Promise<{ -readonly [P in keyof T]: T[P] }>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Promise");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeMapped { modifiers, .. } => {
                            assert_eq!(*modifiers, TypeMappedModifiers {
                                readonly: TypeModifier::Remove,
                                optional: TypeModifier::None,
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_mapped_expression_with_semicolon() {
        let mut test = TestParser::new("type T = { [K in T]: T[K]; }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { [K in T]: T[K]; }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeMapped { .. });
            });
        });
    }

    #[test]
    fn test_parse_type_mapped_expression_with_intersection() {
        let mut test = TestParser::new(
            "type T = { [P in keyof T]: T[P]; } & { [x: string]: PropertyDescriptor; }",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { [P in keyof T]: T[P]; } & { [x: string]: PropertyDescriptor; }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                    assert_node!(parser.tree, *left, Expression::TypeMapped { .. });
                    assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                        assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::NamedExpression { name, key }), value: Some(value), .. } => {
                            assert_string!(parser, *name, "x");
                            assert_node!(parser.tree, *key, Expression::TypeLiteral(TypeLiteral::String));
                            assert_expression_path!(parser, parser.tree.get(*value), "PropertyDescriptor");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_function_type_return_conditional() {
        let mut test = TestParser::new(
            "type Getter<T, P> = (target: T, propertyKey: P) => P extends keyof T ? T[P] : any",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Getter<T, P> = (target: T, propertyKey: P) => P extends keyof T ? T[P] : any
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Declaration(function_id) => {
                    assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeConditional { left, right, then_type, else_type } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "P");
                            assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
                                assert_eq!(*operator, TypeUnaryOperator::Keyof);
                                assert_expression_path!(parser, parser.tree.get(*right), "T");
                            });
                            assert_node!(parser.tree, *then_type, Expression::TypeIndex { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_expression_path!(parser, parser.tree.get(*index), "P");
                            });
                            assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Any));
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_arguments_with_conditional() {
        let mut test = TestParser::new(
            "type Descriptor<P, T> = TypedPropertyDescriptor<P extends keyof T ? T[P] : any>",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Descriptor<P, T> = TypedPropertyDescriptor<P extends keyof T ? T[P] : any>
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { static_arguments: Some(static_arguments), .. } => {
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "P");
                            assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
                                assert_eq!(*operator, TypeUnaryOperator::Keyof);
                                assert_expression_path!(parser, parser.tree.get(*right), "T");
                            });
                            assert_node!(parser.tree, *then_type, Expression::TypeIndex { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_expression_path!(parser, parser.tree.get(*index), "P");
                            });
                            assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Any));
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_index_with_conditional() {
        let mut test =
            TestParser::new("type Lookup<Depth> = Foo[Depth extends -1 ? \"done\" : \"recur\"]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Lookup<Depth> = Foo[Depth extends -1 ? "done" : "recur"]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "Foo");
                    assert_node!(parser.tree, *index, Expression::TypeConditional { left, right, then_type, else_type } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Depth");
                        assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                            assert_eq!(*operator, UnaryOperator::Negate);
                            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                        });
                        assert_node!(parser.tree, *then_type, Expression::ScalarLiteral(ScalarLiteral::String(then_id)) => {
                            assert_string!(parser, *then_id, "done");
                        });
                        assert_node!(parser.tree, *else_type, Expression::ScalarLiteral(ScalarLiteral::String(else_id)) => {
                            assert_string!(parser, *else_id, "recur");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_index_access_chain() {
        let mut test = TestParser::new("type T = A[B][C]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A[B][C]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*index), "C");
                    assert_node!(parser.tree, *left, Expression::TypeIndex { left, index } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "A");
                        assert_expression_path!(parser, parser.tree.get(*index), "B");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_predicate_expression() {
        let mut test = TestParser::new("type T = value is string");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = value is string
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypePredicate { asserts, subject, target } => {
                    assert!(!asserts);
                    assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                    assert_node!(parser.tree, target.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                });
            });
        });
    }

    #[test]
    fn test_parse_type_literal_call_signature() {
        let mut test = TestParser::new("type T = { (): string }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { (): string }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_literal_construct_signature() {
        let mut test = TestParser::new("type T = { new (x: number): Foo }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { new (x: number): Foo }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, .. } => {
                        assert!(key.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::New));
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "x");
                            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
                        });
                        assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "Foo");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_literal_abstract_construct_signature() {
        let mut test = TestParser::new("type T = { abstract new (x: number): Foo }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { abstract new (x: number): Foo }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, .. } => {
                        assert!(key.is_none());
                        assert_eq!(signature.abstraction, FunctionAbstraction::Abstract);
                        assert_eq!(signature.mode, Some(FunctionMode::New));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_literal_index_signature() {
        let mut test = TestParser::new("type T = { readonly [k: string]?: Foo }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { readonly [k: string]?: Foo }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { modifiers, key, value, .. } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(*modifiers, BindingModifier {
                            kind: Some(BindingKind::Maybe),
                            mutability: Some(Mutability::Immutable),
                            ..BindingModifier::default()
                        });
                        let key = key.as_ref().expect("expected key");
                        match key {
                            Key::NamedExpression { name, key } => {
                                assert_string!(parser, *name, "k");
                                assert_node!(parser.tree, *key, Expression::TypeLiteral(TypeLiteral::String));
                            }
                            _ => panic!("expected Key::NamedExpression, got {key:?}"),
                        }
                        assert_expression_path!(parser, parser.tree.get(value.unwrap()), "Foo");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_intrinsic_type_alias() {
        let mut test = TestParser::new("type Uppercase<S extends string> = intrinsic");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Uppercase<S extends string> = intrinsic
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Intrinsic(intrinsic)) => {
                    assert_eq!(*intrinsic, TypeIntrinsic::Uppercase);
                });
            });
        });
    }
}
