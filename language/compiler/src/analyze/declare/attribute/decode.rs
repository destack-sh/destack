use crate::Compiler;
use destack_core::StringId;
use destack_dir::{
    Argument, Binding, Decorator, Expression, LifetimeAnnotation, LocalNodeId, NodeTree,
    ScalarLiteral,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect positional argument values for a decorator.
    pub(crate) fn decorator_argument_values(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        arguments: Option<&[LocalNodeId<Argument>]>,
    ) -> Option<Vec<LocalNodeId<Expression>>> {
        // map arguments to expression values
        let mut values = Vec::new();
        let Some(arguments) = arguments else {
            return Some(values);
        };

        for argument_id in arguments {
            let argument = tree.get(*argument_id);
            let value_id = match argument {
                Argument::Positional { value, .. }
                | Argument::Named { value, .. }
                | Argument::Labeled { value, .. }
                | Argument::Error { value } => *value,
                Argument::Spread { .. } => {
                    self.report_invalid_well_known_decorator(
                        module,
                        profile,
                        annotation_id,
                        &format!("{decorator_name} decorator does not support spread arguments"),
                    );
                    return None;
                }
            };
            values.push(value_id);
        }

        Some(values)
    }

    /// Parse a single string argument for a decorator.
    pub(crate) fn decorator_string_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<StringId>> {
        // validate arity
        if values.is_empty() {
            return Some(None);
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

        // extract string literal
        let expr = tree.get(values[0]);
        let Expression::ScalarLiteral {
            value: ScalarLiteral::String(string_id),
        } = expr
        else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be a string literal"),
            );
            return None;
        };

        Some(Some(*string_id))
    }

    /// Parse binding decorator arguments.
    pub(crate) fn decorator_binding_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Binding> {
        // allow empty bindings: defaults to the declaration name
        if values.is_empty() {
            return Some(Binding { name: None });
        }

        // validate arity
        if values.len() > 2 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "binding decorator expects zero, one, or two arguments",
            );
            return None;
        }

        // parse the optional external binding name
        let name = {
            let expression = tree.get(values[0]);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(string_id),
            } = expression
            else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    "binding decorator first argument must be a string literal",
                );
                return None;
            };

            Some(*string_id)
        };

        // validate the optional binding options payload
        if values.len() == 2 {
            let expression = tree.get(values[1]);
            if !matches!(expression, Expression::ObjectExpression { .. }) {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    "binding decorator second argument must be an object literal",
                );
                return None;
            }
        }

        Some(Binding { name })
    }

    /// Parse zero or more string arguments for a decorator.
    pub(crate) fn decorator_string_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Vec<StringId>> {
        // allow empty argument lists
        if values.is_empty() {
            return Some(Vec::new());
        }

        // collect unique string literal arguments
        let mut labels = Vec::new();
        for value_id in values {
            let expression = tree.get(*value_id);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(string_id),
            } = expression
            else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!("{decorator_name} decorator arguments must be string literals"),
                );
                return None;
            };

            if !labels.contains(string_id) {
                labels.push(*string_id);
            }
        }

        Some(labels)
    }

    /// Parse a single integer argument for a decorator.
    pub(crate) fn decorator_u32_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<u32>> {
        // validate arity
        if values.is_empty() {
            return Some(None);
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

        // extract integer literal
        let expr = tree.get(values[0]);
        let value = match expr {
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => Some(*value),
            Expression::ScalarLiteral {
                value: ScalarLiteral::Bigint(value),
            } => Some(*value),
            _ => None,
        };
        let Some(value) = value else {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be an integer literal"),
            );
            return None;
        };

        // validate non-negative value
        if value < 0 {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator argument must be non-negative"),
            );
            return None;
        }

        Some(Some(value as u32))
    }

    /// Parse lifetime annotation arguments for a decorator.
    pub(crate) fn decorator_lifetime_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Decorator>,
        decorator_name: &str,
        values: &[LocalNodeId<Expression>],
    ) -> Option<Option<LifetimeAnnotation>> {
        // allow empty arguments: defaults to inferred
        if values.is_empty() {
            return Some(None);
        }

        // collect parameter names or static marker
        let mut names = Vec::new();
        let mut is_static = false;

        // scan argument values
        for value_id in values {
            let expr = tree.get(*value_id);

            // extract identifier or string literal name
            let name_id = match expr {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(string_id),
                } => Some(*string_id),
                Expression::UnresolvedPath { path, .. }
                | Expression::LocalReference { path, .. }
                | Expression::ModuleReference { path, .. }
                | Expression::GlobalReference { path, .. } => {
                    if path.segments.len() == 1 {
                        Some(path.segments[0])
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(name_id) = name_id else {
                self.report_invalid_well_known_decorator(
                    module,
                    profile,
                    annotation_id,
                    &format!(
                        "{decorator_name} decorator arguments must be string literals or identifiers"
                    ),
                );
                return None;
            };

            // check for static lifetime
            let name = self.repository.strings.get(name_id);
            if name == "static" {
                is_static = true;
                continue;
            }

            if !names.contains(&name_id) {
                names.push(name_id);
            }
        }

        // disallow mixing static with other names
        if is_static && !names.is_empty() {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                &format!("{decorator_name} decorator cannot mix static and parameters"),
            );
            return None;
        }

        // emit the final annotation
        if is_static {
            return Some(Some(LifetimeAnnotation::Static));
        }

        if names.is_empty() {
            return Some(None);
        }

        Some(Some(LifetimeAnnotation::Parameters(names)))
    }
}
