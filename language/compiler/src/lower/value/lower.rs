use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::block::LocalBinding;
use super::super::{BlockLowerer, ScalarType};

impl BlockLowerer<'_, '_> {
    /// Lower a value expression to its result value and type.
    pub(crate) fn lower_value_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::Parenthesized { expression } => self.lower_value_expression(*expression),
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                let binding = self.local_binding_for_symbol(expression_id, *target_symbol)?;
                let value = self.builder.use_variable(binding.variable);
                Ok((value, binding.ty))
            }
            Expression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => {
                    let value = self.builder.bconst(*value);
                    Ok((value, self.type_lowerer.ty_bool))
                }
                dir::ScalarLiteral::Integer(value) => {
                    match self.scalar_type_for_expression(expression_id) {
                        Some(ScalarType::Float { width }) => {
                            let value = self.builder.fconst(*value as f64, width as u8);
                            let ty = if width == 32 {
                                self.type_lowerer.ty_f32
                            } else {
                                self.type_lowerer.ty_f64
                            };
                            Ok((value, ty))
                        }
                        Some(ScalarType::SignedInt { width }) => {
                            let value = self.builder.iconst(*value, width as u8, true);
                            let ty = if width == 64 {
                                self.type_lowerer.ty_i64
                            } else {
                                self.type_lowerer.ty_i32
                            };
                            Ok((value, ty))
                        }
                        _ => Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: format!("unsupported scalar literal '{value:?}'"),
                        })?,
                    }
                }
                dir::ScalarLiteral::Float(value) => {
                    let width = match self.scalar_type_for_expression(expression_id) {
                        Some(ScalarType::Float { width }) => width,
                        _ => 64,
                    };
                    let value = self.builder.fconst(*value, width as u8);
                    let ty = if width == 32 {
                        self.type_lowerer.ty_f32
                    } else {
                        self.type_lowerer.ty_f64
                    };
                    Ok((value, ty))
                }
                _ => Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unsupported scalar literal '{value:?}'"),
                })?,
            },
            Expression::Cast {
                operator,
                value,
                target_type: _,
                source: _,
            } => {
                // capture the value expression id
                let value_id = *value;

                // lower the cast input first
                let (value, _) = self.lower_value_expression(value_id)?;

                // resolve the target type for the cast
                let target_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;

                // pick the mir cast operator
                let operator = self.lower_cast_operator(expression_id, *operator, value_id)?;

                // emit the cast when needed
                let value = if let Some(operator) = operator {
                    self.builder.cast(operator, value, target_type)
                } else {
                    value
                };

                Ok((value, target_type))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                // short-circuit logical operators need special control flow
                if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
                    return self.lower_logical_operator(expression_id, *operator, *left, *right);
                }

                // lower operands
                let (left_value, _) = self.lower_value_expression(*left)?;
                let (right_value, _) = self.lower_value_expression(*right)?;

                // get result type
                let result_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;

                // emit binary operation
                let op = self.lower_binary_operator(expression_id, *operator, *left)?;
                let value = self.builder.binary_op(op, left_value, right_value);

                // comparisons produce bool, others preserve operand type
                let ty = if op.is_comparison() {
                    self.type_lowerer.ty_bool
                } else {
                    result_type
                };

                Ok((value, ty))
            }
            Expression::Assign { left, right } => {
                // resolve the assignment target
                let target_symbol = match self.dir_tree.get(*left) {
                    Expression::LocalReference { target_symbol, .. } => *target_symbol,
                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported assignment target".to_string(),
                        })?;
                    }
                };
                let binding = self.local_binding_for_symbol(*left, target_symbol)?;

                // lower the assigned value
                let (value, value_type) = self.lower_value_expression(*right)?;

                // update the variable binding
                self.builder.define_variable(binding.variable, value);

                Ok((value, value_type))
            }
            Expression::Call {
                left,
                dynamic_arguments,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "static arguments are not supported".to_string(),
                    })?;
                }

                let callee_symbol = match self.dir_tree.get(*left) {
                    Expression::LocalReference { target_symbol, .. }
                    | Expression::ModuleReference { target_symbol, .. }
                    | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported call expression target".to_string(),
                        })?;
                    }
                };
                let function_id =
                    *self
                        .functions_by_symbol
                        .get(&callee_symbol)
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "missing function symbol".to_string(),
                        })?;

                let mut arguments = Vec::with_capacity(dynamic_arguments.len());
                for argument_id in dynamic_arguments {
                    let argument = self.dir_tree.get(*argument_id);
                    if !matches!(argument, dir::Argument::Positional { .. }) {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported non-positional argument".to_string(),
                        })?;
                    }
                    let (value, _) = self.lower_value_expression(argument.value())?;
                    arguments.push(value);
                }

                let result_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;
                let value = self.builder.call(function_id, arguments).ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "missing function call result type".to_string(),
                    }
                })?;
                Ok((value, result_type))
            }
            Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression_id, elements)
            }
            Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression_id, elements)
            }
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "static arguments on member access are not supported".to_string(),
                    })?;
                }
                self.lower_member_expression(expression_id, *left, *name)
            }
            Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: "missing index expression".to_string(),
                })?;
                self.lower_index_expression(expression_id, *left, index_expr)
            }
            Expression::Unary { operator, right } => {
                // lower operand and emit unary operation
                let (operand_value, operand_type) = self.lower_value_expression(*right)?;
                let op = self.lower_unary_operator(expression_id, *operator, *right)?;
                let value = self.builder.unary_op(op, operand_value);

                // logical NOT produces bool, other unary ops preserve type
                let ty = if matches!(operator, dir::UnaryOperator::Not) {
                    self.type_lowerer.ty_bool
                } else {
                    operand_type
                };

                Ok((value, ty))
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }

    /// Lower a tuple expression to an aggregate value.
    fn lower_tuple_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the tuple type
        let tuple_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value } | dir::Argument::Labeled { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "unsupported tuple element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the tuple
        let value = self.builder.tuple(tuple_type, element_values);
        Ok((value, tuple_type))
    }

    /// Lower an array expression to an array value.
    fn lower_array_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the array type
        let array_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "unsupported array element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the array
        let value = self.builder.array(array_type, element_values);
        Ok((value, array_type))
    }

    /// Lower a member access expression to a field_get.
    fn lower_member_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        field_name: destack_base::StringId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the aggregate value
        let (aggregate_value, aggregate_type) = self.lower_value_expression(left_id)?;

        // look up the field index in the MIR type
        let aggregate_mir_type = self.builder.tree().get(aggregate_type);
        let field_index = match aggregate_mir_type {
            mir::Type::Struct { fields } => {
                // find field by name - compare StringIds directly
                let fields_clone = fields.clone();
                fields_clone.iter().position(|field_id| {
                    let field = self.builder.tree().get(*field_id);
                    field.name == Some(field_name)
                })
            }
            mir::Type::Tuple { elements } => {
                // for tuples, resolve the field index from the inferred type
                // the index is determined during type checking
                self.resolve_tuple_field_index(expression_id, elements.len())
            }
            _ => None,
        }
        .ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id.into_global_any(self.module_id),
            message: "field not found in aggregate type".to_string(),
        })?;

        // get the result type
        let result_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;

        // emit field_get
        let value = self.builder.field_get(aggregate_value, field_index as u32);
        Ok((value, result_type))
    }

    /// Resolve a tuple field index from the member expression.
    ///
    /// For tuple member access like `tuple.0`, we need to parse the numeric index
    /// from the field name. This is stored in the type table from type checking.
    fn resolve_tuple_field_index(
        &self,
        expression_id: LocalNodeId<Expression>,
        tuple_len: usize,
    ) -> Option<usize> {
        // get the expression to find the field name
        let expression = self.dir_tree.get(expression_id);
        if let Expression::Member { name, .. } = expression {
            // try to parse the field name as a numeric index
            let name_str = self.strings.get(*name);
            let index = name_str.parse::<usize>().ok()?;
            if index < tuple_len {
                return Some(index);
            }
        }
        None
    }

    /// Lower an index expression to an element_get.
    fn lower_index_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        index_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the array value and index
        let (array_value, _array_type) = self.lower_value_expression(left_id)?;
        let (index_value, _index_type) = self.lower_value_expression(index_id)?;

        // get the result type (element type)
        let result_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;

        // emit element_get
        let value = self.builder.element_get(array_value, index_value);
        Ok((value, result_type))
    }

    /// Lower a binary operator.
    fn lower_binary_operator(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        operand_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::BinaryOperator> {
        let scalar_type =
            self.scalar_type_for_expression(operand_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;
        let is_float = matches!(scalar_type, ScalarType::Float { .. });
        let is_signed = matches!(scalar_type, ScalarType::SignedInt { .. });

        let op = match (operator, is_float, is_signed) {
            // integer arithmetic
            (dir::BinaryOperator::Add, false, _) => mir::BinaryOperator::Add,
            (dir::BinaryOperator::Subtract, false, _) => mir::BinaryOperator::Subtract,
            (dir::BinaryOperator::Multiply, false, _) => mir::BinaryOperator::Multiply,
            (dir::BinaryOperator::Divide, false, true) => mir::BinaryOperator::SignedDivide,
            (dir::BinaryOperator::Divide, false, false) => mir::BinaryOperator::UnsignedDivide,
            (dir::BinaryOperator::Remainder, false, true) => mir::BinaryOperator::SignedRemainder,
            (dir::BinaryOperator::Remainder, false, false) => {
                mir::BinaryOperator::UnsignedRemainder
            }

            // float arithmetic
            (dir::BinaryOperator::Add, true, _) => mir::BinaryOperator::FloatAdd,
            (dir::BinaryOperator::Subtract, true, _) => mir::BinaryOperator::FloatSubtract,
            (dir::BinaryOperator::Multiply, true, _) => mir::BinaryOperator::FloatMultiply,
            (dir::BinaryOperator::Divide, true, _) => mir::BinaryOperator::FloatDivide,

            // bitwise operators (integers only)
            (dir::BinaryOperator::ElementwiseAnd, false, _) => mir::BinaryOperator::And,
            (dir::BinaryOperator::ElementwiseOr, false, _) => mir::BinaryOperator::Or,
            (dir::BinaryOperator::ElementwiseXor, false, _) => mir::BinaryOperator::Xor,

            // shift operators (integers only)
            (dir::BinaryOperator::ShiftLeft, false, _) => mir::BinaryOperator::ShiftLeft,
            (dir::BinaryOperator::ShiftRight, false, _) => mir::BinaryOperator::ArithmeticShiftRight,
            (dir::BinaryOperator::UnsignedShiftRight, false, _) => {
                mir::BinaryOperator::LogicalShiftRight
            }

            // integer comparison
            (dir::BinaryOperator::Equal, false, _) => mir::BinaryOperator::Equal,
            (dir::BinaryOperator::NotEqual, false, _) => mir::BinaryOperator::NotEqual,
            (dir::BinaryOperator::LessThan, false, true) => mir::BinaryOperator::SignedLessThan,
            (dir::BinaryOperator::LessThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedLessEqual
            }
            (dir::BinaryOperator::GreaterThan, false, true) => {
                mir::BinaryOperator::SignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedGreaterEqual
            }
            (dir::BinaryOperator::LessThan, false, false) => mir::BinaryOperator::UnsignedLessThan,
            (dir::BinaryOperator::LessThanOrEqual, false, false) => {
                mir::BinaryOperator::UnsignedLessEqual
            }
            (dir::BinaryOperator::GreaterThan, false, false) => {
                mir::BinaryOperator::UnsignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, false, false) => {
                mir::BinaryOperator::UnsignedGreaterEqual
            }

            // float comparison
            (dir::BinaryOperator::Equal, true, _) => mir::BinaryOperator::FloatEqual,
            (dir::BinaryOperator::NotEqual, true, _) => mir::BinaryOperator::FloatNotEqual,
            (dir::BinaryOperator::LessThan, true, _) => mir::BinaryOperator::FloatLessThan,
            (dir::BinaryOperator::LessThanOrEqual, true, _) => mir::BinaryOperator::FloatLessEqual,
            (dir::BinaryOperator::GreaterThan, true, _) => mir::BinaryOperator::FloatGreaterThan,
            (dir::BinaryOperator::GreaterThanOrEqual, true, _) => {
                mir::BinaryOperator::FloatGreaterEqual
            }

            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unsupported binary operator '{operator:?}'"),
                });
            }
        };

        Ok(op)
    }

    /// Lower a unary operator.
    fn lower_unary_operator(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::UnaryOperator,
        operand_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::UnaryOperator> {
        let scalar_type = self.scalar_type_for_expression(operand_id);
        let is_float = matches!(scalar_type, Some(ScalarType::Float { .. }));

        let op = match (operator, is_float) {
            // negation
            (dir::UnaryOperator::Negate, false) => mir::UnaryOperator::Negate,
            (dir::UnaryOperator::Negate, true) => mir::UnaryOperator::FloatNegate,

            // bitwise NOT (~) and logical NOT (!) both use the same MIR op
            (dir::UnaryOperator::ElementwiseNot, _) | (dir::UnaryOperator::Not, _) => {
                mir::UnaryOperator::Not
            }

            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unsupported unary operator '{operator:?}'"),
                });
            }
        };

        Ok(op)
    }

    /// Resolve the MIR type for a typed expression.
    fn mir_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // first try scalar types
        if let Some(scalar_type) = self.scalar_type_for_expression(expression_id) {
            return match scalar_type {
                ScalarType::Bool => Some(self.type_lowerer.ty_bool),
                ScalarType::SignedInt { width: 32 } => Some(self.type_lowerer.ty_i32),
                ScalarType::SignedInt { width: 64 } => Some(self.type_lowerer.ty_i64),
                ScalarType::Float { width: 32 } => Some(self.type_lowerer.ty_f32),
                ScalarType::Float { width: 64 } => Some(self.type_lowerer.ty_f64),
                _ => None,
            };
        }

        // for non-scalar types, check the type cache
        let type_id = self.dir_type_id_for_expression(expression_id)?;
        self.type_lowerer.type_cache.get(&type_id).copied()
    }

    /// Resolve the scalar type for a typed expression.
    pub(crate) fn scalar_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<ScalarType> {
        let type_id = self.dir_type_id_for_expression(expression_id)?;
        let dir_type = self.types.get_type(type_id);
        self.type_lowerer.scalar_type_for_dir_type(dir_type)
    }

    /// Resolve the DIR type id for a typed expression.
    fn dir_type_id_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        let expression = self.dir_tree.get(expression_id);
        let type_id = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => self
                .types
                .get_declared_or_inferred_type_id(expression_id.into_global_any(self.module_id))
                .or_else(|| self.types.get_value_type_id(*target_symbol))
                .or_else(|| self.local_symbol_type_id(*target_symbol)),
            _ => {
                let node_id = expression_id.into_global_any(self.module_id);
                self.types.get_declared_or_inferred_type_id(node_id)
            }
        }?;
        Some(type_id)
    }

    /// Resolve a local symbol to its declared or inferred type.
    fn local_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<dir::LocalTypeId> {
        if symbol_id.module_id != self.module_id {
            return None;
        }

        let symbol = self.symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        self.types
            .get_declared_or_inferred_type_id(primary_declaration)
    }

    /// Resolve a local binding for a symbol reference.
    fn local_binding_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<LocalBinding> {
        let binding = self.locals_by_symbol.get(&target_symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: "missing local reference target symbol".to_string(),
            }
        })?;

        Ok(*binding)
    }

    /// Lower a short-circuit logical operator (&& or ||).
    ///
    /// Short-circuit evaluation means:
    /// - `a && b`: if `a` is false, result is false without evaluating `b`
    /// - `a || b`: if `a` is true, result is true without evaluating `b`
    fn lower_logical_operator(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // evaluate LHS first
        let (lhs_value, lhs_type) = self.lower_value_expression(left_id)?;

        // verify LHS is boolean
        if lhs_type != self.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: "logical operator requires boolean operands".to_string(),
            });
        }

        // create blocks for short-circuit evaluation
        let shortcircuit_block = self.builder.create_block();
        let rhs_block = self.builder.create_block();
        let merge_block = self.builder.create_block();

        // create a variable to hold the result (SSA construction will merge)
        let result_variable = self.builder.create_variable(self.type_lowerer.ty_bool);

        // branch based on operator semantics
        match operator {
            dir::BinaryOperator::And => {
                // a && b: if a is true, evaluate b; else short-circuit to false
                self.builder.branch(lhs_value, rhs_block, shortcircuit_block);
            }
            dir::BinaryOperator::Or => {
                // a || b: if a is true, short-circuit to true; else evaluate b
                self.builder.branch(lhs_value, shortcircuit_block, rhs_block);
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unexpected logical operator '{operator:?}'"),
                });
            }
        }

        // short-circuit block: set result to constant and jump to merge
        self.builder.switch_to_block(shortcircuit_block);
        let shortcircuit_value = match operator {
            dir::BinaryOperator::And => self.builder.bconst(false),
            dir::BinaryOperator::Or => self.builder.bconst(true),
            _ => unreachable!(),
        };
        self.builder.define_variable(result_variable, shortcircuit_value);
        self.builder.jump(merge_block);

        // rhs block: evaluate rhs, set result, jump to merge
        self.builder.switch_to_block(rhs_block);
        let (rhs_value, rhs_type) = self.lower_value_expression(right_id)?;

        // verify RHS is boolean
        if rhs_type != self.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: "logical operator requires boolean operands".to_string(),
            });
        }

        // set result and jump to merge
        self.builder.define_variable(result_variable, rhs_value);
        self.builder.jump(merge_block);

        // merge block: use the result variable (SSA will create block parameter)
        self.builder.switch_to_block(merge_block);
        let result_value = self.builder.use_variable(result_variable);

        Ok((result_value, self.type_lowerer.ty_bool))
    }
}
