use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one value expression, honoring its checked coercion.
    pub(in crate::lower) fn lower_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // coerce into union carriers before the value lowers at its own type
        if let Some(coercion) = self.lowerer.coercion(expression)
            && coercion.kind == dir::CoercionKind::Union
        {
            return self.lower_union_coercion(expression, &coercion);
        }

        // coerce borrows before their places load as values
        if let Some(coercion) = self.lowerer.coercion(expression)
            && coercion.kind == dir::CoercionKind::Borrow
        {
            return self.lower_borrow_coercion(expression, &coercion);
        }

        let value = self.lower_expression_value(expression)?;

        // reject unapplied coercions: constants already materialize at their carrier
        if let Some(coercion) = self.lowerer.coercion(expression)
            && !matches!(self.lowerer.node_type(expression)?, dir::Type::Literal(_))
        {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("an implicit {} coercion", coercion.kind.as_str()),
            }
            .into());
        }

        Ok(value)
    }

    /// Lower one value coerced into a borrow of its place or reference.
    fn lower_borrow_coercion(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        coercion: &dir::Coercion,
    ) -> CompilerResult<mir::Value> {
        let target = self
            .lowerer
            .lower_type_id(self.builder.tree_mut(), coercion.target)?;

        self.lower_borrowed_place(expression, target)
    }

    /// Lower one expression into a borrow of its place or reference.
    pub(in crate::lower) fn lower_borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // reference sources borrow as a zero-cost kind change
        let source = self.lowerer.node_type_id(expression)?;
        if self.lowerer.type_is_reference(source)? {
            let value = self.lower_expression_value(expression)?;

            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target));
        }

        // value sources borrow the storage holding them
        match self.lowerer.source().tree().get(expression).clone() {
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.lowerer.source);
                let symbol = self.lowerer.resolved_symbol(node)?;
                match self.values.get(&symbol.local_id).copied() {
                    Some(Binding::Local(local)) => Ok(self.builder.local_addr(local, target)),
                    // borrowed parameters gain a frame home on first borrow
                    Some(Binding::Value(value)) => {
                        let ty = self.lowerer.symbol_type(symbol)?;
                        let slot = self.lowerer.lower_type_id(self.builder.tree_mut(), ty)?;
                        let local = self.builder.local(slot, mir::Mutability::Mutable);
                        self.builder.local_set(local, value);
                        self.values.insert(symbol.local_id, Binding::Local(local));

                        Ok(self.builder.local_addr(local, target))
                    }
                    None => Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a borrow of a module binding".to_string(),
                    }
                    .into()),
                }
            }
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a borrow of a '{}' expression", other.variant_name()),
            }
            .into()),
        }
    }

    /// Lower one value coerced into a union carrier.
    fn lower_union_coercion(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        coercion: &dir::Coercion,
    ) -> CompilerResult<mir::Value> {
        let target = coercion.target;
        let carrier = self.lowerer.lower_type_id(self.builder.tree_mut(), target)?;

        // indexed variants tag the value under its selected member
        if let mir::Type::Variant { .. } = self.builder.tree().get(carrier) {
            return self.lower_variant_coercion(expression, coercion, target, carrier);
        }

        match self.lowerer.node_type(expression)? {
            // nullish values materialize as their carrier constants
            dir::Type::Null => Ok(self.builder.constant(mir::Constant::Null, carrier)),
            dir::Type::Undefined => Ok(self.builder.constant(mir::Constant::Undefined, carrier)),
            // reference values enter their nullable carrier unchanged
            _ => self.lower_expression_value(expression),
        }
    }

    /// Lower one value coerced into an indexed variant case.
    fn lower_variant_coercion(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        coercion: &dir::Coercion,
        target: dir::GlobalTypeId,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // the sealed member selects the case index
        let Some(member) = coercion.member else {
            return Err(CompilerError::Internal {
                message: "checked DIR entered a union without its member".to_string(),
            });
        };
        let dir::Type::Union(union) = self.lowerer.ty(target)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR tagged a value into a non-union carrier".to_string(),
            });
        };
        let Some(case) = self
            .lowerer
            .types(target.module_id)?
            .type_ids(union.elements)
            .iter()
            .position(|element| *element == member)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR selected a member outside its union".to_string(),
            });
        };

        // literals materialize directly at the member carrier
        let value = match self.lowerer.node_type(expression)? {
            // nullish members carry no payload
            dir::Type::Null | dir::Type::Undefined => None,
            dir::Type::Literal(literal) => {
                let member = self.lowerer.ty(member)?;
                let member = self.lowerer.lower_type(&member)?;

                Some(self.lower_constant(literal, member)?)
            }
            _ => Some(self.lower_expression_value(expression)?),
        };

        Ok(self.builder.variant_new(carrier, case as u32, value))
    }

    /// Lower one value expression by form.
    fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // materialize comptime-folded values directly as constants
        if let dir::Type::Literal(literal) = self.lowerer.node_type(expression)? {
            return self.lower_scalar_literal(expression, literal);
        }

        match self.lowerer.source().tree().get(expression).clone() {
            // value
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.lowerer.source);
                let symbol = self.lowerer.resolved_symbol(node)?;
                let Some(binding) = self.values.get(&symbol.local_id).copied() else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a module or captured binding".to_string(),
                    }
                    .into());
                };

                Ok(match binding {
                    Binding::Value(value) => value,
                    Binding::Local(local) => self.builder.local_get(local),
                })
            }

            // 1
            dir::Expression::ScalarLiteral(literal) => {
                self.lower_scalar_literal(expression, literal)
            }

            // a + b
            dir::Expression::Binary { left, right, .. } => {
                let resolution = self.lowerer.call_resolution(expression)?;
                let dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator }) =
                    resolution.target
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a protocol binary operator".to_string(),
                    }
                    .into());
                };

                match operator {
                    // a && b
                    dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                        self.lower_logical(expression, left, operator, right)
                    }
                    operator => self.lower_binary(left, operator, right),
                }
            }

            // -value
            dir::Expression::Unary { right, .. } => {
                let resolution = self.lowerer.call_resolution(expression)?;
                let dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator }) =
                    resolution.target
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a protocol unary operator".to_string(),
                    }
                    .into());
                };

                self.lower_unary(operator, right)
            }

            // match (value) { ... }
            dir::Expression::Match { value, cases, .. } => {
                self.lower_match(expression, value, &cases)
            }

            // cond ? a : b
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
            } => self.lower_ternary(expression, &condition, then_expression, else_expression),

            // Point { x: 1 }
            dir::Expression::StructExpression { properties, .. } => {
                self.lower_struct_expression(expression, &properties)
            }

            // (a, b)
            dir::Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression, &elements)
            }

            // this
            dir::Expression::This => self.this.ok_or_else(|| CompilerError::Internal {
                message: "checked DIR used this outside a method body".to_string(),
            }),

            // point.x
            dir::Expression::Member { left, .. } => self.lower_member(expression, left),

            // pair[0]
            dir::Expression::Index { left, .. } => self.lower_member(expression, left),

            // value as T
            dir::Expression::As {
                expression: value, ..
            } => self.lower_as(expression, value),

            // Meters(5)
            dir::Expression::Call { .. }
                if let Some(resolution) = self.lowerer.construct_resolution(expression) =>
            {
                self.lower_construct(expression, &resolution)
            }

            // new Counter(start)
            dir::Expression::New { .. } => {
                let Some(resolution) = self.lowerer.construct_resolution(expression) else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR is missing a construct resolution for one new"
                            .to_string(),
                    });
                };

                self.lower_construct(expression, &resolution)
            }

            // call(...)
            dir::Expression::Call { .. } => {
                let value = self.lower_call(expression)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "checked DIR typed a void call as a value".to_string(),
                })
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("'{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }
}
