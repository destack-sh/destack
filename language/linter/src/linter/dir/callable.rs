use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the body expression owned by one callable node.
    pub(crate) fn callable_body(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();

        match node.ty {
            // function declaration
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };

                function.body
            }
            // class or interface method
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);
                let dir::Member::Method { body, .. } = view.get(member) else {
                    return None;
                };

                *body
            }
            // object method
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);
                let dir::Property::Method { body, .. } = view.get(property) else {
                    return None;
                };

                *body
            }
            // structural type method
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);
                let dir::TypeMember::Method { body, .. } = view.get(member) else {
                    return None;
                };

                *body
            }
            // non-callable node
            _ => None,
        }
    }

    /// Return the nearest callable body containing one node.
    pub(crate) fn enclosing_callable_body(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut current = Some(node);

        // climb to the first callable owner
        while let Some(node) = current {
            if let Some(body) = self.callable_body(node) {
                return Some(body);
            }

            current = view.get_parent_any(node);
        }

        None
    }

    /// Return the function declaration authored as one lambda expression.
    pub(crate) fn lambda(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::FunctionDeclaration> {
        // select the wrapped function declaration
        let view = self.view();
        let dir::Expression::Declaration(declaration) = view.get(expression) else {
            return None;
        };
        let dir::Declaration::Function(function) = view.get(*declaration) else {
            return None;
        };

        // require the lambda function form
        if function.signature.form != dir::FunctionForm::Lambda {
            return None;
        }

        Some(function)
    }

    /// Return whether one callback is exactly `firstParameter !== undefined`.
    pub(crate) fn is_defined_predicate(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // select one synchronous lambda with a required first parameter
        let Some(lambda) = self.lambda(expression) else {
            return Ok(false);
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
            return Ok(false);
        }
        let Some(parameter) = lambda.signature.parameters.first() else {
            return Ok(false);
        };
        if !matches!(
            self.view().get(*parameter),
            dir::Parameter::Named {
                default: None,
                is_optional: false,
                ..
            }
        ) {
            return Ok(false);
        }

        // require one direct strict inequality body
        let Some(body) = lambda
            .body
            .and_then(|body| self.sole_value_expression(body))
        else {
            return Ok(false);
        };
        let Some((dir::BinaryOperator::NotEqualStrict, [left, right])) =
            self.builtin_binary(body)?
        else {
            return Ok(false);
        };

        // match the parameter and undefined in either operand order
        let parameter = self.declaration_symbol(*parameter)?;
        let left = left.source.local_id;
        let right = right.source.local_id;
        let is_parameter_left = self.selected_symbol(left)? == Some(parameter);
        let is_parameter_right = self.selected_symbol(right)? == Some(parameter);
        let is_undefined_left = self.scalar_constant(left)? == Some(dir::ScalarLiteral::Undefined);
        let is_undefined_right =
            self.scalar_constant(right)? == Some(dir::ScalarLiteral::Undefined);

        Ok(is_parameter_left && is_undefined_right || is_undefined_left && is_parameter_right)
    }

    /// Return whether one expression occurs within an explicit or implicit return value.
    pub(crate) fn is_within_return_value(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let view = self.view();
        let mut current = expression.into_any();

        // climb through the current callable only
        while let Some(parent) = view.get_parent_any(current) {
            // accept an explicit return value
            if let Ok(parent) = parent.try_into_typed::<dir::Expression>()
                && let dir::Expression::Return { value: Some(value) } = view.get(parent)
            {
                return view.is_inside(expression.into_any(), value.into_any());
            }

            // accept a lambda value body and stop at every other callable
            let Some(body) = self.callable_body(parent) else {
                current = parent;
                continue;
            };
            let Ok(declaration) = parent.try_into_typed::<dir::Declaration>() else {
                return false;
            };
            let dir::Declaration::Function(function) = view.get(declaration) else {
                return false;
            };
            if function.signature.form != dir::FunctionForm::Lambda {
                return false;
            }
            let Some(value) = self.value_expression(body) else {
                return false;
            };

            return view.is_inside(expression.into_any(), value.into_any());
        }

        false
    }

    /// Return the checked result type id of one callable expression.
    pub fn callable_return_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        // inspect the callable value beneath placement forms
        let type_id = self.node_type_id(expression.into_any())?;
        let signature = self
            .dir
            .callable_signature_type_id(type_id)?
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked callback expression {expression:?} has non-callable type {type_id:?}"
                ))
            })?;

        self.dir.signature_return_type_id(signature)
    }
}
