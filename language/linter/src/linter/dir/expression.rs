use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return whether one expression belongs directly to module or global scope.
    pub(crate) fn is_module_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        if self.roots.contains(&expression) {
            return Ok(true);
        }
        let view = self.view();
        let parent = view.get_parent_for(expression).ok_or_else(|| {
            ProviderError::internal(format!("expression {expression:?} has no parent"))
        })?;
        if parent.ty != dir::NodeType::Declaration {
            return Ok(false);
        }
        let parent = dir::LocalNodeId::new(parent.id);

        Ok(matches!(
            view.get(parent),
            dir::Declaration::Global(_) | dir::Declaration::Module(_)
        ))
    }

    /// Return the final direct field with one static key.
    pub(crate) fn direct_field(
        &self,
        properties: &[dir::LocalNodeId<dir::Property>],
        key: dir::StaticKey,
    ) -> Option<(
        dir::LocalNodeId<dir::Property>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        // select the final field unless a later spread can replace it
        for property_id in properties.iter().rev() {
            let property = self.view().get(*property_id);
            match property {
                dir::Property::Field { name, value, .. } if dir::StaticKey::from(*name) == key => {
                    return Some((*property_id, *value));
                }
                dir::Property::Spread { .. } => return None,
                dir::Property::Method { .. }
                    if property.slot() == Some(dir::MemberSlot::Key(key)) =>
                {
                    return None;
                }
                _ => {}
            }
        }

        None
    }

    /// Return the ordered operands of one builtin short-circuit chain.
    pub(crate) fn short_circuit_operands(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) -> Result<Vec<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        if !matches!(
            operator,
            dir::BinaryOperator::And | dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce
        ) {
            return Err(ProviderError::internal(format!(
                "operator {operator:?} does not short circuit"
            )));
        }
        let mut operands = Vec::new();
        self.collect_short_circuit_operands(expression, operator, &mut operands)?;

        Ok(operands)
    }

    /// Return whether the parent selects one builtin binary operator.
    pub(crate) fn has_builtin_binary_parent(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) -> Result<bool, ProviderError> {
        let Some(parent) = self.view().get_parent_for(expression) else {
            return Ok(false);
        };
        let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
            return Ok(false);
        };
        let selected = self.builtin_binary(parent)?.map(|(selected, _)| selected);

        Ok(selected == Some(operator))
    }

    /// Append the operands of one builtin short-circuit chain.
    fn collect_short_circuit_operands(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        operands: &mut Vec<dir::LocalNodeId<dir::Expression>>,
    ) -> Result<(), ProviderError> {
        let Some((selected, children)) = self.builtin_binary(expression)? else {
            operands.push(expression);

            return Ok(());
        };
        if selected != operator {
            operands.push(expression);

            return Ok(());
        }

        // flatten both sides of the same short-circuit operation
        for operand in children {
            self.collect_short_circuit_operands(operand.source.local_id, operator, operands)?;
        }

        Ok(())
    }

    /// Return the sole direct binding declarator in one declaration expression.
    pub(crate) fn binding_declarator(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<(dir::Mutability, &dir::Declarator)> {
        // select one declaration with one declarator
        let view = self.view();
        let dir::Expression::Let {
            mutability,
            declarators,
            ..
        } = view.get(expression)
        else {
            return None;
        };
        let [declarator] = declarators.as_slice() else {
            return None;
        };
        let declarator = view.get(*declarator);

        // require a direct rather than destructured binding
        if !matches!(
            view.get(declarator.pattern),
            dir::Pattern::Binding { pattern: None, .. }
        ) {
            return None;
        }

        Some((*mutability, declarator))
    }

    /// Return whether one direct block expression has its value discarded.
    pub(crate) fn is_discarded_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let view = self.view();
        let Some(parent) = view.get_parent_for(expression) else {
            return false;
        };
        let Ok(block) = parent.try_into_typed::<dir::Block>() else {
            return false;
        };

        view.get(block).value_expression() != Some(expression)
    }

    /// Return the expression that directly produces one expression's value.
    pub(crate) fn value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self.view().get(expression) {
            dir::Expression::Block(block) => self.view().get(*block).value_expression(),
            _ => Some(expression),
        }
    }

    /// Return every terminal expression that can produce one expression's value.
    pub(crate) fn terminal_values(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<dir::LocalNodeId<dir::Expression>>> {
        let mut values = Vec::new();
        if !self.collect_terminal_values(expression, &mut values) {
            return None;
        }

        Some(values)
    }

    /// Append every terminal expression that can produce one expression's value.
    fn collect_terminal_values(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        values: &mut Vec<dir::LocalNodeId<dir::Expression>>,
    ) -> bool {
        let view = self.view();

        // follow block tail values
        if let dir::Expression::Block(block) = view.get(expression) {
            let Some(value) = view.get(*block).value_expression() else {
                return true;
            };

            return self.collect_terminal_values(value, values);
        }

        // follow both conditional branches
        if let dir::Expression::If {
            then_expression,
            else_expression,
            ..
        } = view.get(expression)
        {
            let Some(else_expression) = else_expression else {
                return false;
            };

            return self.collect_terminal_values(*then_expression, values)
                && self.collect_terminal_values(*else_expression, values);
        }

        // follow every match arm
        if let dir::Expression::Match { arms, .. } = view.get(expression) {
            if arms.is_empty() {
                return false;
            }
            for arm in arms {
                let value = match view.get(*arm) {
                    dir::MatchArm::Expression { body, .. } => Some(*body),
                    dir::MatchArm::Block { body, .. } => view.get(*body).value_expression(),
                };
                let Some(value) = value else {
                    return false;
                };
                if !self.collect_terminal_values(value, values) {
                    return false;
                }
            }

            return true;
        }

        // retain one indivisible value from a normally completing path
        if !self.flows.is_diverging(expression.into_any()) {
            values.push(expression);
        }

        true
    }

    /// Return the sole expression performed directly or within one block.
    pub(crate) fn sole_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self.view().get(expression) {
            dir::Expression::Block(block) => self.view().get(*block).only_expression(),
            _ => Some(expression),
        }
    }

    /// Return one direct block expression's children in execution order.
    pub(crate) fn block_expressions(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<dir::GlobalNodeIdAny>> {
        let dir::Expression::Block(block) = self.view().get(expression) else {
            return None;
        };

        Some(
            self.view()
                .get(*block)
                .iter_expressions()
                .map(|expression| expression.into_global_any(self.id))
                .collect(),
        )
    }

    /// Return the value expression when its block performs no preceding work.
    pub(crate) fn sole_value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let dir::Expression::Block(block) = view.get(expression) else {
            return Some(expression);
        };
        let block = view.get(*block);
        if !block.leading_expressions.is_empty() {
            return None;
        }

        block.value_expression()
    }

    /// Return the value returned by one direct or single-expression block.
    pub(crate) fn sole_return_value(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let expression = self.sole_expression(expression)?;
        let dir::Expression::Return { value: Some(value) } = self.view().get(expression) else {
            return None;
        };

        Some(*value)
    }

    /// Return the nearest expression that contains one expression in value position.
    pub(crate) fn enclosing_value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut current = expression.into_any();

        // cross structural nodes until reaching an expression or body boundary
        while let Some(parent) = view.get_parent_any(current) {
            match parent.ty {
                // cross an expression block only through its value tail
                dir::NodeType::Block => {
                    let block = dir::LocalNodeId::<dir::Block>::new(parent.id);
                    let Ok(current_expression) = current.try_into_typed::<dir::Expression>() else {
                        return None;
                    };
                    if view.get(block).value_expression() != Some(current_expression) {
                        return None;
                    }

                    current = parent;
                }
                // stop at branch bodies, otherwise return the value parent
                dir::NodeType::Expression => {
                    let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                    let is_body = match view.get(parent) {
                        dir::Expression::If {
                            form: dir::IfForm::If,
                            then_expression,
                            else_expression,
                            ..
                        } => {
                            current == then_expression.into_any()
                                || else_expression.is_some_and(|body| current == body.into_any())
                        }
                        dir::Expression::LetElse { else_branch, .. } => {
                            current == else_branch.into_any()
                        }
                        dir::Expression::Try { body, finally, .. } => {
                            current == body.into_any()
                                || finally.is_some_and(|body| current == body.into_any())
                        }
                        _ => false,
                    };
                    if is_body {
                        return None;
                    }

                    return Some(parent);
                }
                // stop at bindings and control branches
                dir::NodeType::Declarator
                | dir::NodeType::MatchArm
                | dir::NodeType::Catch
                | dir::NodeType::SwitchCase => return None,
                _ => {
                    // stop at callable ownership
                    if current
                        .try_into_typed::<dir::Expression>()
                        .ok()
                        .is_some_and(|body| self.callable_body(parent) == Some(body))
                    {
                        return None;
                    }

                    // cross transparent authored nodes
                    current = parent;
                }
            }
        }

        None
    }

    /// Return whether one expression may be evaluated twice at the same program point.
    pub fn is_duplicable_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        self.is_same_computation(expression, expression)
    }

    /// Return whether one expression may be evaluated on an additional control-flow path.
    pub fn is_speculatable_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        if !self.is_duplicable_expression(expression)? {
            return Ok(false);
        }

        self.can_trap(expression).map(|can_trap| !can_trap)
    }

    /// Return whether one expression keeps the same value throughout a node subtree.
    pub(crate) fn is_invariant_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        subtree: dir::LocalNodeIdAny,
        excluded_symbols: &[dir::GlobalSymbolId],
        occurrences: &[dir::BindingOccurrence],
    ) -> Result<bool, ProviderError> {
        if !self.is_speculatable_expression(expression)? {
            return Ok(false);
        }
        let view = self.view();

        // collect bindings read while evaluating the expression
        let reads = occurrences
            .iter()
            .filter(|occurrence| {
                occurrence.uses.contains(dir::BindingUse::READ)
                    && view.is_inside(occurrence.node, expression.into_any())
            })
            .map(|occurrence| occurrence.symbol)
            .collect::<Vec<_>>();
        if excluded_symbols.iter().any(|symbol| reads.contains(symbol)) {
            return Ok(false);
        }

        // reject bindings changed within the selected subtree
        let changes_read = occurrences.iter().any(|occurrence| {
            occurrence.uses.may_mutate()
                && view.is_inside(occurrence.node, subtree)
                && reads.contains(&occurrence.symbol)
        });

        Ok(!changes_read)
    }

    /// Return whether boolean structure proves one expression implies another.
    pub(crate) fn boolean_implies(
        &self,
        premise: dir::LocalNodeId<dir::Expression>,
        conclusion: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        if !self.is_duplicable_expression(premise)? || !self.is_duplicable_expression(conclusion)? {
            return Ok(false);
        }

        self.boolean_implies_inner(premise, conclusion)
    }

    /// Compare duplicable boolean expressions through builtin conjunctions and disjunctions.
    fn boolean_implies_inner(
        &self,
        premise: dir::LocalNodeId<dir::Expression>,
        conclusion: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // identical computations imply one another
        if self.is_same_computation(premise, conclusion)? {
            return Ok(true);
        }

        // every disjunct in the premise must imply the conclusion
        let operands = self.short_circuit_operands(premise, dir::BinaryOperator::Or)?;
        if operands.len() > 1 {
            for operand in operands {
                if !self.boolean_implies_inner(operand, conclusion)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        // the premise must imply every conjunct in the conclusion
        let operands = self.short_circuit_operands(conclusion, dir::BinaryOperator::And)?;
        if operands.len() > 1 {
            for operand in operands {
                if !self.boolean_implies_inner(premise, operand)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        // any conjunct in the premise may establish the conclusion
        let operands = self.short_circuit_operands(premise, dir::BinaryOperator::And)?;
        if operands.len() > 1 {
            for operand in operands {
                if self.boolean_implies_inner(operand, conclusion)? {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        // the premise may establish any disjunct in the conclusion
        let operands = self.short_circuit_operands(conclusion, dir::BinaryOperator::Or)?;
        if operands.len() > 1 {
            for operand in operands {
                if self.boolean_implies_inner(premise, operand)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Return whether evaluating one expression can trap.
    fn can_trap(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let node = self.view().get(expression);

        // accept scalar values
        if node.as_scalar().is_some() {
            return Ok(false);
        }

        // inspect stable storage without hiding receiver evaluation
        let can_trap = match node {
            dir::Expression::Identifier { .. } | dir::Expression::This | dir::Expression::Super
                if self.access_resolution(expression).is_some() =>
            {
                false
            }
            dir::Expression::Member { left, .. }
                if self.access_resolution(expression).is_some() =>
            {
                self.can_trap(*left)?
            }
            dir::Expression::Chain { expression } => self.can_trap(*expression)?,
            dir::Expression::Index { .. } => true,

            // inspect compiler-defined operations
            dir::Expression::Unary { .. } => {
                let Some((operator, operand)) = self.builtin_unary(expression)? else {
                    return Ok(true);
                };
                let operation_can_trap = operator == dir::UnaryOperator::Negate
                    && operand.is_integral()
                    && self.integral_constant(expression)?.is_none();

                operation_can_trap || self.can_trap(operand.source.local_id)?
            }
            dir::Expression::Binary { .. } => {
                let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
                    return Ok(true);
                };
                let operation_can_trap = left.is_integral()
                    && self.integral_constant(expression)?.is_none()
                    && matches!(
                        operator,
                        dir::BinaryOperator::Exponent
                            | dir::BinaryOperator::Multiply
                            | dir::BinaryOperator::Divide
                            | dir::BinaryOperator::Remainder
                            | dir::BinaryOperator::Add
                            | dir::BinaryOperator::Subtract
                            | dir::BinaryOperator::ShiftLeft
                    );

                operation_can_trap
                    || self.can_trap(left.source.local_id)?
                    || self.can_trap(right.source.local_id)?
            }
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => self.can_trap(*expression)?,
            _ => true,
        };

        Ok(can_trap)
    }

    /// Return whether two expressions denote the same computation.
    pub fn is_same_computation(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // compare stable places through their canonical paths
        let left_access = self.access_resolution(left);
        let right_access = self.access_resolution(right);
        match (left_access, right_access) {
            (Some(left), Some(right)) => return Ok(left == right),
            (Some(_), None) | (None, Some(_)) => return Ok(false),
            (None, None) => {}
        }

        // require identical source types for computed values
        if self.node_type_id(left.into_any())? != self.node_type_id(right.into_any())? {
            return Ok(false);
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
            // compiler-defined unary operations
            (dir::Expression::Unary { .. }, dir::Expression::Unary { .. }) => {
                let left_resolution = self.operator_decision(left.into_any())?;
                let right_resolution = self.operator_decision(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (Some((left_operator, left_operand)), Some((right_operator, right_operand))) = (
                    left_resolution.builtin_unary(),
                    right_resolution.builtin_unary(),
                ) else {
                    return Ok(false);
                };
                if left_operator != right_operator
                    || !matches!(
                        left_operator,
                        dir::UnaryOperator::Not
                            | dir::UnaryOperator::Plus
                            | dir::UnaryOperator::Negate
                            | dir::UnaryOperator::ElementwiseNot
                    )
                {
                    return Ok(false);
                }

                self.is_same_operand(left_operand, right_operand)?
            }

            // compiler-defined binary operations
            (dir::Expression::Binary { .. }, dir::Expression::Binary { .. }) => {
                let left_resolution = self.operator_decision(left.into_any())?;
                let right_resolution = self.operator_decision(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (
                    Some((left_operator, [left_first, left_second])),
                    Some((right_operator, [right_first, right_second])),
                ) = (
                    left_resolution.builtin_binary(),
                    right_resolution.builtin_binary(),
                )
                else {
                    return Ok(false);
                };
                if left_operator != right_operator {
                    return Ok(false);
                }

                self.is_same_operand(left_first, right_first)?
                    && self.is_same_operand(left_second, right_second)?
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
                    && self.is_same_computation(*left_value, *right_value)?
            }

            // reject computations that can differ between evaluations
            _ => false,
        };

        Ok(is_same)
    }

    /// Return whether two builtin operands produce the same value.
    pub fn is_same_operand(
        &self,
        left_operand: &dir::BuiltinOperand,
        right_operand: &dir::BuiltinOperand,
    ) -> Result<bool, ProviderError> {
        if left_operand.ty != right_operand.ty
            || left_operand.scalar_families != right_operand.scalar_families
        {
            return Ok(false);
        }

        let left_coercion = self.coercions.coercion(left_operand.source.into_any());
        let right_coercion = self.coercions.coercion(right_operand.source.into_any());
        if left_coercion != right_coercion {
            return Ok(false);
        }

        let left = left_operand.source.local_id;
        let right = right_operand.source.local_id;

        self.is_same_computation(left, right)
    }
}
