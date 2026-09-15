use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

/// One getter or setter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Accessor {
    /// The accessor node.
    pub(crate) node: dir::LocalNodeIdAny,
    /// The property namespace.
    pub(crate) space: dir::MemberSpace,
    /// The property slot.
    pub(crate) slot: dir::MemberSlot,
    /// Whether this accessor is a getter.
    pub(crate) is_getter: bool,
}

impl DirModule<'_> {
    /// Return one getter or setter.
    pub(crate) fn accessor(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<Accessor>, ProviderError> {
        // read the callable shape shared by declaration member forms
        let view = self.view();
        let (signature, space, slot) = match node.ty {
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);
                let member = view.get(member);

                (member.signature(), member.space(), member.slot())
            }
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);
                let property = view.get(property);

                (property.signature(), property.space(), property.slot())
            }
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);
                let member = view.get(member);

                (member.signature(), member.space(), member.slot())
            }
            _ => return Ok(None),
        };

        // require one named getter or setter
        let Some(signature) = signature else {
            return Ok(None);
        };
        let Some(role @ (dir::FunctionRole::Getter | dir::FunctionRole::Setter)) = signature.role
        else {
            return Ok(None);
        };
        let (Some(space), Some(slot @ dir::MemberSlot::Key(_))) = (space, slot) else {
            return Err(ProviderError::internal(format!(
                "accessor {node:?} has no named member slot"
            )));
        };

        Ok(Some(Accessor {
            node,
            space,
            slot,
            is_getter: role == dir::FunctionRole::Getter,
        }))
    }

    /// Return the authored generic parameters owned by one callable node.
    pub(crate) fn callable_generic_parameters(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<&[dir::LocalNodeId<dir::GenericParameter>]> {
        let view = self.view();

        match node.ty {
            // function declaration
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };

                Some(&function.signature.generic_parameters)
            }
            // class or interface method
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);

                Some(&view.get(member).signature()?.generic_parameters)
            }
            // object method
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);

                Some(&view.get(property).signature()?.generic_parameters)
            }
            // structural type callable
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);

                match view.get(member) {
                    dir::TypeMember::Method { signature, .. } => {
                        Some(&signature.generic_parameters)
                    }
                    dir::TypeMember::CallSignature { signature } => {
                        Some(&signature.generic_parameters)
                    }
                    dir::TypeMember::ConstructSignature { signature } => {
                        Some(&signature.generic_parameters)
                    }
                    _ => None,
                }
            }
            // function or constructor type expression
            dir::NodeType::TypeExpression => {
                let expression = dir::LocalNodeId::<dir::TypeExpression>::new(node.id);

                match view.get(expression) {
                    dir::TypeExpression::Function(function) => Some(&function.generic_parameters),
                    dir::TypeExpression::Constructor(constructor) => {
                        Some(&constructor.generic_parameters)
                    }
                    _ => None,
                }
            }
            // non-callable node
            _ => None,
        }
    }

    /// Return the authored value parameters owned by one callable node.
    pub(crate) fn callable_parameters(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<&[dir::LocalNodeId<dir::Parameter>]> {
        let view = self.view();

        match node.ty {
            // function declaration
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };

                Some(&function.signature.parameters)
            }
            // class or interface method
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);

                Some(&view.get(member).signature()?.parameters)
            }
            // object method
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);

                Some(&view.get(property).signature()?.parameters)
            }
            // structural type callable
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);

                view.get(member).parameters()
            }
            // function or constructor type expression
            dir::NodeType::TypeExpression => {
                let expression = dir::LocalNodeId::<dir::TypeExpression>::new(node.id);

                view.get(expression).parameters()
            }
            // non-callable node
            _ => None,
        }
    }

    /// Return the authored return type owned by one callable node.
    pub(crate) fn callable_return_type(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        let view = self.view();

        match node.ty {
            // function declaration
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };

                function.signature.return_type
            }
            // class or interface method
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);

                view.get(member).signature()?.return_type
            }
            // object method
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);

                view.get(property).signature()?.return_type
            }
            // structural type callable
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);

                match view.get(member) {
                    dir::TypeMember::Method { signature, .. } => signature.return_type,
                    dir::TypeMember::CallSignature { signature } => signature.return_type,
                    dir::TypeMember::ConstructSignature { signature } => signature.return_type,
                    _ => None,
                }
            }
            // function or constructor type expression
            dir::NodeType::TypeExpression => {
                let expression = dir::LocalNodeId::<dir::TypeExpression>::new(node.id);

                match view.get(expression) {
                    dir::TypeExpression::Function(function) => function.return_type,
                    dir::TypeExpression::Constructor(constructor) => constructor.return_type,
                    _ => None,
                }
            }
            // non-callable node
            _ => None,
        }
    }

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

    /// Return the nearest callable declaration containing one node.
    pub(crate) fn enclosing_callable(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeIdAny> {
        let view = self.view();
        let mut current = Some(node);

        // climb to the first callable owner
        while let Some(node) = current {
            if self.callable_body(node).is_some() {
                return Some(node);
            }

            current = view.get_parent_any(node);
        }

        None
    }

    /// Return the nearest callable body containing one node.
    pub(crate) fn enclosing_callable_body(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        self.enclosing_callable(node)
            .and_then(|callable| self.callable_body(callable))
    }

    /// Return every value returned by one callable body.
    pub(crate) fn callable_return_values(
        &self,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<dir::LocalNodeId<dir::Expression>>> {
        let view = self.view();
        let mut values = self.terminal_values(body)?;

        // collect explicit returns owned by this callable
        for (expression, node) in view.iter_nodes::<dir::Expression>() {
            if !view.is_inside(expression.into_any(), body.into_any())
                || self.enclosing_callable_body(expression.into_any()) != Some(body)
            {
                continue;
            }
            let dir::Expression::Return { value: Some(value) } = node else {
                continue;
            };
            let returned = self.terminal_values(*value)?;

            values.extend(returned);
        }

        Some(values)
    }

    /// Return the nominal owner when one member belongs to a definition.
    pub(crate) fn member_owner(
        &self,
        member: dir::LocalNodeId<dir::Member>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        let symbol = self.declaration_symbol(member)?;
        let Some((declaring, definition, _)) = self.definitions.member(symbol) else {
            return Ok(None);
        };

        Ok(definition.member_owner(declaring))
    }

    /// Return the sole value call when a body passes each parameter directly and in order.
    pub(crate) fn parameter_forwarding_call(
        &self,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        // require a synchronous signature without parameter entry behavior
        if signature.asynchrony != dir::Asynchrony::Sync || signature.is_generator {
            return Ok(None);
        }
        if signature.parameters.iter().any(|parameter| {
            !matches!(
                self.view().get(*parameter),
                dir::Parameter::Named {
                    default: None,
                    is_optional: false,
                    ..
                }
            )
        }) {
            return Ok(None);
        }

        // select one direct or explicitly returned call
        let Some(expression) = self.sole_expression(body) else {
            return Ok(None);
        };
        let expression = match self.view().get(expression) {
            dir::Expression::Return { value: Some(value) } => *value,
            _ if self.sole_value_expression(body) == Some(expression) => expression,
            _ => return Ok(None),
        };
        let dir::Expression::Call {
            left: callee,
            generic_arguments,
            arguments,
            is_optional: false,
            ..
        } = self.view().get(expression)
        else {
            return Ok(None);
        };
        if !generic_arguments.is_empty() || arguments.len() != signature.parameters.len() {
            return Ok(None);
        }

        // require the call and every argument to pass their values unchanged
        if !self.is_unadjusted(expression.into_any()) {
            return Ok(None);
        }
        if !matches!(self.view().get(*callee), dir::Expression::Identifier { .. }) {
            return Ok(None);
        }

        // exclude nominal construction before requiring ordinary call resolution
        if let Some(symbol) = self.selected_symbol(*callee)?
            && self.dir.is_nominal_symbol(symbol)?
        {
            return Ok(None);
        }
        for (parameter, argument) in signature.parameters.iter().zip(arguments) {
            let dir::Argument::Positional { value } = self.view().get(*argument) else {
                return Ok(None);
            };
            let parameter = self.declaration_symbol(*parameter)?;
            if self.selected_symbol(*value)? != Some(parameter)
                || !self.is_unadjusted(value.into_any())
            {
                return Ok(None);
            }
        }

        // require the selected function to accept exactly the forwarded arity
        let Some(parameters) = self.call_parameters(expression)? else {
            return Ok(None);
        };
        if parameters.len() != arguments.len()
            || parameters.iter().any(|parameter| parameter.is_rest)
        {
            return Ok(None);
        }

        // require a stable undecorated function declaration
        let Some(function) = self.call_symbol(expression)? else {
            return Ok(None);
        };
        if self.dir.has_decorators(function)? {
            return Ok(None);
        }

        Ok(Some(expression))
    }

    /// Return whether one callable return type borrows a parameter's value type.
    pub(crate) fn return_type_borrows_parameter(
        &self,
        parameter: dir::LocalNodeId<dir::Parameter>,
    ) -> Result<bool, ProviderError> {
        let view = self.view();
        let Some(borrowed_type) = view.get(parameter).declared_type() else {
            return Ok(false);
        };
        let borrowed_type = self.node_type_id(borrowed_type.into_any())?;
        let Some(borrow) = self.dir.borrow_form(borrowed_type)? else {
            return Ok(false);
        };
        let Some(callable) = view.get_parent_for(parameter) else {
            return Ok(false);
        };
        let Some(return_type) = self.callable_return_type(callable) else {
            return Ok(false);
        };
        let target = self.dir.strip_form(borrowed_type)?;

        // find an equal returned borrow tied to the solved parameter lifetime
        for (node, _) in view.iter_nodes::<dir::TypeExpression>() {
            if !view.is_inside(node.into_any(), return_type.into_any()) {
                continue;
            }
            let returned_type = self.node_type_id(node.into_any())?;
            let Some(returned_borrow) = self.dir.borrow_form(returned_type)? else {
                continue;
            };
            let returned_target = self.dir.strip_form(returned_type)?;
            if borrow.region == returned_borrow.region && target == returned_target {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one parameter lifetime occurs elsewhere in its callable.
    pub(crate) fn parameter_lifetime_occurs_elsewhere(
        &self,
        parameter: dir::LocalNodeId<dir::Parameter>,
        lifetime: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<bool, ProviderError> {
        let view = self.view();
        let Some(callable) = view.get_parent_for(parameter) else {
            return Ok(false);
        };
        let lifetime = self.node_type_id(lifetime.into_any())?;

        // find the same authored lifetime outside the parameter declaration
        for (node, _) in view.iter_nodes::<dir::TypeExpression>() {
            if !view.is_inside(node.into_any(), callable)
                || view.is_inside(node.into_any(), parameter.into_any())
            {
                continue;
            }
            if self.node_type_id(node.into_any())? == lifetime {
                return Ok(true);
            }
        }

        Ok(false)
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

    /// Return the removable expression that returns a lambda's first parameter after readonly work.
    pub(crate) fn removable_parameter_return(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        // select one synchronous block lambda with an inferred first parameter
        let Some(lambda) = self.lambda(expression) else {
            return Ok(None);
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
            return Ok(None);
        }
        let Some(parameter) = lambda.signature.parameters.first() else {
            return Ok(None);
        };
        if !matches!(
            self.view().get(*parameter),
            dir::Parameter::Named {
                declared_type: None,
                default: None,
                is_optional: false,
                ..
            }
        ) {
            return Ok(None);
        }
        let Some(body) = lambda.body else {
            return Ok(None);
        };
        let dir::Expression::Block(block) = self.view().get(body) else {
            return Ok(None);
        };

        // select a final return preceded by work
        let block = self.view().get(*block);
        let (returned, removed) = if let Some(returned) = block.tail_expression {
            if block.leading_expressions.is_empty() {
                return Ok(None);
            }

            (returned, returned)
        } else if let [preceding @ .., returned] = block.leading_expressions.as_slice()
            && !preceding.is_empty()
            && let dir::Expression::Return { value: Some(value) } = self.view().get(*returned)
        {
            (*value, *returned)
        } else {
            return Ok(None);
        };

        // require the work to observe its parameter through readonly access
        let parameter = self.declaration_symbol(*parameter)?;
        if self.selected_symbol(returned)? != Some(parameter)
            || !self.binding_accepts_readonly_borrow(
                parameter,
                body.into_any(),
                Some(removed.into_any()),
            )?
        {
            return Ok(None);
        }

        Ok(Some(removed))
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
        let is_undefined_left = self.scalar_constant(left)? == Some(dir::Literal::Undefined);
        let is_undefined_right = self.scalar_constant(right)? == Some(dir::Literal::Undefined);

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

    /// Return whether one callback expression can produce undefined.
    pub(crate) fn produces_undefined(
        &self,
        callback: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // read an unannotated lambda from its body, since its signature carries the contextual result
        if let Some(lambda) = self.lambda(callback)
            && lambda.signature.return_type.is_none()
        {
            let Some(values) = lambda
                .body
                .and_then(|body| self.callable_return_values(body))
            else {
                return Ok(true);
            };
            for value in values {
                let value = self.node_type_id(value.into_any())?;
                if self.dir.type_includes_undefined(value)? {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        let result = self.callable_return_type_id(callback)?;

        self.dir.type_includes_undefined(result)
    }

    /// Return the parameter count of one callable expression.
    pub fn callable_parameter_count(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<usize, ProviderError> {
        let type_id = self.node_type_id(expression.into_any())?;

        self.signature_parameter_count(type_id)
    }

    /// Return the parameter count one callable type's signature declares.
    pub fn signature_parameter_count(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<usize, ProviderError> {
        let signature = self
            .dir
            .callable_signature_type_id(type_id)?
            .ok_or_else(|| ProviderError::internal(format!("type {type_id:?} is not callable")))?;

        Ok(self.dir.signature_parameters(signature)?.len())
    }

    /// Return the result type id of one callable expression.
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
                    "callback expression {expression:?} has non-callable type {type_id:?}"
                ))
            })?;

        self.dir.signature_return_type_id(signature)
    }
}
