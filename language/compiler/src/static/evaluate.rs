use destack_dir as dir;

use super::StaticContext;

/// A failed static phase evaluation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum StaticError {
    /// The expression is not in the static subset for this phase.
    NotStatic(dir::LocalNodeId<dir::Expression>),
    /// The expression did not evaluate to a boolean where one is required.
    NotBoolean(dir::LocalNodeId<dir::Expression>),
}

impl StaticContext<'_> {
    /// Evaluate one expression in the static subset.
    pub(crate) fn evaluate_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        match self.view.get(expression) {
            dir::Expression::Parenthesized { expression } => self.evaluate_expression(*expression),
            dir::Expression::ScalarLiteral(value) => Ok((*value).into()),
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
            _ => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Evaluate one expression in the static subset as a boolean.
    pub(crate) fn evaluate_boolean(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, StaticError> {
        match self.evaluate_expression(expression)? {
            dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::Boolean(value),
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
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some(name) = name else {
            return Err(StaticError::NotStatic(expression));
        };
        let name = self.strings.get(name);

        // import.meta.target members read the resolved target triple
        if self.is_import_meta_target(left) {
            self.evaluate_target_member(expression, name)
        }
        // import.meta members read module and profile metadata
        else if self.is_import_meta(left) {
            self.evaluate_import_meta_member(expression, name)
        }
        // any other receiver is not a static namespace
        else {
            Err(StaticError::NotStatic(expression))
        }
    }

    /// Return whether one expression is source `import.meta`.
    fn is_import_meta(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.view.get(expression) {
            dir::Expression::ImportMeta => true,
            dir::Expression::Member {
                left,
                name: Some(name),
            } => self.strings.get(*name) == "meta" && self.is_import_keyword(*left),
            _ => false,
        }
    }

    /// Return whether one expression is source `import.meta.target`.
    fn is_import_meta_target(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.view.get(expression) {
            dir::Expression::Member {
                left,
                name: Some(name),
            } => self.strings.get(*name) == "target" && self.is_import_meta(*left),
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
    ) -> Result<dir::StaticTerm, StaticError> {
        let conditions = self.conditions;

        match name {
            "url" => Ok(self.string(self.module.uri.as_ref())),
            "path" => Ok(self.module_path_value()),
            "dir" => Ok(self.module_dir_value()),
            "output" => Ok(self.string(self.profile.emit.canonical_tag())),
            "targetName" => Ok(self.optional_string(conditions.target.as_deref())),
            "product" => Ok(self.optional_string(conditions.product.as_deref())),
            "platform" => {
                Ok(self
                    .optional_string(conditions.platform.map(|platform| platform.canonical_tag())))
            }
            "host" => Ok(self.optional_string(conditions.host.map(|host| host.canonical_tag()))),
            "runtime" => {
                Ok(self.optional_string(conditions.runtime.map(|runtime| runtime.canonical_tag())))
            }
            "modes" => Ok(self.string_array(conditions.modes.iter())),
            "roles" => Ok(self.string_array(conditions.roles.iter())),
            "features" => Ok(self.string_array(conditions.features.iter())),
            "tags" => Ok(self.string_array(conditions.tags.iter())),
            "debug" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("debug")).into()),
            "dev" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("dev")).into()),
            "prod" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("prod")).into()),
            "test" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("test")).into()),
            "bench" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("bench")).into()),
            "lint" => Ok(dir::ScalarLiteral::Boolean(conditions.contains_mode("lint")).into()),
            _ => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Evaluate one `import.meta.target` member.
    fn evaluate_target_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
    ) -> Result<dir::StaticTerm, StaticError> {
        let profile = self.profile;

        match name {
            "family" => Ok(self.optional_string(
                self.conditions
                    .platform
                    .map(|platform| platform.family_tag()),
            )),
            "arch" => Ok(self.optional_string(
                profile
                    .target_arch
                    .as_ref()
                    .map(|arch| arch.triple_component()),
            )),
            "vendor" => Ok(self.optional_string(
                profile
                    .target_vendor
                    .as_ref()
                    .map(|vendor| vendor.triple_component()),
            )),
            "abi" => Ok(self.optional_string(
                profile
                    .target_abi
                    .as_ref()
                    .map(|abi| abi.triple_component()),
            )),
            _ => Err(StaticError::NotStatic(expression)),
        }
    }

    /// Build the import-time `import.meta.path` value.
    fn module_path_value(&self) -> dir::StaticTerm {
        self.optional_string(
            self.module
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        )
    }

    /// Build the import-time `import.meta.dir` value.
    fn module_dir_value(&self) -> dir::StaticTerm {
        self.optional_string(
            self.module
                .path
                .as_ref()
                .and_then(|path| path.parent())
                .map(|path| path.to_string_lossy().into_owned()),
        )
    }

    /// Evaluate one unary expression.
    fn evaluate_unary(
        &self,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        match operator {
            dir::UnaryOperator::Not => {
                let right = self.evaluate_boolean(right)?;

                Ok(dir::ScalarLiteral::Boolean(!right).into())
            }
            _ => Err(StaticError::NotStatic(right)),
        }
    }

    /// Evaluate one binary expression.
    fn evaluate_binary(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        match operator {
            // &&
            dir::BinaryOperator::And => {
                let value = self.evaluate_boolean(left)? && self.evaluate_boolean(right)?;

                Ok(dir::ScalarLiteral::Boolean(value).into())
            }
            // ||
            dir::BinaryOperator::Or => {
                let value = self.evaluate_boolean(left)? || self.evaluate_boolean(right)?;

                Ok(dir::ScalarLiteral::Boolean(value).into())
            }
            // ==, ===
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;

                Ok(dir::ScalarLiteral::Boolean(left == right).into())
            }
            // !=, !==
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;

                Ok(dir::ScalarLiteral::Boolean(left != right).into())
            }
            _ => Err(StaticError::NotStatic(left)),
        }
    }

    /// Build one string term, interning the text into the shared pool.
    fn string(&self, value: &str) -> dir::StaticTerm {
        dir::ScalarLiteral::String(self.strings.intern(value)).into()
    }

    /// Build one optional string term, undefined when absent.
    fn optional_string(&self, value: Option<impl AsRef<str>>) -> dir::StaticTerm {
        match value {
            Some(value) => self.string(value.as_ref()),
            None => dir::ScalarLiteral::Undefined.into(),
        }
    }

    /// Build one array term of string elements.
    fn string_array<'s>(&self, values: impl Iterator<Item = &'s String>) -> dir::StaticTerm {
        dir::StaticTerm::Array {
            elements: values.map(|value| self.string(value)).collect(),
        }
    }
}
