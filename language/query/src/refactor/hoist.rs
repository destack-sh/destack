use tspp_dir as dir;

use crate::{ModuleQueryContext, QueryError, QueryResult};

/// The source position before which an expression may be evaluated.
pub(super) enum HoistSite {
    /// Before one complete statement.
    Statement(dir::LocalNodeId<dir::Expression>),
    /// Before one declarator within a shared declaration statement.
    Declarator(dir::LocalNodeId<dir::Declarator>),
}

impl HoistSite {
    /// Resolve the furthest source position reachable without reordering evaluation.
    pub(super) fn resolve(
        expression: dir::LocalNodeId<dir::Expression>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        let view = module.view()?;
        let mut current = expression.into_any();

        loop {
            let Some(parent) = view.get_parent_any(current) else {
                return Ok(None);
            };

            match parent.ty {
                // cross expression parents that evaluate this child first
                dir::NodeType::Expression => {
                    let parent = Self::require_expression(parent, module)?;
                    let child = Self::require_expression(current, module)?;
                    if !Self::can_cross_expression(view.get(parent), child) {
                        return Ok(None);
                    }
                    if view.get(parent).is_statement_boundary() {
                        return Ok(Some(Self::Statement(parent)));
                    }

                    current = parent.into_any();
                }

                // cross the first argument of eligible expression lists
                dir::NodeType::Argument => {
                    let argument = parent
                        .try_into_typed::<dir::Argument>()
                        .map_err(|_| Self::invalid_parent(parent, module))?;
                    let child = Self::require_expression(current, module)?;
                    let Some(owner) = Self::cross_argument(argument, child, view, module)? else {
                        return Ok(None);
                    };

                    current = owner.into_any();
                }

                // cross the first property value
                dir::NodeType::Property => {
                    let property = parent
                        .try_into_typed::<dir::Property>()
                        .map_err(|_| Self::invalid_parent(parent, module))?;
                    let child = Self::require_expression(current, module)?;
                    let Some(owner) = Self::cross_property(property, child, view, module)? else {
                        return Ok(None);
                    };

                    current = owner.into_any();
                }

                // stop before the owning declaration or declarator
                dir::NodeType::Declarator => {
                    let declarator = parent
                        .try_into_typed::<dir::Declarator>()
                        .map_err(|_| Self::invalid_parent(parent, module))?;
                    let child = Self::require_expression(current, module)?;

                    return Self::resolve_declarator(declarator, child, view, module);
                }

                // direct source-body children are complete statements
                dir::NodeType::Block | dir::NodeType::Declaration => {
                    let statement = Self::require_expression(current, module)?;

                    return Ok(Some(Self::Statement(statement)));
                }
                _ => return Ok(None),
            }
        }
    }

    /// Resolve the declaration position owned by one initializer.
    fn resolve_declarator(
        declarator: dir::LocalNodeId<dir::Declarator>,
        child: dir::LocalNodeId<dir::Expression>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        // require the selected initializer
        if view.get(declarator).value != Some(child) {
            return Ok(None);
        }

        // require the declaration statement owner
        let Some(parent) = view.get_parent_for(declarator) else {
            return Err(Self::invalid_parent(declarator.into_any(), module));
        };
        let statement = Self::require_expression(parent, module)?;

        let site = match view.get(statement) {
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } => {
                if declarators.first() == Some(&declarator) {
                    Self::Statement(statement)
                } else {
                    Self::Declarator(declarator)
                }
            }
            dir::Expression::LetElse {
                declarator: owner, ..
            } if *owner == declarator => Self::Statement(statement),
            _ => return Ok(None),
        };

        Ok(Some(site))
    }

    /// Cross one eligible first argument into its owning expression.
    fn cross_argument(
        argument: dir::LocalNodeId<dir::Argument>,
        child: dir::LocalNodeId<dir::Expression>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
        // require the selected argument value
        if view.get(argument).value() != Some(child) {
            return Ok(None);
        }

        // require the argument list owner
        let Some(parent) = view.get_parent_for(argument) else {
            return Err(Self::invalid_parent(argument.into_any(), module));
        };
        let owner = Self::require_expression(parent, module)?;

        let is_first = match view.get(owner) {
            dir::Expression::Call {
                left, arguments, ..
            } => {
                let is_direct = matches!(view.get(*left), dir::Expression::Identifier { .. });

                is_direct && arguments.first() == Some(&argument)
            }
            dir::Expression::New { arguments, .. }
            | dir::Expression::ArrayExpression {
                elements: arguments,
            }
            | dir::Expression::TupleExpression {
                elements: arguments,
            } => arguments.first() == Some(&argument),
            dir::Expression::TemplateExpression {
                value: dir::TemplateLiteral::InterpolatedString { arguments, .. },
            } => arguments.first() == Some(&argument),
            _ => false,
        };

        Ok(is_first.then_some(owner))
    }

    /// Cross one eligible first property value into its owning expression.
    fn cross_property(
        property: dir::LocalNodeId<dir::Property>,
        child: dir::LocalNodeId<dir::Expression>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let value = match view.get(property) {
            dir::Property::Field { value, .. } | dir::Property::Spread { value } => Some(*value),
            dir::Property::Method { .. } | dir::Property::Error => None,
        };

        // require the selected property value
        if value != Some(child) {
            return Ok(None);
        }

        // require the property list owner
        let Some(parent) = view.get_parent_for(property) else {
            return Err(Self::invalid_parent(property.into_any(), module));
        };
        let owner = Self::require_expression(parent, module)?;
        let is_first = match view.get(owner) {
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::StructExpression { properties, .. } => {
                properties.first() == Some(&property)
            }
            _ => false,
        };

        Ok(is_first.then_some(owner))
    }

    /// Return whether the selected child may cross its expression parent.
    fn can_cross_expression(
        expression: &dir::Expression,
        child: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match expression {
            dir::Expression::Return { value }
            | dir::Expression::Break { value, .. }
            | dir::Expression::Yield { value, .. } => *value == Some(child),
            dir::Expression::Await { expression }
            | dir::Expression::AwaitMaybe { expression }
            | dir::Expression::AwaitMust { expression }
            | dir::Expression::Chain { expression }
            | dir::Expression::Const { body: expression }
            | dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => *expression == child,
            dir::Expression::Unary { right, .. } | dir::Expression::BorrowOf { right, .. } => {
                *right == child
            }
            dir::Expression::Member { left, .. }
            | dir::Expression::Instantiation { left, .. }
            | dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
            | dir::Expression::Index { left, .. }
            | dir::Expression::Binary { left, .. } => *left == child,
            dir::Expression::RangeExpression { start, end, .. } => {
                *start == Some(child) || (start.is_none() && *end == Some(child))
            }
            dir::Expression::FixedArrayExpression { value, .. }
            | dir::Expression::Is { value, .. }
            | dir::Expression::InstanceOf { value, .. }
            | dir::Expression::Match { value, .. }
            | dir::Expression::Switch { value, .. } => *value == child,
            dir::Expression::If { condition, .. } => condition.as_expression() == Some(child),
            _ => false,
        }
    }

    /// Require one erased node to be an expression.
    fn require_expression(
        node: dir::LocalNodeIdAny,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<dir::LocalNodeId<dir::Expression>> {
        node.try_into_typed::<dir::Expression>()
            .map_err(|_| Self::invalid_parent(node, module))
    }

    /// Build an error for one incompatible expression parent.
    fn invalid_parent(node: dir::LocalNodeIdAny, module: &ModuleQueryContext<'_>) -> QueryError {
        QueryError::invalid(format!(
            "expression parent: {:?}",
            node.into_global(module.module_id())
        ))
    }
}
