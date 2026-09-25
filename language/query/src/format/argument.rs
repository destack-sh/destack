use tspp_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

/// One signature formatted from an exact call selection.
pub(crate) struct AppliedSignature {
    /// The complete signature label.
    pub(crate) label: String,
    /// The parameter labels in binding order.
    pub(crate) parameters: Vec<String>,
}

impl Formatter<'_, '_, '_> {
    /// Format one signature from its selected call types.
    pub(crate) fn applied_signature(
        &self,
        name: &str,
        generic_arguments: &[dir::GenericArgumentBinding],
        parameter_names: &[Option<String>],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<AppliedSignature> {
        let mut arguments = Vec::with_capacity(generic_arguments.len());

        // format source-declared generic arguments
        for binding in generic_arguments {
            let module = self.program.module(binding.parameter.module_id)?;
            let parameter = module.generics()?.get_parameter(binding.parameter.local_id);
            if parameter.origin == dir::GenericParameterOrigin::Explicit {
                let argument = self.global_type(binding.argument)?;
                arguments.push(argument);
            }
        }

        let generics = if arguments.is_empty() {
            String::new()
        } else {
            format!("<{}>", arguments.join(", "))
        };

        // format parameters in binding order
        if parameter_names.len() != bindings.len() {
            return Err(QueryError::invalid(format!(
                "signature parameter names: expected {}, found {}",
                bindings.len(),
                parameter_names.len()
            )));
        }

        let mut parameters = Vec::with_capacity(bindings.len());
        for (index, binding) in bindings.iter().enumerate() {
            let type_text = self.global_type(binding.parameter_type)?;
            let label = match parameter_names[index].as_deref() {
                Some(name) => format!("{name}: {type_text}"),
                None => type_text,
            };
            parameters.push(label);
        }

        // join the complete signature label
        let return_type = self.global_type(return_type)?;
        let parameter_text = parameters.join(", ");
        let label = format!("{name}{generics}({parameter_text}): {return_type}");

        Ok(AppliedSignature { label, parameters })
    }

    /// Format generic parameters.
    pub(super) fn generics(
        &self,
        generics: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> QueryResult<String> {
        if generics.is_empty() {
            return Ok(String::new());
        }

        let mut formatted = Vec::with_capacity(generics.len());
        for parameter_id in generics {
            formatted.push(self.generic_parameter(*parameter_id)?);
        }

        Ok(format!("<{}>", formatted.join(", ")))
    }

    /// Format one generic parameter.
    pub(super) fn generic_parameter(
        &self,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> QueryResult<String> {
        let parameter = self.module.view()?.get(parameter_id);
        let strings = self.module.strings();

        let text = match parameter {
            dir::GenericParameter::Type {
                name,
                is_const,
                variance,
                constraint,
                default,
            } => {
                let prefix = type_parameter_prefix(*is_const, *variance, false);
                let constraint = self.type_bound(*constraint)?;
                let default = self.type_default(*default)?;

                format!("{prefix}{}{constraint}{default}", strings.get(*name))
            }
            dir::GenericParameter::VariadicType {
                name,
                is_const,
                variance,
                constraint,
                default,
            } => {
                let prefix = type_parameter_prefix(*is_const, *variance, true);
                let constraint = self.type_bound(*constraint)?;
                let default = self.type_default(*default)?;

                format!("{prefix}{}{constraint}{default}", strings.get(*name))
            }
            dir::GenericParameter::Lifetime { name } => strings.get(*name).to_string(),
            dir::GenericParameter::Error => {
                return Err(QueryError::missing("generic parameter formatting"));
            }
        };

        Ok(text)
    }

    /// Format parameter labels in declaration order.
    pub(super) fn parameter_labels(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> QueryResult<Vec<String>> {
        let capacity = parameters.len() + usize::from(this_parameter.is_some());
        let mut formatted = Vec::with_capacity(capacity);

        if let Some(this_parameter) = this_parameter {
            let this_text = self.parameter(this_parameter)?;
            formatted.push(this_text);
        }

        for parameter_id in parameters {
            formatted.push(self.parameter(*parameter_id)?);
        }

        Ok(formatted)
    }

    /// Format one authored parameter.
    pub(super) fn parameter(
        &self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> QueryResult<String> {
        let parameter = self.module.view()?.get(parameter_id);
        let name = self.module.parameter_name(parameter)?;
        let optional = if parameter.is_optional() || parameter.default_value().is_some() {
            "?"
        } else {
            ""
        };
        let node_id = parameter_id.into_global_any(self.module.module_id());
        let type_text = self.node_type(node_id)?;

        Ok(format!("{name}{optional}: {type_text}"))
    }

    /// Format one function type parameter.
    pub(super) fn parameter_type(
        &self,
        parameter: &dir::FunctionParameterType,
        parameter_name: Option<&str>,
    ) -> QueryResult<String> {
        let rest = if parameter.is_rest { "..." } else { "" };
        let optional = if parameter.is_optional { "?" } else { "" };
        let type_text = self.global_type(parameter.ty)?;
        let text = match parameter_name {
            Some(parameter_name) => format!("{rest}{parameter_name}{optional}: {type_text}"),
            None => format!("{rest}{type_text}{optional}"),
        };

        Ok(text)
    }

    /// Format one optional generic type bound.
    fn type_bound(
        &self,
        type_id: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<String> {
        let Some(type_id) = type_id else {
            return Ok(String::new());
        };
        let type_text = self.type_expression(type_id)?;

        Ok(format!(": {type_text}"))
    }

    /// Format one optional generic type default.
    fn type_default(
        &self,
        type_id: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<String> {
        let Some(type_id) = type_id else {
            return Ok(String::new());
        };
        let type_text = self.type_expression(type_id)?;

        Ok(format!(" = {type_text}"))
    }
}

/// Format one type parameter prefix.
fn type_parameter_prefix(
    is_const: bool,
    variance: Option<dir::VarianceModifier>,
    is_variadic: bool,
) -> String {
    let mut prefix = String::new();

    if is_const {
        prefix.push_str("const ");
    }

    match variance {
        Some(dir::VarianceModifier::In) => prefix.push_str("in "),
        Some(dir::VarianceModifier::Out) => prefix.push_str("out "),
        Some(dir::VarianceModifier::InOut) => prefix.push_str("in out "),
        None => {}
    }

    if is_variadic {
        prefix.push_str("...");
    }

    prefix
}
