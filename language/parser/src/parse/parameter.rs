use crate::parse::error::ParserResultExt;
use tspp_dir::{
    Expression, Keyword, LocalNodeId, NodeType, Parameter, Pattern, StringId, ThisForm, TokenType,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

use crate::parse::{
    AwaitKeyword, BindingPosition, DeclarationNesting, ExpressionPosition, ExpressionStop,
    TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};

/// One parameter pattern or named binding.
enum ParameterBinding {
    /// One destructuring pattern.
    Pattern(LocalNodeId<Pattern>),
    /// One named binding and its source range.
    Named { name: StringId, range: ByteRange },
}

impl ParameterBinding {
    /// Return the named binding range when present.
    const fn name_range(&self) -> Option<ByteRange> {
        match self {
            Self::Named { range, .. } => Some(*range),
            Self::Pattern(_) => None,
        }
    }
}

impl Parser {
    /// Parse one standalone parameter fragment.
    ///
    /// Examples:
    /// ```tspp
    /// value: string = "default"
    /// ```
    pub fn parse_parameter_fragment(&mut self) -> ParserResult<LocalNodeId<Parameter>> {
        self.parse_parameter(ExpressionPosition::Value)
    }

    /// Parse one standalone parenthesized parameter list.
    ///
    /// Examples:
    /// ```tspp
    /// (left: int, right: int = 0)
    /// ```
    pub fn parse_parameter_list_fragment(&mut self) -> ParserResult<Vec<LocalNodeId<Parameter>>> {
        self.parse_dynamic_parameters(ExpressionPosition::Value)
    }

    /// Return true when the current keyword should end a malformed parameter list after a newline.
    fn peek_parameter_recovery_boundary(&self, position: ExpressionPosition) -> bool {
        // only statement scoped dynamic parameters should stop at newline led keywords
        if !position.is_in_statement() {
            return false;
        }

        // only newline led keywords can start a following statement
        if !self.peek_is_on_new_line() {
            return false;
        }

        // release declarations from the incomplete parameter list
        if self.peek_declaration_boundary(DeclarationNesting::None) {
            return true;
        }

        // parameter heads win when followed by a parameter continuation
        if matches!(
            self.peek_next_token_type(),
            TokenType::Assign
                | TokenType::CloseParenthesis
                | TokenType::Colon
                | TokenType::Comma
                | TokenType::Maybe
        ) {
            return false;
        }

        // control statements
        let Some(keyword) = self.peek_keyword() else {
            return false;
        };

        matches!(
            keyword,
            Keyword::Break
                | Keyword::Continue
                | Keyword::Do
                | Keyword::For
                | Keyword::If
                | Keyword::Import
                | Keyword::Match
                | Keyword::Return
                | Keyword::Switch
                | Keyword::Try
                | Keyword::While
        )
    }

    /// Parse a parameter default value expression.
    #[inline]
    pub(crate) fn parse_parameter_default(
        &mut self,
        position: ExpressionPosition,
        stops: ExpressionStop,
        owner: NodeType,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut keywords = self.keywords;
        keywords.await_keyword = AwaitKeyword::Forbidden;

        self.with_keywords(keywords, |parser| {
            parser.parse_expression_or_recover_missing(position.nested(), stops, owner)
        })
    }

    /// Parse one parameter.
    ///
    /// Examples:
    /// ```tspp
    /// x
    /// T
    /// x: int32
    /// Validate: bool = false
    /// baz: @someMacro(T)
    /// _
    /// { x }
    /// { x }: MyType = Foo
    /// ...T
    /// ...args: int32[]
    /// ```
    pub(crate) fn parse_parameter(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Parameter>> {
        // parse parameter prefixes
        let documentation = self.parse_documentation();
        let decorators = self.parse_decorators();
        let start = self.mark_parse_start();

        // receiver shorthand
        if let Some(parameter) = self.parse_this_parameter(&start)? {
            self.attach_documentation(parameter, documentation);
            self.attach_decorators(parameter.id, decorators);

            return Ok(parameter);
        }

        // modifiers
        let mut modifiers = self.parse_binding_modifiers(BindingPosition::Parameter);

        // variadic
        let is_variadic = if self.peek_is(TokenType::Spread) {
            self.bump();
            true
        } else {
            false
        };

        // pattern/name
        let binding = self.parse_parameter_binding()?;
        let name_range = binding.name_range();
        // ? maybe
        if self.peek_is(TokenType::Maybe) {
            self.bump();
            modifiers.is_optional = true;
        }

        // type annotation marker
        let (declared_type, ty_range) = {
            // type markers can start on the next line
            let annotation_token_type = self.peek_token_type();
            let has_type_annotation_marker = annotation_token_type == TokenType::Colon;

            if has_type_annotation_marker {
                let type_start = self.mark_parse_start();
                self.bump();
                let declared_type = if self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseParenthesis)
                    || self.peek_type_angle_close()
                    || self.peek_is(TokenType::End)
                {
                    self.recover_missing_type_expression_here(NodeType::Parameter)
                } else {
                    self.parse_type_or_recover_missing(
                        TypePosition::Type,
                        TypeStop::default(),
                        NodeType::Parameter,
                    )?
                };
                let type_range = self.range_since(&type_start);
                (Some(declared_type), Some(type_range))
            } else {
                (None, None)
            }
        };

        if modifiers.visibility.is_some() || modifiers.is_readonly {
            self.report_error(ParserError::unexpected(self.range_since(&start)));
        }

        let is_optional = modifiers.is_optional;

        // = value
        let parameter = {
            let has_default_assign = self.peek_is(TokenType::Assign);
            if !is_variadic && has_default_assign {
                self.bump();

                // value
                let value = self
                    .parse_parameter_default(
                        position,
                        ExpressionStop::default(),
                        NodeType::Parameter,
                    )
                    .in_node(NodeType::Parameter)?;

                // named with default
                match binding {
                    ParameterBinding::Named { name, .. } => Parameter::Named {
                        name,
                        is_optional,
                        declared_type,
                        default: Some(value),
                    },
                    ParameterBinding::Pattern(pattern) => Parameter::Pattern {
                        pattern,
                        is_optional,
                        declared_type,
                        default: Some(value),
                    },
                }
            }
            // variadic parameter (cannot have a default value)
            else if is_variadic {
                match binding {
                    ParameterBinding::Named { name, .. } => Parameter::VariadicNamed {
                        name,
                        declared_type,
                    },
                    ParameterBinding::Pattern(pattern) => Parameter::VariadicPattern {
                        pattern,
                        declared_type,
                    },
                }
            }
            // no default value
            else {
                // named without default
                match binding {
                    ParameterBinding::Named { name, .. } => Parameter::Named {
                        name,
                        is_optional,
                        declared_type,
                        default: None,
                    },
                    ParameterBinding::Pattern(pattern) => Parameter::Pattern {
                        pattern,
                        is_optional,
                        declared_type,
                        default: None,
                    },
                }
            }
        };

        // parameter
        let parameter_id = self.insert_node(parameter, self.range_since(&start));

        // set the name identifier as the main source range
        if let Some(range) = name_range {
            self.tree.set_main_range(parameter_id, range);
        }

        // set the type annotation source range
        if let Some(range) = ty_range {
            self.tree.set_side_range(
                parameter_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                range,
            );
        }

        // attach decorators to the parameter node
        self.attach_documentation(parameter_id, documentation);
        self.attach_decorators(parameter_id.id, decorators);

        Ok(parameter_id)
    }

    /// Parse a receiver shorthand parameter when present.
    fn parse_this_parameter(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<Option<LocalNodeId<Parameter>>> {
        let Some(name_range) = self.this_parameter_range() else {
            return Ok(None);
        };

        let this_id = self.strings.intern("this");
        let declared_type = self.parse_type_or_recover_missing(
            TypePosition::Type,
            TypeStop::default(),
            NodeType::Parameter,
        )?;
        let parameter_id = self.insert_node(
            Parameter::Named {
                name: this_id,
                declared_type: Some(declared_type),
                default: None,
                is_optional: false,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(parameter_id, name_range);

        Ok(Some(parameter_id))
    }

    /// Return the `this` token range for a receiver shorthand.
    fn this_parameter_range(&self) -> Option<ByteRange> {
        // this
        if self.peek_keyword() == Some(Keyword::This) {
            return (self.peek_next_token_type() != TokenType::Colon)
                .then(|| self.peek_token().range());
        }

        // readonly this
        if self.peek_keyword() == Some(Keyword::Readonly) {
            return (self.peek_next_keyword() == Some(Keyword::This))
                .then(|| self.peek_token_at(1).range());
        }

        // reference shorthand
        if !matches!(
            self.peek_token_type(),
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor
        ) {
            return None;
        }

        // skip the optional borrow lifetime
        let is_borrowed = self.peek_is(TokenType::ElementwiseAnd);
        let mut offset = 1;
        if is_borrowed && self.peek_token_type_at(offset) == TokenType::Lifetime {
            offset += 1;
        }

        // leave qualifier validation to the type parser
        loop {
            match self.peek_keyword_at(offset) {
                Some(Keyword::Readonly | Keyword::Const) => offset += 1,
                Some(Keyword::Immutable | Keyword::Exclusive) if is_borrowed => offset += 1,
                _ => break,
            }
        }

        (self.peek_keyword_at(offset) == Some(Keyword::This))
            .then(|| self.peek_token_at(offset).range())
    }

    /// Parse one parameter pattern or named binding.
    fn parse_parameter_binding(&mut self) -> ParserResult<ParameterBinding> {
        // pattern
        if matches!(
            self.peek_token_type(),
            TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::OpenBrace
        ) || self.peek_identifier_is("_")
        {
            let pattern = self.parse_pattern()?;
            return Ok(ParameterBinding::Pattern(pattern));
        }

        // name
        let (name, name_range) = self.eat_binding_identifier_with_range()?;

        Ok(ParameterBinding::Named {
            name,
            range: name_range,
        })
    }

    /// Parse one parameter list.
    /// Parameters may be comma or newline separated.
    ///
    /// Examples:
    /// ```tspp
    /// x: int32
    /// x: int32, y: int32
    /// x: int32
    /// y: int32
    /// ```
    pub(crate) fn parse_parameter_list_body(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Parameter>>> {
        let mut parameters: Vec<LocalNodeId<Parameter>> = Vec::new();
        while self.has_more_tokens() {
            if self.peek_is(TokenType::CloseParenthesis) || self.peek_type_angle_close() {
                break;
            }

            // newline led statement keywords should stay outside malformed parameter lists
            if self.peek_parameter_recovery_boundary(position) {
                break;
            }

            // eat one parameter
            let parameter_start = self.mark_parse_start();
            let mut is_recovered_parameter = false;
            let parameter = self.parse_parameter(position).and_then(|parameter| {
                let has_cast_tail =
                    matches!(self.peek_keyword(), Some(Keyword::As | Keyword::Satisfies));
                if has_cast_tail {
                    Err(ParserError::unexpected(self.peek_token_span()))
                } else {
                    Ok(parameter)
                }
            });
            let parameter = match parameter.in_node(NodeType::Parameter) {
                Ok(parameter) => parameter,
                Err(error) => {
                    is_recovered_parameter = true;
                    let recover_at_statement_keyword =
                        self.peek_parameter_recovery_boundary(position);

                    // newline led keyword statements should stay outside malformed parameter lists
                    if recover_at_statement_keyword {
                        self.report_error(error);
                    } else {
                        self.recover_list_item(
                            self.range_since(&parameter_start),
                            TokenType::CloseParenthesis,
                            error,
                        );
                    }

                    self.insert_node(Parameter::Error, self.range_since(&parameter_start))
                }
            };

            parameters.push(parameter);

            // continue regular parameter lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_token(TokenType::Comma)?;

                if !is_recovered_parameter {
                    continue;
                }

                if self.peek_parameter_recovery_boundary(position) {
                    break;
                }

                if !self.peek_recovered_list_continuation(TokenType::CloseParenthesis) {
                    break;
                }

                continue;
            }
            // stop recovered lists before keyword boundaries
            let recovered_parameter_hits_boundary = is_recovered_parameter
                && (self.peek_parameter_recovery_boundary(position)
                    || !self.peek_recovered_list_continuation(TokenType::CloseParenthesis));

            // require a separator between adjacent parameter heads
            let adjacent_parameter_heads_without_separator = !self.peek_is_on_new_line();
            if recovered_parameter_hits_boundary || adjacent_parameter_heads_without_separator {
                break;
            }
        }
        Ok(parameters)
    }

    /// Parse dynamic parameters, including the `(` and `)` tokens.
    pub(crate) fn parse_dynamic_parameters(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Parameter>>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty dynamic parameters
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump();
            return Ok(vec![]);
        }

        // regular dynamic parameters
        let parameters = self.parse_parameter_list_body(position)?;
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        );
        Ok(parameters)
    }

    /// Split out an explicit `this` parameter (type-only).
    pub(crate) fn split_this_parameter(
        &mut self,
        mut parameters: Vec<LocalNodeId<Parameter>>,
    ) -> (
        Option<ThisForm>,
        Option<LocalNodeId<Parameter>>,
        Vec<LocalNodeId<Parameter>>,
    ) {
        // only the first parameter can be `this`
        let this_parameter = if let Some(first_id) = parameters.first().copied() {
            if let Parameter::Named { name, .. } = self.tree.get(first_id) {
                let this_id = self.strings.intern("this");
                if *name == this_id {
                    Some(first_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if this_parameter.is_some() {
            parameters.remove(0);
        }

        let this_form = this_parameter.map(|this_parameter| {
            if self.is_implicit_this_parameter(this_parameter) {
                ThisForm::Implicit
            } else {
                ThisForm::Explicit
            }
        });

        (this_form, this_parameter, parameters)
    }

    /// Return whether one receiver parameter came from shorthand syntax.
    fn is_implicit_this_parameter(&self, this_parameter: LocalNodeId<Parameter>) -> bool {
        let type_region = NodeSpanType::Region(NodeSpanRegion::Type);

        self.tree
            .get_side_range(this_parameter, type_region)
            .is_none()
    }
}
