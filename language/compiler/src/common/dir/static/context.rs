use destack_artifact::{ConditionSet, EmitFormat, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_workspace::Module;

use crate::common::dir::r#static::{StaticFailure, StaticValue, static_string};

/// A static evaluator over the load-known compiler environment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticContext<'a> {
    /// The DIR view being evaluated.
    pub(crate) view: dir::View<'a>,
    /// The current module.
    pub(crate) module: &'a Module,
    /// The active profile key.
    pub(crate) profile: &'a ProfileKey,
    /// The active profile conditions.
    pub(crate) conditions: &'a ConditionSet,
    /// The shared string pool.
    pub(crate) strings: &'a StringPool,
}

impl<'a> StaticContext<'a> {
    /// Create a static context.
    pub(crate) fn new(
        view: dir::View<'a>,
        module: &'a Module,
        profile: &'a ProfileKey,
        conditions: &'a ConditionSet,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            module,
            profile,
            conditions,
            strings,
        }
    }

    /// Evaluate one expression in the static subset.
    pub(crate) fn evaluate_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<StaticValue, StaticFailure> {
        match self.view.get(expression) {
            dir::Expression::Parenthesized { expression } => self.evaluate_expression(*expression),
            dir::Expression::ScalarLiteral(value) => self.evaluate_scalar(value),
            dir::Expression::ImportMeta => Ok(StaticValue::Object),
            dir::Expression::Member { left, name } => {
                self.evaluate_member(expression, *left, *name)
            }
            dir::Expression::Unary { operator, right } => self.evaluate_unary(*operator, *right),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.evaluate_binary(*left, *operator, *right),
            dir::Expression::Call {
                left, arguments, ..
            } => self.evaluate_call(*left, arguments),
            _ => Err(StaticFailure::NotStatic(expression)),
        }
    }

    /// Evaluate one expression in the static subset as a boolean.
    pub(crate) fn evaluate_boolean(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, StaticFailure> {
        let value = self.evaluate_expression(expression)?;

        value
            .as_boolean()
            .ok_or(StaticFailure::NotBoolean(expression))
    }

    /// Evaluate one scalar literal.
    fn evaluate_scalar(&self, value: &dir::ScalarLiteral) -> Result<StaticValue, StaticFailure> {
        match value {
            dir::ScalarLiteral::Boolean(value) => Ok(StaticValue::Boolean(*value)),
            dir::ScalarLiteral::String(value) => {
                Ok(StaticValue::String(self.strings.get(*value).to_string()))
            }
            _ => Ok(StaticValue::Scalar(value.clone())),
        }
    }

    /// Evaluate one member expression.
    fn evaluate_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> Result<StaticValue, StaticFailure> {
        let Some(name) = name else {
            return Err(StaticFailure::NotStatic(expression));
        };
        let name = self.strings.get(name);

        if self.is_import_meta_target(left) {
            self.evaluate_target_member(expression, name)
        } else if self.is_import_meta(left) {
            self.evaluate_import_meta_member(expression, name)
        } else {
            Err(StaticFailure::NotStatic(expression))
        }
    }

    /// Return whether one expression is source `import.meta`.
    fn is_import_meta(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.view.get(expression) {
            dir::Expression::ImportMeta => true,
            dir::Expression::Member {
                left,
                name: Some(name),
            } => {
                let name = self.strings.get(*name);

                name == "meta" && self.is_import_keyword(*left)
            }
            _ => false,
        }
    }

    /// Return whether one expression is source `import.meta.target`.
    fn is_import_meta_target(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.view.get(expression) {
            dir::Expression::Member {
                left,
                name: Some(name),
            } => {
                let name = self.strings.get(*name);

                name == "target" && self.is_import_meta(*left)
            }
            _ => false,
        }
    }

    /// Return whether one expression is the `import` keyword path head.
    fn is_import_keyword(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.view.get(expression) {
            dir::Expression::Identifier { name } => self.strings.get(*name) == "import",
            _ => false,
        }
    }

    /// Evaluate one direct `import.meta` member.
    fn evaluate_import_meta_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
    ) -> Result<StaticValue, StaticFailure> {
        let conditions = self.conditions;

        match name {
            "url" => Ok(StaticValue::String(self.module.uri.to_string())),
            "path" => Ok(self.module_path_value()),
            "dir" => Ok(self.module_dir_value()),
            "output" => Ok(StaticValue::String(
                emit_format_tag(self.profile.emit).to_string(),
            )),
            "target" => Ok(StaticValue::Object),
            "targetName" => Ok(static_string(conditions.target.as_deref())),
            "product" => Ok(static_string(conditions.product.as_deref())),
            "platform" => Ok(static_string(platform_tag(conditions))),
            "host" => Ok(static_string(host_tag(conditions))),
            "runtime" => Ok(static_string(runtime_tag(conditions))),
            "modes" => Ok(StaticValue::Strings(
                conditions.modes.iter().cloned().collect(),
            )),
            "roles" => Ok(StaticValue::Strings(
                conditions.roles.iter().cloned().collect(),
            )),
            "features" => Ok(StaticValue::Strings(
                conditions.features.iter().cloned().collect(),
            )),
            "tags" => Ok(StaticValue::Strings(
                conditions.tags.iter().cloned().collect(),
            )),
            "debug" => Ok(StaticValue::Boolean(conditions.contains_mode("debug"))),
            "dev" => Ok(StaticValue::Boolean(conditions.contains_mode("dev"))),
            "prod" => Ok(StaticValue::Boolean(conditions.contains_mode("prod"))),
            "test" => Ok(StaticValue::Boolean(conditions.contains_mode("test"))),
            "bench" => Ok(StaticValue::Boolean(conditions.contains_mode("bench"))),
            "lint" => Ok(StaticValue::Boolean(conditions.contains_mode("lint"))),
            _ => Err(StaticFailure::NotStatic(expression)),
        }
    }

    /// Evaluate one `import.meta.target` member.
    fn evaluate_target_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
    ) -> Result<StaticValue, StaticFailure> {
        let profile = self.profile;

        match name {
            "family" => Ok(static_string(target_family_tag(self.conditions))),
            "arch" => Ok(static_string(
                profile
                    .target_arch
                    .as_ref()
                    .map(|arch| arch.triple_component()),
            )),
            "vendor" => Ok(static_string(
                profile
                    .target_vendor
                    .as_ref()
                    .map(|vendor| vendor.triple_component()),
            )),
            "abi" => Ok(static_string(
                profile
                    .target_abi
                    .as_ref()
                    .map(|abi| abi.triple_component()),
            )),
            _ => Err(StaticFailure::NotStatic(expression)),
        }
    }

    /// Build the import-time `import.meta.path` value.
    fn module_path_value(&self) -> StaticValue {
        match self.module.path.as_ref() {
            Some(path) => StaticValue::String(path.to_string_lossy().into_owned()),
            None => StaticValue::Undefined,
        }
    }

    /// Build the import-time `import.meta.dir` value.
    fn module_dir_value(&self) -> StaticValue {
        match self.module.path.as_ref().and_then(|path| path.parent()) {
            Some(path) => StaticValue::String(path.to_string_lossy().into_owned()),
            None => StaticValue::Undefined,
        }
    }

    /// Evaluate one call expression.
    fn evaluate_call(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Result<StaticValue, StaticFailure> {
        let dir::Expression::Member {
            left: receiver_expression,
            name: Some(name),
        } = self.view.get(left)
        else {
            return Err(StaticFailure::NotStatic(left));
        };

        let name = self.strings.get(*name);
        if name != "includes" {
            return Err(StaticFailure::NotStatic(left));
        }

        let [argument] = arguments else {
            return Err(StaticFailure::NotStatic(left));
        };
        let dir::Argument::Positional {
            value: argument_expression,
        } = self.view.get(*argument)
        else {
            return Err(StaticFailure::NotStatic(left));
        };

        let receiver = self.evaluate_expression(*receiver_expression);
        let receiver = receiver?
            .into_strings()
            .ok_or(StaticFailure::NotStatic(*receiver_expression))?;
        let argument = self.evaluate_expression(*argument_expression);
        let argument = argument?
            .into_string()
            .ok_or(StaticFailure::NotStatic(*argument_expression))?;

        Ok(StaticValue::Boolean(receiver.contains(&argument)))
    }

    /// Evaluate one unary expression.
    fn evaluate_unary(
        &self,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<StaticValue, StaticFailure> {
        match operator {
            dir::UnaryOperator::Not => {
                let right = self.evaluate_boolean(right)?;

                Ok(StaticValue::Boolean(!right))
            }
            _ => Err(StaticFailure::NotStatic(right)),
        }
    }

    /// Evaluate one binary expression.
    fn evaluate_binary(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<StaticValue, StaticFailure> {
        match operator {
            dir::BinaryOperator::And => {
                let left = self.evaluate_boolean(left)?;

                if left {
                    let right = self.evaluate_boolean(right)?;

                    Ok(StaticValue::Boolean(right))
                } else {
                    Ok(StaticValue::Boolean(false))
                }
            }
            dir::BinaryOperator::Or => {
                let left = self.evaluate_boolean(left)?;

                if left {
                    Ok(StaticValue::Boolean(true))
                } else {
                    let right = self.evaluate_boolean(right)?;

                    Ok(StaticValue::Boolean(right))
                }
            }
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;

                Ok(StaticValue::Boolean(left == right))
            }
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;

                Ok(StaticValue::Boolean(left != right))
            }
            _ => Err(StaticFailure::NotStatic(left)),
        }
    }
}

/// Return the canonical `import.meta.output` tag for one emit format.
fn emit_format_tag(emit: EmitFormat) -> &'static str {
    match emit {
        EmitFormat::Js => "js",
        EmitFormat::Ts => "ts",
        EmitFormat::Wasm => "wasm",
        EmitFormat::Native => "native",
    }
}

/// Return the active platform tag when present.
fn platform_tag(conditions: &ConditionSet) -> Option<&'static str> {
    conditions.platform.map(|platform| platform.canonical_tag())
}

/// Return the active host tag when present.
fn host_tag(conditions: &ConditionSet) -> Option<&'static str> {
    conditions.host.map(|host| host.canonical_tag())
}

/// Return the active runtime tag when present.
fn runtime_tag(conditions: &ConditionSet) -> Option<&'static str> {
    conditions.runtime.map(|runtime| runtime.canonical_tag())
}

/// Return the active target family tag when present.
fn target_family_tag(conditions: &ConditionSet) -> Option<&'static str> {
    conditions.platform.map(|platform| platform.family_tag())
}
