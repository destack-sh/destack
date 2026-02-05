use crate::parse::timing::tags;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, Declaration, DeclarationDescriptor, Expression, FloatType, IntType, IntrinsicType,
    Keyword, LocalNodeId, Mutability, Name, ScalarLiteral, StringId, TokenType, TypeBinaryOperator,
    TypeKind, TypeLiteral, TypeMappedModifiers, TypeMappedParameter, TypeModifier,
    TypePredicateSubject, TypeUnaryOperator, UnaryOperator, VarianceBound,
};
use destack_source::NodeSpanType;

impl Parser {
    /// Map cached identifier ids to always-available type literals.
    #[inline]
    fn type_literal_always_available_id(&self, identifier_id: StringId) -> Option<TypeLiteral> {
        let ids = &self.type_literal_identifiers;
        if identifier_id == ids.undefined {
            return Some(TypeLiteral::Undefined);
        }
        if identifier_id == ids.unknown {
            return Some(TypeLiteral::Unknown);
        }
        if identifier_id == ids.object {
            return Some(TypeLiteral::Object);
        }
        if identifier_id == ids.null_ {
            return Some(TypeLiteral::Null);
        }
        if identifier_id == ids.any {
            return Some(TypeLiteral::Any);
        }
        if identifier_id == ids.never {
            return Some(TypeLiteral::Never);
        }
        None
    }

    /// Map cached identifier ids to type only literals.
    #[inline]
    fn type_literal_type_context_id(
        &self,
        identifier_id: StringId,
        next_identifier_id: Option<StringId>,
    ) -> Option<TypeLiteral> {
        let ids = &self.type_literal_identifiers;
        if identifier_id == ids.boolean {
            return Some(TypeLiteral::Boolean);
        }
        if identifier_id == ids.void {
            return Some(TypeLiteral::Void);
        }
        if identifier_id == ids.character {
            return Some(TypeLiteral::Character);
        }
        if identifier_id == ids.string {
            return Some(TypeLiteral::String);
        }
        if identifier_id == ids.bigint {
            return Some(TypeLiteral::Bigint);
        }
        if identifier_id == ids.number {
            return Some(TypeLiteral::Number);
        }
        if identifier_id == ids.int {
            return Some(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: true,
            }));
        }
        if identifier_id == ids.isize {
            return Some(TypeLiteral::Int(IntType::Pointer { is_signed: true }));
        }
        if identifier_id == ids.uint {
            return Some(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: false,
            }));
        }
        if identifier_id == ids.usize {
            return Some(TypeLiteral::Int(IntType::Pointer { is_signed: false }));
        }
        if identifier_id == ids.float {
            return Some(TypeLiteral::Float(FloatType { width: None }));
        }
        if identifier_id == ids.symbol {
            return Some(TypeLiteral::Symbol);
        }
        if identifier_id == ids.unique && next_identifier_id == Some(ids.symbol) {
            return Some(TypeLiteral::UniqueSymbol);
        }
        None
    }
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

    /// Peek a type literal.
    /// Certain type literals are only parsed at the AST-level in static or type contexts.
    /// (This prevents shadowing in case we have a variable or parameter named `int` or `number`.)
    pub fn peek_type_literal(&mut self) -> ParseResult<TypeLiteral> {
        let next = *self.peek()?;
        let next_type = next.token.ty;
        let has_split = self.has_active_split();
        let identifier_id = if !has_split && next_type == TokenType::Identifier {
            self.identifier_for_index(self.pos_index())
        } else {
            None
        };

        // always available type literals
        if let Some(identifier_id) = identifier_id
            && let Some(literal) = self.type_literal_always_available_id(identifier_id)
        {
            return Ok(literal);
        }

        // bail if not inside static or type context
        if !self.options.in_type && !self.options.in_static {
            return Err(ParseError::unexpected(next.span));
        }

        let next_next = self.peek_next().ok().copied();
        let next_next_type = next_next.map(|next| next.token.ty);
        if let Some(identifier_id) = identifier_id {
            let next_identifier_id = if !has_split && next_next_type == Some(TokenType::Identifier)
            {
                self.identifier_for_index(self.index_for_next())
            } else {
                None
            };
            if let Some(literal) =
                self.type_literal_type_context_id(identifier_id, next_identifier_id)
            {
                return Ok(literal);
            }
        }
        let next_str = self.get_span_str(next.span);
        let next_next_str = next_next.map(|next| self.get_span_str(next.span));

        // !
        if next_type == TokenType::Not {
            if let Some(next_next_str) = next_next_str
                && let Some(next_next_type) = next_next_type
                && self.is_start_of_expression(next_next_str, next_next_type)
            {
                // do nothing
            } else {
                return Ok(TypeLiteral::Never);
            }
        }

        // regular single-token type literals (also only inside static/type context)
        match next_str {
            // boolean
            "boolean" => Ok(TypeLiteral::Boolean),
            // void
            "void" => Ok(TypeLiteral::Void),
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
            "isize" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: true })),
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
            "usize" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: false })),
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
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_TYPE);

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
            && (self.peek_next_is(TokenType::Assign) || self.peek_next_is(TokenType::LessThan))
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

            // static parameters (speculative: may fail for type expressions like Foo<T[number]>)
            let static_parameters = match self.eat_static_parameters_maybe() {
                Ok(params) => params,
                Err(_) => {
                    // failed to parse as parameters, restore and fall through to expression
                    self.restore(speculative_start.0.clone(), speculative_start.1);
                    None
                }
            };

            // if followed by =, then it's a type alias
            if self.peek_is(TokenType::Assign) {
                let _timing = self.timing_scope(tags::PARSE_TYPE_DECLARATION);
                // =
                self.eat_token(TokenType::Assign)?;
                self.eat_newlines_maybe()?;
                // value
                let mut value_options = self.options.not_in_position().in_type();
                if self.options.in_type_conditional_right {
                    value_options = value_options.in_type_conditional_right();
                }
                let value_id =
                    self.with_options(value_options, |parser| parser.eat_expression())?;
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
                // re-parse from before the name to get static arguments properly
                self.restore(speculative_start.0.clone(), speculative_start.1);
                let mut right_options = self.options.not_in_position().in_type();
                if self.options.in_type_conditional_right {
                    right_options = right_options.in_type_conditional_right();
                }
                let right = self.with_options(right_options, |parser| parser.eat_expression())?;
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
            let mut right_options = self.options.not_in_position().in_type();
            if self.options.in_type_conditional_right {
                right_options = right_options.in_type_conditional_right();
            }
            let right = self.with_options(right_options, |parser| parser.eat_expression())?;
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
    fn type_intrinsic_for_name(&self, name: &Name) -> Option<IntrinsicType> {
        // only identifiers can be intrinsic aliases
        let Name::Identifier(name_id) = name else {
            return None;
        };

        // map identifier to intrinsic
        let name_str = self.strings.get(*name_id);
        IntrinsicType::try_from(name_str).ok()
    }

    /// Check whether `infer ... extends ... ?` should parse as a conditional type.
    fn infer_extends_starts_conditional(&mut self) -> bool {
        // quick reject when extends is not next
        if self.peek_keyword(Keyword::Extends).is_err() {
            return false;
        }

        // prefer infer constraints on conditional right unless nested
        let mut require_nested_close =
            self.options.in_type_conditional_right && !self.options.in_parenthesis;
        if require_nested_close {
            // allow nested conditionals inside delimited lists
            let mut index = self.pos() as isize - 1;
            while index >= 0 {
                let token = self.tokens().get(index as usize);
                let Some(token) = token else {
                    break;
                };
                let token_ty = token.token.ty;
                if token_ty == TokenType::Newline {
                    index -= 1;
                    continue;
                }
                if token_ty == TokenType::Identifier
                    && self.keyword_for_index(index as usize) == Some(Keyword::Infer)
                {
                    index -= 1;
                    while index >= 0 {
                        let token = self.tokens().get(index as usize);
                        let Some(token) = token else {
                            break;
                        };
                        let token_ty = token.token.ty;
                        if token_ty == TokenType::Newline {
                            index -= 1;
                            continue;
                        }
                        if matches!(
                            token_ty,
                            TokenType::OpenBracket
                                | TokenType::OpenBrace
                                | TokenType::OpenParenthesis
                                | TokenType::Comma
                        ) {
                            require_nested_close = false;
                        }
                        break;
                    }
                    break;
                }
                index -= 1;
            }
        }

        // scan until a conditional boundary or a terminating token
        let mut index = self.pos() as usize + 1;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        loop {
            self.token_stream.ensure_token(index);
            let Some(token) = self.tokens().get(index) else {
                break;
            };
            match token.token.ty {
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                }
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => {
                    if bracket_depth == 0 {
                        break;
                    }
                    bracket_depth -= 1;
                }
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                }
                TokenType::Maybe => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        if !require_nested_close {
                            return true;
                        }

                        // confirm the conditional is nested inside a grouping
                        let mut look_index = index + 1;
                        let mut look_paren_depth = 0usize;
                        let mut look_bracket_depth = 0usize;
                        let mut look_brace_depth = 0usize;
                        loop {
                            self.token_stream.ensure_token(look_index);
                            let Some(look_token) = self.tokens().get(look_index) else {
                                break;
                            };
                            match look_token.token.ty {
                                TokenType::OpenParenthesis => look_paren_depth += 1,
                                TokenType::CloseParenthesis => {
                                    if look_paren_depth == 0 {
                                        return true;
                                    }
                                    look_paren_depth -= 1;
                                }
                                TokenType::OpenBracket => look_bracket_depth += 1,
                                TokenType::CloseBracket => {
                                    if look_bracket_depth == 0 {
                                        return true;
                                    }
                                    look_bracket_depth -= 1;
                                }
                                TokenType::OpenBrace => look_brace_depth += 1,
                                TokenType::CloseBrace => {
                                    if look_brace_depth == 0 {
                                        return true;
                                    }
                                    look_brace_depth -= 1;
                                }
                                TokenType::Comma
                                | TokenType::Semicolon
                                | TokenType::Arrow
                                | TokenType::ArrowWide
                                | TokenType::TemplateStringMiddle
                                | TokenType::TemplateStringEnd => {
                                    if look_paren_depth == 0
                                        && look_bracket_depth == 0
                                        && look_brace_depth == 0
                                    {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            look_index += 1;
                        }
                        return false;
                    }
                }
                TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Colon
                | TokenType::Arrow
                | TokenType::ArrowWide
                | TokenType::TemplateStringMiddle
                | TokenType::TemplateStringEnd => {
                    // stop at template literal boundaries to avoid crossing interpolations
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    /// Eat type import arguments.
    fn eat_type_import_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        // open argument list
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty argument list
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // positional arguments
        let arguments = self.with_options(self.options.nested().not_in_position(), |parser| {
            parser.eat_positional_arguments_body(TokenType::CloseParenthesis)
        })?;

        // close argument list
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(arguments)
    }

    /// Eat a type infer expression.
    pub fn eat_type_infer_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // parse infer name
        let start = self.mark();
        self.eat_keyword(Keyword::Infer)?;
        let (name, name_span) = self.eat_identifier_with_span()?;

        // constraint (e.g., infer T extends U)
        let mut constraint_options = self.options.not_in_position().in_type();
        if self.options.in_type_conditional_right {
            constraint_options = constraint_options.in_type_conditional_right();
        }
        let constraint = if self.peek_keyword(Keyword::Extends).is_ok()
            && !self.infer_extends_starts_conditional()
        {
            self.bump(); // eat extends
            self.eat_newlines_maybe()?;
            Some(self.with_options(constraint_options, |parser| parser.eat_expression())?)
        } else {
            None
        };
        let expr_id = self.tree.insert(
            Expression::TypeInfer { name, constraint },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(expr_id, name_span);
        Ok(expr_id)
    }

    /// Eat a type import expression.
    pub fn eat_type_import_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // arguments
        let arguments = self.with_options(self.options.nested().not_in_position(), |parser| {
            parser.eat_type_import_arguments()
        })?;

        // target
        if arguments.is_empty() {
            return Err(ParseError::expected(
                self.get_span_from(&start),
                TokenType::Literal,
            ));
        }
        let (target, target_span) = {
            let first_argument = self.tree.get(arguments[0]);
            let Argument::Positional { value, .. } = first_argument else {
                return Err(ParseError::expected(
                    self.tree.get_span(arguments[0]),
                    TokenType::Literal,
                ));
            };
            let value_expression = self.tree.get(*value);
            let Expression::ScalarLiteral(ScalarLiteral::String(target)) = value_expression else {
                return Err(ParseError::expected(
                    self.tree.get_span(*value),
                    TokenType::Literal,
                ));
            };
            (*target, self.tree.get_span(*value))
        };

        // qualifier (e.g., import("mod").Type)
        let (qualifier, static_arguments) = if self.peek_is(TokenType::Dot) {
            self.bump(); // eat dot
            let qualifier = self.eat_path()?;
            // only consume newlines when a static argument list follows
            let static_arguments = if self.peek_is(TokenType::Newline)
                && (self
                    .peek_token_after_newlines(self.pos(), TokenType::LessThan)
                    .is_ok()
                    || self
                        .peek_token_after_newlines(self.pos(), TokenType::ShiftLeft)
                        .is_ok())
            {
                self.eat_newlines_maybe()?;
                self.eat_static_arguments_maybe()?
            } else {
                self.eat_static_arguments_maybe()?
            };
            (Some(qualifier), static_arguments)
        } else {
            (None, None)
        };

        let expr_id = self.tree.insert(
            Expression::TypeImport {
                target,
                arguments,
                qualifier,
                static_arguments,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(expr_id, target_span);
        Ok(expr_id)
    }

    pub fn eat_type_predicate_asserts(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Asserts)?;
        self.eat_newlines_maybe()?;

        // asserts this | asserts param
        let (subject, subject_span) = if self.peek_keyword(Keyword::This).is_ok() {
            let token = *self.peek()?;
            self.bump(); // eat this
            (TypePredicateSubject::This, token.span)
        } else {
            let (name, span) = self.eat_identifier_with_span()?;
            (TypePredicateSubject::Identifier(name), span)
        };

        // optional target: asserts x is T
        let target = if self.peek_keyword(Keyword::Is).is_ok() {
            self.bump(); // eat is
            self.eat_newlines_maybe()?;
            let mut target_options = self.options.not_in_position().in_type();
            if self.options.in_type_conditional_right {
                target_options = target_options.in_type_conditional_right();
            }
            Some(self.with_options(target_options, |parser| parser.eat_expression())?)
        } else {
            None
        };

        let expr_id = self.tree.insert(
            Expression::TypePredicate {
                asserts: true,
                subject,
                target,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(expr_id, subject_span);
        Ok(expr_id)
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
        let constraint = self.with_options(
            self.options
                .not_in_position()
                .not_in_left_precedence()
                .in_type()
                .in_type_mapped_constraint(),
            |parser| parser.eat_expression(),
        )?;

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
                self.with_options(
                    self.options
                        .not_in_position()
                        .not_in_left_precedence()
                        .in_type(),
                    |parser| parser.eat_expression(),
                )?,
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
        let value = self.with_options(
            self.options
                .not_in_position()
                .not_in_left_precedence()
                .in_type(),
            |parser| parser.eat_expression(),
        )?;
        self.eat_newlines_maybe()?;
        if self.peek_is(TokenType::Semicolon) || self.peek_is(TokenType::Comma) {
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
            self.get_span_from(&start),
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
        else if self.peek_is(TokenType::Subtract)
            && self.peek_next_keyword(Keyword::Readonly).is_ok()
        {
            self.bump(); // eat -
            self.bump(); // eat readonly
            return Ok(TypeModifier::Remove);
        }
        // +readonly
        else if self.peek_is(TokenType::Add) && self.peek_next_keyword(Keyword::Readonly).is_ok()
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
        if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat ?
            return Ok(TypeModifier::Add);
        }
        // -?
        else if self.peek_is(TokenType::Subtract) && self.peek_next_is(TokenType::Maybe) {
            self.bump(); // eat -
            self.bump(); // eat ?
            return Ok(TypeModifier::Remove);
        }
        // +?
        else if self.peek_is(TokenType::Add) && self.peek_next_is(TokenType::Maybe) {
            self.bump(); // eat +
            self.bump(); // eat ?
            return Ok(TypeModifier::Add);
        }
        Ok(TypeModifier::None)
    }

    /// Eat extends types (without the leading keyword).
    pub fn eat_extends_types_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // allow newlines before extends
        let mark = self.mark();
        self.eat_newlines_maybe()?;

        // check for extends keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            self.eat_super_types_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
        } else {
            self.rewind(mark);
            Ok(None)
        }
    }

    /// Eat implements types maybe.
    #[inline]
    pub fn eat_implements_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // allow newlines before implements
        let mark = self.mark();
        self.eat_newlines_maybe()?;

        // check for implements keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            self.eat_super_types_maybe(&[Keyword::With, Keyword::Where])
        } else {
            self.rewind(mark);
            Ok(None)
        }
    }

    /// Eat a super type clause maybe.
    #[inline]
    fn eat_super_types_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        let is_parenthesized = if self.peek_is(TokenType::OpenParenthesis) {
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
        // super type list
        let mut types: Vec<LocalNodeId<Expression>> = Vec::new();

        // collect super types until a terminator is seen
        while self.has_more_tokens() {
            // eat until open brace or close parenthesis
            if self.peek_is(TokenType::OpenBrace)
                || self.peek_is(TokenType::CloseParenthesis)
                || terminators
                    .iter()
                    .any(|terminator| self.peek_keyword(*terminator).is_ok())
            {
                break;
            }
            // stop on newline if the next non-newline token is a terminator
            else if self.peek_is(TokenType::Newline) {
                let mark = self.mark();
                self.eat_newlines_maybe()?;
                let is_terminator = self.peek_is(TokenType::OpenBrace)
                    || self.peek_is(TokenType::CloseParenthesis)
                    || terminators
                        .iter()
                        .any(|terminator| self.peek_keyword(*terminator).is_ok());
                if is_terminator {
                    self.rewind(mark);
                    break;
                } else {
                    continue;
                }
            }
            // consume any stop
            else if self.is_item_stop() {
                self.eat_item_stop_with_newlines()?;
            }
            // keep eating super types
            else {
                // parse the super type expression
                let type_start = self.mark();
                let ty = self.with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })?;

                // record the full type span for super types
                self.tree
                    .set_side_span(ty, NodeSpanType::Type, self.get_span_from(&type_start));

                // record the parsed type
                types.push(ty);
            }
        }

        Ok(types)
    }
}

#[cfg(test)]
mod tests {
    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};
    use destack_ast::{
        Argument, BinaryOperator, BindingKind, BindingModifier, Declaration, Expression,
        FunctionAbstraction, FunctionKind, FunctionMode, IntType, IntrinsicType, Key, Mutability,
        Name, Parameter, Property, ScalarLiteral, TypeBinaryOperator, TypeLiteral,
        TypeMappedModifiers, TypeModifier, TypePredicateSubject, TypeUnaryOperator, UnaryOperator,
    };
    use destack_source::LanguageType;

    #[test]
    fn test_parse_type_alias() {
        let mut test = TestParser::new("type T = int32");
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 1);
        let expr_id = expressions[0];
        // type T = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    /// Parse a type alias followed by a tree literal in TSX.
    #[test]
    fn test_parse_type_alias_before_tree_literal() {
        let mut test = TestParser::new_with_options(
            "type X = typeof Array\n<div>a</div>;",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 2);

        // type X = typeof Array
        assert_node!(parser.tree, expressions[0], Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "X");
            });
        });

        // <div>a</div>
        let tree_id = match parser.tree.get(expressions[1]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[1],
        };
        assert_node!(parser.tree, tree_id, Expression::TreeExpression { .. } => {});
    }

    #[test]
    fn test_parse_bigint_literal_type() {
        let mut test = TestParser::new_with_options("let x: 0n;", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            let declarator = parser.tree.get(declarators[0]);
            let ty = declarator.ty.expect("expected type");
            assert_node!(parser.tree, ty, Expression::ScalarLiteral(ScalarLiteral::Bigint(0)));
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
        let mut test = TestParser::new("type T<A, B> = isize");
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

    /// Parse pointer types in a type alias.
    #[test]
    fn test_parse_pointer_type_alias() {
        let mut test = TestParser::new("type Ptr = *mut int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Ptr");
                assert_node!(parser.tree, *value, Expression::PointerOf { mutability, right } => {
                    assert_eq!(*mutability, Some(Mutability::Mutable));
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                });
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
    fn test_parse_type_parameter_default_conditional() {
        let mut test = TestParser::new(
            "type Wrapper<F extends Function, ReturnType = F extends (...args: any) => infer T ? T : unknown> = ReturnType",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Wrapper<F extends Function, ReturnType = F extends (...args: any) => infer T ? T : unknown> = ReturnType
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { static_parameters: Some(static_parameters), .. } => {
                assert_eq!(static_parameters.len(), 2);
                assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, default: Some(default), .. } => {
                    assert_string!(parser, *name, "ReturnType");
                    assert_node!(parser.tree, *default, Expression::TypeConditional { left, right, then_type, else_type } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "F");
                        assert_node!(parser.tree, *right, Expression::Declaration(function_id) => {
                            assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeInfer { name, constraint } => {
                                    assert_string!(parser, *name, "T");
                                    assert!(constraint.is_none());
                                });
                            });
                        });
                        assert_expression_path!(parser, parser.tree.get(*then_type), "T");
                        assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Unknown));
                    });
                });
            });
        });
    }

    /// Parse conditional types with infer constraints.
    #[test]
    fn test_parse_type_conditional_with_infer_constraint() {
        let mut test = TestParser::new(
            "type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { right, then_type, else_type, .. } => {
                    assert_node!(parser.tree, *right, Expression::TypeInfer { constraint, .. } => {
                        assert!(constraint.is_some());
                    });
                    assert_node!(parser.tree, *then_type, Expression::Path { .. });
                    assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Never));
                });
            });
        });
    }

    /// Parse conditional types where infer-extends is a constraint inside parentheses.
    #[test]
    fn test_parse_type_conditional_infer_extends_parenthesized_constraint() {
        let mut test = TestParser::new_with_options(
            "type X = T extends (infer U extends number) ? 1 : 0",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type X = T extends (infer U extends number) ? 1 : 0
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "T");
                    assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TypeInfer { name, constraint } => {
                            assert_string!(parser, *name, "U");
                            assert_node!(parser.tree, constraint.expect("expected constraint"), Expression::TypeLiteral(TypeLiteral::Number));
                        });
                    });
                    assert_node!(parser.tree, *then_type, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    assert_node!(parser.tree, *else_type, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });
        });
    }

    /// Parse conditional types where infer-extends starts a nested conditional.
    #[test]
    fn test_parse_type_conditional_infer_extends_parenthesized_conditional() {
        let mut test = TestParser::new_with_options(
            "type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "T");
                    assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TypeConditional { left, right, then_type, else_type } => {
                            assert_node!(parser.tree, *left, Expression::TypeInfer { name, constraint } => {
                                assert_string!(parser, *name, "U");
                                assert!(constraint.is_none());
                            });
                            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                            assert_node!(parser.tree, *then_type, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                            assert_node!(parser.tree, *else_type, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                        });
                    });
                    assert_node!(parser.tree, *then_type, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    assert_node!(parser.tree, *else_type, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
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
    fn test_parse_conditional_type_with_abstract_construct_signature() {
        let mut test =
            TestParser::new("type T = A extends abstract new (x: number) => infer U ? U : unknown");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends abstract new (x: number) => infer U ? U : unknown
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, right, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert_node!(parser.tree, *right, Expression::Declaration(function_id) => {
                        assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                            assert_eq!(signature.abstraction, FunctionAbstraction::Abstract);
                            assert_eq!(signature.mode, Some(FunctionMode::New));
                            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeInfer { name, constraint } => {
                                assert_string!(parser, *name, "U");
                                assert!(constraint.is_none());
                            });
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "U");
                    assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Unknown));
                });
            });
        });
    }

    #[test]
    fn test_parse_type_union_with_construct_signature() {
        let mut test = TestParser::new("type T = RegExp | (new() => object)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = RegExp | (new() => object)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_expression_path!(parser, parser.tree.get(*left), "RegExp");
                    assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Declaration(function_id) => {
                            assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                                assert_eq!(signature.mode, Some(FunctionMode::New));
                            });
                        });
                    });
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
        let mut test = TestParser::new(
            r#"type T = A extends B ?
    C extends D ? E : F
    : G"#,
        );
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

    /// Parse extends with readonly array types.
    #[test]
    fn test_parse_type_extends_readonly_array() {
        let mut test = TestParser::new("type T = A extends readonly unknown[]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends readonly unknown[]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, .. } => {
                    assert_eq!(*operator, TypeBinaryOperator::Extends);
                });
            });
        });
    }

    /// Parse multiline conditional types with readonly array constraints.
    #[test]
    fn test_parse_type_conditional_multiline_readonly_array() {
        let mut test = TestParser::new(
            r#"type IsTuple<T> = T extends readonly unknown[]
  ? number extends T["length"]
    ? false
    : true
  : false"#,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type IsTuple<T> = T extends readonly unknown[] ? number extends T["length"] ? false : true : false
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
    fn test_parse_tuple_type_with_spread() {
        let mut test = TestParser::new("type T = [...Parts, string]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [...Parts, string]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], Argument::Spread { label, value, modifiers } => {
                        assert!(label.is_none());
                        assert!(modifiers.is_none());
                        assert_expression_path!(parser, parser.tree.get(*value), "Parts");
                    });
                    assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_optional_tuple_element() {
        let mut test = TestParser::new("type T = [EventTarget?]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [EventTarget?]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers, value } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                        assert_expression_path!(parser, parser.tree.get(*value), "EventTarget");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_optional_tuple_element_trailing_comma() {
        let mut test = TestParser::new("type T = [EventTarget?,]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [EventTarget?,]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers, value } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                        assert_expression_path!(parser, parser.tree.get(*value), "EventTarget");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_optional_labeled_tuple_element() {
        let mut test = TestParser::new("type T = [start?: number]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [start?: number]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Labeled { modifiers, label, value } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                        assert_string!(parser, *label, "start");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_optional_readonly_tuple_element() {
        let mut test = TestParser::new("type T = [readonly EventTarget?]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [readonly EventTarget?]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers, value } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                        assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                        assert_expression_path!(parser, parser.tree.get(*value), "EventTarget");
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
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    // number
                    assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
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
                assert_node!(parser.tree, *value, Expression::TypeImport { target, arguments, qualifier, static_arguments } => {
                    assert_string!(parser, *target, "mod");
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                            assert_string!(parser, *string_id, "mod");
                        });
                    });
                    assert_path!(parser, qualifier.as_ref().unwrap(), "Type");
                    assert!(static_arguments.is_none());
                });
            });
        });
    }

    #[test]
    fn test_parse_type_import_expression_with_static_arguments() {
        let mut test = TestParser::new("type T = import(\"mod\").Type<string, number>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = import("mod").Type<string, number>
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeImport { target, arguments, qualifier, static_arguments } => {
                    assert_string!(parser, *target, "mod");
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                            assert_string!(parser, *string_id, "mod");
                        });
                    });
                    assert_path!(parser, qualifier.as_ref().unwrap(), "Type");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 2);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    assert_node!(parser.tree, static_arguments[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    /// Parse type import expressions with attributes.
    #[test]
    fn test_parse_type_import_expression_with_attributes() {
        let mut test = TestParser::new(
            "type T = import(\"vite\", { with: { \"resolution-mode\": \"import\" } })",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeImport { target, arguments, .. } => {
                    assert_string!(parser, *target, "vite");
                    assert_eq!(arguments.len(), 2);
                    assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
                    });
                });
            });
        });
    }

    /// Parse type import expressions with trailing commas.
    #[test]
    fn test_parse_type_import_expression_with_trailing_comma() {
        let mut test = TestParser::new("type T = import(\"vite\",)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeImport { target, arguments, .. } => {
                    assert_string!(parser, *target, "vite");
                    assert_eq!(arguments.len(), 1);
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
    fn test_parse_type_template_literal_with_static_arguments() {
        let mut test = TestParser::new("type T = `foo-${Capitalize<K>}`");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = `foo-${Capitalize<K>}`
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeTemplateLiteral { strings, spans } => {
                    assert_eq!(strings.len(), 2);
                    assert_eq!(spans.len(), 1);
                    assert_string!(parser, strings[0], "foo-");
                    assert_string!(parser, strings[1], "");
                    assert_node!(parser.tree, spans[0], Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "Capitalize");
                        let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                        assert_eq!(static_arguments.len(), 1);
                        assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "K");
                        });
                    });
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
    fn test_parse_type_mapped_expression_with_key_remap_conditional() {
        let mut test =
            TestParser::new("type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeMapped { parameter, .. } => {
                    assert_string!(parser, parameter.name, "K");
                    let key_remap = parameter.key_remap.expect("expected key remap");
                    assert_node!(parser.tree, key_remap, Expression::TypeConditional { .. });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_mapped_expression_typescript_declaration() {
        let mut test = TestParser::new_with_options(
            "type T = { [K in keyof T]: T[K] }",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { [K in keyof T]: T[K] }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeMapped { .. });
            });
        });
    }

    #[test]
    fn test_parse_type_mapped_expression_with_remap_typescript_declaration() {
        let mut test = TestParser::new_with_options(
            "type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeMapped { parameter, .. } => {
                    assert_string!(parser, parameter.name, "K");
                    assert!(parameter.key_remap.is_some());
                });
            });
        });
    }

    #[test]
    fn test_parse_leading_intersection_with_mapped_types_typescript_declaration() {
        let mut test = TestParser::new_with_options(
            r#"type T = (
  & { [K in keyof T]: T[K] }
  & { [K in keyof T as K extends string ? K : never]: T[K] }
)"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = (& { [K in keyof T]: T[K] } & { [K in keyof T as K extends string ? K : never]: T[K] })
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { operator, left, right } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                        assert_node!(parser.tree, *left, Expression::TypeMapped { .. });
                        assert_node!(parser.tree, *right, Expression::TypeMapped { .. });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_leading_intersection_simple_typescript_declaration() {
        let mut test = TestParser::new_with_options(
            r#"type T = (
  & A
  & B
)"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = (& A & B)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { operator, left, right } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                        assert_expression_path!(parser, parser.tree.get(*left), "A");
                        assert_expression_path!(parser, parser.tree.get(*right), "B");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_leading_intersection_with_mapped_type_and_path_typescript_declaration() {
        let mut test = TestParser::new_with_options(
            r#"type T = (
  & { [K in keyof T]: T[K] }
  & A
)"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = (& { [K in keyof T]: T[K] } & A)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { operator, left, right } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                        assert_node!(parser.tree, *left, Expression::TypeMapped { .. });
                        assert_expression_path!(parser, parser.tree.get(*right), "A");
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

    /// Indexed access type inside generic arguments.
    #[test]
    fn test_parse_generic_with_indexed_access_type() {
        // Foo<T[number]> - indexed access inside generic
        let mut test = TestParser::new("type A = Foo<T[number]>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                    assert_eq!(path.segments.len(), 1);
                    let args = static_arguments.as_ref().unwrap();
                    assert_eq!(args.len(), 1);
                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "T");
                            assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                        });
                    });
                });
            });
        });
    }

    /// Indexed access type followed by array suffix.
    #[test]
    fn test_parse_indexed_access_with_array_suffix() {
        // T[number][] - indexed access followed by array type
        let mut test = TestParser::new("type A = T[number][]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                // outer [] is Index with no index (array type)
                assert_node!(parser.tree, *value, Expression::Index { left, index, .. } => {
                    assert!(index.is_none());
                    // inner T[number] is TypeIndex
                    assert_node!(parser.tree, *left, Expression::TypeIndex { left, index } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                        assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    /// Generic type with indexed access, followed by array suffix.
    #[test]
    fn test_parse_generic_indexed_access_with_array_suffix() {
        // Foo<T[number]>[] - the pattern from deno builtins
        let mut test = TestParser::new("type A = Foo<T[number]>[]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                // outer [] is Index with no index (array type)
                assert_node!(parser.tree, *value, Expression::Index { left, index, .. } => {
                    assert!(index.is_none());
                    // inner Foo<T[number]> is Path with static arguments
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                        assert_eq!(path.segments.len(), 1);
                        let args = static_arguments.as_ref().unwrap();
                        assert_eq!(args.len(), 1);
                        assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                            });
                        });
                    });
                });
            });
        });
    }

    /// Generic type with indexed access in TypeScript declaration file.
    #[test]
    fn test_parse_generic_indexed_access_typescript_declaration() {
        // Foo<T[number]>[] in TypeScript declaration context
        let mut test = TestParser::new_with_options(
            "type A = Foo<T[number]>[]",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                // outer [] is Index with no index (array type)
                assert_node!(parser.tree, *value, Expression::Index { left, index, .. } => {
                    assert!(index.is_none());
                    // inner Foo<T[number]> is Path with static arguments
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                        assert_eq!(path.segments.len(), 1);
                        let args = static_arguments.as_ref().unwrap();
                        assert_eq!(args.len(), 1);
                        assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                            });
                        });
                    });
                });
            });
        });
    }

    /// Nested generic closings should not parse as shift-right operators in type context.
    #[test]
    fn test_parse_nested_generic_closings_in_type() {
        let mut test = TestParser::new("type A = Foo<Bar<Baz<Qux>>>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                    assert_eq!(path.segments.len(), 1);
                    let args = static_arguments.as_ref().unwrap();
                    assert_eq!(args.len(), 1);
                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            assert_eq!(path.segments.len(), 1);
                            let args = static_arguments.as_ref().unwrap();
                            assert_eq!(args.len(), 1);
                            assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                                    assert_eq!(path.segments.len(), 1);
                                    assert_expression_path!(parser, parser.tree.get(*value), "Baz");
                                    let args = static_arguments.as_ref().unwrap();
                                    assert_eq!(args.len(), 1);
                                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                                        assert_expression_path!(parser, parser.tree.get(*value), "Qux");
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    /// Tuple expressions inside static arguments should parse as a single argument.
    #[test]
    fn test_parse_tuple_static_argument() {
        let mut test = TestParser::new_with_options(
            "type A = And<[Left, Right]>",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path: _, static_arguments } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "And");
                    let args = static_arguments.as_ref().unwrap();
                    assert_eq!(args.len(), 1);
                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                            assert_eq!(elements.len(), 2);
                            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "Left");
                            });
                            assert_node!(parser.tree, elements[1], Argument::Positional { value, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "Right");
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parenthesized tuple expressions inside static arguments should stay grouped.
    #[test]
    fn test_parse_parenthesized_tuple_static_argument() {
        let mut test = TestParser::new_with_options(
            "type Alias = Wrap<(number, string)>;",
            LanguageType::Destack,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { static_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "Wrap");
                    let args = static_arguments.as_ref().unwrap();
                    assert_eq!(args.len(), 1);
                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::TupleExpression { elements } => {
                            assert_eq!(elements.len(), 2);
                            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                            });
                            assert_node!(parser.tree, elements[1], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                            });
                        });
                    });
                });
            });
        });
    }

    /// Nested generic arguments inside tuple static arguments should stay grouped.
    #[test]
    fn test_parse_tuple_static_argument_with_nested_generics() {
        let mut test = TestParser::new_with_options(
            r#"type A<Actual> = And<[Extends<PrintType<Actual>, "...">, Not<IsAny<Actual>>]>;"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path: _, static_arguments } => {
                    let args = static_arguments.as_ref().unwrap();
                    assert_eq!(args.len(), 1);
                    assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                            assert_eq!(elements.len(), 2);
                        });
                    });
                });
            });
        });
    }

    /// Tuple static arguments inside a conditional type should stay grouped.
    #[test]
    fn test_parse_tuple_static_argument_in_type_conditional() {
        let mut test = TestParser::new_with_options(
            r#"type A<Actual, Expected> = And<[Extends<PrintType<Actual>, "...">, Not<IsAny<Actual>>]> extends true ? Actual : Expected;"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { left, .. } => {
                    assert_node!(parser.tree, *left, Expression::Path { path: _, static_arguments } => {
                        let args = static_arguments.as_ref().unwrap();
                        assert_eq!(args.len(), 1);
                        assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                                assert_eq!(elements.len(), 2);
                            });
                        });
                    });
                });
            });
        });
    }

    /// Readonly prefix with generic containing indexed access type.
    #[test]
    fn test_parse_readonly_generic_indexed_access() {
        // readonly Foo<T[number]>
        let mut test = TestParser::new_with_options(
            "type A = readonly Foo<T[number]>",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                // readonly Foo<T[number]>
                assert_node!(parser.tree, *value, Expression::TypeUnary { right, .. } => {
                    // Foo<T[number]>
                    assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                        assert_eq!(path.segments.len(), 1);
                        let args = static_arguments.as_ref().unwrap();
                        assert_eq!(args.len(), 1);
                        // T[number]
                        assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                            });
                        });
                    });
                });
            });
        });
    }

    /// Readonly prefix with generic containing indexed access and array suffix.
    #[test]
    fn test_parse_readonly_generic_indexed_access_array() {
        // readonly Foo<T[number]>[]
        let mut test = TestParser::new_with_options(
            "type A = readonly Foo<T[number]>[]",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                // readonly Foo<T[number]>[]
                assert_node!(parser.tree, *value, Expression::TypeUnary { right, .. } => {
                    // Foo<T[number]>[]
                    assert_node!(parser.tree, *right, Expression::Index { left, index, .. } => {
                        assert!(index.is_none());
                        // Foo<T[number]>
                        assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                            assert_eq!(path.segments.len(), 1);
                            let args = static_arguments.as_ref().unwrap();
                            assert_eq!(args.len(), 1);
                            // T[number]
                            assert_node!(parser.tree, args[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::TypeIndex { left, index } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "T");
                                    assert_node!(parser.tree, *index, Expression::TypeLiteral(TypeLiteral::Number));
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    /// Complex conditional type from deno builtins with nested indexed access.
    #[test]
    fn test_parse_deno_conditional_indexed_access() {
        let mut test = TestParser::new_with_options(
            r#"type ToNativeParameterTypes<T extends readonly NativeType[]> =
    [T[number][]] extends [T] ? ToNativeType<T[number]>[]
      : [readonly T[number][]] extends [T] ? readonly ToNativeType<T[number]>[]
      : never"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { static_parameters, value, .. } => {
                let params = static_parameters.as_ref().unwrap();
                assert_eq!(params.len(), 1);
                assert_node!(parser.tree, *value, Expression::TypeConditional { .. });
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
                let predicate_id = *value;
                assert_node!(parser.tree, *value, Expression::TypePredicate { asserts, subject, target } => {
                    assert!(!asserts);
                    assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                    assert_node!(parser.tree, target.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                });

                let main_span = parser
                    .tree
                    .get_main_span(predicate_id)
                    .expect("expected predicate main span");
                assert_eq!(parser.get_span_str(main_span), "value");
            });
        });
    }

    #[test]
    fn test_parse_type_infer_span() {
        let mut test = TestParser::new("type T = infer Value");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let infer_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeInfer { name, constraint } => {
                    assert_string!(parser, *name, "Value");
                    assert!(constraint.is_none());
                });

                let main_span = parser
                    .tree
                    .get_main_span(infer_id)
                    .expect("expected infer main span");
                assert_eq!(parser.get_span_str(main_span), "Value");
            });
        });
    }

    #[test]
    fn test_parse_type_import_span() {
        let mut test = TestParser::new(r#"type T = import("foo").Bar"#);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let import_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeImport { target, arguments, .. } => {
                    assert_string!(parser, *target, "foo");
                    assert_eq!(arguments.len(), 1);
                });

                let main_span = parser
                    .tree
                    .get_main_span(import_id)
                    .expect("expected import main span");
                assert_eq!(parser.get_span_str(main_span), "\"foo\"");
            });
        });
    }

    #[test]
    fn test_parse_type_unary_prefix_operator_span() {
        let mut test = TestParser::new("type T = keyof Value");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let unary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeUnary { operator, right } => {
                    assert_eq!(*operator, TypeUnaryOperator::Keyof);
                    assert_expression_path!(parser, parser.tree.get(*right), "Value");
                });

                let main_span = parser
                    .tree
                    .get_main_span(unary_id)
                    .expect("expected type unary operator span");
                assert_eq!(parser.get_span_str(main_span), "keyof");
            });
        });
    }

    #[test]
    fn test_parse_type_unary_postfix_operator_span() {
        let mut test = TestParser::new("type T = Value as const");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let unary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeUnary { operator, right } => {
                    assert_eq!(*operator, TypeUnaryOperator::AsConst);
                    assert_expression_path!(parser, parser.tree.get(*right), "Value");
                });

                let main_span = parser
                    .tree
                    .get_main_span(unary_id)
                    .expect("expected type unary postfix operator span");
                assert_eq!(parser.get_span_str(main_span), "as const");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_operator_span() {
        let mut test = TestParser::new("type T = Value as Other");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    assert_expression_path!(parser, parser.tree.get(*left), "Value");
                    assert_expression_path!(parser, parser.tree.get(*right), "Other");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "as");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_extends_operator_span() {
        let mut test = TestParser::new("type T = Left extends Right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Extends);
                    assert_expression_path!(parser, parser.tree.get(*left), "Left");
                    assert_expression_path!(parser, parser.tree.get(*right), "Right");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "extends");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_satisfies_operator_span() {
        let mut test = TestParser::new("type T = Value satisfies Constraint");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Satisfies);
                    assert_expression_path!(parser, parser.tree.get(*left), "Value");
                    assert_expression_path!(parser, parser.tree.get(*right), "Constraint");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "satisfies");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_implements_operator_span() {
        let mut test = TestParser::new("type T = Value implements Trait");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Implements);
                    assert_expression_path!(parser, parser.tree.get(*left), "Value");
                    assert_expression_path!(parser, parser.tree.get(*right), "Trait");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "implements");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_in_operator_span() {
        let mut test = TestParser::new("type T = Key in Record");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::In);
                    assert_expression_path!(parser, parser.tree.get(*left), "Key");
                    assert_expression_path!(parser, parser.tree.get(*right), "Record");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "in");
            });
        });
    }

    #[test]
    fn test_parse_type_binary_instanceof_operator_span() {
        let mut test = TestParser::new("type T = Value instanceof Other");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                let binary_id = *value;
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::InstanceOf);
                    assert_expression_path!(parser, parser.tree.get(*left), "Value");
                    assert_expression_path!(parser, parser.tree.get(*right), "Other");
                });

                let main_span = parser
                    .tree
                    .get_main_span(binary_id)
                    .expect("expected type binary operator span");
                assert_eq!(parser.get_span_str(main_span), "instanceof");
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
    fn test_parse_type_literal_call_signature_with_parameters() {
        let mut test = TestParser::new(
            r#"type T = {
    (num: number): number
    (str: string): string
}"#,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { (num: number): number (str: string): string }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 2);
                    // (num: number): number
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "num");
                            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
                        });
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
                    });
                    // (str: string): string
                    assert_node!(parser.tree, properties[1], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "str");
                            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                        });
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

    /// Parse type literal overloads with generic call signatures.
    #[test]
    fn test_parse_type_literal_generic_call_overloads() {
        let mut test = TestParser::new_with_options(
            r#"type Tmp = {
    <N extends number>(num: N): typeof num
    <S extends string>(str: S): typeof str
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Tmp = { <N extends number>(num: N): typeof num <S extends string>(str: S): typeof str }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 2);
                    // <N extends number>(num: N): typeof num
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        let generics = signature.generics.as_ref().expect("expected generics");
                        let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                        assert_eq!(static_parameters.len(), 1);
                        assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                            assert_string!(parser, *name, "N");
                            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
                        });
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                            assert_string!(parser, *name, "num");
                            assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "N");
                            });
                        });
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeUnary { operator, right } => {
                            assert_eq!(*operator, TypeUnaryOperator::Typeof);
                            assert_expression_path!(parser, parser.tree.get(*right), "num");
                        });
                    });
                    // <S extends string>(str: S): typeof str
                    assert_node!(parser.tree, properties[1], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        let generics = signature.generics.as_ref().expect("expected generics");
                        let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                        assert_eq!(static_parameters.len(), 1);
                        assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                            assert_string!(parser, *name, "S");
                            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
                        });
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                            assert_string!(parser, *name, "str");
                            assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "S");
                            });
                        });
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeUnary { operator, right } => {
                            assert_eq!(*operator, TypeUnaryOperator::Typeof);
                            assert_expression_path!(parser, parser.tree.get(*right), "str");
                        });
                    });
                });
            });
        });
    }

    /// Parse type literal overloads with generic call signatures returning paths.
    #[test]
    fn test_parse_type_literal_generic_call_overloads_with_path_returns() {
        let mut test = TestParser::new_with_options(
            r#"type Tmp = {
    <N extends number>(num: N): MyType
    <S extends string>(str: S): MyType
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type Tmp = { <N extends number>(num: N): MyType <S extends string>(str: S): MyType }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 2);
                    assert_node!(parser.tree, properties[0], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "MyType");
                        });
                    });
                    assert_node!(parser.tree, properties[1], Property::Method { key, signature, body, .. } => {
                        assert!(key.is_none());
                        assert!(body.is_none());
                        assert_eq!(signature.mode, Some(FunctionMode::Call));
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "MyType");
                        });
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
    fn test_parse_type_literal_readonly_property_name() {
        let mut test = TestParser::new("type T = { readonly?: boolean }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { readonly?: boolean }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { modifiers, key, value, .. } => {
                        let modifiers = modifiers.as_ref().expect("expected modifiers");
                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                        assert!(modifiers.mutability.is_none());
                        let key = key.as_ref().expect("expected key");
                        match key {
                            Key::Name(Name::Identifier(name)) => {
                                assert_string!(parser, *name, "readonly");
                            }
                            _ => panic!("expected Key::Name, got {key:?}"),
                        }
                        assert_node!(parser.tree, value.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_literal_computed_key() {
        let mut test = TestParser::new("type T = { [mismatch]: string }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { [mismatch]: string }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { key, value, .. } => {
                        let key = key.as_ref().expect("expected key");
                        match key {
                            Key::Expression(key) => {
                                assert_expression_path!(parser, parser.tree.get(*key), "mismatch");
                            }
                            _ => panic!("expected Key::Expression, got {key:?}"),
                        }
                        assert_node!(parser.tree, value.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
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
                    assert_eq!(*intrinsic, IntrinsicType::Uppercase);
                });
            });
        });
    }

    /// Generic arrow function types work in TypeScript declaration files.
    #[test]
    fn test_parse_generic_arrow_function_type_in_typescript() {
        let mut test = TestParser::new_with_options(
            "type ClassDecorator = <TFunction extends Function>(target: TFunction) => TFunction | void",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type ClassDecorator = <TFunction extends Function>(target: TFunction) => TFunction | void
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "ClassDecorator");
            });
        });
    }

    /// Arrow function type with conditional return.
    #[test]
    fn test_parse_type_arrow_with_conditional_return() {
        let mut test = TestParser::new_with_options(
            "type T = <X>() => X extends A | B ? true : false",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = <X>() => X extends A | B ? true : false
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Declaration(fn_id) => {
                    assert_node!(parser.tree, *fn_id, Declaration::Function { signature, .. } => {
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeConditional { .. });
                    });
                });
            });
        });
    }

    /// Nested conditional types with arrow functions (expect-type pattern).
    #[test]
    fn test_parse_type_nested_conditional_with_arrows() {
        let input = r#"type StrictEqual<L, R> =
  (<T>() => T extends (L & T) | T ? true : false) extends <T>() => T extends (R & T) | T ? true : false
    ? IsNever<L> extends IsNever<R>
      ? true
      : false
    : false"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type StrictEqual<L, R> = (outer conditional with nested conditional in then branch)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeConditional { then_type, else_type, .. } => {
                    assert_node!(parser.tree, *then_type, Expression::TypeConditional { .. });
                    assert_node!(parser.tree, *else_type, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                });
            });
        });
    }

    /// Generic arrow function with complex constraint as property type.
    #[test]
    fn test_parse_type_property_generic_arrow_complex_constraint() {
        let input = r#"type T = {
  method: <Expected extends IsUnion<Expected> extends true ? "error" : SomeType>(arg: Expected) => true;
}"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = { method: <...>(...) => true }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { value: Some(val_id), .. } => {
                        assert_node!(parser.tree, *val_id, Expression::Declaration(fn_id) => {
                            assert_node!(parser.tree, *fn_id, Declaration::Function { signature, .. } => {
                                assert!(signature.generics.is_some());
                            });
                        });
                    });
                });
            });
        });
    }

    /// Generic arrow functions in static arguments.
    #[test]
    fn test_parse_type_generic_arrow_in_static_arguments() {
        let input = "type T = Extends<<T>() => T extends X ? true : false, <T>() => T extends Y ? true : false>";
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = Extends<arrow1, arrow2>
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { static_arguments: Some(args), .. } => {
                    assert_eq!(args.len(), 2);
                });
            });
        });
    }
}
