use crate::parse::parser::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserCheckpoint, ParserSpanStart};

use super::operator::{ParseInfixOperator, TypeBinaryOperator, TypeUnaryOperator};
use destack_ast::{
    Argument, AssignOperator, AssignPattern, AssignPatternField, BinaryOperator, Declaration,
    Expression, FunctionDeclaration, FunctionKind, GenericArgument, IfCondition, IfKind, Key,
    Keyword, LiteralType, LocalNodeId, Name, NodeType, PostfixPosition, Property, TokenType,
    TypeExpression, UnaryOperator,
};
use destack_source::{Span, StringId};

const VALUE_TERNARY_PRECEDENCE: u16 = 900;

/// One current token considered for continuation parsing.
#[derive(Clone, Copy, Debug)]
struct ContinuationToken {
    /// The token type.
    token_type: TokenType,
    /// Whether a line break exists before the token.
    has_line_break_before: bool,
}

/// The postfix grammar space for continuation scanning.
#[derive(Clone, Copy, Debug)]
enum PostfixSpace {
    /// Value-space postfix parsing.
    Value,
    /// Type-space postfix parsing.
    Type,
}

/// The right-hand grammar form owned by one infix operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InfixRightKind {
    /// Parse a normal value expression on the right.
    Value,
    /// Parse a type assertion target for `as` or `satisfies`.
    Assertion,
    /// Parse a type predicate target for `value is T`.
    ValuePredicate,
    /// Parse a type operator right side for `A | B`, `A & B`, or `value is T` in type space.
    TypeOperator,
    /// Parse a conditional type right side for `T extends U ? X : Y`.
    TypeConditional,
    /// Reject one type-only operator that cannot lower in value space.
    InvalidValueTypeOperator,
}

/// Whether assignment target lowering may produce defaulted targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AssignPatternDefaultMode {
    /// Reject assignment expressions in the target position.
    Reject,
    /// Convert assignment expressions into defaulted assignment targets.
    Allow,
}

impl InfixRightKind {
    /// Classify one infix operator against the current left-hand grammar space.
    fn new(
        operator: ParseInfixOperator,
        left_is_type_expression: bool,
        is_in_before_block: bool,
        allows_type_predicate: bool,
    ) -> Self {
        match operator {
            ParseInfixOperator::As | ParseInfixOperator::Satisfies => Self::Assertion,
            ParseInfixOperator::Is if !left_is_type_expression => Self::ValuePredicate,
            ParseInfixOperator::Is if allows_type_predicate => Self::TypeOperator,
            ParseInfixOperator::Is => Self::InvalidValueTypeOperator,
            ParseInfixOperator::Binary(
                BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ) if left_is_type_expression => Self::TypeOperator,
            ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                if left_is_type_expression || is_in_before_block =>
            {
                Self::TypeConditional
            }
            ParseInfixOperator::TypeBinary(_) => Self::InvalidValueTypeOperator,
            _ => Self::Value,
        }
    }

    /// Return whether the right side parses in type space.
    fn parses_type_expression(self) -> bool {
        !matches!(self, Self::Value)
    }

    /// Return whether parenthesized value state must stay active on the right.
    fn preserves_parenthesis(self) -> bool {
        matches!(self, Self::Assertion | Self::ValuePredicate)
    }

    /// Return whether conditional-type right-side boundaries stay active.
    fn keeps_type_conditional_boundary(self) -> bool {
        matches!(
            self,
            Self::Assertion | Self::ValuePredicate | Self::TypeConditional
        )
    }
}

impl Parser {
    /// Eat one postfix `as comptime` operator and return its span when present.
    fn eat_as_comptime_postfix_operator_maybe(&mut self) -> ParseResult<Option<Span>> {
        let Some(operator) = self.peek_type_unary_postfix_operator_maybe() else {
            return Ok(None);
        };

        let operator_start = self.span_start();
        self.bump(); // eat type unary operator
        if operator == TypeUnaryOperator::AsComptime {
            self.bump(); // eat second token
        }
        let operator_span = self.get_span_from(&operator_start);

        if operator != TypeUnaryOperator::AsComptime {
            return Err(ParseError::unexpected(operator_span));
        }

        Ok(Some(operator_span))
    }

    /// Return one continuation token at the current parser position.
    fn next_continuation_token_maybe(&mut self) -> Option<ContinuationToken> {
        let token = self.current_token();
        let token_type = token.token.ty;
        if token_type == TokenType::End {
            return None;
        }

        Some(ContinuationToken {
            token_type,
            has_line_break_before: token.token.is_on_new_line,
        })
    }

    /// Return one postfix continuation token after applying newline and boundary rules.
    fn next_postfix_continuation_token(
        &mut self,
        space: PostfixSpace,
        is_in_static: bool,
        is_in_ternary_or_match: bool,
    ) -> Option<ContinuationToken> {
        let token = self.next_continuation_token_maybe()?;
        let token_type = token.token_type;

        // some postfix forms may cross a newline, but only for specific tokens
        if token.has_line_break_before {
            let can_continue_after_newline = match space {
                PostfixSpace::Value => {
                    // tree literal starters own newline led `<...` in value space
                    let starts_tree_literal = token_type == TokenType::LessThan
                        && self.can_start_tree_literal_after_line_break();

                    // typeof query operands must not absorb the next line as generic postfix syntax
                    let continues_typeof_query = self.flags.is_in_typeof_query()
                        && matches!(token_type, TokenType::LessThan | TokenType::ShiftLeft);

                    matches!(
                        token_type,
                        TokenType::OpenParenthesis
                            | TokenType::Dot
                            | TokenType::Maybe
                            | TokenType::LessThan
                            | TokenType::ShiftLeft
                    ) && !starts_tree_literal
                        && !continues_typeof_query
                }
                PostfixSpace::Type => token_type == TokenType::Dot,
            };
            if !can_continue_after_newline {
                return None;
            }
        }

        // postfix parsing must not consume ternary or match boundaries
        if is_in_ternary_or_match && token_type == TokenType::Colon {
            return None;
        }

        // static contexts stop before `>` closers
        if is_in_static && Self::starts_type_angle_close(token_type) {
            return None;
        }

        Some(token)
    }

    /// Return whether one statement expression must stop before continuation parsing.
    fn statement_expression_stops_continuation(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.flags.is_in_statement_position() {
            return false;
        }

        let next_token_type = self.peek_token_type();
        let expression = self.tree.get(left_expression_id);
        let is_continuable_lambda_declaration = matches!(
            expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.kind == FunctionKind::Lambda
                )
        ) && !self.current_token_is_on_new_line()
            && !matches!(
                next_token_type,
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            );

        expression.is_statement_boundary() && !is_continuable_lambda_declaration
    }

    /// Return whether a type expression can start a tagged object literal postfix.
    #[inline]
    pub(super) fn can_start_tagged_object_literal_type(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        match self.tree.get(type_expression_id) {
            TypeExpression::Parenthesized { expression } => {
                self.can_start_tagged_object_literal_type(*expression)
            }
            TypeExpression::Declaration { .. }
            | TypeExpression::FunctionTypeDeclaration(_)
            | TypeExpression::ConstructorTypeDeclaration(_)
            | TypeExpression::Reference { .. } => true,
            TypeExpression::Member { left, .. } => self.can_start_tagged_object_literal_type(*left),
            _ => false,
        }
    }

    /// Return whether a newline direct call should terminate in statement position.
    #[inline]
    fn newline_direct_call_terminates_statement(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.flags.is_in_statement_context() {
            return false;
        }

        // break and continue cannot continue into newline-prefixed calls
        if matches!(
            self.tree.get(left_expression_id),
            Expression::Break { .. } | Expression::Continue { .. }
        ) {
            return true;
        }

        let close_parenthesis_span =
            self.find_matching_close_maybe(TokenType::OpenParenthesis, TokenType::CloseParenthesis);
        let Some(close_parenthesis_span) = close_parenthesis_span else {
            return false;
        };

        let next_token_type = self.lookahead(|parser| {
            while parser.current_token().span.start <= close_parenthesis_span.start {
                parser.bump();
            }

            parser.peek_token_type()
        });
        let is_postfix_or_assign = UnaryOperator::from_postfix_token(next_token_type).is_some()
            || AssignOperator::from_token(next_token_type).is_some();
        let starts_lambda_head = matches!(
            next_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        );

        // postfix and assign continuations force statement termination:
        // `expr\n(arg).member` remains one continued expression
        // lambda-head follows cannot continue a direct call receiver safely
        is_postfix_or_assign || starts_lambda_head
    }

    /// Return true when assignment lhs form is invalid before target lowering.
    #[inline]
    fn assignment_target_has_invalid_form(
        &self,
        expression_id: LocalNodeId<Expression>,
        is_parenthesized: bool,
    ) -> bool {
        let inner_expression_id = self.without_parentheses_expression(expression_id);
        let is_parenthesized = is_parenthesized || inner_expression_id != expression_id;

        match self.tree.get(inner_expression_id) {
            // parenthesized object and array expressions cannot be assignment patterns
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. } => {
                is_parenthesized
            }

            // `satisfies` lhs is valid in parse output only when parenthesized
            Expression::Satisfies { .. } => !is_parenthesized,

            // `as` cast lhs is valid only when parenthesized
            Expression::As { .. } => !is_parenthesized,

            // all other lhs forms are handled by assignment target validation later
            _ => false,
        }
    }

    /// Convert one assignment lhs expression into one assign pattern.
    pub(crate) fn expression_to_assign_pattern(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        self.expression_to_assign_pattern_with_defaults(
            expression_id,
            AssignPatternDefaultMode::Reject,
        )
    }

    /// Convert one assignment target expression into one assign pattern.
    fn expression_to_assign_pattern_with_defaults(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        default_mode: AssignPatternDefaultMode,
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        let inner_expression_id = self.without_parentheses_expression(expression_id);
        let inner_expression = self.tree.get(inner_expression_id).clone();

        // object and array destructuring own recursive assign target lowering
        let assign_pattern = match inner_expression {
            Expression::ObjectExpression {
                ty: None,
                properties,
            } => {
                let fields =
                    self.object_properties_to_assign_pattern_fields(properties.as_slice())?;
                AssignPattern::Object { fields }
            }
            Expression::ArrayExpression { elements } => {
                let fields = self.array_elements_to_assign_pattern_fields(elements.as_slice())?;
                AssignPattern::Array { fields }
            }
            Expression::Assign {
                left,
                operator,
                right,
            } => {
                if default_mode == AssignPatternDefaultMode::Reject {
                    return Err(ParseError::unexpected(
                        self.tree.get_span(inner_expression_id),
                    ));
                }

                if operator != AssignOperator::Assign {
                    return Err(ParseError::unexpected(
                        self.tree.get_span(inner_expression_id),
                    ));
                }

                AssignPattern::Assign {
                    pattern: left,
                    value: right,
                }
            }
            _ => self.expression_to_simple_assign_pattern(inner_expression_id)?,
        };

        Ok(self.insert_node(assign_pattern, self.tree.get_span(expression_id)))
    }

    /// Return whether one expression is a simple assignment target.
    fn expression_is_simple_assignment_target(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression_id = self.without_parentheses_expression(expression_id);

        match self.tree.get(expression_id) {
            Expression::Identifier { .. } => true,
            Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. } => !self.expression_contains_optional_chain(expression_id),
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                self.expression_is_simple_assignment_target(*expression)
            }
            Expression::Must { left, .. } => self.expression_is_simple_assignment_target(*left),
            _ => false,
        }
    }

    /// Return whether one expression target contains optional chaining.
    fn expression_contains_optional_chain(&self, expression_id: LocalNodeId<Expression>) -> bool {
        let expression_id = self.without_parentheses_expression(expression_id);

        match self.tree.get(expression_id) {
            Expression::Maybe { .. } => true,
            Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
                self.expression_contains_optional_chain(*left)
            }
            Expression::Index { left, .. } => self.expression_contains_optional_chain(*left),
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                self.expression_contains_optional_chain(*expression)
            }
            Expression::Must { left, .. } => self.expression_contains_optional_chain(*left),
            _ => false,
        }
    }

    /// Convert one expression into a direct assignment target.
    fn expression_to_simple_assign_pattern(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<AssignPattern> {
        let inner_expression_id = self.without_parentheses_expression(expression_id);
        if !self.expression_is_simple_assignment_target(inner_expression_id) {
            return Err(ParseError::unexpected(
                self.tree.get_span(inner_expression_id),
            ));
        }

        Ok(AssignPattern::Expression {
            value: inner_expression_id,
        })
    }

    /// Convert one array literal element list into assign pattern fields.
    fn array_elements_to_assign_pattern_fields(
        &mut self,
        elements: &[LocalNodeId<Argument>],
    ) -> ParseResult<Vec<LocalNodeId<AssignPatternField>>> {
        let mut fields = Vec::with_capacity(elements.len());

        for element_id in elements {
            let element = self.tree.get(*element_id).clone();
            let field = match element {
                Argument::Positional { value }
                    if matches!(self.tree.get(value), Expression::Stub | Expression::Missing) =>
                {
                    AssignPatternField::Elision
                }
                Argument::Positional { value } => {
                    let pattern = self.expression_to_assign_pattern_with_defaults(
                        value,
                        AssignPatternDefaultMode::Allow,
                    )?;
                    AssignPatternField::Positional { pattern }
                }
                Argument::Spread { value, .. } => {
                    let pattern = self.expression_to_assign_pattern_with_defaults(
                        value,
                        AssignPatternDefaultMode::Reject,
                    )?;
                    AssignPatternField::Spread {
                        pattern: Some(pattern),
                    }
                }
                Argument::Named { .. } | Argument::Labeled { .. } | Argument::Error => {
                    return Err(ParseError::unexpected(self.tree.get_span(*element_id)));
                }
            };

            let field_id = self.insert_node(field, self.tree.get_span(*element_id));
            fields.push(field_id);
        }

        Ok(fields)
    }

    /// Convert one object literal property list into assign pattern fields.
    fn object_properties_to_assign_pattern_fields(
        &mut self,
        properties: &[LocalNodeId<Property>],
    ) -> ParseResult<Vec<LocalNodeId<AssignPatternField>>> {
        let mut fields = Vec::with_capacity(properties.len());

        for property_id in properties {
            let property = self.tree.get(*property_id).clone();
            let field = match property {
                Property::Field {
                    key: Key::Name(name),
                    value,
                    is_shorthand,
                } => {
                    let pattern = self.expression_to_assign_pattern_with_defaults(
                        value,
                        AssignPatternDefaultMode::Allow,
                    )?;

                    // bare shorthand keeps the nested pattern slot empty
                    if is_shorthand && self.assign_pattern_is_simple_name(pattern, name) {
                        AssignPatternField::Named {
                            name,
                            is_shorthand: true,
                            pattern: None,
                        }
                    }
                    // shorthand with default keeps the nested assign pattern
                    else if is_shorthand && self.assign_pattern_is_defaulted_name(pattern, name) {
                        AssignPatternField::Named {
                            name,
                            is_shorthand: true,
                            pattern: Some(pattern),
                        }
                    }
                    // expanded named field
                    else {
                        AssignPatternField::Named {
                            name,
                            is_shorthand: false,
                            pattern: Some(pattern),
                        }
                    }
                }
                Property::Field {
                    key: Key::Expression(key),
                    value,
                    is_shorthand: _,
                } => {
                    let pattern = self.expression_to_assign_pattern_with_defaults(
                        value,
                        AssignPatternDefaultMode::Allow,
                    )?;
                    AssignPatternField::Computed { key, pattern }
                }
                Property::Field {
                    key: Key::Private(_),
                    value: _,
                    is_shorthand: _,
                }
                | Property::Method { .. }
                | Property::Error => {
                    return Err(ParseError::unexpected(self.tree.get_span(*property_id)));
                }
                Property::Spread { value } => {
                    let pattern = self.expression_to_assign_pattern_with_defaults(
                        value,
                        AssignPatternDefaultMode::Reject,
                    )?;
                    AssignPatternField::Spread {
                        pattern: Some(pattern),
                    }
                }
            };

            let field_id = self.insert_node(field, self.tree.get_span(*property_id));
            fields.push(field_id);
        }

        Ok(fields)
    }

    /// Return whether one assign pattern is the plain shorthand form for a property name.
    fn assign_pattern_is_simple_name(
        &self,
        assign_pattern_id: LocalNodeId<AssignPattern>,
        name: Name,
    ) -> bool {
        let assign_pattern = self.tree.get(assign_pattern_id);
        let AssignPattern::Expression { value } = assign_pattern else {
            return false;
        };

        self.expression_is_simple_name(*value, name)
    }

    /// Return whether one assign pattern is the defaulted shorthand form for a property name.
    fn assign_pattern_is_defaulted_name(
        &self,
        assign_pattern_id: LocalNodeId<AssignPattern>,
        name: Name,
    ) -> bool {
        let assign_pattern = self.tree.get(assign_pattern_id);
        let AssignPattern::Assign { pattern, value: _ } = assign_pattern else {
            return false;
        };

        self.assign_pattern_is_simple_name(*pattern, name)
    }

    /// Return whether one expression is the plain shorthand source for a property name.
    fn expression_is_simple_name(
        &self,
        expression_id: LocalNodeId<Expression>,
        name: Name,
    ) -> bool {
        let expression_id = self.without_parentheses_expression(expression_id);
        let expression = self.tree.get(expression_id);

        match expression {
            Expression::Identifier {
                name: expression_name,
            } => *expression_name == name.string(),
            _ => false,
        }
    }

    /// Eat one value-space dot postfix continuation when the parser is already positioned at `.`.
    ///
    /// Examples:
    /// ```
    /// value.method
    /// value?.<T>()
    /// value.[index]
    /// value.!
    /// 0..toString()
    /// ```
    fn eat_value_dot_postfix_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if matches!(
            self.tree.get(left_expression_id),
            Expression::Instantiation { .. }
        ) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        let dot_span = self.peek()?.span;
        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;

        // indirect calls stay in value space
        if next_token_type == TokenType::OpenParenthesis {
            let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
            self.bump(); // eat .
            let expression_id =
                self.eat_call(left_expression_id, None, PostfixPosition::Indirect)?;

            return Ok(Some(expression_id));
        }

        // indirect instantiation or generic call
        if matches!(next_token_type, TokenType::LessThan | TokenType::ShiftLeft) {
            let Some(position) =
                self.value_postfix_generic_arguments_position_maybe(left_expression_id)
            else {
                return Ok(None);
            };

            // generic call or instantiation
            let expression_id = self.try_eat_value_postfix_generic_application(
                start,
                left_expression_id,
                position,
            )?;

            return Ok(expression_id);
        }

        // indirect indexing stays in value space
        if next_token_type == TokenType::OpenBracket {
            self.bump(); // eat .
            let expression_id = self.eat_index(left_expression_id, PostfixPosition::Indirect)?;

            return Ok(Some(expression_id));
        }

        // optional chaining stays in value space
        if next_token_type == TokenType::Maybe && !next_token.token.is_on_new_line {
            self.bump(); // eat .
            self.bump(); // eat ?
            let expression_id = self.insert_node(
                Expression::Maybe {
                    left: left_expression_id,
                    position: PostfixPosition::Indirect,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // `value.!`
        if next_token_type == TokenType::Not && !next_token.token.is_on_new_line {
            self.bump(); // eat .
            self.bump(); // eat !
            let expression_id = self.insert_node(
                Expression::Must {
                    position: PostfixPosition::Indirect,
                    left: left_expression_id,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // preserve a committed member access when the name slot is missing
        if Self::is_expression_slot_boundary_token(next_token_type) {
            self.bump(); // eat .
            self.report_unexpected_for_here(NodeType::Expression);
            let expression_id = self.insert_node(
                Expression::Member {
                    left: left_expression_id,
                    name: None,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // preserve a committed private member access when `#` has no identifier
        if next_token_type == TokenType::Hash && !next_token.token.is_on_new_line {
            let private_name_token_type = self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.peek_token_type()
            });
            if Self::is_expression_slot_boundary_token(private_name_token_type) {
                self.bump(); // eat .
                self.bump(); // eat #
                self.report_unexpected_for_here(NodeType::Expression);
                let expression_id = self.insert_node(
                    Expression::PrivateMember {
                        left: left_expression_id,
                        name: None,
                    },
                    self.get_span_from(start),
                );

                return Ok(Some(expression_id));
            }
        }

        // private members remain in value space
        if next_token_type == TokenType::Hash
            && !next_token.token.is_on_new_line
            && self.next_hash_has_adjacent_identifier()
        {
            self.bump(); // eat .
            self.bump(); // eat #
            let (name, name_span) = self.eat_identifier_with_span()?;
            let expression_id = self.insert_value_private_member_expression(
                start,
                left_expression_id,
                name,
                name_span,
            );

            return Ok(Some(expression_id));
        }

        let mut member_distance = 2;
        let has_decimal_separator = next_token_type == TokenType::Dot
            && dot_span.end == next_token.span.start
            && self.expression_is_decimal_integer_before_dot(left_expression_id, dot_span);
        if has_decimal_separator {
            let separated_member_is_name = self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.current_token_is_dot_member_name()
            });
            if !separated_member_is_name {
                return Ok(None);
            }

            member_distance = 3;
            self.bump(); // eat .
            self.bump(); // eat decimal separator .
        } else if self.next_token_is_dot_member_name() {
            self.bump(); // eat .
        } else {
            return Ok(None);
        }

        if self.invalid_decimal_integer_member_access(left_expression_id, member_distance, dot_span)
        {
            return Err(ParseError::unexpected(self.prev().expect("peeked").span));
        }

        let (name, name_span) = self.eat_member_name_with_span()?;
        let expression_id =
            self.insert_value_member_expression(start, left_expression_id, name, name_span);

        Ok(Some(expression_id))
    }

    /// Insert one canonical value member expression.
    fn insert_value_member_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        name: StringId,
        name_span: Span,
    ) -> LocalNodeId<Expression> {
        let member_id = self.insert_node(
            Expression::Member {
                left,
                name: Some(name),
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        member_id
    }

    /// Insert one canonical private member expression.
    fn insert_value_private_member_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        name: StringId,
        name_span: Span,
    ) -> LocalNodeId<Expression> {
        let member_id = self.insert_node(
            Expression::PrivateMember {
                left,
                name: Some(name),
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        member_id
    }

    /// Eat one type-space dot postfix continuation when the parser is already positioned at `.`.
    ///
    /// Examples:
    /// ```
    /// T.Item
    /// T.<U>
    /// T.!
    /// 0..Member
    /// ```
    fn eat_type_dot_postfix_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        let dot_span = self.peek()?.span;
        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;

        // indirect instantiation or generic call
        if matches!(next_token_type, TokenType::LessThan | TokenType::ShiftLeft) {
            let Some(position) = self.type_postfix_generic_arguments_position_maybe(left_type_id)
            else {
                return Ok(None);
            };

            return self.try_eat_type_postfix_generic_application(start, left_type_id, position);
        }

        // `T.!`
        if next_token_type == TokenType::Not && !next_token.token.is_on_new_line {
            self.bump(); // eat .
            self.bump(); // eat !
            let expression_id = self.insert_node(
                TypeExpression::Must {
                    target_type: left_type_id,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // preserve a committed type projection when the member slot is missing
        if Self::is_expression_slot_boundary_token(next_token_type) {
            self.bump(); // eat .
            self.report_unexpected_for_here(NodeType::TypeExpression);
            let error_id = self.insert_node(TypeExpression::Error, self.get_span_from(start));

            return Ok(Some(error_id));
        }

        // private members are not valid in type space
        if next_token_type == TokenType::Hash
            && !next_token.token.is_on_new_line
            && self.next_hash_has_adjacent_identifier()
        {
            self.bump(); // eat .
            self.bump(); // eat #
            let _ = self.eat_identifier_with_span()?;
            let _ = self
                .try_eat_generic_arguments(false, false)
                .unwrap_or_default();
            self.report_unexpected_for_here(NodeType::TypeExpression);
            let error_id = self.insert_node(TypeExpression::Error, self.get_span_from(start));

            return Ok(Some(error_id));
        }

        let mut member_distance = 2;
        let has_decimal_separator = next_token_type == TokenType::Dot
            && dot_span.end == next_token.span.start
            && self.type_is_decimal_integer_before_dot(left_type_id, dot_span);
        if has_decimal_separator {
            let separated_member_is_name = self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.current_token_is_dot_member_name()
            });
            if !separated_member_is_name {
                return Ok(None);
            }

            member_distance = 3;
            self.bump(); // eat .
            self.bump(); // eat decimal separator .
        } else if self.next_token_is_dot_member_name() {
            self.bump(); // eat .
        } else {
            return Ok(None);
        }

        if self.invalid_decimal_integer_type_member_access(left_type_id, member_distance, dot_span)
        {
            return Err(ParseError::unexpected(self.prev().expect("peeked").span));
        }

        let (name, name_span) = self.eat_member_name_with_span()?;
        let generic_arguments = self
            .try_eat_generic_arguments(false, false)
            .unwrap_or_default();
        let member_id = self.insert_node(
            TypeExpression::Member {
                left: left_type_id,
                name,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(member_id))
    }

    /// Eat one value-space postfix `?` or `as comptime` continuation.
    fn eat_value_postfix_operator_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
        is_destack_language: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // `value as comptime`
        if let Some(operator_span) = self.eat_as_comptime_postfix_operator_maybe()? {
            let expression_id = self.insert_node(
                Expression::Comptime {
                    body: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(expression_id));
        }

        if !self.peek_is(TokenType::Maybe) {
            return Ok(None);
        }

        let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
        let is_direct_postfix_maybe = is_destack_language
            && (self.is_next_any_stop() && !self.previous_token_is_on_new_line()
                || self.is_next_any_close_parenthesis()
                || self.peek_next_assign_operator_is());
        if !is_optional_chain_after_maybe && !is_direct_postfix_maybe {
            return Ok(None);
        }

        self.bump(); // eat ?
        let expression_id = self.insert_node(
            Expression::Maybe {
                left: left_expression_id,
                position: PostfixPosition::Direct,
            },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Eat one type-space postfix `!` or `as comptime` continuation.
    fn eat_type_postfix_operator_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // `T as comptime`
        if let Some(operator_span) = self.eat_as_comptime_postfix_operator_maybe()? {
            let expression_id = self.insert_node(
                TypeExpression::AsComptime {
                    target_type: left_type_id,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(expression_id));
        }

        if !self.peek_is(TokenType::Not) {
            return Ok(None);
        }

        self.bump(); // eat !
        let expression_id = self.insert_node(
            TypeExpression::Must {
                target_type: left_type_id,
            },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Try to eat one tagged object literal postfix.
    fn try_eat_tagged_object_literal_postfix(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.language.is_destack()
            || !self.peek_is(TokenType::OpenBrace)
            || self.flags.is_in_before_block()
        {
            return Ok(None);
        }

        let left_expression_id = self.without_parentheses_expression(left_expression_id);
        let Some(ty) = self.wrapped_type_expression_maybe(left_expression_id) else {
            return Ok(None);
        };

        let properties = self.eat_object_literal()?;
        if !self.can_start_tagged_object_literal_type(ty) {
            return Err(ParseError::unexpected(
                self.tree.get_span(left_expression_id),
            ));
        }

        let expression_id = self.insert_node(
            Expression::ObjectExpression {
                ty: Some(ty),
                properties,
            },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Try to eat one direct value call postfix.
    fn try_eat_value_call_postfix(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        left_is_parenthesized: bool,
        has_statement_boundary_newline: bool,
        is_in_new_receiver: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // maybe receivers and `new` receivers do not take direct calls here
        let left_is_maybe = matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
        if left_is_maybe || is_in_new_receiver {
            return Ok(None);
        }

        // newline direct calls may terminate the statement instead
        let terminates_statement = has_statement_boundary_newline
            && self.newline_direct_call_terminates_statement(left_expression_id);
        if terminates_statement {
            return Ok(None);
        }

        // unparenthesized lambdas need a separator before direct calls
        let left_is_unparenthesized_lambda =
            self.is_unparenthesized_lambda_expression(left_expression_id) && !left_is_parenthesized;
        if left_is_unparenthesized_lambda && has_statement_boundary_newline {
            return Ok(None);
        }

        if left_is_unparenthesized_lambda {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
        let expression_id = self.eat_call(left_expression_id, None, PostfixPosition::Direct)?;

        Ok(Some(expression_id))
    }

    /// Eat one tuple or sequence postfix inside parenthesis.
    fn eat_parenthesized_sequence_postfix(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.bump(); // eat comma

        if self.language.is_destack() {
            let first_element_id = self.insert_node(
                Argument::Positional {
                    value: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.stats.record_with_flags_call();
            let tuple_elements = self.with_flags(self.flags.not_in_position(), |parser| {
                parser
                    .eat_sequence_literal_body(Some(first_element_id), TokenType::CloseParenthesis)
            })?;

            return Ok(self.insert_node(
                Expression::TupleExpression {
                    elements: tuple_elements,
                },
                self.get_span_from(start),
            ));
        }

        let mut expressions = vec![left_expression_id];
        while !self.peek_is(TokenType::CloseParenthesis) {
            if self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma
                continue;
            }

            let expression_flags = self.flags.not_in_position().not_in_sequence_expression();
            let expression_id = self.eat_expression_with_context_unchecked(expression_flags)?;
            expressions.push(expression_id);
        }

        Ok(self.insert_node(
            Expression::SequenceExpression { expressions },
            self.get_span_from(start),
        ))
    }

    /// Eat one value-space postfix step for one normalized continuation token.
    fn try_eat_value_postfix_step(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
        left_is_parenthesized: bool,
        token: ContinuationToken,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // postfix unary operators are the simplest continuation form
        if let Some(operator) = UnaryOperator::from_postfix_token(token.token_type) {
            let operator_start = self.span_start();
            self.bump(); // eat unary operator
            let operator_span = self.get_span_from(&operator_start);
            let expression_id = self.insert_node(
                Expression::Unary {
                    operator,
                    right: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(expression_id));
        }

        // direct calls depend on newline and `new` receiver context
        let has_statement_boundary_newline = token.has_line_break_before;
        let is_in_new_receiver = self.flags.is_in_new_receiver();

        match token.token_type {
            // tagged template literals
            TokenType::TemplateString | TokenType::TemplateStringStart => {
                if !self.is_template_literal_start() {
                    return Ok(None);
                }

                if !self.tagged_template_tag_is_valid(left_expression_id) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let (tag, generic_arguments) =
                    self.split_instantiation_expression(left_expression_id);
                let template_literal = self.eat_tagged_template_literal()?;
                let expression_id = self.insert_node(
                    Expression::TaggedTemplateExpression {
                        tag,
                        generic_arguments,
                        value: template_literal,
                    },
                    self.get_span_from(start),
                );

                Ok(Some(expression_id))
            }

            // postfix calls
            TokenType::OpenParenthesis => self.try_eat_value_call_postfix(
                left_expression_id,
                left_is_parenthesized,
                has_statement_boundary_newline,
                is_in_new_receiver,
            ),

            // dot driven continuations
            TokenType::Dot => self.eat_value_dot_postfix_continuation(start, left_expression_id),

            // direct indexing
            TokenType::OpenBracket => {
                if matches!(
                    self.tree.get(left_expression_id),
                    Expression::Instantiation { .. }
                ) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let left_is_maybe =
                    matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                if left_is_maybe {
                    return Ok(None);
                }
                let expression_id = self.eat_index(left_expression_id, PostfixPosition::Direct)?;

                Ok(Some(expression_id))
            }

            // postfix generic arguments
            TokenType::LessThan | TokenType::ShiftLeft => {
                let Some(position) =
                    self.value_postfix_generic_arguments_position_maybe(left_expression_id)
                else {
                    return Ok(None);
                };

                // generic call or instantiation
                self.try_eat_value_postfix_generic_application(start, left_expression_id, position)
            }

            // postfix `?` and `as comptime`
            TokenType::Identifier | TokenType::Maybe => self
                .eat_value_postfix_operator_continuation(
                    start,
                    left_expression_id,
                    self.language.is_destack(),
                ),

            // direct must postfix
            TokenType::Not => {
                self.bump(); // eat !
                let expression_id = self.insert_node(
                    Expression::Must {
                        position: PostfixPosition::Direct,
                        left: left_expression_id,
                    },
                    self.get_span_from(start),
                );

                Ok(Some(expression_id))
            }

            // tuple or sequence continuations inside parenthesis
            TokenType::Comma if self.flags.is_in_parenthesis() => {
                let expression_id =
                    self.eat_parenthesized_sequence_postfix(start, left_expression_id)?;

                Ok(Some(expression_id))
            }

            // done
            _ => Ok(None),
        }
    }

    /// Eat one type-space postfix step for one normalized continuation token.
    fn try_eat_type_postfix_step(
        &mut self,
        start: &ParserSpanStart,
        left_type_id: LocalNodeId<TypeExpression>,
        token: ContinuationToken,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        match token.token_type {
            // dot driven continuations
            TokenType::Dot => self.eat_type_dot_postfix_continuation(start, left_type_id),

            // direct indexing
            TokenType::OpenBracket => {
                let type_expression_id = self.eat_type_index(left_type_id)?;

                Ok(Some(type_expression_id))
            }

            // postfix generic arguments
            TokenType::LessThan | TokenType::ShiftLeft => {
                let Some(position) =
                    self.type_postfix_generic_arguments_position_maybe(left_type_id)
                else {
                    return Ok(None);
                };

                self.try_eat_type_postfix_generic_application(start, left_type_id, position)
            }

            // postfix `as comptime` and direct must postfix
            TokenType::Identifier | TokenType::Not => {
                self.eat_type_postfix_operator_continuation(start, left_type_id)
            }

            // done
            _ => Ok(None),
        }
    }

    /// Parse value-space postfix continuation operators after a primary expression.
    ///
    /// Examples:
    /// ```
    /// value()
    /// value[index]
    /// value.method?.()
    /// value<T>()
    /// Vector2 { x: 0, y: 1 }
    /// ```
    pub(super) fn eat_value_postfix_continuation(
        &mut self,
        start: &ParserSpanStart,
        mut left_expression_id: LocalNodeId<Expression>,
        mut left_is_parenthesized: bool,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
        let is_in_static = self.flags.is_in_static();
        let is_in_ternary_or_match =
            self.flags.is_in_ternary_condition() || self.flags.is_in_match_case();

        loop {
            // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
            if let Some(next_expression_id) =
                self.try_eat_tagged_object_literal_postfix(start, left_expression_id)?
            {
                left_expression_id = next_expression_id;
                continue;
            }

            // normalize the next postfix token once before branch dispatch
            let Some(token) = self.next_postfix_continuation_token(
                PostfixSpace::Value,
                is_in_static,
                is_in_ternary_or_match,
            ) else {
                break;
            };

            let Some(next_expression_id) = self.try_eat_value_postfix_step(
                start,
                left_expression_id,
                left_is_parenthesized,
                token,
            )?
            else {
                break;
            };

            left_expression_id = next_expression_id;
            left_is_parenthesized = false;
        }

        Ok((left_expression_id, left_is_parenthesized))
    }

    /// Parse type-space postfix continuation operators after a primary expression.
    ///
    /// Examples:
    /// ```
    /// T[]
    /// T[K]
    /// Result<T>.Ok
    /// Promise<T>.!
    /// ```
    pub(super) fn eat_type_postfix_continuation(
        &mut self,
        start: &ParserSpanStart,
        mut left_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
        let is_in_static = self.flags.is_in_static();
        let is_in_ternary_or_match =
            self.flags.is_in_ternary_condition() || self.flags.is_in_match_case();

        loop {
            // normalize the next postfix token once before branch dispatch
            let Some(token) = self.next_postfix_continuation_token(
                PostfixSpace::Type,
                is_in_static,
                is_in_ternary_or_match,
            ) else {
                break;
            };
            let Some(next_type_id) = self.try_eat_type_postfix_step(start, left_type_id, token)?
            else {
                break;
            };

            left_type_id = next_type_id;
        }

        Ok(left_type_id)
    }

    /// Eat one type conditional expression after consuming `extends`.
    fn eat_type_conditional_expression(
        &mut self,
        start: &ParserSpanStart,
        left_type_id: LocalNodeId<TypeExpression>,
        right_context: ParserFlags,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // right side of `extends`
        let right_ambient_context = self.flags.with_type(true);
        let extends_context = right_context
            .not_in_left_precedence()
            .disallow_type_conditional();
        let extends_type = self.eat_type_expression_node_or_recover_missing(
            self.flags
                .with_ambient_context(right_ambient_context)
                .with_expression_context(extends_context),
            NodeType::Expression,
        )?;

        // `?` may follow on the same line or after a newline
        let has_conditional_marker = self.peek_is(TokenType::Maybe);

        // branches
        let (then_type, else_type) = if has_conditional_marker {
            self.bump(); // eat ?

            let then_context = self.flags.not_in_position();
            let then_type = self.eat_type_expression_node_or_recover_missing(
                self.flags
                    .with_type(true)
                    .with_expression_context(then_context),
                NodeType::TypeExpression,
            )?;
            self.eat_colon()?;

            let mut else_context = self.flags.not_in_position();
            if self.flags.is_in_type_conditional_right() {
                else_context = else_context.in_type_conditional_right();
            }
            let else_type = self.eat_type_expression_node_or_recover_missing(
                self.flags
                    .with_type(true)
                    .with_expression_context(else_context),
                NodeType::TypeExpression,
            )?;

            (then_type, else_type)
        } else {
            let then_type = self.insert_missing_type_expression_here();
            let else_type = self.insert_missing_type_expression_here();

            (then_type, else_type)
        };

        let type_expression_id = self.insert_node(
            TypeExpression::Conditional {
                left: left_type_id,
                extends_type,
                then_type,
                else_type,
            },
            self.get_span_from(start),
        );

        Ok(type_expression_id)
    }

    /// Parse infix continuation operators after type postfix parsing.
    ///
    /// Examples:
    /// ```
    /// A | B
    /// A & B & C
    /// value is string
    /// T extends U ? X : Y
    /// ```
    pub(super) fn eat_type_infix_continuation(
        &mut self,
        start: &ParserSpanStart,
        mut left_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
        let left_precedence = self.flags.left_precedence;

        loop {
            // normalize the next infix token once before operator analysis
            let Some(token) = self.next_continuation_token_maybe() else {
                break;
            };
            let token_type = token.token_type;
            let has_line_break_before = token.has_line_break_before;

            // stop before the conditional marker so `extends` owns it explicitly
            if token_type == TokenType::Maybe {
                break;
            }

            // stop before ternary or match boundaries
            if (self.flags.is_in_ternary_condition() || self.flags.is_in_match_case())
                && token_type == TokenType::Colon
            {
                break;
            }

            // type expressions stop before tree literals after a line break
            if has_line_break_before && self.can_start_tree_literal_after_line_break() {
                break;
            }

            // reject tokens that cannot start any infix operator
            let can_start_operator = if token_type == TokenType::Identifier {
                self.current_token_can_start_infix_or_assign_operator()
            } else {
                BinaryOperator::from_token("", token_type).is_some()
            };
            if !can_start_operator {
                break;
            }

            let Some((right_operator, operator_offset)) =
                self.peek_infix_operator_maybe(has_line_break_before)
            else {
                break;
            };

            // infer constraints treat `extends` as an outer boundary unless nested explicitly
            if self.flags.is_disallow_type_conditional()
                && right_operator == ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
            {
                break;
            }

            // mapped constraints stop before the remap `as`
            if self.flags.is_in_type_mapped_constraint() && right_operator == ParseInfixOperator::As
            {
                break;
            }

            // precedence
            if let Some(left_precedence) = left_precedence
                && left_precedence >= right_operator.precedence()
            {
                break;
            }

            // dedicated type parsing only owns type operators
            let is_supported_type_operator = matches!(
                right_operator,
                ParseInfixOperator::Binary(
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ) | ParseInfixOperator::Is
                    | ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
            );
            if !is_supported_type_operator {
                // invalid type operators fail loudly in strict type space
                if matches!(right_operator, ParseInfixOperator::TypeBinary(_)) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                break;
            }

            // align parser position with scanner cursor before consuming operator tokens

            // capture operator span before eating
            let operator_start = self.span_start();
            for _ in 0..operator_offset {
                self.bump(); // eat infix operator
            }
            let operator_span = self.get_span_from(&operator_start);

            // right side context
            let mut right_context = self
                .flags
                .not_in_statement_position()
                .not_in_type_conditional_right()
                .in_left_precedence(right_operator.precedence());
            if self.flags.is_in_type_conditional_right()
                || matches!(
                    right_operator,
                    ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                )
            {
                right_context = right_context.in_type_conditional_right();
            }

            // combine into the new left type expression
            let previous_left_type_id = left_type_id;
            let head_span = self.type_expression_head_span(left_type_id);
            let full_span = self.get_span_from(start);
            let is_allowed_type_predicate = self.flags.allows_type_predicate()
                || self.type_expression_is_bare_this(left_type_id);
            left_type_id =
                if right_operator == ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends) {
                    self.eat_type_conditional_expression(start, left_type_id, right_context)?
                } else if right_operator == ParseInfixOperator::Is && !is_allowed_type_predicate {
                    return Err(ParseError::unexpected(operator_span));
                } else {
                    let right_ambient_context = self.flags.with_type(true);
                    let right_type_id = if self.is_type_expression_boundary() {
                        self.recover_missing_type_expression_here(NodeType::Expression)
                    } else {
                        self.with_flags(
                            self.flags
                                .with_ambient_context(right_ambient_context)
                                .with_expression_context(right_context),
                            |parser| parser.eat_type_expression(),
                        )?
                    };

                    self.make_type_infix_expression(
                        full_span,
                        head_span,
                        left_type_id,
                        right_operator,
                        operator_span,
                        right_type_id,
                    )?
                };

            // operator spans
            self.tree.set_main_span(left_type_id, operator_span);

            // type predicates own the subject span, not the operator span
            if matches!(
                self.tree.get(left_type_id),
                TypeExpression::Predicate { .. }
            ) {
                let subject_span = self
                    .tree
                    .get_main_span(previous_left_type_id)
                    .unwrap_or_else(|| self.tree.get_span(previous_left_type_id));
                self.tree.set_main_span(left_type_id, subject_span);
            }
        }

        Ok(left_type_id)
    }

    /// Eat one `as` or `satisfies` infix expression.
    fn eat_assertion_infix_expression(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        right_operator: ParseInfixOperator,
        operator_span: Span,
        right_context: ParserFlags,
    ) -> ParseResult<(Expression, Option<u32>)> {
        // `as const`
        let parses_const_type_reference =
            right_operator == ParseInfixOperator::As && self.is_keyword(Keyword::Const);
        let mut as_const_operator_end = None;
        let target_type = if parses_const_type_reference {
            let const_span = self.eat_keyword(Keyword::Const)?.span;
            as_const_operator_end = Some(const_span.end);
            self.insert_node(TypeExpression::Const, const_span)
        } else {
            let right_ambient_context = self.flags.with_type(true);
            self.eat_type_expression_node_or_recover_missing(
                self.flags
                    .with_ambient_context(right_ambient_context)
                    .with_expression_context(right_context),
                NodeType::Expression,
            )?
        };
        self.set_node_leading_span(target_type, operator_span.end);

        // `as` and `satisfies` wrap the left expression
        let expression = match right_operator {
            ParseInfixOperator::As => Expression::As {
                expression: left_expression_id,
                target_type,
            },
            ParseInfixOperator::Satisfies => Expression::Satisfies {
                expression: left_expression_id,
                target_type,
            },
            _ => unreachable!(),
        };

        Ok((expression, as_const_operator_end))
    }

    /// Eat one infix right operand in type space or insert a missing node at a hard boundary.
    fn eat_infix_right_type_or_missing(
        &mut self,
        right_context: ParserFlags,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // hard boundaries synthesize a missing type node in place
        if self.is_type_expression_boundary() {
            return Ok(self.insert_missing_type_expression_here());
        }

        // otherwise parse the full right type in ambient type context
        let right_ambient_context = self.flags.with_type(true);
        self.eat_type_expression_node_or_recover_missing(
            self.flags
                .with_ambient_context(right_ambient_context)
                .with_expression_context(right_context),
            NodeType::Expression,
        )
    }

    /// Parse infix continuation operators after postfix parsing.
    ///
    /// Examples:
    /// ```
    /// a + b * c
    /// value as string
    /// value satisfies Foo
    /// value is string
    /// T extends U ? X : Y
    /// x = y ?? z
    /// ```
    pub(super) fn eat_infix_continuation(
        &mut self,
        start: &ParserSpanStart,
        mut left_expression_id: LocalNodeId<Expression>,
        mut left_is_parenthesized: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let left_is_statement = self.flags.is_in_statement_position()
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();

        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
        let left_precedence = self.flags.left_precedence;
        loop {
            // wrapped type expressions keep the explicit value/type boundary
            let left_is_type_expression =
                matches!(self.tree.get(left_expression_id), Expression::Type { .. });

            // normalize the next infix token once before operator analysis
            let Some(token) = self.next_continuation_token_maybe() else {
                break;
            };
            let token_type = token.token_type;
            let has_line_break_before = token.has_line_break_before;

            // new receivers stop before type argument delimiters at top-level receiver scope
            if self.flags.is_in_new_receiver()
                && !self.flags.is_in_parenthesis()
                && (token_type == TokenType::LessThan || token_type == TokenType::ShiftLeft)
            {
                break;
            }

            // stop before conditional boundaries so infix lookahead does not lex past `?`
            if token_type == TokenType::Maybe {
                break;
            }

            // stop before ternary or match case boundary so infix lookahead does not lex past `:`
            if (self.flags.is_in_ternary_condition() || self.flags.is_in_match_case())
                && token_type == TokenType::Colon
            {
                break;
            }

            // statement expressions do not continue across line breaks
            if left_is_statement && has_line_break_before {
                break;
            }

            // type expressions stop before tree literals after a line break
            if left_is_type_expression
                && has_line_break_before
                && self.can_start_tree_literal_after_line_break()
            {
                break;
            }

            // reject tokens that cannot start any infix or assignment operator
            let can_start_operator = if token_type == TokenType::Identifier {
                self.current_token_can_start_infix_or_assign_operator()
            } else {
                AssignOperator::from_token(token_type).is_some()
                    || BinaryOperator::from_token("", token_type).is_some()
            };

            if !can_start_operator {
                break;
            }

            let Some((right_operator, operator_offset)) =
                self.peek_infix_operator_maybe(has_line_break_before)
            else {
                break;
            };

            // infer constraints treat `extends` as an outer boundary unless nested explicitly
            if self.flags.is_disallow_type_conditional()
                && right_operator == ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
            {
                break;
            }

            // mapped constraints stop before the remap `as`
            if self.flags.is_in_type_mapped_constraint() && right_operator == ParseInfixOperator::As
            {
                break;
            }

            // multiline type layout
            // precedence boundary
            if let Some(left_precedence) = left_precedence {
                let should_break = if right_operator.is_right_associative() {
                    left_precedence > right_operator.precedence()
                } else {
                    left_precedence >= right_operator.precedence()
                };

                if should_break {
                    break;
                }
            }

            // reject assignment targets that are invalid in ts/js grammar
            if matches!(right_operator, ParseInfixOperator::Assign(_))
                && self
                    .assignment_target_has_invalid_form(left_expression_id, left_is_parenthesized)
            {
                return Err(ParseError::unexpected(
                    self.tree.get_span(left_expression_id),
                ));
            }

            // align parser position with scanner cursor before consuming operator tokens

            // capture operator span before eating
            let operator_start = self.span_start();
            for _ in 0..operator_offset {
                self.bump(); // eat infix operator
            }
            let operator_span = self.get_span_from(&operator_start);

            // eat right expression
            let subject_id = left_expression_id;
            let right_kind = InfixRightKind::new(
                right_operator,
                left_is_type_expression,
                self.flags.is_in_before_block(),
                self.flags.allows_type_predicate(),
            );
            let mut right_context = self
                .flags
                .not_in_statement_position()
                .not_in_type_conditional_right()
                .in_left_precedence(right_operator.precedence());
            let mut as_const_operator_end = None;

            // cast and satisfies in parenthesized value expressions need
            // the parenthesis flag so the type right side can stop at `)`
            if !right_kind.preserves_parenthesis() {
                right_context = right_context.not_in_parenthesis();
            }

            // type operators in value expressions parse a full type expression on the right
            if right_kind.parses_type_expression() {
                right_context = right_context.not_in_left_precedence();
            }

            // conditional type right sides must keep their boundary marker active
            if self.flags.is_in_type_conditional_right()
                || right_kind.keeps_type_conditional_boundary()
            {
                right_context = right_context.in_type_conditional_right();
            }

            // combine into the new left expression
            let left_expression = match right_kind {
                // `value as T`, `value satisfies T`
                InfixRightKind::Assertion => {
                    let (expression, operator_end) = self.eat_assertion_infix_expression(
                        left_expression_id,
                        right_operator,
                        operator_span,
                        right_context,
                    )?;
                    as_const_operator_end = operator_end;
                    expression
                }

                // `T extends U ? X : Y`
                InfixRightKind::TypeConditional => {
                    let left_type_id = self.expect_wrapped_type_expression(left_expression_id)?;
                    let type_expression_id =
                        self.eat_type_conditional_expression(start, left_type_id, right_context)?;

                    Expression::Type {
                        value: type_expression_id,
                    }
                }

                // `value is T`
                InfixRightKind::ValuePredicate => {
                    let right_type_id = self.eat_infix_right_type_or_missing(right_context)?;
                    self.set_node_leading_span(right_type_id, operator_span.end);

                    Expression::Is {
                        value: left_expression_id,
                        target_type: right_type_id,
                    }
                }

                // `A | B`, `A & B`, `T is U`
                InfixRightKind::TypeOperator => {
                    let left_type_id = self.expect_wrapped_type_expression(left_expression_id)?;
                    let right_type_id = self.eat_infix_right_type_or_missing(right_context)?;
                    let full_span = self.get_span_from(start);
                    let head_span = self.type_expression_head_span(left_type_id);
                    let type_expression_id = self.make_type_infix_expression(
                        full_span,
                        head_span,
                        left_type_id,
                        right_operator,
                        operator_span,
                        right_type_id,
                    )?;

                    Expression::Type {
                        value: type_expression_id,
                    }
                }

                // type-only infix operators must not lower through value space
                InfixRightKind::InvalidValueTypeOperator => {
                    return Err(ParseError::unexpected(operator_span));
                }

                // normal value infix expressions
                InfixRightKind::Value => {
                    let right_expression_id = self.with_flags(
                        self.flags.with_expression_context(right_context),
                        |parser| parser.eat_expression_in_scope(),
                    )?;

                    self.make_value_infix_expression(
                        left_expression_id,
                        right_operator,
                        right_expression_id,
                    )?
                }
            };

            left_expression_id = self.insert_node(left_expression, self.get_span_from(start));
            left_is_parenthesized = false;

            // set main span to the operator
            let operator_main_span = if let Some(as_const_operator_end) = as_const_operator_end {
                Span::new(
                    operator_span.file,
                    operator_span.start,
                    as_const_operator_end,
                )
            } else {
                operator_span
            };
            self.tree
                .set_main_span(left_expression_id, operator_main_span);

            if let Some(value) = self.wrapped_type_expression_maybe(left_expression_id) {
                self.tree.set_main_span(value, operator_main_span);
            }

            // wrapper operators inherit the wrapped head
            if matches!(
                self.tree.get(left_expression_id),
                Expression::As { .. } | Expression::Satisfies { .. }
            ) {
                let head_span = self.expression_head_span(subject_id);
                self.tree.set_head_span(left_expression_id, head_span);
            }

            // use the subject identifier for type predicate spans
            if let Some(value) = self.wrapped_type_expression_maybe(left_expression_id)
                && matches!(self.tree.get(value), TypeExpression::Predicate { .. })
            {
                let subject_span = self
                    .tree
                    .get_main_span(subject_id)
                    .unwrap_or_else(|| self.tree.get_span(subject_id));
                self.tree.set_main_span(left_expression_id, subject_span);
                self.tree.set_main_span(value, subject_span);
            }
        }

        Ok(left_expression_id)
    }

    /// Parse one value-space tail continuation after infix parsing.
    ///
    /// Examples:
    /// ```
    /// value ? then_value : else_value
    /// match value { case => result }
    /// ```
    pub(super) fn eat_value_tail_continuation(
        &mut self,
        start: &ParserSpanStart,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let left_precedence = self.flags.left_precedence;

        // ternary sits between assignment and short-circuit expressions
        // type conditionals already consume `?` in type-space continuation parsing
        if left_precedence.is_none_or(|left_precedence| left_precedence < VALUE_TERNARY_PRECEDENCE)
            && self.peek_is(TokenType::Maybe)
        {
            self.bump(); // eat ?

            let then_flags = self
                .flags
                .not_in_position()
                .in_ternary_condition()
                .not_in_sequence_expression();
            let then_expression_id = self.eat_expression_with_context_unchecked(then_flags)?;
            self.eat_colon()?;

            let else_flags = self.flags.not_in_position().not_in_sequence_expression();
            let else_expression_id = self.eat_expression_with_context_unchecked(else_flags)?;
            let expression = Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression {
                    condition: left_expression_id,
                },
                then_expression: then_expression_id,
                else_expression: Some(else_expression_id),
            };
            left_expression_id = self.insert_node(expression, self.get_span_from(start));
        }

        // sequence expressions only exist in typed and untyped value space
        if left_precedence.is_none()
            && self.flags.allows_sequence_expression()
            && (self.language.is_typescript() || self.language.is_javascript())
            && self.peek_is(TokenType::Comma)
        {
            let mut expressions = vec![left_expression_id];
            while self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma

                let expression_flags = self.flags.not_in_position().not_in_sequence_expression();
                let expression_id = self.eat_expression_with_context_unchecked(expression_flags)?;
                expressions.push(expression_id);
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.insert_node(expression, self.get_span_from(start));
        }

        Ok(left_expression_id)
    }

    /// Parse postfix, infix, and tail continuation after one primary expression.
    ///
    /// Examples:
    /// ```
    /// value.method<T>() ? a : b
    /// value as string
    /// value is string
    /// Vector2 { x: 0, y: 1 }
    /// Foo extends Bar ? Baz : Qux
    /// ```
    pub(crate) fn eat_expression_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
        left_is_parenthesized: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // statement expressions do not accept continuation operators
        if self.statement_expression_stops_continuation(left_expression_id) {
            return Ok(left_expression_id);
        }

        // value-space continuations use postfix, infix, then tail parsing
        let (left_expression_id, left_is_parenthesized) =
            self.eat_value_postfix_continuation(start, left_expression_id, left_is_parenthesized)?;
        let left_expression_id =
            self.eat_infix_continuation(start, left_expression_id, left_is_parenthesized)?;

        self.eat_value_tail_continuation(start, left_expression_id)
    }

    /// Return true when the current token holds a valid dot-member name.
    #[inline]
    fn current_token_is_dot_member_name(&mut self) -> bool {
        let token = self.current_token();
        let token_type = token.token.ty;

        if token_type == TokenType::Identifier {
            return true;
        }

        token_type == TokenType::Literal
            && matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
    }

    /// Return true when the token after `.` is a valid member name.
    #[inline]
    fn next_token_is_dot_member_name(&mut self) -> bool {
        self.lookahead(|parser| {
            parser.bump();
            parser.current_token_is_dot_member_name()
        })
    }

    /// Return true when the token after `.` is `#name` without whitespace.
    #[inline]
    fn next_hash_has_adjacent_identifier(&mut self) -> bool {
        self.lookahead(|parser| {
            parser.bump();
            let hash_token = parser.current_token();
            if hash_token.token.ty != TokenType::Hash {
                return false;
            }

            parser.bump();
            let identifier_token = parser.current_token();
            identifier_token.token.ty == TokenType::Identifier
                && hash_token.span.end == identifier_token.span.start
        })
    }

    /// Return the postfix generic application position at the current cursor.
    fn postfix_generic_arguments_position_maybe(&mut self) -> Option<PostfixPosition> {
        // direct: `<...>` or `<<...>`
        if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
            return Some(PostfixPosition::Direct);
        }

        // indirect: `.<...>` or `.<<...>`
        if self.peek_is(TokenType::Dot)
            && self.lookahead(|parser| {
                parser.bump();
                matches!(
                    parser.peek_token_type(),
                    TokenType::LessThan | TokenType::ShiftLeft
                )
            })
        {
            return Some(PostfixPosition::Indirect);
        }

        None
    }

    /// Return one valid value postfix generic application position at the current cursor.
    fn value_postfix_generic_arguments_position_maybe(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> Option<PostfixPosition> {
        let position = self.postfix_generic_arguments_position_maybe()?;

        // generic postfixes are disabled in `new` receiver and tree contexts
        if matches!(self.tree.get(left_expression_id), Expression::New { .. }) {
            return None;
        }
        if self.flags.is_in_new_receiver() || self.flags.is_in_tree_literal() {
            return None;
        }
        if self.flags.is_in_tree_literal() || self.language.is_javascript() {
            return None;
        }

        // direct generic postfixes do not apply to optional chains
        if position == PostfixPosition::Direct
            && matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
        {
            return None;
        }

        // indirect generic postfixes require optional chaining receivers
        if position == PostfixPosition::Indirect
            && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
        {
            return None;
        }

        Some(position)
    }

    /// Return one valid type postfix generic application position at the current cursor.
    fn type_postfix_generic_arguments_position_maybe(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
    ) -> Option<PostfixPosition> {
        let position = self.postfix_generic_arguments_position_maybe()?;

        // postfix generic arguments are disabled in tree contexts
        if self.flags.is_in_tree_literal() || self.language.is_javascript() {
            return None;
        }

        // type space only permits reference-like instantiation shapes
        if !matches!(
            self.tree.get(left_type_id),
            TypeExpression::Reference { .. }
                | TypeExpression::Member { .. }
                | TypeExpression::Import { .. }
        ) {
            return None;
        }

        Some(position)
    }

    /// Speculatively parse one postfix generic argument list.
    fn try_eat_postfix_generic_arguments(
        &mut self,
        allow_object_literal: bool,
        position: PostfixPosition,
    ) -> Option<(ParserCheckpoint, u32, Vec<LocalNodeId<GenericArgument>>)> {
        // speculative boundary for optional chaining style generic arguments
        let speculative_start = self.checkpoint();
        let speculative_start_idx = self.tree.next_id();

        // indirect generic application consumes the committed dot token first
        if position == PostfixPosition::Indirect {
            self.bump(); // eat .
        }

        // parse `<...>` with regular speculative follow validation
        let generic_arguments = match self.try_eat_generic_arguments(allow_object_literal, false) {
            Some(generic_arguments) => generic_arguments,
            None => {
                self.restore(speculative_start, speculative_start_idx);
                return None;
            }
        };

        Some((speculative_start, speculative_start_idx, generic_arguments))
    }

    /// Speculatively parse postfix generic arguments into either call or instantiation.
    fn try_eat_value_postfix_generic_application(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // tagged object literals can follow postfix instantiations on the same receiver shapes
        let receiver_id = self.without_parentheses_expression(left_expression_id);
        let allow_object_literal = self
            .wrapped_type_expression_maybe(receiver_id)
            .is_some_and(|value| self.can_start_tagged_object_literal_type(value));

        let (_, _, generic_arguments) =
            match self.try_eat_postfix_generic_arguments(allow_object_literal, position) {
                Some(generic_arguments) => generic_arguments,
                None => return Ok(None),
            };

        // call with generic arguments
        if self.peek_is(TokenType::OpenParenthesis) {
            let expression_id =
                self.eat_call(left_expression_id, Some(generic_arguments), position)?;
            return Ok(Some(expression_id));
        }

        let expression_id = self.insert_node(
            Expression::Instantiation {
                left: left_expression_id,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(Some(expression_id))
    }

    /// Speculatively parse postfix generic arguments into one type instantiation.
    fn try_eat_type_postfix_generic_application(
        &mut self,
        start: &ParserSpanStart,
        left_type_id: LocalNodeId<TypeExpression>,
        position: PostfixPosition,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        let (speculative_start, speculative_start_idx, generic_arguments) =
            match self.try_eat_postfix_generic_arguments(false, position) {
                Some(generic_arguments) => generic_arguments,
                None => return Ok(None),
            };

        let type_expression = match self.tree.get(left_type_id).clone() {
            TypeExpression::Reference { path, .. } => TypeExpression::Reference {
                path,
                generic_arguments,
            },
            TypeExpression::Member { left, name, .. } => TypeExpression::Member {
                left,
                name,
                generic_arguments,
            },
            TypeExpression::Import {
                target,
                arguments,
                qualifier,
                ..
            } => TypeExpression::Import {
                target,
                arguments,
                qualifier,
                generic_arguments,
            },
            _ => {
                self.restore(speculative_start, speculative_start_idx);
                return Ok(None);
            }
        };

        let expression_id = self.insert_node(type_expression, self.get_span_from(start));

        Ok(Some(expression_id))
    }
}
