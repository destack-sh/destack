use crate::analyze::StaticSubstitutionEnvironment;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    GlobalSymbolId, LocalInstanceId, LocalTypeId, StaticArgument, StaticExpression, StaticProperty,
    TypeTable, are_types_equal,
};

/// One normalized and committable instance environment key.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedInstanceEnvironment {
    /// Canonical static arguments for keying and commitment.
    pub(crate) arguments: Vec<StaticArgument>,
    /// Canonical static parameter symbols for keying and commitment.
    pub(crate) parameter_symbols: Vec<GlobalSymbolId>,
    /// Number of inherited static arguments at the front of `arguments`.
    pub(crate) inherited_arity: usize,
}

impl Compiler {
    /// Build one internal analyze error.
    pub(crate) fn internal_analyze_error(&self, message: impl Into<String>) -> AnalyzeError {
        AnalyzeError::Internal {
            message: message.into(),
        }
    }

    /// Normalize one environment for committable instance keying and commitment.
    pub(crate) fn normalize_instance_environment_for_commit(
        &self,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
    ) -> AnalyzeResult<NormalizedInstanceEnvironment> {
        let environment = self
            .canonicalize_instance_environment_key(environment)
            .ok_or_else(|| {
                self.internal_analyze_error(format!(
                    "normalize_instance_environment_for_commit: failed to canonicalize committable instance environment for symbol {symbol_id:?}",
                ))
            })?;
        let arguments = environment.arguments().to_vec();
        let parameter_symbols = environment.complete_parameter_symbols().ok_or_else(|| {
            self.internal_analyze_error(format!(
                "normalize_instance_environment_for_commit: missing complete static parameter symbols for symbol {symbol_id:?} with {} static arguments",
                arguments.len()
            ))
        })?;

        Ok(NormalizedInstanceEnvironment {
            arguments,
            parameter_symbols,
            inherited_arity: environment.inherited_arity(),
        })
    }

    /// Look up an existing instance id for one full canonical environment.
    fn instance_argument_types_equal_for_key(
        &self,
        left: LocalTypeId,
        right: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let left = types.unwrap_value_type_id(left);
        let right = types.unwrap_value_type_id(right);
        are_types_equal(left, right, types)
    }

    /// Return whether two optional static-argument lists are equivalent for instance-key matching.
    fn static_argument_list_eq_for_instance_key(
        &self,
        left: &Option<Vec<StaticArgument>>,
        right: &Option<Vec<StaticArgument>>,
        types: &TypeTable,
    ) -> bool {
        match (left, right) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                self.static_arguments_equal_for_instance_key(left, right, types)
            }
            _ => false,
        }
    }

    /// Return whether two static properties are equivalent for instance-key matching.
    fn static_property_equal_for_instance_key(
        &self,
        left: &StaticProperty,
        right: &StaticProperty,
        types: &TypeTable,
    ) -> bool {
        match (left, right) {
            (
                StaticProperty::Unevaluated { node: left },
                StaticProperty::Unevaluated { node: right },
            ) => left == right,
            (
                StaticProperty::Field {
                    key: left_key,
                    value: left_value,
                    symbol: left_symbol,
                },
                StaticProperty::Field {
                    key: right_key,
                    value: right_value,
                    symbol: right_symbol,
                },
            ) => {
                left_key == right_key
                    && left_symbol == right_symbol
                    && self.static_expression_eq_for_instance_key(left_value, right_value, types)
            }
            (
                StaticProperty::Method {
                    key: left_key,
                    signature: left_signature,
                    body: left_body,
                    symbol: left_symbol,
                },
                StaticProperty::Method {
                    key: right_key,
                    signature: right_signature,
                    body: right_body,
                    symbol: right_symbol,
                },
            ) => {
                left_key == right_key
                    && left_signature == right_signature
                    && left_symbol == right_symbol
                    && self.static_expression_eq_for_instance_key(left_body, right_body, types)
            }
            (
                StaticProperty::Spread {
                    value: left_value,
                    symbol: left_symbol,
                },
                StaticProperty::Spread {
                    value: right_value,
                    symbol: right_symbol,
                },
            ) => {
                left_symbol == right_symbol
                    && self.static_expression_eq_for_instance_key(left_value, right_value, types)
            }
            _ => false,
        }
    }

    /// Return whether two static-property vectors are equivalent for instance-key matching.
    fn static_properties_eq_for_instance_key(
        &self,
        left: &[StaticProperty],
        right: &[StaticProperty],
        types: &TypeTable,
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }

        left.iter()
            .zip(right.iter())
            .all(|(left, right)| self.static_property_equal_for_instance_key(left, right, types))
    }

    /// Return whether two static expressions are equivalent for instance-key matching.
    fn static_expression_eq_for_instance_key(
        &self,
        left: &StaticExpression,
        right: &StaticExpression,
        types: &TypeTable,
    ) -> bool {
        match (left, right) {
            (
                StaticExpression::Unevaluated { node: left },
                StaticExpression::Unevaluated { node: right },
            ) => left == right,
            (
                StaticExpression::ScalarLiteral { value: left },
                StaticExpression::ScalarLiteral { value: right },
            ) => left == right,
            (
                StaticExpression::TypeLiteral { value: left },
                StaticExpression::TypeLiteral { value: right },
            ) => left == right,
            (StaticExpression::Type { ty: left }, StaticExpression::Type { ty: right }) => {
                self.instance_argument_types_equal_for_key(*left, *right, types)
            }
            (
                StaticExpression::Declaration {
                    declaration: left_declaration,
                    static_arguments: left_arguments,
                },
                StaticExpression::Declaration {
                    declaration: right_declaration,
                    static_arguments: right_arguments,
                },
            ) => {
                left_declaration == right_declaration
                    && self.static_argument_list_eq_for_instance_key(
                        left_arguments,
                        right_arguments,
                        types,
                    )
            }
            (
                StaticExpression::ArrayExpression { elements: left },
                StaticExpression::ArrayExpression { elements: right },
            )
            | (
                StaticExpression::TupleExpression { elements: left },
                StaticExpression::TupleExpression { elements: right },
            ) => {
                if left.len() != right.len() {
                    return false;
                }

                left.iter().zip(right.iter()).all(|(left, right)| {
                    self.static_expression_eq_for_instance_key(left, right, types)
                })
            }
            (
                StaticExpression::ObjectExpression { properties: left },
                StaticExpression::ObjectExpression { properties: right },
            ) => self.static_properties_eq_for_instance_key(left, right, types),
            _ => false,
        }
    }

    /// Return whether two static arguments are equivalent for instance-key matching.
    fn static_argument_equal_for_instance_key(
        &self,
        left: &StaticArgument,
        right: &StaticArgument,
        types: &TypeTable,
    ) -> bool {
        match (left, right) {
            (
                StaticArgument::Unevaluated { node: left },
                StaticArgument::Unevaluated { node: right },
            ) => left == right,
            (
                StaticArgument::Evaluated { value: left, .. },
                StaticArgument::Evaluated { value: right, .. },
            ) => self.static_expression_eq_for_instance_key(left, right, types),
            _ => false,
        }
    }

    /// Return whether two static-argument vectors are equivalent for instance-key matching.
    fn static_arguments_equal_for_instance_key(
        &self,
        left: &[StaticArgument],
        right: &[StaticArgument],
        types: &TypeTable,
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }

        left.iter()
            .zip(right.iter())
            .all(|(left, right)| self.static_argument_equal_for_instance_key(left, right, types))
    }

    /// Look up an existing instance id for one full canonical environment.
    pub(crate) fn query_instance_for_symbol_environment(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: &[StaticArgument],
        parameter_symbols: &[GlobalSymbolId],
        inherited_arity: usize,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let candidate_ids =
            types.query_instance_interner_candidates(symbol_id, static_arguments.len());

        for instance_id in candidate_ids {
            let instance = types.get_instance(instance_id);
            if !self.static_arguments_equal_for_instance_key(
                &instance.static_arguments,
                static_arguments,
                types,
            ) {
                continue;
            }

            if instance.static_parameter_symbols == parameter_symbols
                && instance.inherited_static_argument_count == inherited_arity
            {
                return Ok(Some(instance_id));
            }

            return Err(self.internal_analyze_error(format!(
                "query_instance_for_symbol_environment: conflicting canonical instance environment for symbol {symbol_id:?}: arguments={static_arguments:?}, parameters={parameter_symbols:?}, inherited_arity={inherited_arity}"
            )));
        }

        Ok(None)
    }

    /// Normalize one static argument for canonical instance-key usage.
    pub(crate) fn canonicalize_instance_argument_for_key(
        &self,
        argument: &StaticArgument,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { node } => StaticArgument::Unevaluated { node: *node },
            StaticArgument::Evaluated { value, .. } => StaticArgument::Evaluated {
                name: None,
                value: value.clone(),
            },
        }
    }

    /// Normalize static arguments for canonical instance-key usage.
    pub(crate) fn canonicalize_instance_arguments_for_key(
        &self,
        static_arguments: Vec<StaticArgument>,
    ) -> Vec<StaticArgument> {
        static_arguments
            .iter()
            .map(|argument| self.canonicalize_instance_argument_for_key(argument))
            .collect()
    }

    /// Canonicalize one environment for instance commit keying.
    fn canonicalize_instance_environment_key(
        &self,
        environment: StaticSubstitutionEnvironment,
    ) -> Option<StaticSubstitutionEnvironment> {
        let (arguments, parameter_symbols, inherited_arity) = environment.into_parts();
        let arguments = self.canonicalize_instance_arguments_for_key(arguments);
        StaticSubstitutionEnvironment::from_optional_parameter_symbols(
            arguments,
            parameter_symbols,
            inherited_arity,
        )
    }
}
