use destack_codegen_js as js;
use destack_repository::Target;

use super::super::linker::OutputModule;
use crate::{LinkResult, ScriptLinker};

/// One stateful rewriter for one linked script module.
pub(in super::super) struct Rewriter<'module, 'a> {
    /// The output target policy.
    pub(super) target: &'a Target,
    /// The linked module being minified.
    pub(super) module: &'module mut js::Module,
}

impl<'module, 'a> Rewriter<'module, 'a> {
    /// Create one rewriter for one linked script module.
    pub(in super::super) fn new(target: &'a Target, module: &'module mut js::Module) -> Self {
        Self { target, module }
    }

    /// Minify syntax forms within the owned linked module.
    pub(in super::super) fn minify_syntax(&mut self) {
        // equality and binary expressions
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            let js::Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            let rewritten = match operator {
                js::BinaryOperator::Equal | js::BinaryOperator::NotEqual => {
                    self.fold_comparison_expression(left, operator, right)
                }

                js::BinaryOperator::EqualStrict
                | js::BinaryOperator::NotEqualStrict
                | js::BinaryOperator::LessThan
                | js::BinaryOperator::LessThanOrEqual
                | js::BinaryOperator::GreaterThan
                | js::BinaryOperator::GreaterThanOrEqual
                | js::BinaryOperator::Or
                | js::BinaryOperator::And
                | js::BinaryOperator::Coalesce
                | js::BinaryOperator::Add
                | js::BinaryOperator::Subtract
                | js::BinaryOperator::Multiply
                | js::BinaryOperator::Divide
                | js::BinaryOperator::Remainder
                | js::BinaryOperator::Exponent
                | js::BinaryOperator::ShiftLeft
                | js::BinaryOperator::ShiftRight
                | js::BinaryOperator::UnsignedShiftRight
                | js::BinaryOperator::ElementwiseAnd
                | js::BinaryOperator::ElementwiseXor
                | js::BinaryOperator::ElementwiseOr => {
                    self.fold_binary_expression(left, operator, right)
                }

                _ => None,
            };

            let Some(rewritten) = rewritten else {
                continue;
            };
            let (rewritten_expression, rewritten_symbol_id) = rewritten;

            let expression = self.module.tree.get_mut(expression_id);
            *expression = rewritten_expression;

            if let Some(symbol_id) = rewritten_symbol_id {
                self.module.tree.set_symbol(expression_id, symbol_id);
            }
        }

        // ternaries
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            let js::Expression::IfTernary {
                condition,
                then_expression,
                else_expression: Some(else_expression),
            } = expression
            else {
                continue;
            };

            let rewritten = self.fold_ternary_expression(
                expression_id,
                condition,
                then_expression,
                else_expression,
            );
            let Some(rewritten) = rewritten else {
                continue;
            };
            let (rewritten_expression, rewritten_symbol_id) = rewritten;

            let expression = self.module.tree.get_mut(expression_id);
            *expression = rewritten_expression;

            if let Some(symbol_id) = rewritten_symbol_id {
                self.module.tree.set_symbol(expression_id, symbol_id);
            }
        }

        // sequence expressions
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            let js::Expression::SequenceExpression { expressions } = expression else {
                continue;
            };

            let Some(rewritten) = self.fold_sequence_expression(&expressions) else {
                continue;
            };
            let (rewritten_expression, rewritten_symbol_id) = rewritten;

            let expression = self.module.tree.get_mut(expression_id);
            *expression = rewritten_expression;

            if let Some(symbol_id) = rewritten_symbol_id {
                self.module.tree.set_symbol(expression_id, symbol_id);
            }
        }

        // unary expressions
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            let js::Expression::Unary { operator, right } = expression else {
                continue;
            };

            let Some(rewritten) = self.fold_unary_expression(operator, right) else {
                continue;
            };

            let expression = self.module.tree.get_mut(expression_id);
            *expression = rewritten;
        }

        // statement cleanup
        Self::elide_undefined_returns(self.module);
        Self::elide_undefined_let_initializers(self.module);
        Self::merge_adjacent_binding_statements(self.module);

        // property key spellings
        for property_id in self.module.tree.get_nodes::<js::Property>() {
            let property = self.module.tree.get(property_id).clone();

            match property {
                js::Property::Field { key, .. } => {
                    let Some(name) = self.rewritten_property_name(&key) else {
                        continue;
                    };

                    let property = self.module.tree.get_mut(property_id);
                    let js::Property::Field { key, .. } = property else {
                        continue;
                    };
                    Self::set_property_key_name(key, name);
                }
                js::Property::Method { key: Some(key), .. } => {
                    let Some(name) = self.rewritten_property_name(&key) else {
                        continue;
                    };

                    let property = self.module.tree.get_mut(property_id);
                    let js::Property::Method { key, .. } = property else {
                        continue;
                    };
                    let Some(key) = key else {
                        continue;
                    };
                    Self::set_property_key_name(key, name);
                }
                js::Property::Spread { .. } => {}
                js::Property::Method { key: None, .. } => {}
            }
        }

        // member key spellings
        for member_id in self.module.tree.get_nodes::<js::Member>() {
            let member = self.module.tree.get(member_id).clone();

            match member {
                js::Member::Field { key, .. } => {
                    let Some(name) = self.rewritten_property_name(&key) else {
                        continue;
                    };

                    let member = self.module.tree.get_mut(member_id);
                    let js::Member::Field { key, .. } = member else {
                        continue;
                    };
                    Self::set_property_key_name(key, name);
                }
                js::Member::Method { key: Some(key), .. } => {
                    let Some(name) = self.rewritten_property_name(&key) else {
                        continue;
                    };

                    let member = self.module.tree.get_mut(member_id);
                    let js::Member::Method { key, .. } = member else {
                        continue;
                    };
                    let Some(key) = key else {
                        continue;
                    };
                    Self::set_property_key_name(key, name);
                }
                js::Member::StaticBlock { .. } => {}
                js::Member::Method { key: None, .. } => {}
            }
        }

        // shorthand object fields
        if !self.target.minify.identifiers {
            self.use_object_shorthand_fields();
        }

        // member access spellings
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            match expression {
                js::Expression::Index {
                    position,
                    left,
                    right,
                } => {
                    if position != js::PostfixPosition::Direct {
                        continue;
                    }

                    let js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::String(name),
                    } = self.module.tree.get(right)
                    else {
                        continue;
                    };
                    let name = *name;

                    if !self.can_use_identifier_property_name(&self.module.strings.get(name)) {
                        continue;
                    }

                    let expression = self.module.tree.get_mut(expression_id);
                    *expression = js::Expression::Member { left, name };
                }

                js::Expression::Member { left, name } => {
                    if self.can_use_identifier_property_name(&self.module.strings.get(name)) {
                        continue;
                    }

                    let right = self.module.tree.insert_from(
                        js::Expression::ScalarLiteral {
                            value: js::ScalarLiteral::String(name),
                        },
                        expression_id,
                    );

                    let expression = self.module.tree.get_mut(expression_id);
                    *expression = js::Expression::Index {
                        position: js::PostfixPosition::Direct,
                        left,
                        right,
                    };
                }

                _ => {}
            }
        }

        // tiny path and scalar spellings
        for expression_id in self.module.tree.get_nodes::<js::Expression>() {
            let expression = self.module.tree.get(expression_id).clone();

            match expression {
                js::Expression::Path {
                    path,
                    generic_arguments,
                } => {
                    if !generic_arguments.is_empty() {
                        continue;
                    }

                    if let Some(rewritten) =
                        self.fold_global_infinity_reference(expression_id, path)
                    {
                        let expression = self.module.tree.get_mut(expression_id);
                        *expression = rewritten;
                    }
                }

                js::Expression::Unary {
                    operator: js::UnaryOperator::Typeof,
                    right,
                } => {
                    let Some(rewritten) = self.fold_typeof_literal(right) else {
                        continue;
                    };

                    let expression = self.module.tree.get_mut(expression_id);
                    *expression = rewritten;
                }

                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Undefined,
                } => {
                    let zero = self.module.tree.insert_from(
                        js::Expression::ScalarLiteral {
                            value: js::ScalarLiteral::Number(0.0),
                        },
                        expression_id,
                    );

                    let expression = self.module.tree.get_mut(expression_id);
                    *expression = js::Expression::Unary {
                        operator: js::UnaryOperator::Void,
                        right: zero,
                    };
                }

                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Boolean(value),
                } => {
                    let number = if value { 0.0 } else { 1.0 };
                    let right = self.module.tree.insert_from(
                        js::Expression::ScalarLiteral {
                            value: js::ScalarLiteral::Number(number),
                        },
                        expression_id,
                    );

                    let expression = self.module.tree.get_mut(expression_id);
                    *expression = js::Expression::Unary {
                        operator: js::UnaryOperator::Not,
                        right,
                    };
                }

                _ => {}
            }
        }
    }
}

impl ScriptLinker<'_> {
    /// Minify syntax forms across one linked output.
    pub(in super::super) fn minify_output_syntax(
        &self,
        modules: &mut [OutputModule],
    ) -> LinkResult<()> {
        for (_, module) in modules {
            let mut rewriter = Rewriter::new(self.target, module);
            rewriter.minify_syntax();
        }

        Ok(())
    }
}
