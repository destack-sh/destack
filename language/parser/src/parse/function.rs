use crate::parse::DeclarationHeader;
use crate::parse::error::ParserResultExt;
use crate::parse::flags::ParserFlags;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_core::StringId;
use destack_dir::{
    Asynchrony, BlockContext, ConstructorType, Declaration, Expression, FunctionDeclaration,
    FunctionForm, FunctionPhase, FunctionRole, FunctionSignature, FunctionTypeExpression,
    GenericParameter, Keyword, LocalNodeId, Name, NodeType, Parameter, TokenType, TypeExpression,
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

impl ParsedFunction {
    /// Create empty parsed function state for one declaration header.
    #[inline]
    fn new(header: DeclarationHeader) -> Self {
        Self {
            header,
            name: None,
            name_span: None,
            signature: FunctionSignature {
                asynchrony: Asynchrony::Sync,
                role: None,
                form: FunctionForm::Lambda,
                phase: FunctionPhase::Normal,
                generic_parameters: Vec::new(),
                where_clauses: Vec::new(),
                this_form: None,
                this_parameter: None,
                parameters: Vec::new(),
                return_type: None,
                is_abstract: header.is_abstract,
                is_override: false,
                is_generator: false,
            },
            body: None,
            generic_parameter_span: None,
            parameter_span: None,
            return_type_span: None,
            body_span: None,
        }
    }
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
    #[inline(never)]
    pub(crate) fn eat_function(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let mut function = ParsedFunction::new(header);
        self.eat_function_syntax(&mut function)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Eat a function type expression.
    pub(crate) fn eat_function_type_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut function = ParsedFunction::new(header);
        self.eat_function_syntax(&mut function)?;

        if function.signature.form == FunctionForm::Lambda && function.body.is_none() {
            return self.insert_function_type_expression(start, function);
        }

        Err(ParserError::unexpected(self.anchor_span_here()))
    }

    /// Eat one function before inserting a grammar-specific node.
    #[inline(never)]
    fn eat_function_syntax(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        self.eat_function_head(function)?;
        self.require_function_name(function)?;
        self.eat_function_parameters(function)?;
        self.eat_function_tail(function)
    }

    /// Eat function syntax after its head and parameters are known.
    #[inline(never)]
    fn eat_function_tail(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        self.eat_function_return(function)?;
        self.eat_function_body(function)
    }

    /// Eat a bare lambda after its identifier parameter has been consumed.
    #[inline(never)]
    pub(crate) fn eat_bare_lambda(
        &mut self,
        start: &ParserSpanStart,
        parameter_name: StringId,
        parameter_span: Span,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let parameter = Parameter::Named {
            name: parameter_name,
            is_optional: false,
            is_comptime: false,
            declared_type: None,
            default: None,
        };
        let parameter = self.insert_node(parameter, parameter_span);
        let mut function = ParsedFunction::new(header);
        function.signature.parameters.push(parameter);
        function.parameter_span = Some(parameter_span);
        self.eat_function_tail(&mut function)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Insert parsed function syntax as a function declaration.
    #[inline(never)]
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
    #[inline(never)]
    fn eat_function_head(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // absorb an explicit abstract modifier
        if self.is_keyword(Keyword::Abstract) && !function.header.is_abstract {
            self.bump();
            function.header.is_abstract = true;
        }

        // parse the function head components
        let phase = self.eat_function_phase_modifier();
        let is_async = self.eat_function_async_modifier();
        let role = self.eat_function_role();
        let is_generator = self.eat_token_if(TokenType::Multiply);
        let (form, is_generator) = self.eat_function_form(is_generator);
        let (name, name_span) = self.eat_function_name(form)?;
        let (generic_parameters, generic_parameter_span) = self.eat_function_generics()?;

        // record the parsed head
        function.name = name;
        function.name_span = name_span;
        function.signature.asynchrony = if is_async {
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };
        function.signature.role = role;
        function.signature.form = form;
        function.signature.phase = phase;
        function.signature.generic_parameters = generic_parameters.unwrap_or_default();
        function.signature.is_abstract = function.header.is_abstract;
        function.signature.is_generator = is_generator;
        function.generic_parameter_span = generic_parameter_span;

        Ok(())
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
    fn require_function_name(&self, function: &ParsedFunction) -> ParserResult<()> {
        if function.signature.form == FunctionForm::Function && function.name.is_none() {
            Err(ParserError::expected(self.peek(), TokenType::Identifier))
        } else {
            Ok(())
        }
    }

    /// Eat function parameters.
    #[inline(never)]
    fn eat_function_parameters(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // select the parameter grammar from the function form and source position
        let has_parenthesized_parameters = function.signature.form == FunctionForm::Function
            || self.flags.is_in_type()
            || self.peek_is(TokenType::OpenParenthesis);
        if has_parenthesized_parameters {
            self.eat_parenthesized_function_parameters(function)
        } else {
            self.eat_bare_function_parameter(function)
        }
    }

    /// Eat a parenthesized function parameter list.
    fn eat_parenthesized_function_parameters(
        &mut self,
        function: &mut ParsedFunction,
    ) -> ParserResult<()> {
        // parse the parameter list
        let start = self.span_start();
        self.eat_token(TokenType::OpenParenthesis)?;

        let parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            let flags = self
                .flags
                .with_generator(function.signature.is_generator)
                .with_forbid_yield(function.signature.is_generator)
                .with_forbid_await(function.signature.asynchrony == Asynchrony::Async);
            self.with_flags(flags, |parser| parser.eat_parameters_body())?
        };

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        );

        // separate the receiver parameter and record the list
        let (this_form, this_parameter, parameters) = self.split_this_parameter_maybe(parameters);
        function.signature.this_form = this_form;
        function.signature.this_parameter = this_parameter;
        function.signature.parameters = parameters;
        function.parameter_span = Some(self.get_span_from(&start));

        Ok(())
    }

    /// Eat a single bare lambda parameter.
    fn eat_bare_function_parameter(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // reject a reserved generator parameter
        if function.signature.is_generator && self.is_keyword(Keyword::Yield) {
            return Err(ParserError::unexpected(self.peek()));
        }

        // build the single named parameter
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

        function.signature.parameters.push(parameter);
        function.parameter_span = Some(parameter_span);

        Ok(())
    }

    /// Eat function return type syntax and where clauses.
    #[inline(never)]
    fn eat_function_return(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // select return syntax from the function form and source position
        if function.signature.form == FunctionForm::Lambda && self.has_lambda_return_type_marker() {
            self.eat_lambda_return_type(function)
        } else if function.signature.form == FunctionForm::Function || self.flags.is_in_type() {
            self.eat_regular_return_type(function)
        } else {
            Ok(())
        }
    }

    /// Eat a lambda return type.
    fn eat_lambda_return_type(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // consume the return marker
        let start = self.span_start();
        self.bump();

        // build the nested type parser flags
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

        // parse and record the return clauses
        let return_type =
            self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;
        let where_clauses = self.eat_where_maybe()?;

        function.signature.return_type = Some(return_type);
        function.signature.where_clauses = where_clauses.unwrap_or_default();
        function.return_type_span = Some(self.get_span_from(&start));

        Ok(())
    }

    /// Eat a function return type.
    fn eat_regular_return_type(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // parse an explicit return type
        if self.has_regular_return_type_marker() {
            let start = self.span_start();
            self.bump();

            let flags = self.function_return_type_flags();
            let return_type =
                self.eat_type_expression_or_recover_missing(flags, NodeType::Declaration)?;

            function.signature.return_type = Some(return_type);
            function.return_type_span = Some(self.get_span_from(&start));
        }

        // parse trailing where clauses
        let where_clauses = self.eat_where_maybe()?;
        function.signature.where_clauses = where_clauses.unwrap_or_default();

        Ok(())
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
    #[inline(never)]
    fn eat_function_body(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // parse a declared function block
        if function.signature.form == FunctionForm::Function && self.peek_is(TokenType::OpenBrace) {
            let start = self.span_start();
            let flags = self.function_block_body_flags(function);
            let block_id =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;
            let span = self.get_span_from(&start);
            let body = self.tree.insert(Expression::Block(block_id), span);

            function.body = Some(body);
            function.body_span = Some(span);

            Ok(())
        }
        // parse a lambda body
        else if function.signature.form == FunctionForm::Lambda
            && !self.flags.is_in_type()
            && self.peek_arrow_is()
        {
            self.eat_lambda_body(function)
        } else {
            Ok(())
        }
    }

    /// Eat a lambda body.
    fn eat_lambda_body(&mut self, function: &mut ParsedFunction) -> ParserResult<()> {
        // consume the lambda arrow
        self.eat_arrow()?;

        // parse the block or expression body
        let start = self.span_start();
        let body = if self.is_block_start() {
            let flags = self.function_block_body_flags(function);
            let block_id =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;

            self.tree
                .insert(Expression::Block(block_id), self.get_span_from(&start))
        } else {
            let mut flags = self
                .flags
                .in_before_block()
                .not_in_decorator()
                .with_generator(function.signature.is_generator);
            let is_async = function.signature.asynchrony == Asynchrony::Async;
            flags.set_forbid_await(flags.is_forbid_await() && !is_async);

            self.eat_expression(flags)?
        };
        let span = self.get_span_from(&start);

        // record the parsed body
        function.body = Some(body);
        function.body_span = Some(span);

        Ok(())
    }

    /// Build parser flags for a function block body.
    fn function_block_body_flags(&self, function: &ParsedFunction) -> ParserFlags {
        let mut flags = self
            .flags
            .in_statement_position()
            .in_before_block()
            .not_in_decorator()
            .with_generator(function.signature.is_generator);
        let is_async = function.signature.asynchrony == Asynchrony::Async;
        flags.set_forbid_await(flags.is_forbid_await() && !is_async);

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
