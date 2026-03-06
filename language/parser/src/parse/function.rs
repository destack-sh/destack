use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Asynchrony, BlockContext, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    DependencyMode, Expression, FunctionAbstraction, FunctionCardinality, FunctionKind,
    FunctionMode, FunctionSignature, Generics, Keyword, LocalNodeId, NodeType, Parameter,
    TokenType,
};
use destack_source::{NodeSpanType, Span};

/// The keywords that can appear before a function declaration.
pub static FUNCTION_MODIFIERS: [Keyword; 7] = [
    Keyword::Async,
    Keyword::Abstract,
    Keyword::Override,
    Keyword::Get,
    Keyword::Set,
    Keyword::Constructor,
    Keyword::New,
];

/// Maximum token budget for plain parenthesized lambda heads.
const PLAIN_PARENTHESIZED_LAMBDA_MAX_TOKENS: usize = 24;

/// The plain head shapes accepted by the parenthesized lambda path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParenthesizedLambdaHeadShape {
    /// No dynamic parameters: `()`.
    Empty,
    /// One named parameter: `(value)` or `(value: Type)`.
    Named {
        /// Whether the parameter has a type annotation.
        has_type_annotation: bool,
    },
}

/// Precomputed follow facts for a parenthesized lambda head.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParenthesizedLambdaHint {
    /// The matching close parenthesis token index.
    close_index: usize,
    /// The token type that follows the close parenthesis.
    follow_token_type: TokenType,
}

impl Parser {
    /// Parse a block with temporary parser options.
    #[inline]
    fn eat_block_with_options(
        &mut self,
        options: ParserOptions,
        block_context: BlockContext,
    ) -> ParseResult<LocalNodeId<destack_ast::Block>> {
        if self.options == options {
            return self.eat_block(block_context);
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }

        let old_options = self.swap_options(options);
        let result = self.eat_block(block_context);
        self.restore_options(old_options);
        result
    }

    /// Parse function parameters with temporary parser options.
    #[inline]
    fn eat_parameters_body_with_options(
        &mut self,
        options: ParserOptions,
    ) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        if self.options == options {
            return self.eat_parameters_body();
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }

        let old_options = self.swap_options(options);
        let result = self.eat_parameters_body();
        self.restore_options(old_options);
        result
    }

    /// Return true when the plain lambda path can be used.
    fn can_parse_plain_lambda(
        &self,
        descriptor: &DeclarationDescriptor,
        expect_maybe: bool,
        expect_body: bool,
    ) -> bool {
        !self.options.is_in_type()
            && !self.options.is_in_match_case()
            && !expect_maybe
            && !expect_body
            && *descriptor == DeclarationDescriptor::default()
    }

    /// Return the `new` keyword index when the current position starts a construct signature head.
    #[inline]
    fn construct_signature_new_index_maybe(&mut self) -> Option<usize> {
        let new_index = self.next_non_newline_index_from(self.pos_index());
        let has_new_keyword = self.token_type_at(new_index) == TokenType::Identifier
            && self.keyword_for_index(new_index) == Some(Keyword::New);
        if !has_new_keyword {
            return None;
        }

        let after_new_index = self.next_non_newline_index_from(new_index.saturating_add(1));
        let has_construct_signature_head = self.token_type_at(after_new_index)
            == TokenType::LessThan
            || self.token_type_at(after_new_index) == TokenType::OpenParenthesis;
        if !has_construct_signature_head {
            return None;
        }

        Some(new_index)
    }

    /// Parse a lambda body after the arrow.
    fn eat_plain_lambda_body(
        &mut self,
        body_start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.is_block_start() {
            let mut options = self
                .options
                .in_statement_position()
                .in_before_block()
                .not_in_decorator();
            options.set_allow_sequence_expression(true);
            let block_id = self.eat_block_with_options(options, BlockContext::Expression)?;
            let body = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(body_start));
            Ok(body)
        } else {
            let mut options = self.options.in_before_block().not_in_decorator();
            options.set_allow_sequence_expression(false);
            self.eat_expression(options)
        }
    }

    /// Build a plain lambda declaration from parsed parameters and body.
    fn build_plain_lambda_declaration(
        &mut self,
        start: &ParserMark,
        descriptor: &DeclarationDescriptor,
        dynamic_parameters: Vec<LocalNodeId<Parameter>>,
        return_type: Option<LocalNodeId<Expression>>,
        return_type_span: Option<Span>,
        body: LocalNodeId<Expression>,
    ) -> LocalNodeId<Declaration> {
        let (this_parameter, dynamic_parameters) =
            self.split_this_parameter_maybe(dynamic_parameters);
        let signature = FunctionSignature {
            abstraction: FunctionAbstraction::Concrete,
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            mode: None,
            kind: FunctionKind::Lambda,
            generics: None,
            this_parameter,
            dynamic_parameters,
            return_type,
        };
        let function_id = self.tree.insert(
            Declaration::Function {
                descriptor: *descriptor,
                signature,
                body: Some(body),
            },
            self.get_span_from(start),
        );
        if let Some(span) = return_type_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Type, span);
        }

        function_id
    }

    /// Classify a parenthesized lambda head for the plain path.
    fn classify_plain_parenthesized_lambda_head(
        &mut self,
        open_index: usize,
        close_index: usize,
    ) -> Option<ParenthesizedLambdaHeadShape> {
        let tokens = self.tokens();
        let head_tokens = tokens.get(open_index + 1..close_index)?;

        // common plain heads: (), (x), (x: T)
        if head_tokens.is_empty() {
            return Some(ParenthesizedLambdaHeadShape::Empty);
        }
        if head_tokens.len() == 1 && head_tokens[0].token.ty == TokenType::Identifier {
            return Some(ParenthesizedLambdaHeadShape::Named {
                has_type_annotation: false,
            });
        }
        if head_tokens.len() == 3
            && head_tokens[0].token.ty == TokenType::Identifier
            && head_tokens[1].token.ty == TokenType::Colon
            && matches!(
                head_tokens[2].token.ty,
                TokenType::Identifier | TokenType::Literal
            )
        {
            return Some(ParenthesizedLambdaHeadShape::Named {
                has_type_annotation: true,
            });
        }

        // track the plain head state
        let mut has_parameter = false;
        let mut has_type_annotation = false;
        let mut has_type_tokens = false;

        // depth counters keep top level comma checks cheap
        let mut parenthesis_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;

        for token in head_tokens {
            let token_type = token.token.ty;
            if token_type == TokenType::Newline {
                continue;
            }

            // require at most one named parameter
            if !has_parameter {
                if token_type == TokenType::Identifier {
                    has_parameter = true;
                    continue;
                }

                return None;
            }

            // optionally allow one top level type annotation marker
            if !has_type_annotation {
                if token_type == TokenType::Colon {
                    has_type_annotation = true;
                    continue;
                }

                return None;
            }

            // reject additional top level parameters and defaults
            let is_top_level = parenthesis_depth == 0
                && brace_depth == 0
                && bracket_depth == 0
                && angle_depth == 0;
            if is_top_level && matches!(token_type, TokenType::Comma | TokenType::Assign) {
                return None;
            }

            // track nested structures inside the type annotation
            match token_type {
                TokenType::OpenParenthesis => parenthesis_depth += 1,
                TokenType::CloseParenthesis => {
                    if parenthesis_depth == 0 {
                        return None;
                    }
                    parenthesis_depth -= 1;
                }
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => {
                    if brace_depth == 0 {
                        return None;
                    }
                    brace_depth -= 1;
                }
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => {
                    if bracket_depth == 0 {
                        return None;
                    }
                    bracket_depth -= 1;
                }
                TokenType::LessThan => angle_depth += 1,
                TokenType::GreaterThan => {
                    if angle_depth == 0 {
                        return None;
                    }
                    angle_depth -= 1;
                }
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                _ => {}
            }
            has_type_tokens = true;
        }

        if has_parameter {
            if has_type_annotation
                && (!has_type_tokens
                    || parenthesis_depth != 0
                    || brace_depth != 0
                    || bracket_depth != 0
                    || angle_depth != 0)
            {
                return None;
            }

            return Some(ParenthesizedLambdaHeadShape::Named {
                has_type_annotation,
            });
        }

        Some(ParenthesizedLambdaHeadShape::Empty)
    }

    /// Try to parse plain `() => body`, `(identifier) => body`, or `(identifier: Type) => body` lambdas.
    fn try_eat_plain_parenthesized_lambda(
        &mut self,
        start: &ParserMark,
        descriptor: &DeclarationDescriptor,
        hint: Option<ParenthesizedLambdaHint>,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_lambda_plain_calls += 1;
        }

        // require a precomputed matching close
        let open_index = self.pos_index();
        let (close_index, follow_token_type) = if let Some(hint) = hint {
            (hint.close_index, hint.follow_token_type)
        } else {
            let Some(close_index) = self.matching_pair_or_lex(open_index) else {
                if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                    speculation_stats.parenthesized_lambda_plain_misses += 1;
                }
                return Ok(None);
            };
            let follow_index = self.next_non_newline_index_from(close_index + 1);
            let follow_token_type = self.token_type_at(follow_index);
            (close_index, follow_token_type)
        };
        if close_index <= open_index {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_lambda_plain_misses += 1;
            }
            return Ok(None);
        }
        let token_count_inside = close_index.saturating_sub(open_index + 1);
        if token_count_inside > PLAIN_PARENTHESIZED_LAMBDA_MAX_TOKENS {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_lambda_plain_misses += 1;
            }
            return Ok(None);
        }

        // require an arrow or a return type marker after the group
        if !matches!(
            follow_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        ) {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_lambda_plain_misses += 1;
            }
            return Ok(None);
        }

        // classify the head shape
        let Some(head_shape) =
            self.classify_plain_parenthesized_lambda_head(open_index, close_index)
        else {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_lambda_plain_misses += 1;
            }
            return Ok(None);
        };

        // parse the parenthesized head
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        let mut dynamic_parameters = Vec::with_capacity(1);
        if let ParenthesizedLambdaHeadShape::Named {
            has_type_annotation,
        } = head_shape
        {
            let parameter_start = self.mark_span();
            let (parameter_name, parameter_name_span) = self.eat_binding_identifier_with_span()?;
            let (parameter_type, parameter_type_span) = if has_type_annotation {
                let type_start = self.mark_span();
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::Colon)?;
                self.eat_newlines_maybe()?;
                let mut type_options = self
                    .options
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type();
                if self.options.is_in_type_conditional_right() {
                    type_options = type_options.in_type_conditional_right();
                }
                let parameter_type = self
                    .eat_expression(type_options)
                    .for_node_type(NodeType::Parameter)?;
                let parameter_type_span = self.get_span_from(&type_start);
                (Some(parameter_type), Some(parameter_type_span))
            } else {
                (None, None)
            };
            self.eat_newlines_maybe()?;

            let parameter_id = self.tree.insert(
                Parameter::Named {
                    modifiers: None,
                    name: parameter_name,
                    ty: parameter_type,
                    default: None,
                },
                self.get_span_from(&parameter_start),
            );
            self.tree.set_main_span(parameter_id, parameter_name_span);
            if let Some(span) = parameter_type_span {
                self.tree
                    .set_side_span(parameter_id, NodeSpanType::Type, span);
            }
            dynamic_parameters.push(parameter_id);
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        // parse an explicit lambda return type when present
        let (return_type, return_type_span) = if self.has_lambda_return_type_marker() {
            let type_start = self.mark_span();
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Colon)?;
            self.eat_newlines_maybe()?;

            let mut return_type_options = self.options.nested().in_type();
            if self.options.is_in_type_conditional_right() {
                return_type_options = return_type_options.in_type_conditional_right();
            }
            if self.options.is_in_static() {
                return_type_options = return_type_options.in_static();
            }
            return_type_options = return_type_options.in_arrow_return_type();
            let return_type = self.eat_expression(return_type_options)?;
            let return_type_span = self.get_span_from(&type_start);

            (Some(return_type), Some(return_type_span))
        } else {
            (None, None)
        };

        // parse the body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let body = self.eat_plain_lambda_body(&body_start)?;

        let function_id = self.build_plain_lambda_declaration(
            start,
            descriptor,
            dynamic_parameters,
            return_type,
            return_type_span,
            body,
        );

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_lambda_plain_hits += 1;
        }

        Ok(Some(function_id))
    }

    /// Try to parse a parenthesized lambda value without entering full function parsing.
    fn try_eat_parenthesized_lambda_value(
        &mut self,
        start: &ParserMark,
        descriptor: &DeclarationDescriptor,
        hint: Option<ParenthesizedLambdaHint>,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        // require an arrow or return type marker after the parenthesized head
        let open_index = self.pos_index();
        let follow_token_type = if let Some(hint) = hint {
            if hint.close_index <= open_index {
                return Ok(None);
            }
            hint.follow_token_type
        } else {
            let Some(close_index) = self.matching_pair_or_lex(open_index) else {
                return Ok(None);
            };
            if close_index <= open_index {
                return Ok(None);
            }
            let follow_index = self.next_non_newline_index_from(close_index + 1);
            self.token_type_at(follow_index)
        };
        if !matches!(
            follow_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        ) {
            return Ok(None);
        }

        // dynamic parameters
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        let parameter_options = self.options.with_generator(false).with_forbid_yield(false);
        let dynamic_parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            self.eat_parameters_body_with_options(parameter_options)?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // explicit lambda return type
        let (return_type, return_type_span) = if self.has_lambda_return_type_marker() {
            let type_start = self.mark_span();
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Colon)?;
            self.eat_newlines_maybe()?;

            let mut return_type_options = self.options.nested().in_type();
            if self.options.is_in_type_conditional_right() {
                return_type_options = return_type_options.in_type_conditional_right();
            }
            if self.options.is_in_static() {
                return_type_options = return_type_options.in_static();
            }
            return_type_options = return_type_options.in_arrow_return_type();
            let return_type = self.eat_expression(return_type_options)?;
            let return_type_span = self.get_span_from(&type_start);

            (Some(return_type), Some(return_type_span))
        } else {
            (None, None)
        };

        // body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let body = self.eat_plain_lambda_body(&body_start)?;

        let function_id = self.build_plain_lambda_declaration(
            start,
            descriptor,
            dynamic_parameters,
            return_type,
            return_type_span,
            body,
        );

        Ok(Some(function_id))
    }

    /// Try to parse a plain `identifier => body` lambda with minimal branching.
    fn try_eat_plain_identifier_lambda(
        &mut self,
        start: &ParserMark,
        descriptor: &DeclarationDescriptor,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.identifier_lambda_plain_calls += 1;
        }

        // parse the single named parameter
        let parameter_name = self.eat_identifier()?;
        let parameter_id = self.tree.insert(
            Parameter::Named {
                modifiers: None,
                name: parameter_name,
                ty: None,
                default: None,
            },
            self.get_span_from(start),
        );

        // parse the lambda body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let body = self.eat_plain_lambda_body(&body_start)?;

        // build the declaration
        let function_id = self.build_plain_lambda_declaration(
            start,
            descriptor,
            vec![parameter_id],
            None,
            None,
            body,
        );

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.identifier_lambda_plain_hits += 1;
        }

        Ok(Some(function_id))
    }

    /// Eat a function or "lambda" declaration or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    ///
    /// Examples:
    /// ```
    /// // lambda style (type context)
    /// (a: int32) => int32
    /// (int32) => (boolean, int32)
    /// (x): int32 => x
    ///
    /// // lambda style (value context)
    /// (a) => a > 2
    /// (a): int32 => a > 2
    /// (a: int32) => {
    ///    print("Hello, world!")
    /// }
    ///
    /// // function style
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    ///
    /// // optional , if newline-delimited
    /// function longBar<Validate: boolean>(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   // regular comment
    ///   c: Vector2
    /// ) => (
    ///    int32,
    ///    isGood: boolean
    /// ) with (
    ///   Time
    /// ) {
    ///    ...
    /// }
    /// ```
    pub fn eat_function(
        &mut self,
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_function_inner(start, descriptor, expect_maybe, expect_body, None)
    }

    /// Eat a function with a precomputed parenthesized head follow hint.
    pub(crate) fn eat_function_with_parenthesized_head_hint(
        &mut self,
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
        expect_maybe: bool,
        expect_body: bool,
        close_index: usize,
        follow_token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let hint = Some(ParenthesizedLambdaHint {
            close_index,
            follow_token_type,
        });
        self.eat_function_inner(start, descriptor, expect_maybe, expect_body, hint)
    }

    /// Eat a function using optional parenthesized lambda head hints.
    fn eat_function_inner(
        &mut self,
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
        expect_maybe: bool,
        expect_body: bool,
        plain_parenthesized_hint: Option<ParenthesizedLambdaHint>,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_FUNCTION);
        let can_parse_plain_lambda =
            self.can_parse_plain_lambda(&descriptor, expect_maybe, expect_body);

        // parse plain lambda heads only when the token shape matches
        if can_parse_plain_lambda && self.peek_is(TokenType::OpenParenthesis) {
            if let Some(function_id) = self.try_eat_plain_parenthesized_lambda(
                start,
                &descriptor,
                plain_parenthesized_hint,
            )? {
                return Ok(function_id);
            }

            if let Some(function_id) = self.try_eat_parenthesized_lambda_value(
                start,
                &descriptor,
                plain_parenthesized_hint,
            )? {
                return Ok(function_id);
            }
        } else if can_parse_plain_lambda
            && self.peek_is(TokenType::Identifier)
            && matches!(
                self.peek_next_token_type(),
                TokenType::Arrow | TokenType::ArrowWide
            )
            && let Some(function_id) = self.try_eat_plain_identifier_lambda(start, &descriptor)?
        {
            return Ok(function_id);
        }

        // abstraction
        if self.is_keyword(Keyword::Abstract)
            && descriptor.abstraction == DeclarationAbstraction::Concrete
        {
            self.bump(); // eat abstract keyword
            descriptor.abstraction = DeclarationAbstraction::Abstract;
        }

        // async
        let is_async = if self.is_keyword(Keyword::Async) {
            let next_token_type = self.peek_next_token_type();
            let treats_async_as_parameter =
                matches!(next_token_type, TokenType::Arrow | TokenType::ArrowWide);
            if treats_async_as_parameter {
                false
            } else {
                self.bump(); // eat async keyword
                true
            }
        } else {
            false
        };

        // new
        let mode = if let Some(new_index) = self.construct_signature_new_index_maybe() {
            if new_index != self.pos_index() {
                self.advance_to(new_index);
            }
            self.bump(); // eat new keyword
            Some(FunctionMode::New)
        } else {
            None
        };

        // function style
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;
        let (kind, is_generator) = {
            // regular `function` style
            if self.is_keyword(Keyword::Function) {
                self.bump(); // eat function keyword
                let is_generator = is_generator || self.eat_token_maybe(TokenType::Multiply)?;
                (FunctionKind::Function, is_generator)
            }
            // lambda style
            else {
                (FunctionKind::Lambda, is_generator)
            }
        };

        // function style, name, static parameters
        let (name, name_span, static_parameters) = {
            if kind == FunctionKind::Function {
                // name
                let (name, name_span) = if let Some((n, s)) = self.eat_name_maybe_with_span()? {
                    (Some(n), Some(s))
                } else {
                    (None, None)
                };

                // maybe keyword after name (maybe)
                if expect_maybe {
                    self.eat_token(TokenType::Maybe)?;
                }

                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe(false)
                    .for_node_type(NodeType::Declaration)?;

                (name, name_span, static_parameters)
            } else {
                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe(false)
                    .for_node_type(NodeType::Declaration)?;

                (None, None, static_parameters)
            }
        };
        descriptor = descriptor.with_name_maybe(name);

        // declarations in statement position require a name unless default-exported
        if kind == FunctionKind::Function
            && self.options.is_in_statement_position()
            && descriptor.name.is_none()
            && descriptor.export != Some(DependencyMode::Default)
        {
            return Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ));
        }

        // dynamic parameters
        let dynamic_parameters = {
            // regular `(...) => ...` function/lambda
            let has_parenthesized_parameters = kind == FunctionKind::Function
                || self.options.is_in_type()
                || self.peek_is(TokenType::OpenParenthesis)
                || self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis);
            if has_parenthesized_parameters {
                // allow line breaks before the parameter list
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::OpenParenthesis)?;
                self.eat_newlines_maybe()?;

                // dynamic parameters
                let dynamic_parameters = if self.peek_is(TokenType::CloseParenthesis) {
                    vec![]
                } else {
                    let parameter_options = self
                        .options
                        .with_generator(is_generator)
                        .with_forbid_yield(is_generator);
                    self.eat_parameters_body_with_options(parameter_options)?
                };
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseParenthesis)?;

                dynamic_parameters
            }
            // plain no-parentheses `x => y` lambda value
            else {
                if is_generator && self.is_keyword(Keyword::Yield) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                let parameter_name = self.eat_identifier()?;
                let parameter_id = self.tree.insert(
                    Parameter::Named {
                        modifiers: None,
                        name: parameter_name,
                        ty: None,
                        default: None,
                    },
                    self.get_span_from(start),
                );

                vec![parameter_id]
            }
        };

        // return type info (including where)
        // only for functions or lambda types
        let (return_type, return_type_span, where_clauses) = {
            // lambda with explicit return type
            if kind == FunctionKind::Lambda && self.has_lambda_return_type_marker() {
                let type_start = self.mark_span();
                self.eat_newlines_maybe()?;
                self.bump(); // eat colon or arrow
                self.eat_newlines_maybe()?;

                // return type
                let mut return_type_options = self.options.nested().in_type();
                if self.options.is_in_type_conditional_right() {
                    return_type_options = return_type_options.in_type_conditional_right();
                }
                if self.options.is_in_static() {
                    return_type_options = return_type_options.in_static();
                }
                if !self.options.is_in_type() {
                    return_type_options = return_type_options.in_arrow_return_type();
                }
                let return_type = self.eat_expression(return_type_options)?;
                let return_type_span = self.get_span_from(&type_start);

                // where clauses are a destack only feature
                let where_clauses = if self.language.is_destack() {
                    self.eat_where_maybe()?
                } else {
                    None
                };

                (Some(return_type), Some(return_type_span), where_clauses)
            }
            // regular function with return type or lambda type
            else if kind == FunctionKind::Function || self.options.is_in_type() {
                // return type
                let has_return_type_marker = self.peek_arrow_is()
                    || self.peek_colon_is()
                    || self.peek_is(TokenType::Newline)
                        && (self.is_token_after_newlines(self.pos(), TokenType::Arrow)
                            || self.is_token_after_newlines(self.pos(), TokenType::Colon));
                let (return_type, return_type_span) = if has_return_type_marker {
                    let type_start = self.mark_span();
                    self.eat_newlines_maybe()?;
                    self.bump(); // eat arrow or colon
                    self.eat_newlines_maybe()?;

                    // return type
                    let mut return_type_options = self.options.nested().in_type().in_before_block();
                    if self.options.is_in_type_conditional_right() {
                        return_type_options = return_type_options.in_type_conditional_right();
                    }
                    if self.options.is_in_static() {
                        return_type_options = return_type_options.in_static();
                    }
                    let return_type = self.eat_expression(return_type_options)?;
                    (Some(return_type), Some(self.get_span_from(&type_start)))
                } else {
                    (None, None)
                };

                // where clauses are a destack only feature
                let where_clauses = if self.language.is_destack() {
                    self.eat_where_maybe()?
                } else {
                    None
                };

                (return_type, return_type_span, where_clauses)
            }
            // nothing
            else {
                (None, None, None)
            }
        };

        // body
        // only for functions or lambda values
        let body = {
            // function bodies may start on the next line in js and ts
            if kind == FunctionKind::Function && !self.options.is_in_type() {
                self.eat_newlines_maybe()?;
            }

            // expect body but no opening brace
            if expect_body && !self.peek_is(TokenType::OpenBrace) {
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::OpenBrace,
                ));
            }

            // function with body
            if kind == FunctionKind::Function && self.peek_is(TokenType::OpenBrace) {
                let mut options = self
                    .options
                    .in_statement_position()
                    .in_before_block()
                    .not_in_decorator()
                    .with_generator(is_generator);
                options.set_allow_sequence_expression(true);
                options.set_forbid_await(options.is_forbid_await() && !is_async);
                let body_start = self.mark_span();
                let block_id = self.eat_block_with_options(options, BlockContext::Expression)?;
                let body = self
                    .tree
                    .insert(Expression::Block(block_id), self.get_span_from(&body_start));
                Some(body)
            }
            // lambda with body
            else if kind == FunctionKind::Lambda
                && !self.options.is_in_type()
                && self.peek_arrow_is()
            {
                self.eat_arrow()?;
                self.eat_newlines_maybe()?;
                let body_start = self.mark_span();
                let body = if self.is_block_start() {
                    let mut options = self
                        .options
                        .in_statement_position()
                        .in_before_block()
                        .not_in_decorator()
                        .with_generator(is_generator);
                    // block bodies are delimited, so sequence expressions stay local
                    options.set_allow_sequence_expression(true);
                    options.set_forbid_await(options.is_forbid_await() && !is_async);
                    let block_id =
                        self.eat_block_with_options(options, BlockContext::Expression)?;
                    self.tree
                        .insert(Expression::Block(block_id), self.get_span_from(&body_start))
                } else {
                    let mut options = self
                        .options
                        .in_before_block()
                        .not_in_decorator()
                        .with_generator(is_generator);
                    // avoid swallowing commas from surrounding contexts
                    options.set_allow_sequence_expression(false);
                    options.set_forbid_await(options.is_forbid_await() && !is_async);
                    self.eat_expression(options)?
                };
                Some(body)
            }
            // no body
            else {
                None
            }
        };

        // split out explicit this parameter
        let (this_parameter, dynamic_parameters) =
            self.split_this_parameter_maybe(dynamic_parameters);

        // function
        let generics = Generics::new(static_parameters, where_clauses).into_option();
        let asynchrony = if is_async {
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };
        let cardinality = if is_generator {
            FunctionCardinality::Generator
        } else {
            FunctionCardinality::Scalar
        };
        let abstraction = match descriptor.abstraction {
            DeclarationAbstraction::Abstract => FunctionAbstraction::Abstract,
            DeclarationAbstraction::Concrete => FunctionAbstraction::Concrete,
        };
        let signature = FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            kind,
            mode,
            generics,
            this_parameter,
            dynamic_parameters,
            return_type,
        };
        let function_id = self.tree.insert(
            Declaration::Function {
                descriptor,
                signature,
                body,
            },
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(function_id, span);
        }

        // set type span for return type annotation
        if let Some(span) = return_type_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Type, span);
        }

        Ok(function_id)
    }

    /// Check whether a lambda return type marker is present.
    fn has_lambda_return_type_marker(&mut self) -> bool {
        // check for a colon return type
        let has_colon =
            self.peek_colon_is() || self.is_token_after_newlines(self.pos(), TokenType::Colon);
        if has_colon {
            return true;
        }

        // arrow return types only apply in type positions
        if !self.options.is_in_type() {
            return false;
        }

        self.peek_arrow_is()
            || self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Arrow)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Asynchrony, BinaryOperator, CommentStyle, Declaration, DeclarationDescriptor,
        Expression, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, IntType,
        Parameter, ScalarLiteral, TypeLiteral, VarianceModifier, WhereClause, YieldCardinality,
    };

    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_function_lambda_with_newlines() {
        let mut test = TestParser::new(
            r#"(x: number):
    number =>
    x"#,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        // (x: number): number => x
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, body: Some(body), .. } => {
            assert_eq!(descriptor.name, None);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x: number
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
            // number
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_plain_parenthesized_lambda_with_newlines() {
        let mut test = TestParser::new("(\nvalue\n) => value");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, default, .. } => {
                assert_string!(parser, *name, "value");
                assert!(ty.is_none());
                assert!(default.is_none());
            });
            assert!(signature.return_type.is_none());
            assert_node!(parser.tree, body.expect("expected body"), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "value");
            });
        });
    }

    #[test]
    fn test_parse_plain_parenthesized_typed_lambda() {
        let mut test = TestParser::new("(value: number) => value");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, default, .. } => {
                assert_string!(parser, *name, "value");
                assert!(default.is_none());
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
            assert_node!(parser.tree, body.expect("expected body"), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "value");
            });
        });
    }

    /// Parse a function type with an explicit this parameter.
    #[test]
    fn test_parse_function_type_with_this_parameter() {
        let mut test = TestParser::new("type T = (this: Foo, value: Bar) => Baz");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();

        // type T = (this: Foo, value: Bar) => Baz
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert!(signature.this_parameter.is_some());
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        // value: Bar
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "value");
                            assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "Bar");
                        });
                    });
                });
            });
        });
    }

    /// Parse an arrow function with an explicit this parameter.
    #[test]
    fn test_parse_arrow_function_with_this_parameter() {
        let mut test =
            TestParser::new_with_options("(this: string) => {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();

        // (this: string) => {}
        assert_node!(parser.tree, function_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.this_parameter.is_some());
            assert!(signature.dynamic_parameters.is_empty());
            assert!(body.is_some());
        });
    }

    #[test]
    fn test_parse_function_lambda_with_explicit_return_type() {
        let mut test = TestParser::new("(x): int32 => x");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        // (x): int32 => x
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, body: Some(body), .. } => {
            assert_eq!(descriptor.name, None);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // int32
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    /// Parse parenthesized void return types in arrow functions.
    #[test]
    fn test_parse_function_parenthesized_void_return_type() {
        let mut test = TestParser::new_with_options("(): (void) => {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(body.is_some());
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    /// Parse default parameters followed by required parameters.
    #[test]
    fn test_parse_function_default_parameter_followed_by_required() {
        let mut test = TestParser::new_with_options(
            r#"function func(greeting: string = "Hello", target: string) {}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();

        // function func(greeting: string = "Hello", target: string) {}
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.dynamic_parameters.len(), 2);
                // greeting: string = "Hello"
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, default, .. } => {
                    assert_string!(parser, *name, "greeting");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    assert_node!(parser.tree, default.unwrap(), Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                        assert_string!(parser, *value, "Hello");
                    });
                });
                // target: string
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { name, ty, default, .. } => {
                    assert_string!(parser, *name, "target");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    assert!(default.is_none());
                });
            });
        });
    }

    #[test]
    fn test_parse_function_new_type() {
        let mut test = TestParser::new("new(): $");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        // new (x) => int32
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert!(descriptor.name.is_none());
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.dynamic_parameters.is_empty());
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "$");
        });
    }

    #[test]
    fn test_parse_function_new_type_with_static_arguments() {
        let mut test = TestParser::new("new <T>(x: int32) => T");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        // new <T>(x: int32) => T
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert!(descriptor.name.is_none());
            // new
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            // <T>
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert!(ty.is_none());
            });
            // x: int32
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
            // T
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
        });
    }

    #[test]
    fn test_parse_function_with_where_clause() {
        let mut test = TestParser::new(
            r###"
function foo() => int32 where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // function name
            assert_string!(parser, descriptor.name.unwrap().string(), "foo");
            let generics = signature.generics.as_ref().expect("expected generics");
            // where Guard: Limit
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
            // return type
            let ret = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    }

    #[test]
    fn test_parse_function_with_static_and_dynamic_parameters() {
        let mut test = TestParser::new(
            r"
function compute<Validate: boolean, Precision: uint8>(data: uint8[]) {
    body
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // compute
            assert_string!(parser, descriptor.name.unwrap().string(), "compute");
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            // <Validate: bool, Precision: uint8>
            assert_eq!(static_parameters.len(), 2);

            // Validate: bool
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Validate");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            });

            // Precision: uint8
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Precision");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(8), is_signed: false })));
            });
            // data: uint8[]
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "data");
            });
        });
    }

    #[test]
    fn test_parse_function_abstract_new_type_with_newline() {
        let mut test = TestParser::new("abstract\nnew (): T");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();

        // abstract\nnew (): T
        assert_node!(parser.tree, function_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.abstraction, FunctionAbstraction::Abstract);
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.dynamic_parameters.is_empty());
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
        });
    }

    #[test]
    fn test_parse_function_with_newline_between_generics_and_parameters() {
        let mut test = TestParser::new(
            r"
function h<T>
    (tag: T): T;
            ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, body } => {
            // function name
            assert_string!(parser, descriptor.name.unwrap().string(), "h");

            // static parameter T
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert!(ty.is_none());
            });

            // dynamic parameter tag: T
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "tag");
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, static_arguments: None } => {
                    assert_path!(parser, *path, "T");
                });
            });

            // return type T
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, static_arguments: None } => {
                assert_path!(parser, *path, "T");
            });

            // declaration signature has no body
            assert!(body.is_none());
        });
    }

    #[test]
    fn test_parse_function_with_newline_before_return_type_colon_typescript() {
        let mut test = TestParser::new_with_options(
            r#"function f<T>(value: T)
  : T {
  return value as never
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();

        assert_node!(parser.tree, function_id, Declaration::Function { signature, body, .. } => {
            let return_type = signature.return_type.expect("expected return type");
            assert_expression_path!(parser, parser.tree.get(return_type), "T");

            let body_id = body.expect("expected function body");
            assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.expressions.len(), 1);
                assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                    assert_node!(parser.tree, *statement_id, Expression::Return { value } => {
                        let value = value.expect("expected return value");
                        assert_node!(parser.tree, value, Expression::TypeBinary { .. });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_function_with_variance_parameters() {
        let mut test = TestParser::new(
            r"
function transform<in T, out U>(value: T): U {
    value as U
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "transform");
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 2);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, modifiers: Some(modifiers), .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(modifiers.variance, Some(VarianceModifier::In));
            });
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, modifiers: Some(modifiers), .. } => {
                assert_string!(parser, *name, "U");
                assert_eq!(modifiers.variance, Some(VarianceModifier::Out));
            });
        });
    }

    #[test]
    fn test_parse_function_with_invariant_parameter() {
        let mut test = TestParser::new(
            r"
function invariant<in out T>(value: T): T {
    value
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "invariant");
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, modifiers: Some(modifiers), .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(modifiers.variance, Some(VarianceModifier::InOut));
            });
        });
    }

    #[test]
    fn test_parse_function_with_function_return_type() {
        let mut test = TestParser::new("function foo() => (str: string) => boolean {}");
        let mut parser = test.prepare();

        // function foo() => (str: string) => boolean
        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // foo
            assert_string!(parser, descriptor.name.unwrap().string() , "foo");

            // (str: string) => boolean
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    // str: string
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    });

                    // boolean
                    assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
                });
            });
        });
    }

    #[test]
    fn test_parse_function_return_type_with_static_arguments() {
        let mut test = TestParser::new("function read<T, E>() => AliasBranch<T, E> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();

        assert_node!(parser.tree, function_id, Declaration::Function { signature, .. } => {
            let return_type = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, return_type, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "AliasBranch");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 2);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });
                assert_node!(parser.tree, static_arguments[1], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "E");
                });
            });
        });
    }

    #[test]
    fn test_parse_function_with_async_generator() {
        let mut test = TestParser::new(
            r#"
async function* foo() => int32 {
    yield 1
    yield 2
    yield 3
}"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        // async function* foo() => int32 { body }
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // foo
            assert_string!(parser, descriptor.name.unwrap().string(), "foo");
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.cardinality, FunctionCardinality::Generator);
        });
    }

    /// Parse a generator function with a bare yield call argument.
    #[test]
    fn test_parse_function_generator_call_argument_with_bare_yield_javascript() {
        // source: function* a() { b.c(yield); }
        let mut test =
            TestParser::new_with_options("function* a() { b.c(yield); }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function* a() { b.c(yield); }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { b.c(yield); }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    // b.c(yield);
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_eq!(dynamic_arguments.len(), 1);
                            // b.c
                            assert_expression_path!(parser, parser.tree.get(*left), "b.c");
                            // yield
                            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::Yield { cardinality, value } => {
                                    assert_eq!(*cardinality, YieldCardinality::Scalar);
                                    assert!(value.is_none());
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parse nested generator yield expressions.
    #[test]
    fn test_parse_function_generator_nested_yield_javascript() {
        // source: function *a() { yield yield }
        let mut test =
            TestParser::new_with_options("function *a() { yield yield }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield yield }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield yield }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    // yield yield
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Scalar);
                            assert!(value.is_some());
                            // yield
                            assert_node!(parser.tree, value.unwrap(), Expression::Yield { cardinality, value } => {
                                assert_eq!(*cardinality, YieldCardinality::Scalar);
                                assert!(value.is_none());
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parse delegated generator yield with a direct identifier operand.
    #[test]
    fn test_parse_function_generator_delegate_yield_javascript() {
        // source: function *a() { yield *a }
        let mut test =
            TestParser::new_with_options("function *a() { yield *a }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield *a }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield *a }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    // yield *a
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Generator);
                            assert!(value.is_some());
                            assert_expression_path!(parser, parser.tree.get(value.unwrap()), "a");
                        });
                    });
                });
            });
        });
    }

    /// Parse delegated generator yield with a nested bare yield operand.
    #[test]
    fn test_parse_function_generator_delegate_nested_yield_javascript() {
        // source: function *a() { yield *yield }
        let mut test = TestParser::new_with_options(
            "function *a() { yield *yield }",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield *yield }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield *yield }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    // yield *yield
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Generator);
                            assert!(value.is_some());
                            // yield
                            assert_node!(parser.tree, value.unwrap(), Expression::Yield { cardinality, value } => {
                                assert_eq!(*cardinality, YieldCardinality::Scalar);
                                assert!(value.is_none());
                            });
                        });
                    });
                });
            });
        });
    }

    /// Reject delegated generator yield when a line terminator appears before `*`.
    #[test]
    fn test_reject_function_generator_delegate_after_newline_javascript() {
        // source: function *a(){yield
        // *a}
        let mut test =
            TestParser::new_with_options("function *a(){yield\n*a}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression(parser.options).unwrap_err();

        // *
        assert_eq!(parser.get_span_str(error.leaf_span()), "*");
    }

    /// Parse generator yield in class heritage expression.
    #[test]
    fn test_parse_function_generator_yield_in_class_heritage_javascript() {
        // source: function* a(){(class extends (yield) {});}
        let mut test = TestParser::new_with_options(
            "function* a(){(class extends (yield) {});}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function* a(){(class extends (yield) {});}
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { (class extends (yield) {}); }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Parenthesized { .. });
                    });
                });
            });
        });
    }

    /// Parse generator yield in computed property keys and assignment targets.
    #[test]
    fn test_parse_function_generator_yield_in_computed_keys_javascript() {
        // source: function* a(){(class {[yield](){}})};
        let mut test = TestParser::new_with_options(
            "function* a(){(class {[yield](){}})};",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Parenthesized { .. });
                    });
                });
            });
        });

        // source: function* a(){({[yield]:a}=1)}
        let mut test = TestParser::new_with_options(
            "function* a(){({[yield]:a}=1)}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                        assert_node!(parser.tree, *statement_id, Expression::Parenthesized { .. });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_function_with_nested_lambda_type() {
        let mut test = TestParser::new(
            r#"
function onResolve(
    callback: (args) => {
        path: string;
        namespace?: string;
    } | void,
) => void;
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "onResolve");
            // callback: (args) => { .. } | void
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "callback");
                assert_node!(parser.tree, ty.unwrap(), Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        // args
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                            assert_string!(parser, *name, "args");
                        });
                        // { .. } | void
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Binary { operator, left, right } => {
                            // { .. }
                            assert_node!(parser.tree, *left, Expression::ObjectExpression { ty: None, properties } => {
                                assert_eq!(properties.len(), 2);
                            });
                            // |
                            assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                            // void
                            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Void));
                        });
                    });
                });
            });
            // void
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
        });
    }

    #[test]
    fn test_parse_lambda_return_type_with_optional_parameter_function_type() {
        let mut test = TestParser::new_with_options(
            "(runtime, effect, options: Runtime.RunCallbackOptions<any, any> = {}): (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<any, any> | undefined) => void => 0",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_eq!(signature.dynamic_parameters.len(), 3);

                // (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<any, any> | undefined) => void
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Declaration(return_declaration_id) => {
                    assert_node!(parser.tree, *return_declaration_id, Declaration::Function { signature, body: None, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        assert_eq!(signature.dynamic_parameters.len(), 2);

                        // fiberId?: FiberId.FiberId
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "fiberId");
                            assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "FiberId.FiberId");
                        });

                        // options?: Runtime.RunCallbackOptions<any, any> | undefined
                        assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { name, ty, .. } => {
                            assert_string!(parser, *name, "options");
                            assert_node!(parser.tree, ty.unwrap(), Expression::Binary { operator, .. } => {
                                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                            });
                        });

                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                    });
                });

                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    }

    #[test]
    fn test_parse_lambda_head_boundary_comment_on_function_owner() {
        let mut test =
            TestParser::new_with_options("(x) /* lambda-head */ => x", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        parser.attach_trivia();
        assert_node!(parser.tree, function_id, Declaration::Function { .. } => {
            let annotations = parser.tree.get_annotations(function_id.id);
            assert!(annotations.is_empty());
        });
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Star, " lambda-head");
    }

    #[test]
    fn test_parse_lambda_body_boundary_comment_on_body_owner() {
        let mut test =
            TestParser::new_with_options("(x) =>\n// lambda-body\nx", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        parser.attach_trivia();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { body: Some(body_id), .. } => {
                assert_expression_path!(parser, parser.tree.get(*body_id), "x");

                let annotations = parser.tree.get_annotations(body_id.id);
                assert!(annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "lambda-body");
    }

    /// Reject direct calls on unparenthesized arrow functions.
    #[test]
    fn test_reject_unparenthesized_arrow_call() {
        // source: () => {}()
        let mut test = TestParser::new_with_options("() => {}()", LanguageType::Destack);
        let mut parser = test.prepare();
        let error = parser.eat_expression(parser.options).unwrap_err();

        // (
        assert_eq!(parser.get_span_str(error.leaf_span()), "(");

        // source: a => {}()
        let mut test = TestParser::new_with_options("a => {}()", LanguageType::Destack);
        let mut parser = test.prepare();
        let error = parser.eat_expression(parser.options).unwrap_err();

        // (
        assert_eq!(parser.get_span_str(error.leaf_span()), "(");
    }

    /// Parse direct calls on parenthesized arrow functions.
    #[test]
    fn test_parse_parenthesized_arrow_call() {
        // source: (() => {})()
        let mut test = TestParser::new_with_options("(() => {})()", LanguageType::Destack);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // (() => {})()
        assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                    });
                });
            });
        });
    }
}
