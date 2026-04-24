use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark, is_semantic};

use destack_ast::{
    Asynchrony, BlockContext, ConstructorTypeDeclaration, Declaration, ExportMode, Expression,
    FunctionCardinality, FunctionDeclaration, FunctionKind, FunctionMode, FunctionSignature,
    FunctionTypeDeclaration, Keyword, LocalNodeId, Name, NodeType, Parameter, TokenType,
    TypeExpression,
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

/// Parsed function signature syntax.
struct ParsedFunctionSignature {
    /// The declaration header.
    header: DeclarationHeader,
    /// The optional function name.
    name: Option<Name>,
    /// The optional function name span.
    name_span: Option<Span>,
    /// The function signature.
    signature: FunctionSignature,
    /// The optional function body.
    body: Option<LocalNodeId<Expression>>,
    /// The generic parameter container span.
    generic_parameter_span: Option<Span>,
    /// The parameter container span.
    parameter_span: Option<Span>,
    /// The return type span.
    return_type_span: Option<Span>,
    /// The body container span.
    body_span: Option<Span>,
}

impl Parser {
    /// Insert parsed function syntax as a function declaration.
    fn insert_function_declaration(
        &mut self,
        start: &ParserMark,
        function: ParsedFunctionSignature,
    ) -> LocalNodeId<Declaration> {
        let function_id = self.insert_node(
            Declaration::Function(FunctionDeclaration {
                name: function.name,
                export: function.header.export,
                ambient: function.header.ambient,
                signature: function.signature,
                body: function.body,
            }),
            self.get_span_from(start),
        );

        // name
        if let Some(span) = function.name_span {
            self.tree.set_main_span(function_id, span);
        }

        // return type
        if let Some(span) = function.return_type_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Type, span);
        }

        // generic parameters
        if let Some(span) = function.generic_parameter_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::GenericParameters, span);
        }

        // parameters
        if let Some(span) = function.parameter_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Parameters, span);
        }

        // body
        if let Some(span) = function.body_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Body, span);
        }

        function_id
    }

    /// Insert parsed function syntax as a type expression.
    fn insert_function_type_expression(
        &mut self,
        start: &ParserMark,
        function: ParsedFunctionSignature,
    ) -> LocalNodeId<TypeExpression> {
        debug_assert_eq!(function.signature.kind, FunctionKind::Lambda);
        debug_assert!(function.body.is_none());

        // node
        let type_expression = match function.signature.mode {
            Some(FunctionMode::New) => {
                TypeExpression::ConstructorTypeDeclaration(ConstructorTypeDeclaration {
                    is_abstract: function.signature.is_abstract,
                    generic_parameters: function.signature.generic_parameters,
                    where_clauses: function.signature.where_clauses,
                    parameters: function.signature.parameters,
                    return_type: function.signature.return_type,
                })
            }
            None => TypeExpression::FunctionTypeDeclaration(FunctionTypeDeclaration {
                generic_parameters: function.signature.generic_parameters,
                where_clauses: function.signature.where_clauses,
                this_parameter: function.signature.this_parameter,
                parameters: function.signature.parameters,
                return_type: function.signature.return_type,
            }),
            _ => unreachable!("expected function or constructor type mode"),
        };
        let type_expression_id = self.insert_node(type_expression, self.get_span_from(start));

        // return type
        if let Some(span) = function.return_type_span {
            self.tree
                .set_side_span(type_expression_id, NodeSpanType::Type, span);
        }

        // generic parameters
        if let Some(span) = function.generic_parameter_span {
            self.tree
                .set_side_span(type_expression_id, NodeSpanType::GenericParameters, span);
        }

        // parameters
        if let Some(span) = function.parameter_span {
            self.tree
                .set_side_span(type_expression_id, NodeSpanType::Parameters, span);
        }

        type_expression_id
    }

    /// Return true when the plain lambda path can be used.
    fn can_parse_plain_lambda(
        &self,
        header: &DeclarationHeader,
        expect_maybe: bool,
        expect_body: bool,
    ) -> bool {
        !self.options.is_in_type()
            && !self.options.is_in_match_case()
            && !expect_maybe
            && !expect_body
            && *header == DeclarationHeader::default()
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
    ) -> ParseResult<(LocalNodeId<Expression>, Span)> {
        if self.is_block_start() {
            let mut options = self
                .options
                .in_statement_position()
                .in_before_block()
                .not_in_decorator();
            options.set_allow_sequence_expression(true);
            let block_id =
                self.with_options(options, |parser| parser.eat_block(BlockContext::Expression))?;
            let body_span = self.get_span_from(body_start);
            let body = self.tree.insert(Expression::Block(block_id), body_span);

            Ok((body, body_span))
        } else {
            let mut options = self.options.in_before_block().not_in_decorator();
            options.set_allow_sequence_expression(false);
            let body = self.eat_expression(options)?;
            let body_span = self.get_span_from(body_start);

            Ok((body, body_span))
        }
    }

    /// Build a plain lambda declaration from parsed parameters and body.
    fn build_plain_lambda_declaration(
        &mut self,
        start: &ParserMark,
        header: &DeclarationHeader,
        parameters: Vec<LocalNodeId<Parameter>>,
        generic_parameter_container_span: Option<Span>,
        parameter_container_span: Option<Span>,
        body_container_span: Option<Span>,
        return_type: Option<LocalNodeId<TypeExpression>>,
        return_type_span: Option<Span>,
        body: LocalNodeId<Expression>,
    ) -> LocalNodeId<Declaration> {
        let (this_parameter, parameters) = self.split_this_parameter_maybe(parameters);
        let signature = FunctionSignature {
            is_abstract: false,
            is_override: false,
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            mode: None,
            kind: FunctionKind::Lambda,
            generic_parameters: vec![],
            where_clauses: vec![],
            this_parameter,
            parameters,
            return_type,
        };
        let function_id = self.insert_node(
            Declaration::Function(FunctionDeclaration {
                name: None,
                export: header.export,
                ambient: header.ambient,
                signature,
                body: Some(body),
            }),
            self.get_span_from(start),
        );
        if let Some(span) = return_type_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Type, span);
        }

        if let Some(span) = generic_parameter_container_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::GenericParameters, span);
        }

        if let Some(span) = parameter_container_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Parameters, span);
        }

        if let Some(span) = body_container_span {
            self.tree
                .set_side_span(function_id, NodeSpanType::Body, span);
        }

        function_id
    }

    /// Scan a simple parenthesized lambda head without forcing a full pair lookup.
    fn scan_plain_parenthesized_lambda_head(
        &mut self,
        open_index: usize,
    ) -> Option<(usize, ParenthesizedLambdaHeadShape)> {
        if self.token_type_at(open_index) != TokenType::OpenParenthesis {
            return None;
        }

        // track the plain head state
        let mut semantic_token_count = 0usize;
        let mut first_token_type = None;
        let mut second_token_type = None;
        let mut third_token_type = None;
        let mut has_parameter = false;
        let mut has_type_annotation = false;
        let mut has_type_tokens = false;

        // depth counters keep top level comma checks cheap
        let mut parenthesis_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;

        let mut token_index = open_index + 1;
        let close_index = loop {
            let token_type = self.token_type_at(token_index);
            if token_type == TokenType::End {
                return None;
            }

            // top level close: finalize the head
            if token_type == TokenType::CloseParenthesis
                && parenthesis_depth == 0
                && brace_depth == 0
                && bracket_depth == 0
                && angle_depth == 0
            {
                break token_index;
            }

            if !is_semantic(token_type) || token_type == TokenType::Newline {
                token_index += 1;
                continue;
            }

            semantic_token_count += 1;
            if semantic_token_count > PLAIN_PARENTHESIZED_LAMBDA_MAX_TOKENS {
                return None;
            }

            match semantic_token_count {
                1 => first_token_type = Some(token_type),
                2 => second_token_type = Some(token_type),
                3 => third_token_type = Some(token_type),
                _ => {}
            }

            // require at most one named parameter
            if !has_parameter {
                if token_type == TokenType::Identifier {
                    has_parameter = true;
                    token_index += 1;
                    continue;
                }

                return None;
            }

            // optionally allow one top level type annotation marker
            if !has_type_annotation {
                if token_type == TokenType::Colon {
                    has_type_annotation = true;
                    token_index += 1;
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
                TokenType::ShiftRight => angle_depth = angle_depth.saturating_sub(2),
                TokenType::UnsignedShiftRight => angle_depth = angle_depth.saturating_sub(3),
                _ => {}
            }
            has_type_tokens = true;
            token_index += 1;
        };

        // common plain heads: (), (x), (x: T)
        let head_shape = if semantic_token_count == 0 {
            ParenthesizedLambdaHeadShape::Empty
        } else if semantic_token_count == 1 && first_token_type == Some(TokenType::Identifier) {
            ParenthesizedLambdaHeadShape::Named {
                has_type_annotation: false,
            }
        } else if semantic_token_count == 3
            && first_token_type == Some(TokenType::Identifier)
            && second_token_type == Some(TokenType::Colon)
            && matches!(
                third_token_type,
                Some(TokenType::Identifier | TokenType::Literal)
            )
        {
            ParenthesizedLambdaHeadShape::Named {
                has_type_annotation: true,
            }
        } else if has_parameter {
            if has_type_annotation
                && (!has_type_tokens
                    || parenthesis_depth != 0
                    || brace_depth != 0
                    || bracket_depth != 0
                    || angle_depth != 0)
            {
                return None;
            }

            ParenthesizedLambdaHeadShape::Named {
                has_type_annotation,
            }
        } else {
            ParenthesizedLambdaHeadShape::Empty
        };

        Some((close_index, head_shape))
    }

    /// Return the close index for a plain parenthesized lambda head.
    pub(crate) fn plain_parenthesized_lambda_close_index(
        &mut self,
        open_index: usize,
    ) -> Option<usize> {
        let (close_index, _) = self.scan_plain_parenthesized_lambda_head(open_index)?;

        Some(close_index)
    }

    /// Try to parse plain `() => body`, `(identifier) => body`, or `(identifier: Type) => body` lambdas.
    fn try_eat_plain_parenthesized_lambda(
        &mut self,
        start: &ParserMark,
        header: &DeclarationHeader,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        self.stats.record_parenthesized_lambda_plain_call();

        let open_index = self.pos_index();
        let Some((close_index, head_shape)) = self.scan_plain_parenthesized_lambda_head(open_index)
        else {
            self.stats.record_parenthesized_lambda_plain_miss();
            return Ok(None);
        };

        let follow_index = self.next_non_newline_index_from(close_index + 1);
        let follow_token_type = self.token_type_at(follow_index);
        if close_index <= open_index {
            self.stats.record_parenthesized_lambda_plain_miss();
            return Ok(None);
        }

        // require an arrow or a return type marker after the group
        if !matches!(
            follow_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        ) {
            self.stats.record_parenthesized_lambda_plain_miss();
            return Ok(None);
        }

        // parse the parenthesized head
        let parameter_container_start = self.mark_span();
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        let mut parameters = Vec::with_capacity(1);
        if let ParenthesizedLambdaHeadShape::Named {
            has_type_annotation,
        } = head_shape
        {
            let parameter_start = self.mark_span();
            let (parameter_name, parameter_name_span) = self.eat_binding_identifier_with_span()?;
            let (parameter_type, parameter_type_span) = if has_type_annotation {
                self.eat_newlines_maybe()?;
                let type_start = self.mark_span();
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
                let parameter_type = self.eat_type_expression_node_or_recover_missing(
                    type_options,
                    NodeType::Parameter,
                )?;
                let parameter_type_span = self.get_span_from(&type_start);
                (Some(parameter_type), Some(parameter_type_span))
            } else {
                (None, None)
            };
            self.eat_newlines_maybe()?;

            let parameter_id = self.insert_node(
                Parameter::Named {
                    name: parameter_name,
                    visibility: None,
                    is_readonly: false,
                    is_optional: false,
                    declared_type: parameter_type,
                    default: None,
                },
                self.get_span_from(&parameter_start),
            );
            self.tree.set_main_span(parameter_id, parameter_name_span);
            if let Some(span) = parameter_type_span {
                self.tree
                    .set_side_span(parameter_id, NodeSpanType::Type, span);
            }
            parameters.push(parameter_id);
        }
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;
        let parameter_container_span = Some(self.get_span_from(&parameter_container_start));

        // parse an explicit lambda return type when present
        let (return_type, return_type_span) = if self.has_lambda_return_type_marker() {
            self.eat_newlines_maybe()?;
            let type_start = self.mark_span();
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
            let return_type = self.eat_type_expression_node_or_recover_missing(
                return_type_options,
                NodeType::Declaration,
            )?;
            let return_type_span = self.get_span_from(&type_start);

            (Some(return_type), Some(return_type_span))
        } else {
            (None, None)
        };

        // parse the body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let (body, body_container_span) = self.eat_plain_lambda_body(&body_start)?;

        let function_id = self.build_plain_lambda_declaration(
            start,
            header,
            parameters,
            None,
            parameter_container_span,
            Some(body_container_span),
            return_type,
            return_type_span,
            body,
        );

        self.stats.record_parenthesized_lambda_plain_hit();

        Ok(Some(function_id))
    }

    /// Try to parse a parenthesized lambda value without entering full function parsing.
    fn try_eat_parenthesized_lambda_value(
        &mut self,
        start: &ParserMark,
        header: &DeclarationHeader,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        // require an arrow or return type marker after the parenthesized head
        let open_index = self.pos_index();
        let Some((close_index, _)) = self.scan_plain_parenthesized_lambda_head(open_index) else {
            return Ok(None);
        };
        if close_index <= open_index {
            return Ok(None);
        }
        let follow_index = self.next_non_newline_index_from(close_index + 1);
        let follow_token_type = self.token_type_at(follow_index);
        if !matches!(
            follow_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        ) {
            return Ok(None);
        }

        // dynamic parameters
        let parameter_container_start = self.mark_span();
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        let parameter_options = self.options.with_generator(false).with_forbid_yield(false);
        let parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            self.with_options(parameter_options, |parser| parser.eat_parameters_body())?
        };
        self.eat_newlines_maybe()?;
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;
        let parameter_container_span = Some(self.get_span_from(&parameter_container_start));

        // explicit lambda return type
        let (return_type, return_type_span) = if self.has_lambda_return_type_marker() {
            self.eat_newlines_maybe()?;
            let type_start = self.mark_span();
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
            let return_type = self.eat_type_expression_node_or_recover_missing(
                return_type_options,
                NodeType::Declaration,
            )?;
            let return_type_span = self.get_span_from(&type_start);

            (Some(return_type), Some(return_type_span))
        } else {
            (None, None)
        };

        // body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let (body, body_container_span) = self.eat_plain_lambda_body(&body_start)?;

        let function_id = self.build_plain_lambda_declaration(
            start,
            header,
            parameters,
            None,
            parameter_container_span,
            Some(body_container_span),
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
        header: &DeclarationHeader,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        self.stats.record_identifier_lambda_plain_call();

        // parse the single named parameter
        let parameter_name = self.eat_identifier()?;
        let parameter_span = self.get_span_from(start);
        let parameter_id = self.insert_node(
            Parameter::Named {
                name: parameter_name,
                visibility: None,
                is_readonly: false,
                is_optional: false,
                declared_type: None,
                default: None,
            },
            self.get_span_from(start),
        );

        // parse the lambda body
        self.eat_arrow()?;
        self.eat_newlines_maybe()?;
        let body_start = self.mark_span();
        let (body, body_container_span) = self.eat_plain_lambda_body(&body_start)?;

        // build the declaration
        let function_id = self.build_plain_lambda_declaration(
            start,
            header,
            vec![parameter_id],
            None,
            Some(parameter_span),
            Some(body_container_span),
            None,
            None,
            body,
        );

        self.stats.record_identifier_lambda_plain_hit();

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
    pub(crate) fn eat_function(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_function_inner(start, header, expect_maybe, expect_body)
    }

    /// Eat a function.
    fn eat_function_inner(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_FUNCTION);
        let can_parse_plain_lambda =
            self.can_parse_plain_lambda(&header, expect_maybe, expect_body);

        // parse plain lambda heads only when the token shape matches
        if can_parse_plain_lambda && self.peek_is(TokenType::OpenParenthesis) {
            if let Some(function_id) = self.try_eat_plain_parenthesized_lambda(start, &header)? {
                return Ok(function_id);
            }

            if let Some(function_id) = self.try_eat_parenthesized_lambda_value(start, &header)? {
                return Ok(function_id);
            }
        } else if can_parse_plain_lambda
            && self.peek_is(TokenType::Identifier)
            && matches!(
                self.peek_next_token_type(),
                TokenType::Arrow | TokenType::ArrowWide
            )
            && let Some(function_id) = self.try_eat_plain_identifier_lambda(start, &header)?
        {
            return Ok(function_id);
        }

        let function = self.eat_function_parts(start, header, expect_maybe, expect_body)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Eat a function type expression.
    pub(crate) fn eat_function_type_expression(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_FUNCTION);
        let function = self.eat_function_parts(start, header, expect_maybe, expect_body)?;

        if function.signature.kind == FunctionKind::Lambda && function.body.is_none() {
            return Ok(self.insert_function_type_expression(start, function));
        }

        let function_id = self.insert_function_declaration(start, function);

        Ok(self.insert_declaration_type_expression(start, function_id))
    }

    /// Eat shared function syntax before inserting a grammar-specific node.
    fn eat_function_parts(
        &mut self,
        start: &ParserMark,
        mut header: DeclarationHeader,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<ParsedFunctionSignature> {
        // abstraction
        if self.is_keyword(Keyword::Abstract) && !header.is_abstract {
            self.bump(); // eat abstract keyword
            header.is_abstract = true;
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

        // function style, name, generic parameters
        let (name, name_span, generic_parameters, generic_parameter_container_span) = {
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

                // generic parameters
                let generic_parameter_container_start = self.mark_span();
                let generic_parameters = self
                    .eat_generic_parameters_maybe(false)
                    .for_node_type(NodeType::Declaration)?;
                let generic_parameter_container_span = generic_parameters
                    .as_ref()
                    .map(|_| self.get_span_from(&generic_parameter_container_start));

                (
                    name,
                    name_span,
                    generic_parameters,
                    generic_parameter_container_span,
                )
            } else {
                // generic parameters
                let generic_parameter_container_start = self.mark_span();
                let generic_parameters = self
                    .eat_generic_parameters_maybe(false)
                    .for_node_type(NodeType::Declaration)?;
                let generic_parameter_container_span = generic_parameters
                    .as_ref()
                    .map(|_| self.get_span_from(&generic_parameter_container_start));

                (
                    None,
                    None,
                    generic_parameters,
                    generic_parameter_container_span,
                )
            }
        };

        // declarations in statement position require a name unless default-exported
        if kind == FunctionKind::Function
            && self.options.is_in_statement_position()
            && name.is_none()
            && header.export != Some(ExportMode::Default)
        {
            return Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ));
        }

        // dynamic parameters
        let (parameters, parameter_container_span) = {
            // regular `(...) => ...` function/lambda
            let has_parenthesized_parameters = kind == FunctionKind::Function
                || self.options.is_in_type()
                || self.peek_is(TokenType::OpenParenthesis)
                || self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis);
            if has_parenthesized_parameters {
                // allow line breaks before the parameter list
                self.eat_newlines_maybe()?;
                let parameter_container_start = self.mark_span();
                self.eat_token(TokenType::OpenParenthesis)?;
                self.eat_newlines_maybe()?;

                // dynamic parameters
                let parameters = if self.peek_is(TokenType::CloseParenthesis) {
                    vec![]
                } else {
                    let parameter_options = self
                        .options
                        .with_generator(is_generator)
                        .with_forbid_yield(is_generator);
                    self.with_options(parameter_options, |parser| parser.eat_parameters_body())?
                };
                self.eat_newlines_maybe()?;
                self.eat_list_close_token_or_recover_missing(
                    TokenType::CloseParenthesis,
                    NodeType::Parameter,
                )?;
                let parameter_container_span = Some(self.get_span_from(&parameter_container_start));

                (parameters, parameter_container_span)
            }
            // plain no-parentheses `x => y` lambda value
            else {
                if is_generator && self.is_keyword(Keyword::Yield) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                let parameter_name = self.eat_identifier()?;
                let parameter_id = self.insert_node(
                    Parameter::Named {
                        name: parameter_name,
                        visibility: None,
                        is_readonly: false,
                        is_optional: false,
                        declared_type: None,
                        default: None,
                    },
                    self.get_span_from(start),
                );

                (vec![parameter_id], None)
            }
        };

        // return type info (including where)
        // only for functions or lambda types
        let (return_type, return_type_span, where_clauses) = {
            // lambda with explicit return type
            if kind == FunctionKind::Lambda && self.has_lambda_return_type_marker() {
                self.eat_newlines_maybe()?;
                let type_start = self.mark_span();
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
                let return_type = self.eat_type_expression_node_or_recover_missing(
                    return_type_options,
                    NodeType::Declaration,
                )?;
                let return_type_span = self.get_span_from(&type_start);

                // where clauses are only enabled in the extended grammar
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
                    self.eat_newlines_maybe()?;
                    let type_start = self.mark_span();
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
                    let return_type = self.eat_type_expression_node_or_recover_missing(
                        return_type_options,
                        NodeType::Declaration,
                    )?;
                    (Some(return_type), Some(self.get_span_from(&type_start)))
                } else {
                    (None, None)
                };

                // where clauses are only enabled in the extended grammar
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
        let (body, body_container_span) = {
            // semicolon statement function bodies may start on the next line
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
                let block_id = self
                    .with_options(options, |parser| parser.eat_block(BlockContext::Expression))?;
                let body_span = self.get_span_from(&body_start);
                let body = self.tree.insert(Expression::Block(block_id), body_span);
                (Some(body), Some(body_span))
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
                    let block_id = self.with_options(options, |parser| {
                        parser.eat_block(BlockContext::Expression)
                    })?;
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
                let body_span = self.get_span_from(&body_start);
                (Some(body), Some(body_span))
            }
            // no body
            else {
                (None, None)
            }
        };

        // split out explicit this parameter
        let (this_parameter, parameters) = self.split_this_parameter_maybe(parameters);

        // signature
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
        let signature = FunctionSignature {
            is_abstract: header.is_abstract,
            is_override: false,
            asynchrony,
            cardinality,
            kind,
            mode,
            generic_parameters: generic_parameters.unwrap_or_default(),
            where_clauses: where_clauses.unwrap_or_default(),
            this_parameter,
            parameters,
            return_type,
        };

        Ok(ParsedFunctionSignature {
            header,
            name,
            name_span,
            signature,
            body,
            generic_parameter_span: generic_parameter_container_span,
            parameter_span: parameter_container_span,
            return_type_span,
            body_span: body_container_span,
        })
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
        Argument, Asynchrony, BlockContext, BlockFormat, ClassDeclaration, CommentKind,
        CommentPosition, Declaration, Declarator, Expression, FunctionCardinality,
        FunctionDeclaration, FunctionKind, FunctionMode, GenericArgument, GenericParameter,
        IntType, NodeType, Parameter, Pattern, ScalarLiteral, TypeDeclaration, TypeExpression,
        TypeLiteral, VarianceModifier, WhereClause, YieldCardinality,
    };

    use destack_source::{LanguageType, NodeSpanType};

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{
        ParserSettings, TestParser, assert_comment, assert_expression_path, assert_name,
        assert_node, assert_path, assert_string,
    };

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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        // (x: number): number => x
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body: Some(body), .. }) => {
            assert_eq!(*name, None);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x: number
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
            // number
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
            // x
            assert_node!(parser.tree, *body, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_missing_close_paren_keeps_following_declaration() {
        let mut test = TestParser::new_with_options(
            r#"
export function broken( {}
export function stableLater(): void {}
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let expressions = parser
            .eat_block_body_in_context(BlockFormat::Implicit, BlockContext::Statement)
            .unwrap();

        assert_eq!(expressions.len(), 2);

        // export function broken( {}
        let first_declaration_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
                assert_name!(parser, name.unwrap(), "broken");
            });
        });

        // export function stableLater(): void {}
        let second_declaration_id = parser.unwrap_labelled_expression(expressions[1]);
        assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
                assert_name!(parser, name.unwrap(), "stableLater");
            });
        });
    }

    #[test]
    fn test_parse_function_missing_close_paren_before_following_function_keeps_declaration() {
        let mut test = TestParser::new_with_options(
            r#"
function broken(
function stableLater(): void {}
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let expressions = parser
            .eat_block_body_in_context(BlockFormat::Implicit, BlockContext::Statement)
            .unwrap();

        assert_eq!(expressions.len(), 2);

        // function broken(
        let first_declaration_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
                assert_name!(parser, name.unwrap(), "broken");
            });
        });

        // function stableLater(): void {}
        let second_declaration_id = parser.unwrap_labelled_expression(expressions[1]);
        assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
                assert_name!(parser, name.unwrap(), "stableLater");
            });
        });
    }

    #[test]
    fn test_parse_function_missing_close_paren_before_following_const_keeps_statement() {
        let mut test = TestParser::new_with_options(
            r#"
function broken(
const value = 1
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let expressions = parser
            .eat_block_body_in_context(BlockFormat::Implicit, BlockContext::Statement)
            .unwrap();

        assert_eq!(expressions.len(), 2);

        // function broken(
        let first_declaration_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
                assert_name!(parser, name.unwrap(), "broken");
            });
        });

        // const value = 1
        let second_expression_id = parser.unwrap_labelled_expression(expressions[1]);
        assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_function_parameter_named_type_after_newline() {
        let mut test = TestParser::new_with_options(
            r#"
function configure(
    type: string,
): void {}
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newlines_maybe().unwrap();

        let start = parser.mark_span();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        // function configure(type: string): void {}
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_name!(parser, name.unwrap(), "configure");
            assert_eq!(signature.parameters.len(), 1);

            // type: string
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "type");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    }

    #[test]
    fn test_parse_function_parameter_named_namespace_after_newline() {
        let mut test = TestParser::new(
            r#"
function setns(
    namespace: ProcessNamespaceKind,
): void {}
"#,
        );
        let mut parser = test.prepare();
        parser.eat_newlines_maybe().unwrap();

        let start = parser.mark_span();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        // function setns(namespace: ProcessNamespaceKind): void {}
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_name!(parser, name.unwrap(), "setns");
            assert_eq!(signature.parameters.len(), 1);

            // namespace: ProcessNamespaceKind
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "namespace");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "ProcessNamespaceKind");
                });
            });
        });
    }

    #[test]
    fn test_parse_plain_parenthesized_lambda_with_newlines() {
        let mut test = TestParser::new("(\nvalue\n) => value");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "value");
                assert!(declared_type.is_none());
                assert!(default.is_none());
            });
            assert!(signature.return_type.is_none());
            assert_node!(parser.tree, body.expect("expected body"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "value");
            });
        });

        let parameter_container_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Parameters)
            .unwrap();
        assert_eq!(parser.get_span_str(parameter_container_span), "(\nvalue\n)");

        let body_container_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Body)
            .unwrap();
        assert_eq!(parser.get_span_str(body_container_span), "value");
    }

    #[test]
    fn test_parse_plain_identifier_lambda_parameters_span() {
        let mut test = TestParser::new("value => value");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        let parameters_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Parameters)
            .unwrap();
        assert_eq!(parser.get_span_str(parameters_span), "value");

        let body_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Body)
            .unwrap();
        assert_eq!(parser.get_span_str(body_span), "value");
    }

    #[test]
    fn test_parse_plain_lambda_parenthesized_body_span() {
        let mut test = TestParser::new("value => ({ key: value })");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        let body_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Body)
            .unwrap();
        assert_eq!(parser.get_span_str(body_span), "({ key: value })");
    }

    #[test]
    fn test_parse_plain_parenthesized_typed_lambda() {
        let mut test = TestParser::new("(value: number) => value");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "value");
                assert!(default.is_none());
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
            assert_node!(parser.tree, body.expect("expected body"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "value");
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
            assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
                assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                    assert!(function.this_parameter.is_some());
                    assert_eq!(function.parameters.len(), 1);
                    // value: Bar
                    assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "value");
                        assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "Bar");
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        // (this: string) => {}
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.this_parameter.is_some());
            assert!(signature.parameters.is_empty());
            assert!(body.is_some());
        });
    }

    #[test]
    fn test_parse_function_lambda_with_explicit_return_type() {
        let mut test = TestParser::new("(x): int32 => x");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        // (x): int32 => x
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body: Some(body), .. }) => {
            assert!(name.is_none());
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // int32
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true }));
            });
            // x
            assert_node!(parser.tree, *body, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
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
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(body.is_some());
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
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
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.parameters.len(), 2);
                // greeting: string = "Hello"
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
                    assert_string!(parser, *name, "greeting");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                    assert_node!(parser.tree, default.unwrap(), Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                        assert_string!(parser, *value, "Hello");
                    });
                });
                // target: string
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default, .. } => {
                    assert_string!(parser, *name, "target");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        // new (x) => int32
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert!(name.is_none());
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.parameters.is_empty());
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "$");
        });
    }

    #[test]
    fn test_parse_function_new_type_with_generic_arguments() {
        let mut test = TestParser::new("new <T>(x: int32) => T");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        // new <T>(x: int32) => T
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert!(name.is_none());
            // new
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let generic_parameters = &signature.generic_parameters;
            // <T>
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                assert!(constraint.is_none());
            });
            // x: int32
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true }));
                });
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            // function name
            assert_string!(parser, name.expect("expected name").string(), "foo");
            // where Guard: Limit
            let where_clauses = &signature.where_clauses;
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
            // return type
            let ret = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, ret, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true }));
            });
        });
    }

    #[test]
    fn test_parse_function_with_generic_and_dynamic_parameters() {
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            // compute
            assert_string!(parser, name.expect("expected name").string(), "compute");
            let generic_parameters = &signature.generic_parameters;
            // <Validate: bool, Precision: uint8>
            assert_eq!(generic_parameters.len(), 2);

            // Validate: bool
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "Validate");
                assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Boolean);
                });
            });

            // Precision: uint8
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "Precision");
                assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary { width: Some(8), is_signed: false }));
                });
            });
            // data: uint8[]
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        // abstract\nnew (): T
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert!(signature.is_abstract);
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.parameters.is_empty());
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body, .. }) => {
            // function name
            assert_string!(parser, name.expect("expected name").string(), "h");

            // generic parameter: T
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                assert!(constraint.is_none());
            });

            // dynamic parameter tag: T
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "tag");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "T");
                    assert!(generic_arguments.is_empty());
                });
            });

            // return type T
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "T");
                assert!(generic_arguments.is_empty());
            });

            // declaration signature has no body
            assert!(body.is_none());
        });

        let generic_parameter_container_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::GenericParameters)
            .unwrap();
        assert_eq!(parser.get_span_str(generic_parameter_container_span), "<T>");

        let parameter_container_span = parser
            .tree
            .get_side_span(function_id, NodeSpanType::Parameters)
            .unwrap();
        assert_eq!(parser.get_span_str(parameter_container_span), "(tag: T)");
    }

    #[test]
    fn test_parse_function_with_newline_before_return_type_colon() {
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            let return_type = signature.return_type.expect("expected return type");
            assert_expression_path!(parser, parser.tree.get(return_type), "T");

            let body_id = body.expect("expected function body");
            assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Return { value } => {
                        let value = value.expect("expected return value");
                        assert_node!(parser.tree, value, Expression::As { .. });
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "transform");
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 2);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, variance, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(*variance, Some(VarianceModifier::In));
            });
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, variance, .. } => {
                assert_string!(parser, *name, "U");
                assert_eq!(*variance, Some(VarianceModifier::Out));
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "invariant");
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, variance, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(*variance, Some(VarianceModifier::InOut));
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            // foo
            assert_string!(parser, name.expect("expected name").string(), "foo");

            // (str: string) => boolean
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
                assert_eq!(function.parameters.len(), 1);
                // str: string
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "str");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });

                // boolean
                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Boolean);
                });
            });
        });
    }

    #[test]
    fn test_parse_function_return_type_with_generic_arguments() {
        let mut test = TestParser::new("function read<T, E>() => AliasBranch<T, E> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();

        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            let return_type = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, return_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "AliasBranch");
                assert_eq!(generic_arguments.len(), 2);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });
                assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        // async function* foo() => int32 { body }
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            // foo
            assert_string!(parser, name.unwrap().string(), "foo");
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.cardinality, FunctionCardinality::Generator);
        });
    }

    /// Parse a generator function with a bare yield call argument.
    #[test]
    fn test_parse_function_generator_call_argument_with_bare_yield() {
        // source: function* a() { b.c(yield); }
        let mut test =
            TestParser::new_with_options("function* a() { b.c(yield); }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function* a() { b.c(yield); }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { b.c(yield); }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    // b.c(yield);
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Call { left, arguments, .. } => {
                            assert_eq!(arguments.len(), 1);
                            // b.c
                            assert_expression_path!(parser, parser.tree.get(*left), "b.c");
                            // yield
                            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::Yield { cardinality, value } => {
                                    assert_eq!(*cardinality, YieldCardinality::Scalar);
                                    assert!(value.is_none());
                                });
                            });
                    });
                });
            });
        });
    }

    /// Preserve the enclosing function when a call argument is missing before the block close.
    #[test]
    fn test_parse_function_body_preserves_declaration_for_missing_call_argument_before_block_close()
    {
        // source
        let mut test = TestParser::new(
            r#"
function greet(name: string, suffix: string) {}

function main() {
    const userName = "Alice";
    greet(userName,
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[(None, None, "}"), (Some(NodeType::Expression), None, "}")],
        );

        // top level expressions
        assert_eq!(expressions.len(), 2);

        // function main() { ... }
        assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, body: Some(body), .. }) => {
                assert_string!(parser, name.unwrap().string(), "main");

                // { const userName = "Alice"; greet(userName, }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    let tail_expression = block.tail_expression.expect("expected trailing malformed call");

                    // const userName = "Alice";
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Let { declarators, .. } => {
                        assert_eq!(declarators.len(), 1);
                    });

                    // greet(userName,
                    assert_node!(parser.tree, tail_expression, Expression::Call { left, arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "greet");
                        assert_eq!(arguments.len(), 2);

                        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "userName");
                        });

                        assert_node!(parser.tree, arguments[1], Argument::Error);
                    });
                });
            });
        });
    }

    /// Parse nested generator yield expressions.
    #[test]
    fn test_parse_function_generator_nested_yield() {
        // source: function *a() { yield yield }
        let mut test =
            TestParser::new_with_options("function *a() { yield yield }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield yield }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield yield }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    // yield yield
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
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
    }

    /// Parse delegated generator yield with a direct identifier operand.
    #[test]
    fn test_parse_function_generator_delegate_yield() {
        // source: function *a() { yield *a }
        let mut test =
            TestParser::new_with_options("function *a() { yield *a }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield *a }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield *a }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    // yield *a
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Generator);
                            assert!(value.is_some());
                            assert_expression_path!(parser, parser.tree.get(value.unwrap()), "a");
                    });
                });
            });
        });
    }

    /// Parse delegated generator yield with a nested bare yield operand.
    #[test]
    fn test_parse_function_generator_delegate_nested_yield() {
        // source: function *a() { yield *yield }
        let mut test = TestParser::new_with_options(
            "function *a() { yield *yield }",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a() { yield *yield }
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { yield *yield }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    // yield *yield
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
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
    }

    /// Recover delegated generator yield when a line terminator appears before `*`.
    #[test]
    fn test_recover_function_generator_delegate_after_newline() {
        // source: function *a(){yield
        // *a}
        let mut test =
            TestParser::new_with_options("function *a(){yield\n*a}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function *a(){yield
        // *a}
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);

                // { yield \n *a }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 2);
                    assert!(block.tail_expression.is_none());

                    // yield
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Scalar);
                            assert!(value.is_none());
                    });

                    // *a
                    assert_node!(parser.tree, block.leading_expressions[1], Expression::Error);
                });
            });
        });
    }

    /// Recover delegated generator yield without an operand before a following const statement.
    #[test]
    fn test_recover_function_generator_delegate_before_following_const() {
        // source: function *a(){yield*
        // const value = 1}
        let mut test = TestParser::new_with_options(
            "function *a(){yield*\nconst value = 1}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "\n")]);

        // function *a(){yield*
        // const value = 1}
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);

                // { yield* \n const value = 1 }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 2);
                    assert!(block.tail_expression.is_none());

                    // yield*
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Generator);
                            assert_node!(parser.tree, value.expect("expected missing generator operand"), Expression::Missing);
                    });

                    // const value = 1
                    assert_node!(parser.tree, block.leading_expressions[1], Expression::Let { declarators, .. } => {
                        assert_eq!(declarators.len(), 1);
                    });
                });
            });
        });
    }

    /// Parse generator yield in class heritage expression.
    #[test]
    fn test_parse_function_generator_yield_in_class_heritage() {
        // source: function* a(){(class extends (yield) {});}
        let mut test = TestParser::new_with_options(
            "function* a(){(class extends (yield) {});}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // function* a(){(class extends (yield) {});}
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                // { (class extends (yield) {}); }
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Declaration(class_id) => {
                            assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
                                assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                                    assert_node!(parser.tree, *expression, Expression::Yield { cardinality, value } => {
                                        assert_eq!(*cardinality, YieldCardinality::Scalar);
                                        assert!(value.is_none());
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parse generator yield in computed property keys and assignment targets.
    #[test]
    fn test_parse_function_generator_yield_in_computed_keys() {
        // source: function* a(){(class {[yield](){}})};
        let mut test = TestParser::new_with_options(
            "function* a(){(class {[yield](){}})};",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { .. });
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
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { .. });
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_string!(parser, name.unwrap().string(), "onResolve");
            // callback: (args) => { .. } | void
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "callback");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 1);
                    // args
                    assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                        assert_string!(parser, *name, "args");
                    });
                    // { .. } | void
                    assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);

                        // { .. }
                        assert_node!(parser.tree, elements[0], TypeExpression::Object { members: properties } => {
                            assert_eq!(properties.len(), 2);
                        });

                        // void
                        assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Void);
                        });
                    });
                });
            });
            // void
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Void);
            });
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
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_eq!(signature.parameters.len(), 3);

                // (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<any, any> | undefined) => void
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 2);

                    // fiberId?: FiberId.FiberId
                    assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "fiberId");
                        assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "FiberId.FiberId");
                    });

                    // options?: Runtime.RunCallbackOptions<any, any> | undefined
                    assert_node!(parser.tree, function.parameters[1], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "options");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Union { elements } => {
                            assert_eq!(elements.len(), 2);
                        });
                    });

                    assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
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
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        parser.attach_comments();
        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { .. }) => {
            let annotations = parser.tree.get_decorators(function_id.id);
            assert!(annotations.is_empty());
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::SingleLineBlock, " lambda-head");
    }

    #[test]
    fn test_parse_lambda_body_boundary_comment_on_body_owner() {
        let mut test =
            TestParser::new_with_options("(x) =>\n// lambda-body\nx", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        parser.attach_comments();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_expression_path!(parser, parser.tree.get(*body_id), "x");

                let annotations = parser.tree.get_decorators(body_id.id);
                assert!(annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "lambda-body");
    }

    /// Parse empty parenthesized lambda heads that only contain comments.
    #[test]
    fn test_parse_empty_parenthesized_lambda_with_comment() {
        let mut test =
            TestParser::new_with_options("(/* empty */) => {}", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        parser.attach_comments();

        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(signature.parameters.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::SingleLineBlock, " empty");
    }

    #[test]
    fn test_parse_lambda_comment_only_block_body_attaches_inside_block() {
        let mut test =
            TestParser::new_with_options("() => {\n  // code\n}", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        parser.attach_comments();

        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                let body_span = parser.tree.get_span(*body_id);
                let comment = parser.tree.comments()[0];

                assert_eq!(comment.position, CommentPosition::Leading);
                assert!(comment.span.start >= body_span.start);
                assert!(comment.span.end <= body_span.end);
            });
        });

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "code");
    }

    #[test]
    fn test_parse_function_body_boundary_line_comment_stays_trailing() {
        let mut test = TestParser::new_with_options(
            "function f(): void // body\n{}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let function_id = parser
            .eat_function(&start, DeclarationHeader::default(), false, false)
            .unwrap();
        parser.attach_comments();

        assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            let body_span = parser.tree.get_span(*body_id);
            let comment = parser.tree.comments()[0];

            assert_eq!(comment.position, CommentPosition::Trailing);
            assert_eq!(comment.attached_to, 0);
            assert!(comment.span.end <= body_span.start);
        });

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "body");
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
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                    });
                });
            });
        });
    }

    /// Parse direct calls on parenthesized arrow functions without preserved wrappers.
    #[test]
    fn test_parse_parenthesized_arrow_call_without_preserved_wrappers() {
        // source: (() => {})()
        let mut test = TestParser::new_with_options("(() => {})()", LanguageType::Destack);
        let mut parser = test.prepare();
        parser.apply_settings(ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        });
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // (() => {})()
        assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                });
            });
        });
    }
}
