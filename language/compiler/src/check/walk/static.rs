use destack_dir as dir;

use crate::check::{CheckModuleState, Decorator, StaticIfCondition, StaticTerm, VariableId};
use crate::common::dir::r#static::{StaticContext, StaticFailure};

impl CheckModuleState {
    /// Return whether static decorators attached to one owner allow it.
    pub(in crate::check) fn static_allows(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
    ) -> bool {
        let decorators = self.decorators_for_owner(owner);

        // evaluate visible static guards in source order
        for decorator in decorators {
            match decorator {
                Decorator::StaticIf(decorator) => {
                    let StaticIfCondition::Present(condition) = decorator.condition else {
                        self.report_invalid_static_condition(decorator.condition_anchor());

                        return false;
                    };
                    if !self.evaluate_static_guard(tree, condition) {
                        return false;
                    }
                }
                Decorator::Other(call) => {
                    let decorator_node = self.input.view().get(call.decorator).clone();

                    self.walk_decorator(tree, call.decorator, &decorator_node);
                }
                Decorator::LanguageItem(_)
                | Decorator::Intrinsic(_)
                | Decorator::Representation(_)
                | Decorator::Capture(_)
                | Decorator::Diagnostic(_)
                | Decorator::Foreign(_)
                | Decorator::Restriction(_)
                | Decorator::System(_)
                | Decorator::Stability(_)
                | Decorator::Taint(_)
                | Decorator::Macro(_) => {}
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
                    // check deferred guard terms in static context
                    let before_condition = self.checkpoint_flow();

                    self.walk_expression(tree, condition, tree.get(condition));
                    self.restore_flow(before_condition);

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
            // type relation
            dir::Expression::Type { value } => self.is_deferred_static_type_guard(tree, *value),
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

    /// Return whether one type expression belongs to solver-known static state.
    fn is_deferred_static_type_guard(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> bool {
        match tree.get(id) {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.is_deferred_static_type_guard(tree, *expression)
            }
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                self.type_reference_is_generic_parameter(id, path)
                    || generic_arguments
                        .iter()
                        .any(|argument| self.is_deferred_static_generic_argument(tree, *argument))
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                generic_arguments,
                ..
            } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || generic_arguments
                        .iter()
                        .any(|argument| self.is_deferred_static_generic_argument(tree, *argument))
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *extends_type)
                    || self.is_deferred_static_type_guard(tree, *then_type)
                    || self.is_deferred_static_type_guard(tree, *else_type)
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            // T implements U
            | dir::TypeExpression::Implements { left, right } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *right)
            }
            // readonly T, local T, and similar unary type forms
            dir::TypeExpression::Readonly { target_type }
            | dir::TypeExpression::Local { target_type }
            | dir::TypeExpression::Shared { target_type }
            | dir::TypeExpression::KeyOf { target_type }
            | dir::TypeExpression::Must { target_type }
            | dir::TypeExpression::Not { target_type }
            | dir::TypeExpression::OwnedOf { target_type, .. }
            | dir::TypeExpression::BorrowedOf { target_type, .. }
            | dir::TypeExpression::PointerOf { target_type, .. } => {
                self.is_deferred_static_type_guard(tree, *target_type)
            }
            // T | U
            dir::TypeExpression::Union { elements }
            // T & U
            | dir::TypeExpression::Intersection { elements } => elements
                .iter()
                .any(|element| self.is_deferred_static_type_guard(tree, *element)),
            // T[]
            dir::TypeExpression::Array { element }
            // [T]
            | dir::TypeExpression::Slice { element } => {
                self.is_deferred_static_type_guard(tree, *element)
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                self.is_deferred_static_type_guard(tree, *element)
                    || self.is_deferred_static_guard(tree, *length)
            }
            // typeof value
            dir::TypeExpression::TypeOfValue { value } => self.is_deferred_static_guard(tree, *value),
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *index)
            }
            // infer T extends U
            dir::TypeExpression::Infer { constraint, .. } => constraint
                .is_some_and(|constraint| self.is_deferred_static_type_guard(tree, constraint)),
            // value is T
            dir::TypeExpression::Predicate { target, .. } => target
                .is_some_and(|target| self.is_deferred_static_type_guard(tree, target)),
            // type forms without static generic leaves
            _ => false,
        }
    }

    /// Return whether one generic argument belongs to solver-known static state.
    fn is_deferred_static_generic_argument(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> bool {
        match tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value }
            // <...T>
            | dir::GenericArgument::SpreadType { value } => {
                self.is_deferred_static_type_guard(tree, *value)
            }
            // <C>
            dir::GenericArgument::Value { value }
            // <...C>
            | dir::GenericArgument::SpreadValue { value } => {
                self.is_deferred_static_guard(tree, *value)
            }
            // ignore damaged syntax
            dir::GenericArgument::Error => false,
        }
    }

    /// Return whether one reference names a generic parameter.
    fn type_reference_is_generic_parameter(
        &self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
    ) -> bool {
        let [name] = path.segments.as_slice() else {
            return false;
        };

        if let Some(symbol) = self
            .lookup_name(source.into_any(), *name, dir::SymbolSpace::Type)
            .unique_symbol()
            && self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericTypeParameter)
        {
            return true;
        }

        self.lookup_name(source.into_any(), *name, dir::SymbolSpace::Value)
            .unique_symbol()
            .is_some_and(|symbol| {
                self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericValueParameter)
            })
    }

    /// Return one static argument variable.
    pub(in crate::check) fn static_argument_variable(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> VariableId {
        let source = id.into_global_any(self.input.module_id);
        let variable = self.intern_node_static_variable(source);

        if let Some(term) = self.build_static_argument_term(id, tree) {
            self.define_static(variable, term);
        }

        variable
    }

    /// Return one static argument term.
    fn build_static_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let term = match tree.get(id) {
            // <(C)>
            dir::TypeExpression::Parenthesized { expression } => {
                return self.build_static_argument_term(*expression, tree);
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
            dir::TypeExpression::Object { members } => {
                self.build_static_object_term(members, tree)?
            }
            // <[1, 2]>
            dir::TypeExpression::Tuple { elements } => {
                self.build_static_tuple_term(elements, tree)?
            }
            // <readonly [1, 2]>
            dir::TypeExpression::ArrayTuple { elements } => {
                self.build_static_tuple_term(elements, tree)?
            }
            // <C>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let [name] = path.segments.as_slice() else {
                    return None;
                };
                if generic_arguments.is_empty()
                    && let Some(symbol) = self
                        .lookup_name(id.into_any(), *name, dir::SymbolSpace::Value)
                        .unique_symbol()
                    && self.symbol_kind(symbol) == Some(dir::SymbolKind::GenericValueParameter)
                {
                    let variable = self.intern_symbol_static_variable(symbol);

                    StaticTerm::Variable(variable)
                }
                // <LifetimeOf<T>>
                else if let Some(symbol) = self
                    .lookup_name(id.into_any(), *name, dir::SymbolSpace::Type)
                    .unique_symbol()
                    && let Some(item) = self.input.environment.language.item(symbol)
                    && Self::memory_static_intrinsic_item(item)
                {
                    let arguments =
                        self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);

                    StaticTerm::Intrinsic {
                        item,
                        arguments: arguments.into(),
                    }
                }
                // unsupported reference syntax
                else {
                    return None;
                }
            }
            // <T.Value>
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.build_generic_arguments(generic_arguments, tree);

                StaticTerm::Member {
                    source: Some(id.into_global_any(self.input.module_id)),
                    owner: self.intern_local_type_variable(*left),
                    key: dir::StaticKey::Name(*name),
                    arguments,
                }
            }
            // unsupported static argument syntax
            _ => return None,
        };

        Some(term)
    }

    /// Return one static object term.
    fn build_static_object_term(
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
                    let StaticTerm::Literal(value) =
                        self.build_static_argument_term(*value, tree)?
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
    fn build_static_tuple_term(
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
                    let StaticTerm::Literal(value) =
                        self.build_static_argument_term(*value, tree)?
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
