use crate::{
    Argument, ArrayElement, ArrowFunctionBody, Asynchrony, BinaryOperator, Expression, LocalNodeId,
    Module, Precedence, UpdatePosition,
};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Expression {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        printer.expression_value(self, Precedence::Lowest, module)
    }
}

impl Printer {
    /// Print one expression under a required parent precedence.
    pub(super) fn expression(
        &mut self,
        id: LocalNodeId<Expression>,
        parent_precedence: Precedence,
        module: &Module,
    ) -> Result<(), PrintError> {
        let expression = module.tree.get(id);
        let provenance = module.tree.provenance(id);

        self.write_node(provenance, |printer| {
            printer.expression_value(expression, parent_precedence, module)
        })
    }

    /// Print an expression in statement position.
    pub(super) fn expression_statement(
        &mut self,
        id: LocalNodeId<Expression>,
        module: &Module,
    ) -> Result<(), PrintError> {
        let expression = module.tree.get(id);
        let needs_parentheses = expression.needs_statement_parentheses(&module.tree);

        if needs_parentheses {
            self.token("(")?;
        }

        self.expression(id, Precedence::Lowest, module)?;

        if needs_parentheses {
            self.token(")")?;
        }

        Ok(())
    }

    /// Print one expression in an ECMAScript `NoIn` context.
    pub(super) fn expression_without_in(
        &mut self,
        id: LocalNodeId<Expression>,
        module: &Module,
    ) -> Result<(), PrintError> {
        let needs_parentheses = module.tree.get(id).needs_no_in_parentheses(&module.tree);

        if needs_parentheses {
            self.token("(")?;
        }
        self.expression(id, Precedence::Lowest, module)?;
        if needs_parentheses {
            self.token(")")?;
        }

        Ok(())
    }

    /// Print one expression value.
    fn expression_value(
        &mut self,
        expression: &Expression,
        parent_precedence: Precedence,
        module: &Module,
    ) -> Result<(), PrintError> {
        let current_precedence = expression.precedence();
        let needs_parentheses = current_precedence < parent_precedence;

        if needs_parentheses {
            self.token("(")?;
        }

        match expression {
            Expression::Declaration { declaration } => self.node(*declaration, module)?,
            Expression::Identifier { identifier } => self.identifier(*identifier, module)?,
            Expression::ImportMeta => {
                self.word("import")?;
                self.token(".")?;
                self.word("meta")?;
            }
            Expression::This => self.word("this")?,
            Expression::Super => self.word("super")?,
            Expression::Literal { value } => self.literal(value, module)?,
            Expression::TemplateLiteral { value } => self.template(value, module)?,
            Expression::ArrayLiteral { elements } => self.array(elements, module)?,
            Expression::SequenceExpression { expressions } => {
                for (index, expression) in expressions.iter().copied().enumerate() {
                    if index > 0 {
                        self.token(",")?;
                    }

                    self.expression(expression, Precedence::Assignment, module)?;
                }
            }
            Expression::ObjectLiteral { properties } => {
                self.token("{")?;
                self.properties(properties, module)?;
                self.token("}")?;
            }
            Expression::Parenthesized { expression } => {
                self.token("(")?;
                self.expression(*expression, Precedence::Lowest, module)?;
                self.token(")")?;
            }
            Expression::Await { value } => {
                self.word("await")?;
                self.expression(*value, Precedence::Prefix, module)?;
            }
            Expression::Yield { is_delegate, value } => {
                self.word("yield")?;
                if *is_delegate {
                    self.token("*")?;
                }
                if let Some(value) = value {
                    self.expression(*value, Precedence::Assignment, module)?;
                }
            }
            Expression::Unary { operator, right } => {
                if operator.is_keyword() {
                    self.word(operator.as_str())?;
                } else {
                    self.token(operator.as_str())?;
                }
                self.expression(*right, Precedence::Prefix, module)?;
            }
            Expression::Update {
                place,
                operator,
                position,
            } => match position {
                UpdatePosition::Prefix => {
                    self.token(operator.as_str())?;
                    self.node(*place, module)?;
                }
                UpdatePosition::Postfix => {
                    self.node(*place, module)?;
                    self.token(operator.as_str())?;
                }
            },
            Expression::Binary {
                left,
                operator,
                right,
            } => self.binary(*left, *operator, *right, module)?,
            Expression::Assign { left, right } => {
                self.node(*left, module)?;
                self.token("=")?;
                self.expression(*right, Precedence::Assignment, module)?;
            }
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                self.node(*left, module)?;
                self.token(operator.as_str())?;
                self.expression(*right, Precedence::Assignment, module)?;
            }
            Expression::PrivateIn { identifier, object } => {
                self.token("#")?;
                self.identifier(*identifier, module)?;
                self.word("in")?;
                self.expression(*object, Precedence::Compare.tighter(), module)?;
            }
            Expression::Member {
                object,
                property,
                is_optional,
            } => {
                let needs_extra_dot = self.member_object(*object, *is_optional, module)?;
                if needs_extra_dot {
                    self.token(".")?;
                }
                self.token(if *is_optional { "?." } else { "." })?;
                self.identifier_name(*property, module)?;
            }
            Expression::PrivateMember { object, property } => {
                let needs_extra_dot = self.member_object(*object, false, module)?;
                if needs_extra_dot {
                    self.token(".")?;
                }
                self.token(".")?;
                self.token("#")?;
                self.identifier(*property, module)?;
            }
            Expression::Index {
                left,
                right,
                is_optional,
            } => {
                self.expression(*left, Precedence::Call, module)?;
                self.token(if *is_optional { "?.[" } else { "[" })?;
                self.expression(*right, Precedence::Lowest, module)?;
                self.token("]")?;
            }
            Expression::Call {
                left,
                arguments,
                is_optional,
            } => {
                self.expression(*left, Precedence::Call, module)?;
                self.token(if *is_optional { "?.(" } else { "(" })?;
                self.arguments(arguments, module)?;
                self.token(")")?;
            }
            Expression::ImportCall { specifier, options } => {
                self.word("import")?;
                self.token("(")?;
                self.expression(*specifier, Precedence::Assignment, module)?;
                if let Some(options) = options {
                    self.token(",")?;
                    self.expression(*options, Precedence::Assignment, module)?;
                }
                self.token(")")?;
            }
            Expression::New { left, arguments } => {
                self.word("new")?;
                let needs_parentheses = module.tree.get(*left).is_optional_chain(&module.tree);
                if needs_parentheses {
                    self.token("(")?;
                }
                self.expression(*left, Precedence::Member, module)?;
                if needs_parentheses {
                    self.token(")")?;
                }
                self.token("(")?;
                self.arguments(arguments, module)?;
                self.token(")")?;
            }
            Expression::ArrowFunction {
                asynchrony,
                parameters,
                rest,
                body,
            } => {
                if *asynchrony == Asynchrony::Async {
                    self.word("async")?;
                }
                self.arrow_parameters(parameters, *rest, module)?;
                self.token("=>")?;

                match body {
                    ArrowFunctionBody::Expression(body) => {
                        let body_expression = module.tree.get(*body);
                        let needs_parentheses =
                            body_expression.needs_arrow_parentheses(&module.tree);
                        if needs_parentheses {
                            self.token("(")?;
                        }
                        self.expression(*body, Precedence::Assignment, module)?;
                        if needs_parentheses {
                            self.token(")")?;
                        }
                    }
                    ArrowFunctionBody::Block(body) => self.node(*body, module)?,
                }
            }
            Expression::IfTernary {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression(*condition, Precedence::Conditional.tighter(), module)?;
                self.token("?")?;
                self.expression(*then_expression, Precedence::Assignment, module)?;
                self.token(":")?;
                self.expression(*else_expression, Precedence::Assignment, module)?;
            }
        }

        if needs_parentheses {
            self.token(")")?;
        }

        Ok(())
    }

    /// Print one binary operation with ECMAScript associativity restrictions.
    fn binary(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
        module: &Module,
    ) -> Result<(), PrintError> {
        let precedence = operator.precedence();
        let left_precedence = if operator == BinaryOperator::Exponent {
            Precedence::Postfix
        } else {
            precedence
        };
        let right_precedence = if operator == BinaryOperator::Exponent {
            precedence
        } else {
            precedence.tighter()
        };

        self.coalesce_operand(left, operator, left_precedence, module)?;
        if operator.is_keyword() {
            self.word(operator.as_str())?;
        } else {
            self.token(operator.as_str())?;
        }
        self.coalesce_operand(right, operator, right_precedence, module)
    }

    /// Parenthesize logical operands nested directly under nullish coalescing.
    fn coalesce_operand(
        &mut self,
        id: LocalNodeId<Expression>,
        parent: BinaryOperator,
        precedence: Precedence,
        module: &Module,
    ) -> Result<(), PrintError> {
        let expression = module.tree.get(id);
        let needs_parentheses = parent == BinaryOperator::Coalesce
            && matches!(
                expression,
                Expression::Binary {
                    operator: BinaryOperator::And | BinaryOperator::Or,
                    ..
                }
            );

        if needs_parentheses {
            self.token("(")?;
        }
        self.expression(id, precedence, module)?;
        if needs_parentheses {
            self.token(")")?;
        }

        Ok(())
    }

    /// Print one member object and report whether a decimal member needs another dot.
    pub(super) fn member_object(
        &mut self,
        id: LocalNodeId<Expression>,
        is_optional: bool,
        module: &Module,
    ) -> Result<bool, PrintError> {
        let expression = module.tree.get(id);
        self.expression(id, Precedence::Call, module)?;

        let needs_extra_dot = expression.needs_decimal_member_dot(is_optional);

        Ok(needs_extra_dot)
    }

    /// Print one array literal, preserving trailing elisions.
    fn array(
        &mut self,
        elements: &[LocalNodeId<ArrayElement>],
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("[")?;
        for (index, element) in elements.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(element, module)?;
        }
        if elements
            .last()
            .is_some_and(|element| matches!(module.tree.get(*element), ArrayElement::Elision))
        {
            self.token(",")?;
        }
        self.token("]")
    }

    /// Print one call argument list.
    fn arguments(
        &mut self,
        arguments: &[LocalNodeId<Argument>],
        module: &Module,
    ) -> Result<(), PrintError> {
        for (index, argument) in arguments.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(argument, module)?;
        }

        Ok(())
    }
}
