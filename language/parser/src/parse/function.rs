use crate::parse::error::ParserResultExt;
use crate::parse::{
    DeclarationHeader, DeclarationNesting, ExpressionPosition, ExpressionStop, TypePosition,
    TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use tspp_core::StringId;
use tspp_dir::{
    Asynchrony, BlockContext, ConstructorType, Declaration, Expression, FunctionDeclaration,
    FunctionForm, FunctionPhase, FunctionRole, FunctionSignature, FunctionTypeExpression,
    GenericParameter, Keyword, LocalNodeId, Name, NodeType, Parameter, TokenType, TypeExpression,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// The interpretation of `yield` in one function position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum YieldKeyword {
    /// Treat `yield` as an identifier.
    #[default]
    Identifier,
    /// Parse `yield` as a generator expression.
    Expression,
    /// Reject `yield` in this position.
    Forbidden,
}

/// The interpretation of `await` in one function position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum AwaitKeyword {
    /// Parse `await` as an asynchronous expression.
    #[default]
    Expression,
    /// Reject `await` in this position.
    Forbidden,
}

/// The lexical interpretation of function-sensitive keywords.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct FunctionKeywords {
    /// The active `yield` interpretation.
    pub(crate) yield_keyword: YieldKeyword,
    /// The active `await` interpretation.
    pub(crate) await_keyword: AwaitKeyword,
}

/// The source modifiers that control one function.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct FunctionModifiers {
    /// The function asynchrony.
    pub(crate) asynchrony: Asynchrony,
    /// Whether the function is a generator.
    pub(crate) is_generator: bool,
}

impl FunctionKeywords {
    /// Return the keyword interpretation active in parameter initializers.
    pub(crate) fn parameters(self, modifiers: FunctionModifiers) -> Self {
        Self {
            yield_keyword: if modifiers.is_generator {
                YieldKeyword::Forbidden
            } else {
                self.yield_keyword
            },
            await_keyword: if modifiers.asynchrony == Asynchrony::Async {
                AwaitKeyword::Forbidden
            } else {
                self.await_keyword
            },
        }
    }

    /// Return the keyword interpretation active in a function body.
    pub(crate) fn body(self, modifiers: FunctionModifiers) -> Self {
        Self {
            yield_keyword: if modifiers.is_generator {
                YieldKeyword::Expression
            } else {
                YieldKeyword::Identifier
            },
            await_keyword: if modifiers.asynchrony == Asynchrony::Async {
                AwaitKeyword::Expression
            } else {
                AwaitKeyword::Forbidden
            },
        }
    }
}

/// One function name and its source range.
struct FunctionName {
    /// The function name.
    name: Name,
    /// The name source range.
    range: ByteRange,
}

/// The namespace containing one function.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum FunctionSpace {
    /// Value space.
    Value,
    /// Type space.
    Type,
}

/// One function accumulated during parsing.
struct Function {
    /// The namespace containing the function.
    space: FunctionSpace,
    /// The expression position surrounding the function.
    position: ExpressionPosition,
    /// The declaration header.
    header: DeclarationHeader,
    /// The optional function name.
    name: Option<FunctionName>,
    /// The function signature.
    signature: FunctionSignature,
    /// The optional function body.
    body: Option<LocalNodeId<Expression>>,
    /// The generic parameter container range.
    generic_parameter_range: Option<ByteRange>,
    /// The parameter container range.
    parameter_range: Option<ByteRange>,
    /// The return type range.
    return_type_range: Option<ByteRange>,
    /// The body container range.
    body_range: Option<ByteRange>,
}

impl Function {
    /// Create one function for a declaration header.
    #[inline]
    fn new(header: DeclarationHeader, space: FunctionSpace, position: ExpressionPosition) -> Self {
        Self {
            space,
            position,
            header,
            name: None,
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
            generic_parameter_range: None,
            parameter_range: None,
            return_type_range: None,
            body_range: None,
        }
    }

    /// Return this function's source modifiers.
    fn modifiers(&self) -> FunctionModifiers {
        FunctionModifiers {
            asynchrony: self.signature.asynchrony,
            is_generator: self.signature.is_generator,
        }
    }
}

impl Parser {
    /// Parse a function or lambda declaration.
    ///
    /// Examples:
    /// ```tspp
    /// function parse<T>(value: T): T {
    ///     return value;
    /// }
    /// (value: int32): int32 => value
    /// ```
    #[inline(never)]
    pub(crate) fn parse_function(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let mut function = Function::new(header, FunctionSpace::Value, position);
        self.parse_function_head(&mut function)?;
        self.require_function_name(&function)?;
        self.parse_function_parameters(&mut function)?;
        self.parse_function_return(&mut function)?;
        self.parse_function_body(&mut function)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Parse a function type expression.
    pub(crate) fn parse_function_type(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut function = Function::new(header, FunctionSpace::Type, ExpressionPosition::Value);
        self.parse_function_head(&mut function)?;
        self.require_function_name(&function)?;
        self.parse_function_parameters(&mut function)?;
        self.parse_function_return(&mut function)?;
        self.parse_function_body(&mut function)?;

        if function.signature.form == FunctionForm::Lambda && function.body.is_none() {
            return self.insert_function_type_expression(start, function);
        }

        Err(ParserError::unexpected(self.peek_token().range()))
    }

    /// Parse a bare lambda after its identifier parameter has been consumed.
    #[inline(never)]
    pub(crate) fn parse_bare_lambda(
        &mut self,
        start: &ParseStart,
        parameter_name: StringId,
        parameter_range: ByteRange,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let parameter = Parameter::Named {
            name: parameter_name,
            is_optional: false,
            declared_type: None,
            default: None,
        };
        let parameter = self.insert_node(parameter, parameter_range);
        let mut function = Function::new(header, FunctionSpace::Value, ExpressionPosition::Value);
        function.signature.parameters.push(parameter);
        function.parameter_range = Some(parameter_range);
        self.parse_function_return(&mut function)?;
        self.parse_function_body(&mut function)?;

        Ok(self.insert_function_declaration(start, function))
    }

    /// Insert one declaration from a complete function.
    #[inline(never)]
    fn insert_function_declaration(
        &mut self,
        start: &ParseStart,
        function: Function,
    ) -> LocalNodeId<Declaration> {
        let name_range = function.name.as_ref().map(|name| name.range);
        let name = function.name.map(|name| name.name);
        let function_id = self.insert_node(
            Declaration::Function(FunctionDeclaration {
                name,
                export: function.header.export,
                is_ambient: function.header.is_ambient,
                signature: function.signature,
                body: function.body,
            }),
            self.range_since(start),
        );

        // name
        if let Some(range) = name_range {
            self.tree.set_main_range(function_id, range);
        }

        // return type
        if let Some(range) = function.return_type_range {
            self.tree.set_side_range(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                range,
            );
        }

        // generic parameters
        if let Some(range) = function.generic_parameter_range {
            self.tree.set_side_range(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        // parameters
        if let Some(range) = function.parameter_range {
            self.tree.set_side_range(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                range,
            );
        }

        // body
        if let Some(range) = function.body_range {
            self.tree.set_side_range(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Body),
                range,
            );
        }

        function_id
    }

    /// Insert one type expression from a complete function.
    fn insert_function_type_expression(
        &mut self,
        start: &ParseStart,
        function: Function,
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
            _ => return Err(ParserError::unexpected(self.peek_token().range())),
        };
        let type_expression_id = self.insert_node(type_expression, self.range_since(start));

        // return type
        if let Some(range) = function.return_type_range {
            self.tree.set_side_range(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                range,
            );
        }

        // generic parameters
        if let Some(range) = function.generic_parameter_range {
            self.tree.set_side_range(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        // parameters
        if let Some(range) = function.parameter_range {
            self.tree.set_side_range(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                range,
            );
        }

        Ok(type_expression_id)
    }

    /// Parse function modifiers, form, name, and generic parameters.
    #[inline(never)]
    fn parse_function_head(&mut self, function: &mut Function) -> ParserResult<()> {
        // absorb an explicit abstract modifier
        if self.peek_is_keyword(Keyword::Abstract) && !function.header.is_abstract {
            self.bump();
            function.header.is_abstract = true;
        }

        // parse the function head components
        let phase = self.parse_function_phase();
        let asynchrony = self.parse_function_asynchrony();
        let role = self.parse_function_role();
        let mut is_generator = self.eat_token_if(TokenType::Multiply);
        let form = if self.peek_is_keyword(Keyword::Function) {
            self.bump();
            is_generator |= self.eat_token_if(TokenType::Multiply);
            FunctionForm::Function
        } else {
            FunctionForm::Lambda
        };
        let name = self.parse_function_name(form)?;
        let function_modifiers = FunctionModifiers {
            asynchrony,
            is_generator,
        };
        let parameter_keywords = self.keywords.parameters(function_modifiers);
        let (generic_parameters, generic_parameter_range) = self
            .with_keywords(parameter_keywords, |parser| {
                parser.parse_function_generics()
            })?;

        // record the function head
        function.name = name;
        function.signature.asynchrony = asynchrony;
        function.signature.role = role;
        function.signature.form = form;
        function.signature.phase = phase;
        function.signature.generic_parameters = generic_parameters;
        function.signature.is_abstract = function.header.is_abstract;
        function.signature.is_generator = is_generator;
        function.generic_parameter_range = generic_parameter_range;

        Ok(())
    }

    /// Parse a function phase marker.
    fn parse_function_phase(&mut self) -> FunctionPhase {
        // const marks the phase only directly before the function noun
        let is_phase_marker = self.peek_is_keyword(Keyword::Const)
            && self.peek_next_keyword() == Some(Keyword::Function);
        if !is_phase_marker {
            return FunctionPhase::Normal;
        }

        self.bump();

        FunctionPhase::Const
    }

    /// Parse an async modifier when it is not a lambda parameter.
    fn parse_function_asynchrony(&mut self) -> Asynchrony {
        if !self.peek_is_keyword(Keyword::Async) {
            return Asynchrony::Sync;
        }

        if self.peek_next_token_type() == TokenType::ArrowWide {
            return Asynchrony::Sync;
        }

        self.bump();

        Asynchrony::Async
    }

    /// Parse one function role marker.
    fn parse_function_role(&mut self) -> Option<FunctionRole> {
        let is_construct_signature = self.peek_is_keyword(Keyword::New)
            && matches!(
                self.peek_next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        if is_construct_signature {
            self.bump();
            Some(FunctionRole::New)
        } else {
            None
        }
    }

    /// Parse a function name when the form permits one.
    fn parse_function_name(&mut self, form: FunctionForm) -> ParserResult<Option<FunctionName>> {
        if form != FunctionForm::Function {
            return Ok(None);
        }

        if let Some((name, range)) = self.eat_name_with_range_if_present()? {
            Ok(Some(FunctionName { name, range }))
        } else {
            Ok(None)
        }
    }

    /// Parse function generic parameters and their container range.
    fn parse_function_generics(
        &mut self,
    ) -> ParserResult<(Vec<LocalNodeId<GenericParameter>>, Option<ByteRange>)> {
        let start = self.mark_parse_start();
        let generic_parameters = self
            .parse_generic_parameters_if_present(false)
            .in_node(NodeType::Declaration)?;
        let range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&start));

        Ok((generic_parameters.unwrap_or_default(), range))
    }

    /// Require function forms to have names.
    fn require_function_name(&self, function: &Function) -> ParserResult<()> {
        if function.signature.form == FunctionForm::Function && function.name.is_none() {
            Err(ParserError::expected(
                self.peek_token_span(),
                TokenType::Identifier,
            ))
        } else {
            Ok(())
        }
    }

    /// Parse function parameters.
    #[inline(never)]
    fn parse_function_parameters(&mut self, function: &mut Function) -> ParserResult<()> {
        // select the parameter form from the function form and source position
        let has_parenthesized_parameters = function.signature.form == FunctionForm::Function
            || function.space == FunctionSpace::Type
            || self.peek_is(TokenType::OpenParenthesis);
        if has_parenthesized_parameters {
            self.parse_parenthesized_function_parameters(function)
        } else {
            self.parse_bare_function_parameter(function)
        }
    }

    /// Parse a parenthesized function parameter list.
    fn parse_parenthesized_function_parameters(
        &mut self,
        function: &mut Function,
    ) -> ParserResult<()> {
        // recover an absent parameter list before the next declaration
        if self.peek_declaration_boundary(DeclarationNesting::None) {
            self.report_expected_here(TokenType::OpenParenthesis, NodeType::Declaration);

            return Ok(());
        }

        // parse the parameter list
        let start = self.mark_parse_start();
        self.eat_token(TokenType::OpenParenthesis)?;

        let parameters = if self.peek_is(TokenType::CloseParenthesis) {
            vec![]
        } else {
            let parameter_keywords = self.keywords.parameters(function.modifiers());
            self.with_keywords(parameter_keywords, |parser| {
                parser.parse_parameter_list_body(function.position)
            })?
        };

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        );

        // separate the receiver parameter and record the list
        let (this_form, this_parameter, parameters) = self.split_this_parameter(parameters);
        function.signature.this_form = this_form;
        function.signature.this_parameter = this_parameter;
        function.signature.parameters = parameters;
        function.parameter_range = Some(self.range_since(&start));

        Ok(())
    }

    /// Parse a single bare lambda parameter.
    fn parse_bare_function_parameter(&mut self, function: &mut Function) -> ParserResult<()> {
        // reject a reserved generator parameter
        if function.signature.is_generator && self.peek_is_keyword(Keyword::Yield) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // insert the single named parameter
        let start = self.mark_parse_start();
        let name = self.eat_identifier()?;
        let parameter = Parameter::Named {
            name,
            is_optional: false,
            declared_type: None,
            default: None,
        };
        let parameter_range = self.range_since(&start);
        let parameter = self.insert_node(parameter, parameter_range);

        function.signature.parameters.push(parameter);
        function.parameter_range = Some(parameter_range);

        Ok(())
    }

    /// Parse a function return type and where clauses.
    #[inline(never)]
    fn parse_function_return(&mut self, function: &mut Function) -> ParserResult<()> {
        // select the return marker from the function form and source position
        if function.signature.form == FunctionForm::Lambda
            && self.peek_lambda_return_type_marker(function.space)
        {
            self.parse_lambda_return_type(function)
        } else if function.signature.form == FunctionForm::Function
            || function.space == FunctionSpace::Type
        {
            self.parse_regular_return_type(function)
        } else {
            Ok(())
        }
    }

    /// Parse a lambda return type.
    fn parse_lambda_return_type(&mut self, function: &mut Function) -> ParserResult<()> {
        // consume the return marker
        let start = self.mark_parse_start();
        self.bump();

        let position = if function.space == FunctionSpace::Type {
            TypePosition::Type
        } else {
            TypePosition::ArrowReturn
        };
        let parameter_keywords = self.keywords.parameters(function.modifiers());
        let (return_type, where_clauses) = self.with_keywords(parameter_keywords, |parser| {
            let return_type = parser.parse_type_or_recover_missing(
                position,
                TypeStop::default(),
                NodeType::Declaration,
            )?;
            let where_clauses = parser.parse_where_clauses()?;

            Ok((return_type, where_clauses))
        })?;

        function.signature.return_type = Some(return_type);
        function.signature.where_clauses = where_clauses;
        function.return_type_range = Some(self.range_since(&start));

        Ok(())
    }

    /// Parse a function return type.
    fn parse_regular_return_type(&mut self, function: &mut Function) -> ParserResult<()> {
        // parse an explicit return type
        if self.peek_regular_return_type_marker() {
            let start = self.mark_parse_start();
            self.bump();

            let parameter_keywords = self.keywords.parameters(function.modifiers());
            let return_type = self.with_keywords(parameter_keywords, |parser| {
                parser.parse_type_or_recover_missing(
                    TypePosition::Type,
                    TypeStop::default(),
                    NodeType::Declaration,
                )
            })?;

            function.signature.return_type = Some(return_type);
            function.return_type_range = Some(self.range_since(&start));
        }

        // parse trailing where clauses
        let parameter_keywords = self.keywords.parameters(function.modifiers());
        let where_clauses =
            self.with_keywords(parameter_keywords, |parser| parser.parse_where_clauses())?;
        function.signature.where_clauses = where_clauses;

        Ok(())
    }

    /// Return whether a function return type marker is present.
    fn peek_regular_return_type_marker(&self) -> bool {
        self.peek_is(TokenType::ArrowWide) || self.peek_is(TokenType::Colon)
    }

    /// Parse a function body when the source form owns one.
    #[inline(never)]
    fn parse_function_body(&mut self, function: &mut Function) -> ParserResult<()> {
        // parse a declared function block
        if function.signature.form == FunctionForm::Function && self.peek_is(TokenType::OpenBrace) {
            let start = self.mark_parse_start();
            let body_keywords = self.keywords.body(function.modifiers());
            let block_id = self.with_keywords(body_keywords, |parser| {
                parser.parse_block(BlockContext::Expression)
            })?;
            let range = self.range_since(&start);
            let body = self.insert_node(Expression::Block(block_id), range);

            function.body = Some(body);
            function.body_range = Some(range);

            Ok(())
        }
        // parse a lambda body
        else if function.signature.form == FunctionForm::Lambda
            && function.space == FunctionSpace::Value
            && self.peek_is(TokenType::ArrowWide)
        {
            self.parse_lambda_body(function)
        } else {
            Ok(())
        }
    }

    /// Parse a lambda body.
    fn parse_lambda_body(&mut self, function: &mut Function) -> ParserResult<()> {
        // consume the lambda arrow
        self.eat_token(TokenType::ArrowWide)?;

        // parse the block or expression body
        let start = self.mark_parse_start();
        let body_keywords = self.keywords.body(function.modifiers());
        let body = self.with_keywords(body_keywords, |parser| {
            if parser.peek_block() {
                let block_id = parser.parse_block(BlockContext::Expression)?;

                Ok(parser.insert_node(Expression::Block(block_id), parser.range_since(&start)))
            } else {
                parser.parse_expression(ExpressionPosition::Value, ExpressionStop::default())
            }
        })?;
        let range = self.range_since(&start);

        // record the function body
        function.body = Some(body);
        function.body_range = Some(range);

        Ok(())
    }

    /// Check whether a lambda return type marker is present.
    fn peek_lambda_return_type_marker(&self, space: FunctionSpace) -> bool {
        // value arrows use colon return types
        if self.peek_is(TokenType::Colon) {
            return true;
        }

        // arrow return types only apply in type positions
        if space == FunctionSpace::Value {
            return false;
        }

        self.peek_is(TokenType::ArrowWide)
    }
}
