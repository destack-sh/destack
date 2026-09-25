use crate::parse::{ExpressionPosition, ExpressionStop};
use tspp_dir::{
    AssignOperator, AssignPattern, Asynchrony, Expression, FunctionRole, LocalNodeId, Name,
    NodeType, OperatorPrecedence, Property, TokenLiteral, TokenType,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

use crate::parse::member::{MemberHead, Method};
use crate::parse::{BindingPosition, DeclarationNesting};
use crate::{ParseStart, Parser, ParserError, ParserResult};

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse an object literal (including the surrounding braces).
    ///
    /// Examples:
    /// ```tspp
    /// { }
    /// { a: 1, b }
    /// { a(x): void }
    /// ```
    pub(crate) fn parse_object_literal(&mut self) -> ParserResult<Vec<LocalNodeId<Property>>> {
        self.eat_token(TokenType::OpenBrace)?;
        let properties = self.parse_object_properties()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;

        Ok(properties)
    }

    /// Parse one spread or embed value expression.
    #[inline]
    fn parse_property_value(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())
    }

    /// Insert the assignment expression used to preserve one property value with a default.
    fn insert_property_default_expression(
        &mut self,
        value: LocalNodeId<Expression>,
        default: LocalNodeId<Expression>,
        assign_operator_range: Option<ByteRange>,
    ) -> LocalNodeId<Expression> {
        let value_range = self.tree.get_range(value);
        let default_range = self.tree.get_range(default);
        let assign_range = ByteRange {
            start: value_range.start,
            end: default_range.end,
        };
        let left = self.insert_node(AssignPattern::Place { expression: value }, value_range);
        let assign_id = self.insert_node(
            Expression::Assign {
                left,
                operator: AssignOperator::Assign,
                right: default,
            },
            assign_range,
        );

        if let Some(assign_operator_range) = assign_operator_range {
            self.tree.set_main_range(assign_id, assign_operator_range);
        }

        assign_id
    }

    /// Parse an object literal body.
    ///
    /// Examples:
    /// ```tspp
    /// key: value
    /// key
    /// ...other
    /// ```
    pub(crate) fn parse_object_properties(&mut self) -> ParserResult<Vec<LocalNodeId<Property>>> {
        let mut properties = Vec::new();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on object close
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                break;
            }

            // stop at declaration recovery boundaries
            if token_type == TokenType::Semicolon {
                if self.peek_semicolon_declaration_boundary(DeclarationNesting::None) {
                    break;
                }

                self.eat_any_stop()?;
                continue;
            }

            // skip separators
            if token_type == TokenType::Comma {
                self.eat_token(TokenType::Comma)?;
                continue;
            }

            let property = self.parse_object_property_or_recover();
            properties.push(property);
        }

        Ok(properties)
    }

    /// Parse one object property, recovering malformed input as an error node.
    fn parse_object_property_or_recover(&mut self) -> LocalNodeId<Property> {
        let documentation = self.parse_documentation();
        let property_id = match self.parse_object_property() {
            Ok(property_id) => property_id,
            Err(error) => {
                let error = error.in_node(NodeType::Property);
                let recovered_range = self.recover_body(error.range(), error);

                self.insert_node(Property::Error, recovered_range)
            }
        };
        if !matches!(self.tree.get(property_id), Property::Error) {
            self.attach_documentation(property_id, documentation);
        }

        property_id
    }

    /// Parse one object literal property.
    ///
    /// Examples:
    /// ```tspp
    /// key: value
    /// key = fallback
    /// method() {}
    /// ```
    fn parse_object_property(&mut self) -> ParserResult<LocalNodeId<Property>> {
        let start = self.mark_parse_start();

        if self.peek_simple_object_property() {
            let property_id = self.parse_simple_object_property(&start)?;

            return Ok(property_id);
        }

        self.parse_property()
    }

    /// Return whether the current object property can use the simple field parser.
    fn peek_simple_object_property(&self) -> bool {
        let token_type = self.peek_token_type();
        if token_type == TokenType::Spread {
            return true;
        }

        if !self.is_simple_object_name_start(token_type) {
            return false;
        }

        let peek_next_token_type = self.peek_next_token_type();
        matches!(
            peek_next_token_type,
            TokenType::Colon | TokenType::Assign | TokenType::Comma | TokenType::CloseBrace
        ) || Self::is_any_stop_token(peek_next_token_type)
    }

    /// Return whether one token starts a simple object literal name.
    fn is_simple_object_name_start(&self, token_type: TokenType) -> bool {
        match token_type {
            TokenType::Identifier => true,
            TokenType::Literal => matches!(
                self.peek_token().literal(),
                Some(
                    TokenLiteral::String {
                        is_terminated: true,
                        has_invalid_escape: false,
                    } | TokenLiteral::Int { .. }
                        | TokenLiteral::Float { .. }
                )
            ),
            _ => false,
        }
    }

    /// Parse the simple object literal property forms.
    ///
    /// Examples:
    /// ```tspp
    /// key: value
    /// key
    /// key = fallback
    /// ```
    fn parse_simple_object_property(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<LocalNodeId<Property>> {
        if self.peek_is(TokenType::Spread) {
            self.bump();
            let value = self.parse_property_value()?;
            let property = Property::Spread { value };

            return Ok(self.insert_node(property, self.range_since(start)));
        }

        let (name, name_range) = self.eat_property_name_with_range()?;

        if self.peek_is(TokenType::Colon) {
            return self.parse_simple_object_colon_field(start, name, name_range);
        }

        if self.peek_is(TokenType::Assign) {
            return self.parse_simple_object_default_field(start, name, name_range);
        }

        if self.peek_shorthand_object_property_end() {
            return self.parse_simple_object_shorthand_field(start, name, name_range);
        }

        Err(ParserError::unexpected(self.peek_token_span()))
    }

    /// Parse one simple `key: value` object field.
    ///
    /// Examples:
    /// ```tspp
    /// key: value
    /// "key": call()
    /// 0: first
    /// ```
    fn parse_simple_object_colon_field(
        &mut self,
        start: &ParseStart,
        name: Name,
        name_range: ByteRange,
    ) -> ParserResult<LocalNodeId<Property>> {
        let type_start = self.mark_parse_start();
        self.bump();

        let value = self.parse_simple_object_field_value()?;
        let property_id = self.insert_node(
            Property::Field {
                name,
                value,
                is_shorthand: false,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(property_id, name_range);
        self.tree.set_side_range(
            property_id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.range_since(&type_start),
        );

        Ok(property_id)
    }

    /// Parse one simple `key = fallback` object field.
    ///
    /// Examples:
    /// ```tspp
    /// key = fallback
    /// value = call()
    /// item = defaultItem
    /// ```
    fn parse_simple_object_default_field(
        &mut self,
        start: &ParseStart,
        name: Name,
        name_range: ByteRange,
    ) -> ParserResult<LocalNodeId<Property>> {
        let Name::Identifier(identifier) = name else {
            return Err(ParserError::unexpected(name_range));
        };

        let assign_start = self.mark_parse_start();
        self.bump();
        let default = self.parse_simple_object_field_value()?;
        let value = self.insert_node(Expression::Identifier { name: identifier }, name_range);
        self.tree.set_main_range(value, name_range);
        let value = self.insert_property_default_expression(
            value,
            default,
            Some(self.range_since(&assign_start)),
        );
        let property_id = self.insert_node(
            Property::Field {
                name,
                value,
                is_shorthand: true,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(property_id, name_range);

        Ok(property_id)
    }

    /// Parse one simple shorthand object field.
    ///
    /// Examples:
    /// ```tspp
    /// key
    /// value
    /// item
    /// ```
    fn parse_simple_object_shorthand_field(
        &mut self,
        start: &ParseStart,
        name: Name,
        name_range: ByteRange,
    ) -> ParserResult<LocalNodeId<Property>> {
        let Name::Identifier(identifier) = name else {
            return Err(ParserError::unexpected(name_range));
        };

        let value = self.insert_node(Expression::Identifier { name: identifier }, name_range);
        self.tree.set_main_range(value, name_range);
        let property_id = self.insert_node(
            Property::Field {
                name,
                value,
                is_shorthand: true,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(property_id, name_range);

        Ok(property_id)
    }

    /// Return whether the current token ends a shorthand object property.
    fn peek_shorthand_object_property_end(&self) -> bool {
        let token_type = self.peek_token_type();

        token_type == TokenType::Comma
            || token_type == TokenType::CloseBrace
            || Self::is_any_stop_token(token_type)
    }

    /// Parse one simple object field value.
    ///
    /// Examples:
    /// ```tspp
    /// value
    /// call()
    /// condition ? yes : no
    /// ```
    fn parse_simple_object_field_value(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let token_type = self.peek_token_type();
        if token_type == TokenType::Assign
            || token_type == TokenType::Comma
            || token_type == TokenType::CloseBrace
            || Self::is_any_stop_token(token_type)
        {
            return Ok(self.recover_missing_expression_here(NodeType::Property));
        }

        self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())
    }

    /// Parse a property.
    ///
    /// Examples:
    /// ```tspp
    /// // field
    /// x: int32
    /// x
    /// a: T
    /// a?: T
    /// private b: int32 = 4
    /// public static c: int32 = 4
    ///
    /// // method
    /// foo()
    /// <T>(): T
    /// get x(): int32
    /// set x(value: int32): void
    /// private static foo(): void
    /// ```
    pub(crate) fn parse_property(&mut self) -> ParserResult<LocalNodeId<Property>> {
        let start = self.mark_parse_start();

        // spread property
        if self.peek_is(TokenType::Spread) {
            let start = self.mark_parse_start();
            self.bump();
            let value = self.parse_property_value()?;
            let property = Property::Spread { value };
            return Ok(self.insert_node(property, self.range_since(&start)));
        }

        // modifiers and head
        let modifiers = self.parse_binding_modifiers(BindingPosition::Property);
        let MemberHead {
            modifiers,
            name,
            name_range,
            role,
            role_range: _,
            function_modifiers,
            is_method,
            associated_const_name,
        } = self.parse_member_head(modifiers, None)?;

        // object fields cannot start with an unnamed call signature
        if name.is_none()
            && role.is_none()
            && function_modifiers.asynchrony == Asynchrony::Sync
            && !function_modifiers.is_generator
            && self.peek_is(TokenType::OpenParenthesis)
        {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // modifiers without a name or call signature are invalid
        if name.is_none() && !modifiers.is_empty() && !is_method {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // unnamed field separators are invalid
        if name.is_none() && (self.peek_is(TokenType::Colon) || self.peek_is(TokenType::Assign)) {
            return Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::Identifier,
            ));
        }

        // getters and setters require method form
        if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && !is_method {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        if is_method {
            // associated consts cannot use method form
            if associated_const_name.is_some() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }
            if modifiers.is_const_block {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            // abstraction
            // methods without a name or role are implicit calls
            let role = if name.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
            };
            if modifiers.is_virtual
                && matches!(role, Some(FunctionRole::Constructor | FunctionRole::New))
            {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            // modifiers postfix (again after parameters)
            let modifiers = self.parse_postfix_binding_modifier(modifiers);
            let Method {
                signature,
                body,
                generic_parameter_range: _,
                parameter_range: _,
                return_type_range,
            } = self.parse_method(
                NodeType::Property,
                role,
                function_modifiers,
                modifiers.is_abstract,
                modifiers.is_override,
                true,
            )?;

            // method property
            let property = Property::Method {
                name,
                signature,
                body,
            };
            let property_id = self.insert_node(property, self.range_since(&start));

            // set the main source range to the name
            if let Some(range) = name_range {
                self.tree.set_main_range(property_id, range);
            }

            // set the type source range for return type annotation
            if let Some(range) = return_type_range {
                self.tree.set_side_range(
                    property_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    range,
                );
            }

            Ok(property_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, type_range): (Option<LocalNodeId<Expression>>, Option<ByteRange>) =
                if self.peek_is(TokenType::Colon) {
                    let type_start = self.mark_parse_start();
                    self.bump();

                    let value = if self.peek_is(TokenType::Assign)
                        || self.peek_is(TokenType::Comma)
                        || self.peek_is(TokenType::CloseBrace)
                        || self.peek_any_stop()
                    {
                        self.recover_missing_expression_here(NodeType::Property)
                    } else {
                        self.parse_expression_at(
                            ExpressionPosition::Value,
                            ExpressionStop::default(),
                            OperatorPrecedence::Assignment,
                        )?
                    };
                    let type_range = self.range_since(&type_start);
                    (Some(value), Some(type_range))
                } else {
                    (None, None)
                };

            // default
            let (default, assign_operator_span) = if self.peek_is(TokenType::Assign) {
                let assign_start = self.mark_parse_start();
                self.bump();

                // keep associated const defaults in expression mode
                let default = if self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.peek_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?
                };
                (Some(default), Some(self.range_since(&assign_start)))
            } else {
                (None, None)
            };

            // cover initialized shorthand fields as assignment expressions
            let is_defaulted_shorthand = value.is_none()
                && default.is_some()
                && modifiers.is_empty()
                && matches!(name, Some(Name::Identifier(_)));

            // preserve both the declared type and the default
            let value = match (value, default) {
                (Some(value), Some(default)) => Some(self.insert_property_default_expression(
                    value,
                    default,
                    assign_operator_span,
                )),
                (None, Some(default)) if is_defaulted_shorthand => {
                    let Some((Name::Identifier(identifier), name_range)) = name.zip(name_range)
                    else {
                        return Err(ParserError::unexpected(self.range_since(&start)));
                    };

                    let value =
                        self.insert_node(Expression::Identifier { name: identifier }, name_range);
                    self.tree.set_main_range(value, name_range);

                    Some(self.insert_property_default_expression(
                        value,
                        default,
                        assign_operator_span,
                    ))
                }
                (Some(value), None) => Some(value),
                (None, Some(default)) => Some(default),
                (None, None) => None,
            };

            // shorthand field value
            let is_bare_shorthand = value.is_none()
                && default.is_none()
                && modifiers.is_empty()
                && matches!(name, Some(Name::Identifier(_)));
            let is_shorthand = is_bare_shorthand || is_defaulted_shorthand;

            let value = if is_bare_shorthand {
                match name {
                    Some(Name::Identifier(identifier)) => {
                        let Some(name_range) = name_range else {
                            return Err(ParserError::unexpected(self.range_since(&start)));
                        };
                        let value = self
                            .insert_node(Expression::Identifier { name: identifier }, name_range);
                        self.tree.set_main_range(value, name_range);

                        Some(value)
                    }
                    _ => value,
                }
            } else {
                value
            };

            // unnamed empty heads are not properties
            if modifiers.is_empty() && name.is_none() && value.is_none() && default.is_none() {
                return Err(ParserError::expected(
                    self.peek_token().range(),
                    TokenType::Identifier,
                ));
            }

            // field construction requires a name
            let Some(name) = name else {
                return Err(ParserError::expected(
                    self.peek_token().range(),
                    TokenType::Identifier,
                ));
            };

            let Some(value) = value else {
                return Err(ParserError::expected(
                    self.peek_token_span(),
                    TokenType::Colon,
                ));
            };

            let property = Property::Field {
                name,
                value,
                is_shorthand,
            };
            let property_id = self.insert_node(property, self.range_since(&start));

            // set the main source range to the name
            if let Some(range) = name_range {
                self.tree.set_main_range(property_id, range);
            }

            // set the type source range for field type annotation
            if let Some(range) = type_range {
                self.tree.set_side_range(
                    property_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    range,
                );
            }

            Ok(property_id)
        }
    }
}
