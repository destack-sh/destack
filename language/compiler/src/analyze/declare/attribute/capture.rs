use crate::Compiler;
use destack_core::StringId;
use destack_dir::{
    CaptureDirective, CaptureKind, CapturePolicy, CaptureRule, Declaration, Decorator, Expression,
    Key, LocalNodeId, LocalNodeIdAny, NodeTree, Property, ScalarLiteral,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the declaration targeted by a capture decorator.
    pub(crate) fn capture_decorator_target(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        node_id: LocalNodeIdAny,
    ) -> Option<LocalNodeId<Declaration>> {
        // accept direct declaration nodes
        if let Ok(declaration_id) = node_id.try_into_typed::<Declaration>() {
            return Some(declaration_id);
        }

        // accept expression nodes that wrap a declaration
        if let Ok(expression_id) = node_id.try_into_typed::<Expression>()
            && let Expression::Declaration(declaration) = tree.get(expression_id)
        {
            return Some(*declaration);
        }

        self.report_invalid_well_known_decorator(
            module,
            profile,
            annotation_id,
            "capture decorator is only supported on function declarations",
        );

        None
    }

    /// Parse capture directive arguments for a decorator.
    pub(crate) fn decorator_capture_directive(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<CaptureDirective> {
        // validate arity
        if values.is_empty() {
            return Some(CaptureDirective::default());
        }
        if values.len() != 1 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator expects zero or one argument"),
            );
            return None;
        }

        // unwrap trivial wrappers
        let expression_id = self.unwrap_capture_argument(tree, values[0]);
        let expression = tree.get(expression_id);

        // parse string literal policy arguments
        if let Expression::ScalarLiteral {
            value: ScalarLiteral::String(string_id),
        } = expression
        {
            let policy =
                self.capture_policy_from_string(module, profile, annotation_id, *string_id)?;
            return Some(CaptureDirective {
                policy,
                rules: Vec::new(),
            });
        }

        // parse object literal overrides
        if let Expression::ObjectExpression { properties, .. } = expression {
            return self.capture_directive_from_object_literal(
                module,
                profile,
                tree,
                annotation_id,
                decorator_name,
                properties,
            );
        }

        // reject non literal arguments
        self.report_invalid_well_known_decorator(
            module,
            profile,
            annotation_id,
            &format!("{decorator_name} decorator argument must be a string or object literal"),
        );
        None
    }

    /// Parse a capture directive from an object literal expression.
    pub(crate) fn capture_directive_from_object_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        properties: &[LocalNodeId<Property>],
    ) -> Option<CaptureDirective> {
        // initialize capture directive state
        let mut directive = CaptureDirective::default();
        let mut seen_names = HashSet::new();

        // validate object literal fields
        for property_id in properties {
            let property = tree.get(*property_id);
            let Property::Field { key, value, .. } = property else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator only supports field properties"),
                );
                return None;
            };

            // reject dynamic or invalid names
            let Key::Name(name) = key else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator requires string property names"),
                );
                return None;
            };
            let property_name = name.string();
            if !seen_names.insert(property_name) {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator contains duplicate keys"),
                );
                return None;
            }

            // get capture value
            let value_id = self.unwrap_capture_argument(tree, *value);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(value_id),
            } = tree.get(value_id)
            else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator values must be string literals"),
                );
                return None;
            };

            // set policy for that name
            if self.repository.strings.get(property_name) == "default" {
                let policy =
                    self.capture_policy_from_string(module, profile, annotation_id, *value_id)?;
                directive.policy = policy;
            } else {
                let kind =
                    self.capture_kind_from_string(module, profile, annotation_id, *value_id)?;
                directive.rules.push(CaptureRule {
                    name: property_name,
                    kind,
                });
            }
        }

        Some(directive)
    }

    /// Resolve the innermost capture argument expression.
    pub(crate) fn unwrap_capture_argument(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        // walk through trivial wrappers
        let mut current = expression_id;
        loop {
            let expression = tree.get(current);
            match expression {
                Expression::Parenthesized { expression } => {
                    current = *expression;
                }
                Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                    current = *expression;
                }
                Expression::OwnershipCast { value, .. } => {
                    current = *value;
                }
                _ => return current,
            }
        }
    }

    /// Parse a capture policy from a string literal.
    pub(crate) fn capture_policy_from_string(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        value: StringId,
    ) -> Option<CapturePolicy> {
        // compare without holding the string pool lock during diagnostics
        let value = self.repository.strings.get(value);
        let is_by_value = &*value == "byValue";
        let is_by_reference = &*value == "byReference";
        let is_by_move = &*value == "byMove";
        drop(value);

        let policy = if is_by_value {
            CapturePolicy::ByValue
        } else if is_by_reference {
            CapturePolicy::ByReference
        } else if is_by_move {
            CapturePolicy::ByMove
        } else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "capture policy must be \"byValue\", \"byReference\", or \"byMove\"",
            );
            return None;
        };

        Some(policy)
    }

    /// Parse a capture kind from a string literal.
    pub(crate) fn capture_kind_from_string(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        value: StringId,
    ) -> Option<CaptureKind> {
        // compare without holding the string pool lock during diagnostics
        let value = self.repository.strings.get(value);
        let is_by_value = &*value == "byValue";
        let is_by_reference = &*value == "byReference";
        let is_by_move = &*value == "byMove";
        drop(value);

        let kind = if is_by_value {
            CaptureKind::ByValue
        } else if is_by_reference {
            CaptureKind::ByReference
        } else if is_by_move {
            CaptureKind::ByMove
        } else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "capture kind must be \"byValue\", \"byReference\", or \"byMove\"",
            );
            return None;
        };

        Some(kind)
    }
}
