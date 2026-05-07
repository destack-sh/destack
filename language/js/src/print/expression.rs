use super::printer::Printer;
use crate::tree::Precedence;
use crate::{
    ArrayElement, Asynchrony, Expression, JsPrintResult, Keyword, LocalNodeId, Parameter,
    PostfixPosition, ScalarLiteral,
};
use destack_source::NodeSpanType;

impl<'a> Printer<'a> {
    /// Print one expression.
    pub(crate) fn print_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        parent_precedence: Precedence,
    ) -> JsPrintResult<()> {
        // preserve explicit grouping when postfix continuation would change
        if let Expression::Parenthesized { expression } = expression
            && self.should_preserve_parenthesized_grouping(*expression, parent_precedence)
        {
            self.write_punct("(");
            self.print_expression_id(*expression)?;
            self.write_punct(")");

            return Ok(());
        }

        let expression = Expression::without_parentheses(self.tree, expression);
        let current_precedence = expression.precedence();
        let needs_wrap = current_precedence < parent_precedence;

        if needs_wrap {
            self.write_punct("(");
        }

        if !self.include_types && expression.is_type_only(self.tree) {
            return Ok(());
        }

        match expression {
            Expression::Declaration { declaration } => {
                self.print_declaration_id(*declaration)?;
            }
            Expression::ArrowFunction { signature, body } => {
                if signature.asynchrony == Asynchrony::Async {
                    self.write_keyword(Keyword::Async);
                }

                if signature.is_generator {
                    self.write_punct("*");
                }

                if self.include_types && !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                if self.can_print_bare_arrow_parameter(signature) {
                    let parameter = self.tree.get(signature.parameters[0]);
                    let Parameter::Named { name, .. } = parameter else {
                        unreachable!("bare arrow parameters must be simple named parameters");
                    };

                    self.write_string_id(*name);
                } else {
                    self.write_punct("(");
                    self.print_function_signature_parameters(signature)?;
                    self.write_punct(")");
                }

                if self.include_types
                    && let Some(return_type) = signature.return_type
                {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }

                self.write_punct("=>");

                match body {
                    crate::ArrowFunctionBody::Expression(body) => {
                        self.print_expression_id_with_precedence(*body, Precedence::Assignment)?;
                    }
                    crate::ArrowFunctionBody::Block(body) => {
                        self.print_block_id(*body)?;
                    }
                }
            }
            Expression::Path {
                path,
                generic_arguments,
            } => {
                self.print_path(path);

                if self.include_types && !generic_arguments.is_empty() {
                    self.print_type_arguments(generic_arguments)?;
                }
            }
            Expression::ImportMeta => {
                self.write_punct("import.meta");
            }
            Expression::This => {
                self.write_keyword(Keyword::This);
            }
            Expression::Super => {
                self.write_keyword(Keyword::Super);
            }
            Expression::NewTarget => {
                self.write_punct("new.target");
            }
            Expression::PrivateIdentifier { name } => {
                self.write_punct("#");
                self.write_string_id(*name);
            }
            Expression::ScalarLiteral { value } => {
                self.print_scalar_literal(value);
            }
            Expression::TemplateLiteral { value } => {
                self.print_template_literal(value)?;
            }
            Expression::ArrayLiteral { elements } => {
                self.write_punct("[");
                self.print_array_element_list(elements)?;
                self.write_punct("]");
            }
            Expression::SequenceExpression { expressions } => {
                self.print_expression_list(expressions)?;
            }
            Expression::ObjectLiteral { properties } => {
                self.write_punct("{");
                self.print_property_list(properties)?;
                self.write_punct("}");
            }
            Expression::As {
                expression,
                target_type,
            } => {
                self.print_expression_id_with_precedence(*expression, Precedence::Compare)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::As);
                self.write_punct(" ");
                self.print_type_id(*target_type)?;
            }
            Expression::Satisfies {
                expression,
                target_type,
            } => {
                self.print_expression_id_with_precedence(*expression, Precedence::Compare)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::Satisfies);
                self.write_punct(" ");
                self.print_type_id(*target_type)?;
            }
            Expression::InstanceOf { value, target } => {
                self.print_expression_id_with_precedence(*value, Precedence::Compare)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::InstanceOf);
                self.write_punct(" ");
                self.print_expression_id_with_precedence(*target, Precedence::Compare.tighter())?;
            }
            Expression::Await { value } => {
                self.write_keyword(Keyword::Await);
                self.print_expression_id_with_precedence(*value, Precedence::Prefix)?;
            }
            Expression::Yield { is_delegate, value } => {
                self.write_keyword(Keyword::Yield);

                if *is_delegate {
                    self.write_punct("*");
                }

                if let Some(value) = value {
                    self.print_expression_id_with_precedence(*value, Precedence::Assignment)?;
                }
            }
            Expression::Unary { operator, right } => {
                self.write_unary_operator(*operator);

                self.print_expression_id_with_precedence(*right, Precedence::Prefix)?;
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let precedence = operator.precedence();
                let left_precedence = if *operator == crate::BinaryOperator::Exponent {
                    precedence.tighter()
                } else {
                    precedence
                };
                let right_precedence = if *operator == crate::BinaryOperator::Exponent {
                    precedence
                } else {
                    precedence.tighter()
                };

                if *operator == crate::BinaryOperator::Coalesce
                    && self.is_logical_coalesce_operand(*left)
                {
                    self.write_punct("(");
                    self.print_expression_id(*left)?;
                    self.write_punct(")");
                } else {
                    self.print_expression_id_with_precedence(*left, left_precedence)?;
                }

                self.write_binary_operator(*operator);

                if *operator == crate::BinaryOperator::Coalesce
                    && self.is_logical_coalesce_operand(*right)
                {
                    self.write_punct("(");
                    self.print_expression_id(*right)?;
                    self.write_punct(")");
                } else {
                    self.print_expression_id_with_precedence(*right, right_precedence)?;
                }
            }
            Expression::Assign { left, right } => {
                self.print_assign_pattern_id(*left)?;
                self.write_punct("=");
                self.print_expression_id_with_precedence(*right, Precedence::Assignment)?;
            }
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;
                self.write_assign_operator(*operator);
                self.print_expression_id_with_precedence(*right, Precedence::Assignment)?;
            }
            Expression::Maybe { position, left } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if *position == PostfixPosition::Indirect {
                    self.write_punct(".");
                }

                self.write_punct("?");
            }
            Expression::Must { position, left } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if *position == PostfixPosition::Indirect {
                    self.write_punct(".");
                }

                self.write_punct("!");
            }
            Expression::Member { left, name } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;
                self.write_punct(".");
                self.write_string_id(*name);
            }
            Expression::PrivateMember { left, name } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;
                self.write_punct(".#");
                self.write_string_id(*name);
            }
            Expression::Index {
                position,
                left,
                right,
            } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if *position == PostfixPosition::Indirect {
                    self.write_punct(".");
                }

                self.write_punct("[");
                self.print_expression_id_with_precedence(*right, Precedence::Lowest)?;
                self.write_punct("]");
            }
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if self.include_types {
                    self.print_type_arguments(generic_arguments)?;
                }
            }
            Expression::Call {
                position,
                left,
                generic_arguments,
                arguments,
            } => {
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if *position == PostfixPosition::Indirect {
                    self.write_punct(".");
                }

                if self.include_types && !generic_arguments.is_empty() {
                    self.print_type_arguments(generic_arguments)?;
                }

                self.write_punct("(");
                self.print_argument_list(arguments)?;
                self.write_punct(")");
            }
            Expression::ImportCall {
                target, arguments, ..
            } => {
                let target_expression = self.tree.get(*target);
                let target_span = self.source_part_span(expression_id.id, NodeSpanType::Main);

                self.write_punct("import");
                self.write_punct("(");

                if let Expression::ScalarLiteral {
                    value: ScalarLiteral::String(value),
                } = target_expression
                {
                    self.write_string_literal_with_source_span(*value, target_span);
                } else {
                    self.print_expression_id_with_precedence(*target, Precedence::Lowest)?;
                }

                if !arguments.is_empty() {
                    self.write_punct(",");
                    self.print_argument_list(arguments)?;
                }

                self.write_punct(")");
            }
            Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                self.write_keyword(Keyword::New);
                self.print_expression_id_with_precedence(*left, Precedence::Postfix)?;

                if self.include_types && !generic_arguments.is_empty() {
                    self.print_type_arguments(generic_arguments)?;
                }

                self.write_punct("(");
                self.print_argument_list(arguments)?;
                self.write_punct(")");
            }
            Expression::IfTernary {
                condition,
                then_expression,
                else_expression,
            } => {
                self.print_expression_id_with_precedence(*condition, Precedence::Conditional)?;
                self.write_punct("?");
                self.print_expression_id_with_precedence(*then_expression, Precedence::Assignment)?;
                self.write_punct(":");

                if let Some(else_expression) = else_expression {
                    self.print_expression_id_with_precedence(
                        *else_expression,
                        Precedence::Assignment,
                    )?;
                }
            }
            Expression::Parenthesized { .. } => {
                unreachable!("parenthesized expressions are unwrapped")
            }
            Expression::Missing => {
                self.write_punct("/* MISSING */");
            }
            Expression::Stub => {}
            Expression::Error => {
                self.write_punct("/* ERROR */");
            }
        }

        if needs_wrap {
            self.write_punct(")");
        }

        Ok(())
    }

    /// Print one array element.
    pub(crate) fn print_array_element(
        &mut self,
        array_element: &ArrayElement,
    ) -> JsPrintResult<()> {
        match array_element {
            ArrayElement::Expression { value } => {
                self.print_expression_id(*value)?;
            }
            ArrayElement::Spread { value } => {
                self.write_punct("...");
                self.print_expression_id(*value)?;
            }
            ArrayElement::Elision => {}
        }

        Ok(())
    }

    /// Return whether one arrow function may omit parameter parentheses.
    pub(crate) fn can_print_bare_arrow_parameter(
        &self,
        signature: &crate::FunctionSignature,
    ) -> bool {
        let parameters = signature.parameters.as_slice();

        if self.include_types || signature.this_parameter.is_some() || parameters.len() != 1 {
            return false;
        }

        matches!(
            self.tree.get(parameters[0]),
            Parameter::Named {
                modifiers: None,
                ty: None,
                default: None,
                ..
            }
        )
    }

    /// Return whether one coalesce operand must stay grouped against logical operators.
    fn is_logical_coalesce_operand(&self, expression_id: LocalNodeId<Expression>) -> bool {
        match self.tree.get(expression_id) {
            Expression::Parenthesized { expression } => {
                self.is_logical_coalesce_operand(*expression)
            }
            Expression::Binary { operator, .. } => {
                matches!(
                    operator,
                    crate::BinaryOperator::And | crate::BinaryOperator::Or
                )
            }
            _ => false,
        }
    }

    /// Return whether one explicit parenthesized expression must stay grouped.
    fn should_preserve_parenthesized_grouping(
        &self,
        expression_id: LocalNodeId<Expression>,
        parent_precedence: Precedence,
    ) -> bool {
        if parent_precedence != Precedence::Postfix {
            return false;
        }

        self.expression_contains_optional_postfix_chain(expression_id)
    }

    /// Return whether one expression contains one optional postfix chain.
    fn expression_contains_optional_postfix_chain(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        match self.tree.get(expression_id) {
            Expression::Parenthesized { expression } => {
                self.expression_contains_optional_postfix_chain(*expression)
            }
            Expression::Maybe { .. } => true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::New { left, .. } => {
                self.expression_contains_optional_postfix_chain(*left)
            }
            Expression::Index { left, .. } | Expression::Call { left, .. } => {
                self.expression_contains_optional_postfix_chain(*left)
            }
            _ => false,
        }
    }
}
