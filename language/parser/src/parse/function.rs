use crate::parse::DeclarationHeader;
use crate::parse::error::ParserResultExt;
use crate::parse::flags::ParserFlags;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Asynchrony, BlockContext, ConstructorType, Declaration, Expression, FunctionDeclaration,
    FunctionForm, FunctionPhase, FunctionRole, FunctionSignature, FunctionTypeExpression,
    GenericParameter, Keyword, LocalNodeId, Name, NodeType, Parameter, TokenType, TypeExpression,
    WhereClause,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// The parsed head shape that precedes a possible arrow tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArrowHeadKind {
    /// A parenthesized identifier that could also be a ternary condition value.
    ParenthesizedIdentifier,
    /// Any arrow head where a following colon can only start a return type.
    ParameterList,
}

impl ArrowHeadKind {
    /// Return whether a colon may start an arrow return type after this head.
    pub(crate) const fn allows_return_type_colon(self, flags: ParserFlags) -> bool {
        !matches!(self, Self::ParenthesizedIdentifier) || !flags.is_in_ternary_condition()
    }
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

impl Parser {
    /// Eat a function or lambda declaration.
    ///
    /// Examples:
    /// ```ds
    /// function parse<T>(value: T): T {
    ///     return value;
    /// }
    /// (value: int32): int32 => value
    /// ```
    pub(crate) fn eat_function(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let function = self.eat_function_syntax(header)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Eat a function type expression.
    pub(crate) fn eat_function_type_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let function = self.eat_function_syntax(header)?;

        if function.signature.form == FunctionForm::Lambda && function.body.is_none() {
            return self.insert_function_type_expression(start, function);
        }

        Err(ParserError::unexpected(self.anchor_span_here()))
    }

    /// Eat one function before inserting a grammar-specific node.
    fn eat_function_syntax(&mut self, header: DeclarationHeader) -> ParserResult<ParsedFunction> {
        let head = self.eat_function_head(header)?;
        self.require_function_name(&head)?;

        let parameters = self.eat_function_parameters(&head)?;
        let return_part = self.eat_function_return(&head)?;
        let body = self.eat_function_body(&head)?;

        let parameter_span = parameters.parameter_span;
        let (this_form, this_parameter, parameters) =
            self.split_this_parameter_maybe(parameters.parameters);

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
            this_form,
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
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
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
            None => TypeExpression::Function(FunctionTypeExpression {
                generic_parameters: function.signature.generic_parameters,
                where_clauses: function.signature.where_clauses,
                this_form: function.signature.this_form,
                this_parameter: function.signature.this_parameter,
                parameters: function.signature.parameters,
                return_type: function.signature.return_type,
            }),
            _ => return Err(ParserError::unexpected(self.anchor_span_here())),
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

        Ok(type_expression_id)
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
        let is_generator = self.eat_token_if(TokenType::Multiply);
        let (form, is_generator) = self.eat_function_form(is_generator);
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
    fn eat_function_form(&mut self, is_generator: bool) -> (FunctionForm, bool) {
        if self.is_keyword(Keyword::Function) {
            self.bump();
            let is_generator = is_generator || self.eat_token_if(TokenType::Multiply);

            (FunctionForm::Function, is_generator)
        } else {
            (FunctionForm::Lambda, is_generator)
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

    /// Require function forms to have names.
    fn require_function_name(&self, head: &ParsedFunctionHead) -> ParserResult<()> {
        if head.form == FunctionForm::Function && head.name.is_none() {
            Err(ParserError::expected(self.peek(), TokenType::Identifier))
        } else {
            Ok(())
        }
    }

    /// Eat function parameters.
    fn eat_function_parameters(
        &mut self,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionParameters> {
        let has_parenthesized_parameters = head.form == FunctionForm::Function
            || self.flags.is_in_type()
            || self.peek_is(TokenType::OpenParenthesis);
        if has_parenthesized_parameters {
            self.eat_parenthesized_function_parameters(head)
        } else {
            self.eat_bare_function_parameter(head)
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
                .with_forbid_yield(head.is_generator)
                .with_forbid_await(head.is_async);
            self.with_flags(flags, |parser| parser.eat_parameters_body())?
        };

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        );

        Ok(ParsedFunctionParameters {
            parameters,
            parameter_span: Some(self.get_span_from(&start)),
        })
    }

    /// Eat a single bare lambda parameter.
    fn eat_bare_function_parameter(
        &mut self,
        head: &ParsedFunctionHead,
    ) -> ParserResult<ParsedFunctionParameters> {
        if head.is_generator && self.is_keyword(Keyword::Yield) {
            return Err(ParserError::unexpected(self.peek()));
        }

        let start = self.span_start();
        let name = self.eat_identifier()?;
        let parameter = Parameter::Named {
            name,
            is_optional: false,
            is_comptime: false,
            declared_type: None,
            default: None,
        };
        let parameter_span = self.get_span_from(&start);
        let parameter = self.insert_node(parameter, parameter_span);

        Ok(ParsedFunctionParameters {
            parameters: vec![parameter],
            parameter_span: Some(parameter_span),
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
        if !self.flags.is_in_type() {
            flags = flags.in_arrow_return_type();
        }

        let return_type =
            self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;
        let where_clauses = self.eat_where_maybe()?;

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

        let where_clauses = self.eat_where_maybe()?;

        Ok(ParsedFunctionReturn {
            return_type,
            return_type_span,
            where_clauses,
        })
    }

    /// Return whether a function return type marker is present.
    fn has_regular_return_type_marker(&mut self) -> bool {
        self.peek_arrow_is() || self.peek_colon_is()
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

        flags
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
        flags.set_forbid_await(flags.is_forbid_await() && !head.is_async);

        flags
    }

    /// Check whether a lambda return type marker is present.
    fn has_lambda_return_type_marker(&mut self) -> bool {
        // value arrows use colon return types
        if self.peek_colon_is() {
            return true;
        }

        // arrow return types only apply in type positions
        if !self.flags.is_in_type() {
            return false;
        }

        self.peek_arrow_is()
    }
}
