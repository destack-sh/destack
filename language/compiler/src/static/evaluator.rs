use tspp_artifact::ProfileKey;
use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_repository::{Environment, Module, Package};

/// A static expression evaluation failure.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum StaticError {
    /// The expression is not statically decidable by this evaluator.
    NotStatic(dir::LocalNodeId<dir::Expression>),
    /// The expression did not evaluate to a boolean where one is required.
    NotBoolean(dir::LocalNodeId<dir::Expression>),
}

/// One static term or metadata namespace.
enum StaticValue {
    /// A concrete static term.
    Term(dir::StaticTerm),
    /// The `import.meta` namespace.
    ImportMeta,
    /// The structured `import.meta.target` namespace.
    Target,
    /// The `import.meta.labels` namespace.
    Labels,
    /// The `import.meta.env` namespace.
    Environment,
}

/// An evaluator over module, package, profile, and revision inputs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticEvaluator<'a> {
    /// The DIR view being evaluated.
    view: dir::View<'a>,
    /// The current module.
    module: &'a Module,
    /// The package containing the current module.
    package: &'a Package,
    /// The ambient environment captured by the current revision.
    environment: &'a Environment,
    /// The active profile key.
    profile: &'a ProfileKey,
    /// The shared string pool.
    strings: &'a StringPool,
}

impl<'a> StaticEvaluator<'a> {
    /// Create a static evaluator.
    pub(crate) fn new(
        view: dir::View<'a>,
        module: &'a Module,
        package: &'a Package,
        environment: &'a Environment,
        profile: &'a ProfileKey,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            module,
            package,
            environment,
            profile,
            strings,
        }
    }

    /// Evaluate one expression in the static subset.
    pub(crate) fn evaluate_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        match self.evaluate_value(expression)? {
            StaticValue::Term(value) => Ok(value),
            StaticValue::ImportMeta
            | StaticValue::Target
            | StaticValue::Labels
            | StaticValue::Environment => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Evaluate one expression as a term or metadata namespace.
    fn evaluate_value(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<StaticValue, StaticError> {
        match self.view.get(expression) {
            dir::Expression::Literal(value) => Ok(StaticValue::Term((*value).into())),
            dir::Expression::ImportMeta => Ok(StaticValue::ImportMeta),
            dir::Expression::Member {
                left,
                name,
                is_optional: false,
            } => self.evaluate_member(expression, *left, *name),
            dir::Expression::Index {
                position: dir::PostfixPosition::Direct,
                left,
                index: Some(index),
                is_optional: false,
            } => self.evaluate_index(expression, *left, *index),
            dir::Expression::Unary { operator, right } => self
                .evaluate_unary(expression, *operator, *right)
                .map(StaticValue::Term),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self
                .evaluate_binary(expression, *left, *operator, *right)
                .map(StaticValue::Term),
            dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left,
                generic_arguments,
                arguments,
                is_optional: false,
            } if generic_arguments.is_empty() => self
                .evaluate_call(expression, *left, arguments)
                .map(StaticValue::Term),
            _ => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Evaluate one expression in the static subset as a boolean.
    pub(crate) fn evaluate_boolean(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, StaticError> {
        match self.evaluate_expression(expression)? {
            dir::StaticTerm::Literal {
                value: dir::Literal::Boolean(value),
            } => Ok(value),
            _ => Err(StaticError::NotBoolean(expression)),
        }
    }

    /// Evaluate one member expression.
    fn evaluate_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> Result<StaticValue, StaticError> {
        let Some(name) = name else {
            return Err(StaticError::NotStatic(expression));
        };
        let owner = self.evaluate_value(left)?;

        self.evaluate_key(expression, owner, dir::StaticKey::Name(name))
    }

    /// Evaluate one statically indexed expression.
    fn evaluate_index(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: dir::LocalNodeId<dir::Expression>,
    ) -> Result<StaticValue, StaticError> {
        let owner = self.evaluate_value(left)?;
        let index = self.evaluate_expression(index)?;
        let key = match index.as_scalar() {
            Some(dir::Literal::String(name)) => dir::StaticKey::Name(name),
            Some(dir::Literal::Integer(index)) => {
                let Ok(index) = usize::try_from(index) else {
                    return Err(StaticError::NotStatic(expression));
                };

                dir::StaticKey::Index(index)
            }
            _ => return Err(StaticError::NotStatic(expression)),
        };

        self.evaluate_key(expression, owner, key)
    }

    /// Read one key from an evaluated static value.
    fn evaluate_key(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        owner: StaticValue,
        key: dir::StaticKey,
    ) -> Result<StaticValue, StaticError> {
        match owner {
            StaticValue::ImportMeta => self.import_meta_member(expression, key),
            StaticValue::Target => self.target_member(expression, key).map(StaticValue::Term),
            StaticValue::Labels => Ok(StaticValue::Term(self.label_member(key))),
            StaticValue::Environment => Ok(StaticValue::Term(self.environment_member(key))),
            StaticValue::Term(owner) => self.evaluate_term_key(expression, &owner, key),
        }
    }

    /// Read one key from a concrete static term.
    fn evaluate_term_key(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        owner: &dir::StaticTerm,
        key: dir::StaticKey,
    ) -> Result<StaticValue, StaticError> {
        match (owner, key) {
            (
                dir::StaticTerm::Array { elements } | dir::StaticTerm::Tuple { elements },
                dir::StaticKey::Index(index),
            ) => {
                let value = elements
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| dir::Literal::Undefined.into());

                Ok(StaticValue::Term(value))
            }
            _ => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Read one key from the `import.meta` namespace.
    fn import_meta_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        key: dir::StaticKey,
    ) -> Result<StaticValue, StaticError> {
        let Some(name) = key.name() else {
            return Err(StaticError::NotStatic(expression));
        };
        let name = self.strings.get(name);
        let conditions = &self.profile.conditions;

        let value = match name {
            "url" => StaticValue::Term(self.string(self.module.uri.as_ref())),
            "path" | "file" | "filename" => StaticValue::Term(self.module_path_value(expression)?),
            "dir" | "dirname" => StaticValue::Term(self.module_dir_value(expression)?),
            "output" => StaticValue::Term(self.string(self.profile.output.canonical_tag())),
            "platform" => StaticValue::Term(self.string(conditions.platform.canonical_tag())),
            "runtime" => StaticValue::Term(self.string(conditions.runtime.canonical_tag())),
            "target" => StaticValue::Target,
            "targetName" => StaticValue::Term(self.optional_string(conditions.target.as_deref())),
            "product" => StaticValue::Term(self.optional_string(conditions.product.as_deref())),
            "version" => StaticValue::Term(self.optional_string(self.package.version.as_deref())),
            "stability" => StaticValue::Term(
                self.optional_string(self.profile.stability.map(|stability| stability.name())),
            ),
            "derive" => return Err(StaticError::NotStatic(expression)),
            "role" => StaticValue::Term(self.optional_string(conditions.role.as_deref())),
            "labels" => StaticValue::Labels,
            "host" => StaticValue::Term(self.string(conditions.host.canonical_tag())),
            "modes" => StaticValue::Term(self.string_array(conditions.modes.iter())),
            "roles" => StaticValue::Term(self.string_array(conditions.roles.iter())),
            "features" => StaticValue::Term(self.string_array(conditions.features.iter())),
            "tags" => StaticValue::Term(self.string_array(conditions.tags.iter())),
            "debug" => {
                StaticValue::Term(dir::Literal::Boolean(conditions.contains_mode("debug")).into())
            }
            "dev" => {
                StaticValue::Term(dir::Literal::Boolean(conditions.contains_mode("dev")).into())
            }
            "prod" => {
                StaticValue::Term(dir::Literal::Boolean(conditions.contains_mode("prod")).into())
            }
            "test" => {
                StaticValue::Term(dir::Literal::Boolean(conditions.contains_mode("test")).into())
            }
            "lint" => {
                StaticValue::Term(dir::Literal::Boolean(conditions.contains_mode("lint")).into())
            }
            "env" => StaticValue::Environment,
            _ => return Err(StaticError::NotStatic(expression)),
        };

        Ok(value)
    }

    /// Read one key from structured target metadata.
    fn target_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        key: dir::StaticKey,
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some(name) = key.name() else {
            return Err(StaticError::NotStatic(expression));
        };
        let name = self.strings.get(name);

        let value = match name {
            "family" => self.string(self.profile.conditions.platform.family_tag()),
            "arch" => self.optional_string(
                self.profile
                    .architecture
                    .as_ref()
                    .map(|value| value.triple_component()),
            ),
            "vendor" => self.optional_string(
                self.profile
                    .vendor
                    .as_ref()
                    .map(|value| value.triple_component()),
            ),
            "abi" => self.optional_string(
                self.profile
                    .abi
                    .as_ref()
                    .map(|value| value.triple_component()),
            ),
            _ => return Err(StaticError::NotStatic(expression)),
        };

        Ok(value)
    }

    /// Read one active source graph label.
    fn label_member(&self, key: dir::StaticKey) -> dir::StaticTerm {
        let Some(name) = key.name() else {
            return dir::Literal::Undefined.into();
        };
        let name = self.strings.get(name);

        self.profile
            .conditions
            .labels
            .get(name)
            .map(|values| self.string_array(values.iter()))
            .unwrap_or_else(|| dir::Literal::Undefined.into())
    }

    /// Read one environment variable allowed by the active profile.
    fn environment_member(&self, key: dir::StaticKey) -> dir::StaticTerm {
        let Some(name) = key.name() else {
            return dir::Literal::Undefined.into();
        };
        let name = self.strings.get(name);
        if !self.profile.env.contains(name) {
            return dir::Literal::Undefined.into();
        }

        self.optional_string(self.environment.get(name))
    }

    /// Build the import-time `import.meta.path` value.
    fn module_path_value(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some(path) = &self.module.path else {
            return Ok(dir::Literal::Undefined.into());
        };
        let Some(path) = path.to_str() else {
            return Err(StaticError::NotStatic(expression));
        };

        Ok(self.string(path))
    }

    /// Build the import-time `import.meta.dir` value.
    fn module_dir_value(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some(path) = &self.module.path else {
            return Ok(dir::Literal::Undefined.into());
        };
        let Some(directory) = path.parent() else {
            return Ok(dir::Literal::Undefined.into());
        };
        let Some(directory) = directory.to_str() else {
            return Err(StaticError::NotStatic(expression));
        };

        Ok(self.string(directory))
    }

    /// Evaluate one unary expression.
    fn evaluate_unary(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        let Ok(operator) = dir::StaticUnaryOperator::try_from(operator) else {
            return Err(StaticError::NotStatic(expression));
        };
        let Some(right) = self.evaluate_expression(right)?.as_scalar() else {
            return Err(StaticError::NotStatic(expression));
        };

        match operator.apply(right) {
            Ok(value) => Ok(value.into()),
            Err(_) => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Evaluate one binary expression.
    fn evaluate_binary(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        match operator {
            // &&
            dir::BinaryOperator::And => {
                let value = self.evaluate_boolean(left)? && self.evaluate_boolean(right)?;

                Ok(dir::Literal::Boolean(value).into())
            }
            // ||
            dir::BinaryOperator::Or => {
                let value = self.evaluate_boolean(left)? || self.evaluate_boolean(right)?;

                Ok(dir::Literal::Boolean(value).into())
            }
            // scalar equality
            dir::BinaryOperator::Equal
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::NotEqualStrict => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                let (Some(left), Some(right)) = (left.as_scalar(), right.as_scalar()) else {
                    return Err(StaticError::NotStatic(expression));
                };

                // reject reference-valued regex literals
                if matches!(left, dir::Literal::RegexString { .. })
                    || matches!(right, dir::Literal::RegexString { .. })
                {
                    return Err(StaticError::NotStatic(expression));
                }

                // compare scalar values
                let is_equal = left == right;
                let value = if operator.is_negative_equality() {
                    !is_equal
                } else {
                    is_equal
                };

                Ok(dir::Literal::Boolean(value).into())
            }
            // scalar operators evaluate over literal operands
            _ => {
                let Ok(operator) = dir::StaticBinaryOperator::try_from(operator) else {
                    return Err(StaticError::NotStatic(expression));
                };
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                let (Some(left), Some(right)) = (left.as_scalar(), right.as_scalar()) else {
                    return Err(StaticError::NotStatic(expression));
                };

                // concatenate string literals through the shared pool
                if let (
                    dir::StaticBinaryOperator::Add,
                    dir::Literal::String(left),
                    dir::Literal::String(right),
                ) = (operator, left, right)
                {
                    let joined = format!("{}{}", self.strings.get(left), self.strings.get(right));

                    return Ok(self.string(&joined));
                }

                match operator.apply(left, right) {
                    Ok(value) => Ok(value.into()),
                    Err(_) => Err(StaticError::NotStatic(expression)),
                }
            }
        }
    }

    /// Build one string term, interning the text into the shared pool.
    fn string(&self, value: &str) -> dir::StaticTerm {
        dir::Literal::String(self.strings.intern(value)).into()
    }

    /// Build one optional string term, undefined when absent.
    fn optional_string(&self, value: Option<&str>) -> dir::StaticTerm {
        match value {
            Some(value) => self.string(value),
            None => dir::Literal::Undefined.into(),
        }
    }

    /// Build one array term of string elements.
    fn string_array<'s>(&self, values: impl Iterator<Item = &'s String>) -> dir::StaticTerm {
        dir::StaticTerm::Array {
            elements: values.map(|value| self.string(value)).collect(),
        }
    }

    /// Evaluate one `array.includes(value)` membership call.
    fn evaluate_call(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some((receiver, value)) = self.array_includes_call(callee, arguments) else {
            return Err(StaticError::NotStatic(expression));
        };

        // evaluate the string array intrinsic
        let dir::StaticTerm::Array { elements } = self.evaluate_expression(receiver)? else {
            return Err(StaticError::NotStatic(receiver));
        };
        let dir::StaticTerm::Literal {
            value: dir::Literal::String(value),
        } = self.evaluate_expression(value)?
        else {
            return Err(StaticError::NotStatic(value));
        };
        let mut contains = false;
        for element in elements {
            let dir::StaticTerm::Literal {
                value: dir::Literal::String(element),
            } = element
            else {
                return Err(StaticError::NotStatic(receiver));
            };

            contains |= element == value;
        }

        Ok(dir::Literal::Boolean(contains).into())
    }

    /// Match one `array.includes(value)` call into its receiver and value expressions.
    fn array_includes_call(
        &self,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        // match a receiver with the exact `includes` member name
        let dir::Expression::Member {
            left: receiver,
            name: Some(name),
            is_optional: false,
        } = self.view.get(callee)
        else {
            return None;
        };
        if self.strings.get(*name) != "includes" {
            return None;
        }

        // require one positional argument
        let [argument] = arguments else {
            return None;
        };
        let dir::Argument::Positional { value } = self.view.get(*argument) else {
            return None;
        };

        Some((*receiver, *value))
    }
}
