use crate::parse::flags::ParserFlags;
use crate::parse::prelude::*;
use crate::parse::scan::DelimiterDepth;
use crate::parse::{DeclarationHeader, RecoveryPoint};
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Asynchrony, BlockContext, ConstructorType, Declaration, ExportKind, Expression,
    FunctionDeclaration, FunctionForm, FunctionPhase, FunctionRole, FunctionSignature,
    FunctionType, GenericParameter, Keyword, LocalNodeId, Name, NodeType, Parameter, TokenType,
    TypeExpression, WhereClause,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// The keywords that can appear before a function declaration.
pub static FUNCTION_MODIFIERS: [Keyword; 8] = [
    Keyword::Async,
    Keyword::Comptime,
    Keyword::Abstract,
    Keyword::Override,
    Keyword::Get,
    Keyword::Set,
    Keyword::Constructor,
    Keyword::New,
];

/// The head shapes accepted by the fast arrow path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrowHeadShape {
    /// No dynamic parameters: `()`.
    Empty,
    /// One named parameter: `(value)` or `(value: Type)`.
    Named {
        /// Whether the parameter has a type annotation.
        has_type_annotation: bool,
        /// Whether the parameter has a prefix modifier.
        has_modifier: bool,
    },
}

/// Parsed function signature syntax.
struct ParsedFunction {
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

/// Parsed function head syntax before parameters.
struct ParsedFunctionHead {
    /// The declaration header.
    header: DeclarationHeader,
    /// Whether the function is async.
    is_async: bool,
    /// The optional function role.
    role: Option<FunctionRole>,
    /// The source form of the function.
    form: FunctionForm,
    /// When the function may be called.
    phase: FunctionPhase,
    /// Whether the function is a generator.
    is_generator: bool,
    /// The optional function name.
    name: Option<Name>,
    /// The optional function name span.
    name_span: Option<Span>,
    /// The generic parameters.
    generic_parameters: Option<Vec<LocalNodeId<GenericParameter>>>,
    /// The generic parameter container span.
    generic_parameter_span: Option<Span>,
}

/// Parsed function parameter list syntax.
struct ParsedFunctionParameters {
    /// The parsed parameters.
    parameters: Vec<LocalNodeId<Parameter>>,
    /// The parameter container span.
    parameter_span: Option<Span>,
}

/// Parsed function return syntax.
struct ParsedFunctionReturn {
    /// The optional return type.
    return_type: Option<LocalNodeId<TypeExpression>>,
    /// The return type span.
    return_type_span: Option<Span>,
    /// The parsed where clauses.
    where_clauses: Option<Vec<LocalNodeId<WhereClause>>>,
}

/// Parsed function body syntax.
struct ParsedFunctionBody {
    /// The optional body expression.
    body: Option<LocalNodeId<Expression>>,
    /// The body container span.
    body_span: Option<Span>,
}

/// Parsed arrow function syntax.
struct ParsedArrowFunction {
    /// The declaration header.
    header: DeclarationHeader,
    /// The parsed parameters.
    parameters: Vec<LocalNodeId<Parameter>>,
    /// The generic parameter container span.
    generic_parameter_span: Option<Span>,
    /// The parameter container span.
    parameter_span: Option<Span>,
    /// The optional return type.
    return_type: Option<LocalNodeId<TypeExpression>>,
    /// The return type span.
    return_type_span: Option<Span>,
    /// The parsed body expression.
    body: LocalNodeId<Expression>,
    /// The body container span.
    body_span: Span,
}

impl Parser {
    /// Eat a function or lambda declaration.
    ///
    /// A signature without a body represents an external declaration.
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
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let can_parse_arrow_value = !self.flags.is_in_type()
            && !self.flags.is_in_match_case()
            && header == DeclarationHeader::default();

        // parse arrow heads only when the token shape matches
        if can_parse_arrow_value && self.peek_is(TokenType::OpenParenthesis) {
            if let Some(function_id) = self.eat_simple_parenthesized_arrow(start, &header)? {
                return Ok(function_id);
            }

            if let Some(function_id) = self.eat_parenthesized_arrow(start, &header)? {
                return Ok(function_id);
            }
        } else if can_parse_arrow_value
            && self.peek_is(TokenType::Identifier)
            && matches!(self.next_token_type(), TokenType::ArrowWide)
            && let Some(function_id) = self.eat_identifier_arrow(start, &header)?
        {
            return Ok(function_id);
        }

        let function = self.eat_function_syntax(start, header)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Eat a function type expression.
    pub(crate) fn eat_function_type_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let function = self.eat_function_syntax(start, header)?;

        if function.signature.form == FunctionForm::Lambda && function.body.is_none() {
            return Ok(self.insert_function_type_expression(start, function));
        }

        Err(ParserError::unexpected(self.anchor_span_here()))
    }

    /// Eat one function before inserting a grammar-specific node.
    fn eat_function_syntax(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<ParsedFunction> {
        let head = self.eat_function_head(header)?;
        self.require_function_name(&head)?;

        let parameters = self.eat_function_parameters(start, &head)?;
        let return_part = self.eat_function_return(&head)?;
        let body = self.eat_function_body(&head)?;

        let parameter_span = parameters.parameter_span;
        let (this_parameter, parameters) = self.split_this_parameter_maybe(parameters.parameters);

        let asynchrony = if head.is_async {
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };
        let signature = FunctionSignature {
            asynchrony,
            form: head.form,
            phase: head.phase,
            role: head.role,
            generic_parameters: head.generic_parameters.unwrap_or_default(),
            where_clauses: return_part.where_clauses.unwrap_or_default(),
            this_parameter,
            parameters,
            return_type: return_part.return_type,
            is_abstract: head.header.is_abstract,
            is_override: false,
            is_generator: head.is_generator,
        };

        Ok(ParsedFunction {
            header: head.header,
            name: head.name,
            name_span: head.name_span,
            signature,
            body: body.body,
            generic_parameter_span: head.generic_parameter_span,
            parameter_span,
            return_type_span: return_part.return_type_span,
            body_span: body.body_span,
        })
    }

    /// Insert parsed function syntax as a function declaration.
    fn insert_function_declaration(
        &mut self,
        start: &ParserSpanStart,
        function: ParsedFunction,
    ) -> LocalNodeId<Declaration> {
        let function_id = self.insert_node(
            Declaration::Function(FunctionDeclaration {
                name: function.name,
                export: function.header.export,
                is_ambient: function.header.is_ambient,
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
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
        }

        // generic parameters
        if let Some(span) = function.generic_parameter_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        // parameters
        if let Some(span) = function.parameter_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                span,
            );
        }

        // body
        if let Some(span) = function.body_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Body),
                span,
            );
        }

        function_id
    }

    /// Insert parsed function syntax as a type expression.
    fn insert_function_type_expression(
        &mut self,
        start: &ParserSpanStart,
        function: ParsedFunction,
    ) -> LocalNodeId<TypeExpression> {
        debug_assert_eq!(function.signature.form, FunctionForm::Lambda);
        debug_assert!(function.body.is_none());

        // node
        let type_expression = match function.signature.role {
            Some(FunctionRole::New) => TypeExpression::Constructor(ConstructorType {
                is_abstract: function.signature.is_abstract,
                generic_parameters: function.signature.generic_parameters,
                where_clauses: function.signature.where_clauses,
                parameters: function.signature.parameters,
                return_type: function.signature.return_type,
            }),
            None => TypeExpression::Function(FunctionType {
                generic_parameters: function.signature.generic_parameters,
                where_clauses: function.signature.where_clauses,
                this_parameter: function.signature.this_parameter,
                parameters: function.signature.parameters,
                return_type: function.signature.return_type,
            }),
            _ => unreachable!("expected function or constructor type role"),
        };
        let type_expression_id = self.insert_node(type_expression, self.get_span_from(start));

        // return type
        if let Some(span) = function.return_type_span {
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
        }

        // generic parameters
        if let Some(span) = function.generic_parameter_span {
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        // parameters
        if let Some(span) = function.parameter_span {
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                span,
            );
        }

        type_expression_id
    }

    /// Eat an arrow body.
    fn eat_arrow_body(
        &mut self,
        body_start: &ParserSpanStart,
    ) -> ParserResult<(LocalNodeId<Expression>, Span)> {
        if self.is_block_start() {
            let mut flags = self
                .flags
                .in_statement_position()
                .in_before_block()
                .not_in_decorator();
            flags.set_allow_sequence_expression(true);
            let block_id =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;
            let body_span = self.get_span_from(body_start);
            let body = self.tree.insert(Expression::Block(block_id), body_span);

            Ok((body, body_span))
        } else {
            let mut flags = self.flags.in_before_block().not_in_decorator();
            flags.set_allow_sequence_expression(false);
            let body = self.eat_expression(flags)?;
            let body_span = self.get_span_from(body_start);

            Ok((body, body_span))
        }
    }

    /// Insert an arrow function declaration from parsed parameters and body.
    fn insert_arrow_declaration(
        &mut self,
        start: &ParserSpanStart,
        arrow: ParsedArrowFunction,
    ) -> LocalNodeId<Declaration> {
        let (this_parameter, parameters) = self.split_this_parameter_maybe(arrow.parameters);
        let signature = FunctionSignature {
            asynchrony: Asynchrony::Sync,
            role: None,
            form: FunctionForm::Lambda,
            phase: FunctionPhase::Normal,
            generic_parameters: vec![],
            where_clauses: vec![],
            this_parameter,
            parameters,
            return_type: arrow.return_type,
            is_abstract: false,
            is_override: false,
            is_generator: false,
        };
        let function_id = self.insert_node(
            Declaration::Function(FunctionDeclaration {
                name: None,
                export: arrow.header.export,
                is_ambient: arrow.header.is_ambient,
                signature,
                body: Some(arrow.body),
            }),
            self.get_span_from(start),
        );
        if let Some(span) = arrow.return_type_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
        }

        if let Some(span) = arrow.generic_parameter_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        if let Some(span) = arrow.parameter_span {
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                span,
            );
        }

        self.tree.set_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Body),
            arrow.body_span,
        );

        function_id
    }

    /// Scan a parenthesized arrow head without forcing a full pair lookup.
    fn scan_parenthesized_arrow_head(&mut self) -> Option<(ArrowHeadShape, TokenType)> {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return None;
        }

        self.lookahead(|parser| parser.scan_parenthesized_arrow_head_here())
    }

    /// Scan a parenthesized arrow head at the current open parenthesis.
    fn scan_parenthesized_arrow_head_here(&mut self) -> Option<(ArrowHeadShape, TokenType)> {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return None;
        }

        // track the head state
        let mut semantic_token_count = 0usize;
        let mut first_token_type = None;
        let mut second_token_type = None;
        let mut third_token_type = None;
        let mut has_parameter = false;
        let mut has_modifier = false;
        let mut has_type_annotation = false;
        let mut has_type_tokens = false;

        // track nested type annotation delimiters
        let mut depth = DelimiterDepth::default();

        // eat (
        self.bump();
        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End {
                return None;
            }

            // recover before rescanning later statements
            if self.current_semicolon_precedes_recovery_point(token_type, RecoveryPoint::Statement)
            {
                return None;
            }

            // top level close: finalize the head
            if token_type == TokenType::CloseParenthesis && depth.is_top_level() {
                break;
            }

            semantic_token_count += 1;

            match semantic_token_count {
                1 => first_token_type = Some(token_type),
                2 => second_token_type = Some(token_type),
                3 => third_token_type = Some(token_type),
                _ => {}
            }

            // require at most one named parameter
            if !has_parameter {
                if self.language.is_destack()
                    && (self.is_keyword(Keyword::Comptime)
                        || self.current_identifier_str_is("comptime"))
                {
                    has_modifier = true;
                    self.bump();
                    continue;
                }

                if token_type == TokenType::Identifier {
                    has_parameter = true;
                    self.bump();
                    continue;
                }

                return None;
            }

            // optionally allow one top level type annotation marker
            if !has_type_annotation {
                if token_type == TokenType::Colon {
                    has_type_annotation = true;
                    self.bump();
                    continue;
                }

                return None;
            }

            // reject additional top level parameters and defaults
            if depth.is_top_level() && matches!(token_type, TokenType::Comma | TokenType::Assign) {
                return None;
            }

            // track nested structures inside the type annotation
            if !depth.advance(token_type) {
                return None;
            }
            has_type_tokens = true;
            self.bump();
        }

        // common heads: (), (x), (x: T)
        let head_shape = if semantic_token_count == 0 {
            ArrowHeadShape::Empty
        } else if semantic_token_count == 1 && first_token_type == Some(TokenType::Identifier) {
            ArrowHeadShape::Named {
                has_type_annotation: false,
                has_modifier,
            }
        } else if semantic_token_count == 3
            && first_token_type == Some(TokenType::Identifier)
            && second_token_type == Some(TokenType::Colon)
            && matches!(
                third_token_type,
                Some(TokenType::Identifier | TokenType::Literal)
            )
        {
            ArrowHeadShape::Named {
                has_type_annotation: true,
                has_modifier,
            }
        } else if has_parameter {
            if has_type_annotation && (!has_type_tokens || !depth.is_top_level()) {
                return None;
            }

            ArrowHeadShape::Named {
                has_type_annotation,
                has_modifier,
            }
        } else {
            ArrowHeadShape::Empty
        };

        self.bump();
        let follow_token_type = self.peek_token_type();

        Some((head_shape, follow_token_type))
    }

    /// Eat a simple parenthesized arrow when present.
    ///
    /// Examples:
    /// ```ds
    /// () => value
    /// (value) => value
    /// (value: Type) => value
    /// ```
    fn eat_simple_parenthesized_arrow(
        &mut self,
        start: &ParserSpanStart,
        header: &DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        let Some((head_shape, follow_token_type)) = self.scan_parenthesized_arrow_head() else {
            return Ok(None);
        };

        // require an arrow or a return type marker after the group
        if !matches!(follow_token_type, TokenType::ArrowWide | TokenType::Colon) {
            return Ok(None);
        }

        // let the full parameter parser handle modifiers
        if matches!(
            head_shape,
            ArrowHeadShape::Named {
                has_modifier: true,
                ..
            }
        ) {
            return Ok(None);
        }

        // parse the parenthesized head
        let parameter_container_start = self.span_start();
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut parameters = Vec::with_capacity(1);
        if let ArrowHeadShape::Named {
            has_type_annotation,
            has_modifier: _,
        } = head_shape
        {
            let parameter_start = self.span_start();
            let (parameter_name, parameter_name_span) = self.eat_binding_identifier_with_span()?;
            let (parameter_type, parameter_type_span) = if has_type_annotation {
                let type_start = self.span_start();
                self.eat_token(TokenType::Colon)?;
                let mut type_flags = self.flags.not_in_position().in_type();
                if self.flags.is_in_type_conditional_right() {
                    type_flags = type_flags.in_type_conditional_right();
                }
                let parameter_type =
                    self.eat_type_expression_or_recover_missing(type_flags, NodeType::Parameter)?;
                let parameter_type_span = self.get_span_from(&type_start);
                (Some(parameter_type), Some(parameter_type_span))
            } else {
                (None, None)
            };

            let parameter_id = self.insert_node(
                Parameter::Named {
                    name: parameter_name,
                    is_optional: false,
                    is_comptime: false,
                    declared_type: parameter_type,
                    default: None,
                },
                self.get_span_from(&parameter_start),
            );
            self.tree.set_main_span(parameter_id, parameter_name_span);
            if let Some(span) = parameter_type_span {
                self.tree.set_side_span(
                    parameter_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
            }
            parameters.push(parameter_id);
        }
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;
        let parameter_container_span = Some(self.get_span_from(&parameter_container_start));

        let function_id =
            self.eat_arrow_tail(start, *header, parameters, parameter_container_span)?;

        Ok(Some(function_id))
    }

    /// Eat a full parameter-list arrow when present.
    ///
    /// Examples:
    /// ```ds
    /// (first, second) => first + second
    /// ({ value }) => value
    /// (...items) => items
    /// ```
    fn eat_parenthesized_arrow(
        &mut self,
        start: &ParserSpanStart,
        header: &DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        // require an arrow or return type marker after the parenthesized head
        let follow_token_type =
            if let Some((_, follow_token_type)) = self.scan_parenthesized_arrow_head() {
                follow_token_type
            } else if let Some(follow_token_type) = self.scan_parenthesized_modified_arrow_head() {
                follow_token_type
            } else {
                return Ok(None);
            };
        if !matches!(follow_token_type, TokenType::ArrowWide | TokenType::Colon) {
            return Ok(None);
        }

        // dynamic parameters
        let parameter_container_start = self.span_start();
        self.eat_token(TokenType::OpenParenthesis)?;
        let parameter_flags = self.flags.with_generator(false).with_forbid_yield(false);
        let parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            self.with_flags(parameter_flags, |parser| parser.eat_parameters_body())?
        };
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;
        let parameter_container_span = Some(self.get_span_from(&parameter_container_start));

        let function_id =
            self.eat_arrow_tail(start, *header, parameters, parameter_container_span)?;

        Ok(Some(function_id))
    }

    /// Scan a parenthesized arrow head that starts with a parameter modifier.
    fn scan_parenthesized_modified_arrow_head(&mut self) -> Option<TokenType> {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return None;
        }

        self.lookahead(|parser| parser.scan_parenthesized_modified_arrow_head_here())
    }

    /// Scan a parenthesized modified arrow head at the current open parenthesis.
    fn scan_parenthesized_modified_arrow_head_here(&mut self) -> Option<TokenType> {
        self.bump();

        let starts_modified_parameter = self.language.is_destack()
            && (self.is_keyword(Keyword::Comptime) || self.current_identifier_str_is("comptime"));
        if !starts_modified_parameter {
            return None;
        }

        let mut depth = DelimiterDepth::default();
        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End {
                return None;
            }

            if self.current_semicolon_precedes_recovery_point(token_type, RecoveryPoint::Statement)
            {
                return None;
            }

            if token_type == TokenType::CloseParenthesis && depth.is_top_level() {
                break;
            }

            if !depth.advance(token_type) {
                return None;
            }
            self.bump();
        }

        self.bump();

        Some(self.peek_token_type())
    }

    /// Eat an arrow return type and body.
    ///
    /// Examples:
    /// ```ds
    /// => value
    /// : string => value
    /// : asserts value is Ready => value
    /// ```
    fn eat_arrow_tail(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        parameters: Vec<LocalNodeId<Parameter>>,
        parameter_span: Option<Span>,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let (return_type, return_type_span) = self.eat_arrow_return_type()?;

        self.eat_arrow()?;
        let body_start = self.span_start();
        let (body, body_span) = self.eat_arrow_body(&body_start)?;

        Ok(self.insert_arrow_declaration(
            start,
            ParsedArrowFunction {
                header,
                parameters,
                generic_parameter_span: None,
                parameter_span,
                return_type,
                return_type_span,
                body,
                body_span,
            },
        ))
    }

    /// Eat an explicit arrow return type when present.
    ///
    /// Examples:
    /// ```ds
    /// : string
    /// : value is Ready
    /// : asserts value is Ready
    /// ```
    fn eat_arrow_return_type(
        &mut self,
    ) -> ParserResult<(Option<LocalNodeId<TypeExpression>>, Option<Span>)> {
        if !self.has_lambda_return_type_marker() {
            return Ok((None, None));
        }

        // marker
        let type_start = self.span_start();
        self.eat_token(TokenType::Colon)?;

        // type
        let flags = self.arrow_return_type_flags();
        let return_type =
            self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;
        let return_type_span = self.get_span_from(&type_start);

        Ok((Some(return_type), Some(return_type_span)))
    }

    /// Return parser flags for an arrow return type.
    fn arrow_return_type_flags(&self) -> ParserFlags {
        let mut flags = self.flags.nested().in_type();

        if self.flags.is_in_type_conditional_right() {
            flags = flags.in_type_conditional_right();
        }

        if self.flags.is_in_static() {
            flags = flags.in_static();
        }

        flags.in_arrow_return_type().allow_type_predicate()
    }

    /// Eat an identifier arrow when present.
    ///
    /// Examples:
    /// ```ds
    /// value => value
    /// async => async
    /// item => item.id
    /// ```
    fn eat_identifier_arrow(
        &mut self,
        start: &ParserSpanStart,
        header: &DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        // parse the single named parameter
        let parameter_name = self.eat_identifier()?;
        let parameter_span = self.get_span_from(start);
        let parameter_id = self.insert_node(
            Parameter::Named {
                name: parameter_name,
                is_optional: false,
                is_comptime: false,
                declared_type: None,
                default: None,
            },
            self.get_span_from(start),
        );

        let function_id =
            self.eat_arrow_tail(start, *header, vec![parameter_id], Some(parameter_span))?;

        Ok(Some(function_id))
    }

    /// Eat function modifiers, form, name, and generic parameters.
    fn eat_function_head(
        &mut self,
        mut header: DeclarationHeader,
    ) -> ParserResult<ParsedFunctionHead> {
        if self.is_keyword(Keyword::Abstract) && !header.is_abstract {
            self.bump();
            header.is_abstract = true;
        }

        let phase = self.eat_function_phase_modifier();
        let is_async = self.eat_function_async_modifier();
        let role = self.eat_function_role();
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;
        let (form, is_generator) = self.eat_function_form(is_generator)?;
        let (name, name_span) = self.eat_function_name(form)?;
        let (generic_parameters, generic_parameter_span) = self.eat_function_generics()?;

        Ok(ParsedFunctionHead {
            header,
            is_async,
            role,
            form,
            phase,
            is_generator,
            name,
            name_span,
            generic_parameters,
            generic_parameter_span,
        })
    }

    /// Eat a function phase marker.
    fn eat_function_phase_modifier(&mut self) -> FunctionPhase {
        if !self.is_keyword(Keyword::Comptime) {
            return FunctionPhase::Normal;
        }

        self.bump();

        FunctionPhase::Comptime
    }

    /// Eat a function async modifier when it is not a lambda parameter.
    fn eat_function_async_modifier(&mut self) -> bool {
        if !self.is_keyword(Keyword::Async) {
            return false;
        }

        if self.next_token_type() == TokenType::ArrowWide {
            return false;
        }

        self.bump();

        true
    }

    /// Eat one function role marker.
    fn eat_function_role(&mut self) -> Option<FunctionRole> {
        let starts_construct_signature = self.is_keyword(Keyword::New)
            && matches!(
                self.next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        if starts_construct_signature {
            self.bump();
            Some(FunctionRole::New)
        } else {
            None
        }
    }

    /// Eat one function form marker.
    fn eat_function_form(&mut self, is_generator: bool) -> ParserResult<(FunctionForm, bool)> {
        if self.is_keyword(Keyword::Function) {
            self.bump();
            let is_generator = is_generator || self.eat_token_maybe(TokenType::Multiply)?;

            Ok((FunctionForm::Function, is_generator))
        } else {
            Ok((FunctionForm::Lambda, is_generator))
        }
    }

    /// Eat a function name when the syntax owns one.
    fn eat_function_name(
        &mut self,
        form: FunctionForm,
    ) -> ParserResult<(Option<Name>, Option<Span>)> {
        if form != FunctionForm::Function {
            return Ok((None, None));
        }

        if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            Ok((Some(name), Some(span)))
        } else {
            Ok((None, None))
        }
    }

    /// Eat function generic parameters and their container span.
    fn eat_function_generics(
        &mut self,
    ) -> ParserResult<(Option<Vec<LocalNodeId<GenericParameter>>>, Option<Span>)> {
        let start = self.span_start();
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)
            .for_node_type(NodeType::Declaration)?;
        let span = generic_parameters
            .as_ref()
            .map(|_| self.get_span_from(&start));

        Ok((generic_parameters, span))
    }

    /// Require statement function declarations to have names.
    fn require_function_name(&mut self, head: &ParsedFunctionHead) -> ParserResult<()> {
        if head.form == FunctionForm::Function
            && self.flags.is_in_statement_position()
            && head.name.is_none()
            && head.header.export != Some(ExportKind::Default)
        {
            Err(ParserError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        } else {
            Ok(())
        }
    }

    /// Eat function parameters.
    fn eat_function_parameters(
        &mut self,
        start: &ParserSpanStart,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionParameters> {
        let has_parenthesized_parameters = head.form == FunctionForm::Function
            || self.flags.is_in_type()
            || self.peek_is(TokenType::OpenParenthesis)
            || self.next_token_type() == TokenType::OpenParenthesis;
        if has_parenthesized_parameters {
            self.eat_parenthesized_function_parameters(head)
        } else {
            self.eat_bare_function_parameter(start, head)
        }
    }

    /// Eat a parenthesized function parameter list.
    fn eat_parenthesized_function_parameters(
        &mut self,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionParameters> {
        let start = self.span_start();
        self.eat_token(TokenType::OpenParenthesis)?;

        let parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            let flags = self
                .flags
                .with_generator(head.is_generator)
                .with_forbid_yield(head.is_generator);
            self.with_flags(flags, |parser| parser.eat_parameters_body())?
        };

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;

        Ok(ParsedFunctionParameters {
            parameters,
            parameter_span: Some(self.get_span_from(&start)),
        })
    }

    /// Eat a single bare lambda parameter.
    fn eat_bare_function_parameter(
        &mut self,
        start: &ParserSpanStart,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionParameters> {
        if head.is_generator && self.is_keyword(Keyword::Yield) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        let name = self.eat_identifier()?;
        let parameter = Parameter::Named {
            name,
            is_optional: false,
            is_comptime: false,
            declared_type: None,
            default: None,
        };
        let parameter = self.insert_node(parameter, self.get_span_from(start));

        Ok(ParsedFunctionParameters {
            parameters: vec![parameter],
            parameter_span: None,
        })
    }

    /// Eat function return type syntax and where clauses.
    fn eat_function_return(
        &mut self,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionReturn> {
        if head.form == FunctionForm::Lambda && self.has_lambda_return_type_marker() {
            self.eat_lambda_return_type()
        } else if head.form == FunctionForm::Function || self.flags.is_in_type() {
            self.eat_regular_return_type()
        } else {
            Ok(ParsedFunctionReturn {
                return_type: None,
                return_type_span: None,
                where_clauses: None,
            })
        }
    }

    /// Eat a lambda return type.
    fn eat_lambda_return_type(&mut self) -> ParserResult<ParsedFunctionReturn> {
        let start = self.span_start();
        self.bump();

        let mut flags = self.flags.nested().in_type();
        if self.flags.is_in_type_conditional_right() {
            flags = flags.in_type_conditional_right();
        }
        if self.flags.is_in_static() {
            flags = flags.in_static();
        }
        flags = flags.allow_type_predicate();
        if !self.flags.is_in_type() {
            flags = flags.in_arrow_return_type();
        }

        let return_type =
            self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;
        let where_clauses = if self.language.is_destack() {
            self.eat_where_maybe()?
        } else {
            None
        };

        Ok(ParsedFunctionReturn {
            return_type: Some(return_type),
            return_type_span: Some(self.get_span_from(&start)),
            where_clauses,
        })
    }

    /// Eat a function return type.
    fn eat_regular_return_type(&mut self) -> ParserResult<ParsedFunctionReturn> {
        let (return_type, return_type_span) = if self.has_regular_return_type_marker() {
            let start = self.span_start();
            self.bump();

            let flags = self.function_return_type_flags();
            let return_type =
                self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;

            (Some(return_type), Some(self.get_span_from(&start)))
        } else {
            (None, None)
        };

        let where_clauses = if self.language.is_destack() {
            self.eat_where_maybe()?
        } else {
            None
        };

        Ok(ParsedFunctionReturn {
            return_type,
            return_type_span,
            where_clauses,
        })
    }

    /// Return whether a function return type marker is present.
    fn has_regular_return_type_marker(&mut self) -> bool {
        self.peek_arrow_is()
            || self.peek_colon_is()
            || self.current_token_is_on_new_line() && (self.peek_arrow_is() || self.peek_colon_is())
    }

    /// Build parser flags for a regular function return type.
    fn function_return_type_flags(&self) -> ParserFlags {
        let mut flags = self.flags.nested().in_type().in_before_block();
        if self.flags.is_in_type_conditional_right() {
            flags = flags.in_type_conditional_right();
        }
        if self.flags.is_in_static() {
            flags = flags.in_static();
        }

        flags.allow_type_predicate()
    }

    /// Eat a function body when the source form owns one.
    fn eat_function_body(&mut self, head: &ParsedFunctionHead) -> ParserResult<ParsedFunctionBody> {
        if head.form == FunctionForm::Function && self.peek_is(TokenType::OpenBrace) {
            let start = self.span_start();
            let flags = self.function_block_body_flags(head);
            let block_id =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;
            let span = self.get_span_from(&start);
            let body = self.tree.insert(Expression::Block(block_id), span);

            Ok(ParsedFunctionBody {
                body: Some(body),
                body_span: Some(span),
            })
        } else if head.form == FunctionForm::Lambda
            && !self.flags.is_in_type()
            && self.peek_arrow_is()
        {
            self.eat_lambda_body(head)
        } else {
            Ok(ParsedFunctionBody {
                body: None,
                body_span: None,
            })
        }
    }

    /// Eat a lambda body.
    fn eat_lambda_body(&mut self, head: &ParsedFunctionHead) -> ParserResult<ParsedFunctionBody> {
        self.eat_arrow()?;
        let start = self.span_start();
        let body = if self.is_block_start() {
            let flags = self.function_block_body_flags(head);
            let block_id =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;

            self.tree
                .insert(Expression::Block(block_id), self.get_span_from(&start))
        } else {
            let mut flags = self
                .flags
                .in_before_block()
                .not_in_decorator()
                .with_generator(head.is_generator);
            flags.set_allow_sequence_expression(false);
            flags.set_forbid_await(flags.is_forbid_await() && !head.is_async);

            self.eat_expression(flags)?
        };
        let span = self.get_span_from(&start);

        Ok(ParsedFunctionBody {
            body: Some(body),
            body_span: Some(span),
        })
    }

    /// Build parser flags for a function block body.
    fn function_block_body_flags(&self, head: &ParsedFunctionHead) -> ParserFlags {
        let mut flags = self
            .flags
            .in_statement_position()
            .in_before_block()
            .not_in_decorator()
            .with_generator(head.is_generator);
        flags.set_allow_sequence_expression(true);
        flags.set_forbid_await(flags.is_forbid_await() && !head.is_async);

        flags
    }

    /// Check whether a lambda return type marker is present.
    fn has_lambda_return_type_marker(&mut self) -> bool {
        // check for a colon return type
        let has_colon = self.peek_colon_is() || self.next_token_type() == TokenType::Colon;
        if has_colon {
            return true;
        }

        // arrow return types only apply in type positions
        if !self.flags.is_in_type() {
            return false;
        }

        self.peek_arrow_is() || self.current_token_is_on_new_line() && self.peek_arrow_is()
    }
}
