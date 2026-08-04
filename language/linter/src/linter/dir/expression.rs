use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return authored expression text grouped for use as a postfix operand.
    pub fn postfix_source(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<String, ProviderError> {
        let span = self.source_extent(expression.into_any())?;
        let source = self.source(span)?;
        let is_parenthesized = self.source_parentheses(expression.into_any()).is_some();

        // retain existing grouping or add the grouping required by postfix precedence
        let source = if is_parenthesized
            || self.view().get(expression).precedence() >= dir::OperatorPrecedence::Postfix
        {
            source.to_string()
        } else {
            format!("({source})")
        };

        Ok(source)
    }

    /// Return authored expression text with one precedence-safe negation.
    pub fn negated_source(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<String, ProviderError> {
        let span = self.source_extent(expression.into_any())?;
        let source = self.source(span)?;
        let is_parenthesized = self.source_parentheses(expression.into_any()).is_some();

        // retain existing grouping or add the grouping required by prefix precedence
        let source = if is_parenthesized
            || self.view().get(expression).precedence() >= dir::OperatorPrecedence::Prefix
        {
            format!("!{source}")
        } else {
            format!("!({source})")
        };

        Ok(source)
    }

    /// Return the unique symbol selected directly by one checked expression.
    pub fn symbol(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        let view = self.view();
        let global = node.into_global_any(self.id);
        let symbol = match view.get(node) {
            dir::Expression::Identifier { .. } => {
                let resolution = self.resolutions.name_resolution(global).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "checked identifier expression {} in module {:?} has no name resolution",
                        node.id, self.id
                    ))
                })?;
                let [symbol] = resolution.symbols() else {
                    return Ok(None);
                };

                *symbol
            }
            dir::Expression::Member { .. } => {
                let Some(resolution) = self.member_resolution(node)? else {
                    return Ok(None);
                };
                let dir::OperationResolution::One(access) = resolution else {
                    return Ok(None);
                };
                let Some(symbol) = access.target.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            _ => return Ok(None),
        };

        Ok(Some(symbol))
    }

    /// Return the access resolution of one repeatable expression.
    pub fn access_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::AccessResolution> {
        let global = node.into_global_any(self.id);

        self.resolutions.access_resolution(global)
    }

    /// Return whether two checked expressions repeat one deterministic computation.
    pub fn is_repeated_expression(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // require identical checked source types
        if self.node_type_id(left.into_any())? != self.node_type_id(right.into_any())? {
            return Ok(false);
        }

        // compare repeatable places through their canonical paths
        let left_access = self.access_resolution(left);
        let right_access = self.access_resolution(right);
        match (left_access, right_access) {
            (Some(left), Some(right)) => return Ok(left == right),
            (Some(_), None) | (None, Some(_)) => return Ok(false),
            (None, None) => {}
        }

        let view = self.view();
        let left_expression = view.get(left);
        let right_expression = view.get(right);

        // compare every scalar expression through its canonical DIR value
        if let (Some(left), Some(right)) =
            (left_expression.as_scalar(), right_expression.as_scalar())
        {
            return Ok(left == right);
        }

        let is_same = match (left_expression, right_expression) {
            // repeatable compiler-defined unary operations
            (
                dir::Expression::Unary {
                    operator: left_operator,
                    right: left_value,
                },
                dir::Expression::Unary {
                    operator: right_operator,
                    right: right_value,
                },
            ) if left_operator == right_operator
                && matches!(
                    left_operator,
                    dir::UnaryOperator::Not
                        | dir::UnaryOperator::Plus
                        | dir::UnaryOperator::Negate
                        | dir::UnaryOperator::ElementwiseNot
                        | dir::UnaryOperator::Typeof
                        | dir::UnaryOperator::Void
                ) =>
            {
                let left_resolution = self.operator_resolution(left.into_any())?;
                let right_resolution = self.operator_resolution(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (Some([left_operand]), Some([right_operand])) = (
                    left_resolution.builtin_operands(),
                    right_resolution.builtin_operands(),
                ) else {
                    return Ok(false);
                };

                self.is_repeated_operand(*left_value, left_operand, *right_value, right_operand)?
            }

            // repeatable compiler-defined binary operations
            (
                dir::Expression::Binary {
                    left: left_left,
                    operator: left_operator,
                    right: left_right,
                },
                dir::Expression::Binary {
                    left: right_left,
                    operator: right_operator,
                    right: right_right,
                },
            ) if left_operator == right_operator => {
                let left_resolution = self.operator_resolution(left.into_any())?;
                let right_resolution = self.operator_resolution(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (Some([left_first, left_second]), Some([right_first, right_second])) = (
                    left_resolution.builtin_operands(),
                    right_resolution.builtin_operands(),
                ) else {
                    return Ok(false);
                };

                self.is_repeated_operand(*left_left, left_first, *right_left, right_first)?
                    && self.is_repeated_operand(
                        *left_right,
                        left_second,
                        *right_right,
                        right_second,
                    )?
            }

            // compiler-defined casts and static assertions
            (
                dir::Expression::As {
                    expression: left_value,
                    target_type: left_target,
                },
                dir::Expression::As {
                    expression: right_value,
                    target_type: right_target,
                },
            )
            | (
                dir::Expression::Satisfies {
                    expression: left_value,
                    target_type: left_target,
                },
                dir::Expression::Satisfies {
                    expression: right_value,
                    target_type: right_target,
                },
            ) => {
                self.node_type_id(left_target.into_any())?
                    == self.node_type_id(right_target.into_any())?
                    && self.is_repeated_expression(*left_value, *right_value)?
            }

            // reject computations that can differ between evaluations
            _ => false,
        };

        Ok(is_same)
    }

    /// Return whether two builtin operands repeat one checked runtime value.
    pub fn is_repeated_operand(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        left_operand: &dir::BuiltinOperand,
        right: dir::LocalNodeId<dir::Expression>,
        right_operand: &dir::BuiltinOperand,
    ) -> Result<bool, ProviderError> {
        if left_operand.ty != right_operand.ty
            || left_operand.scalar_families != right_operand.scalar_families
        {
            return Ok(false);
        }

        let left_global = left.into_global_any(self.id);
        let right_global = right.into_global_any(self.id);
        let left_coercion = self.coercions.coercion(left_global);
        let right_coercion = self.coercions.coercion(right_global);
        if left_coercion != right_coercion {
            return Ok(false);
        }

        self.is_repeated_expression(left, right)
    }

    /// Return the operator resolution selected for one checked node.
    pub fn operator_resolution(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<&dir::OperatorResolution>, ProviderError> {
        let global = node.into_global(self.id);
        let Some(resolution) = self.resolutions.operator_resolution(global) else {
            if self.node_type(node)?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked operator node {} in module {:?} has no operator resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return the call resolution selected for one checked expression.
    pub fn call_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::CallResolution>, ProviderError> {
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.resolutions.call_resolution(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked call expression {} in module {:?} has no call resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return one operand selected for a checked builtin operator application.
    pub fn builtin_operand(
        &self,
        application: dir::LocalNodeIdAny,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::BuiltinOperand>, ProviderError> {
        let source = source.into_global_any(self.id);
        let Some(operands) = self.builtin_operands(application)? else {
            return Ok(None);
        };
        let operand = operands
            .iter()
            .find(|operand| operand.source == source)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked builtin operator node {} in module {:?} has no operand {source:?}",
                    application.id, self.id
                ))
            })?;

        Ok(Some(operand))
    }

    /// Return the operands selected for one checked builtin operator application.
    pub fn builtin_operands(
        &self,
        application: dir::LocalNodeIdAny,
    ) -> Result<Option<&[dir::BuiltinOperand]>, ProviderError> {
        let operands = self
            .operator_resolution(application)?
            .and_then(dir::OperatorResolution::builtin_operands);

        Ok(operands)
    }

    /// Return the member resolution selected for one checked expression.
    pub fn member_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::MemberResolution>, ProviderError> {
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.resolutions.member_resolution(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked member expression {} in module {:?} has no member resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }
}
