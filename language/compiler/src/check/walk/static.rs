use destack_dir as dir;

use crate::check::{CheckModuleState, StaticTerm, VariableId};
use crate::common::dir::r#static::{
    StaticContext, StaticFailure, StaticGuard, StaticGuardError, static_guard,
};

impl CheckModuleState {
    /// Return whether static decorators attached to one owner allow it.
    pub(in crate::check) fn static_allows(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
    ) -> bool {
        let decorators = self.input.view().get_decorators_any(owner);

        // evaluate visible static guards in source order
        for decorator in decorators {
            let view = self.input.view();
            let guard = static_guard(view, &self.input.strings, decorator);

            match guard {
                // ordinary decorator
                StaticGuard::Ordinary => {
                    let decorator_node = view.get(decorator).clone();

                    self.walk_decorator(tree, decorator, &decorator_node);
                }
                // malformed @if decorator
                StaticGuard::Rejected(error) => {
                    self.report_static_guard_error(error);

                    return false;
                }
                // @if(condition)
                StaticGuard::Condition(condition) => {
                    if !self.evaluate_static_guard(tree, condition) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Evaluate one static guard condition.
    fn evaluate_static_guard(
        &mut self,
        tree: &dir::Tree,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let context = StaticContext::new(
            self.input.view(),
            self.input.module.as_ref(),
            &self.input.profile,
            &self.input.profile.conditions,
            &self.input.strings,
        );

        match context.evaluate_boolean(condition) {
            Ok(true) => true,
            Ok(false) => false,
            Err(StaticFailure::NotBoolean(expression)) => {
                self.report_invalid_static_condition(expression.into_any());

                false
            }
            Err(StaticFailure::NotStatic(expression)) => {
                if self.is_deferred_static_guard(tree, condition) {
                    true
                } else {
                    self.walk_expression(tree, expression, tree.get(expression));
                    self.report_invalid_static_condition(expression.into_any());

                    false
                }
            }
        }
    }

    /// Return whether one guard belongs to solver-known static state.
    fn is_deferred_static_guard(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression } => {
                self.is_deferred_static_guard(tree, *expression)
            }
            // this or this.X
            dir::Expression::This => true,
            // C && D, C == D, and similar static compositions
            dir::Expression::Binary { left, right, .. } => {
                self.is_deferred_static_guard(tree, *left)
                    || self.is_deferred_static_guard(tree, *right)
            }
            // !C
            dir::Expression::Unary { right, .. } => self.is_deferred_static_guard(tree, *right),
            // C.X
            dir::Expression::Member { left, .. } => self.is_deferred_static_guard(tree, *left),
            // C[I]
            dir::Expression::Index { left, index, .. } => {
                self.is_deferred_static_guard(tree, *left)
                    || index.is_some_and(|index| self.is_deferred_static_guard(tree, index))
            }
            // not solver-known static guard syntax
            _ => false,
        }
    }

    /// Report one malformed static guard.
    fn report_static_guard_error(&mut self, error: StaticGuardError) {
        match error {
            // @if
            StaticGuardError::MissingCondition { node }
            // @if(a, b)
            | StaticGuardError::MultipleConditions { node }
            // @if(<invalid>)
            | StaticGuardError::InvalidCondition { node } => {
                self.report_invalid_static_condition(node);
            }
        }
    }

    /// Return one static argument variable.
    pub(in crate::check) fn static_argument_variable(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> VariableId {
        let source = id.into_global_any(self.input.module_id);
        let variable = self.intern_node_static_variable(source);

        if let Some(term) = self.static_argument_term(id, tree) {
            self.define_static_term(variable, term);
        }

        variable
    }

    /// Return one static argument term.
    fn static_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let term = match tree.get(id) {
            // <(C)>
            dir::TypeExpression::Parenthesized { expression } => {
                return self.static_argument_term(*expression, tree);
            }
            // <1>
            dir::TypeExpression::ScalarLiteral { value } => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: value.clone(),
                })
            }
            // <"name">
            dir::TypeExpression::Literal { value } => {
                StaticTerm::Literal(dir::StaticTerm::TypeLiteral {
                    value: value.clone(),
                })
            }
            // <{ name: "value" }>
            dir::TypeExpression::Object { members } => self.static_object_term(members, tree)?,
            // <[1, 2]>
            dir::TypeExpression::Tuple { elements } => self.static_tuple_term(elements, tree)?,
            // <readonly [1, 2]>
            dir::TypeExpression::ArrayTuple { elements } => {
                self.static_tuple_term(elements, tree)?
            }
            // <C>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } if generic_arguments.is_empty() => {
                let [name] = path.segments.as_slice() else {
                    return None;
                };
                let symbol = self
                    .lookup_name(id.into_any(), *name, dir::SymbolSpace::Value)
                    .unique_symbol()?;

                if self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericValueParameter) {
                    let variable = self.intern_symbol_static_variable(symbol);

                    StaticTerm::Variable(variable)
                } else {
                    return None;
                }
            }
            // unsupported static argument syntax
            _ => return None,
        };

        Some(term)
    }

    /// Return one static object term.
    fn static_object_term(
        &mut self,
        members: &[dir::LocalNodeId<dir::TypeMember>],
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let mut properties = Vec::with_capacity(members.len());

        for member in members {
            match tree.get(*member) {
                // <{ key: value }>
                dir::TypeMember::Field {
                    key,
                    declared_type: Some(value),
                    is_static: false,
                    is_optional: false,
                    ..
                } => {
                    let key = key.static_key(tree)?;
                    let StaticTerm::Literal(value) = self.static_argument_term(*value, tree)?
                    else {
                        return None;
                    };

                    properties.push(dir::StaticProperty::Field { key, value });
                }
                // unsupported object member syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Object { properties }))
    }

    /// Return one static tuple term.
    fn static_tuple_term(
        &mut self,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let mut values = Vec::with_capacity(elements.len());

        for element in elements {
            match tree.get(*element) {
                // <[value]>
                dir::TupleElement::Element {
                    value,
                    is_optional: false,
                    ..
                } => {
                    let StaticTerm::Literal(value) = self.static_argument_term(*value, tree)?
                    else {
                        return None;
                    };

                    values.push(value);
                }
                // unsupported tuple element syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Tuple {
            elements: values,
        }))
    }
}
