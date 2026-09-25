use tspp_dir as dir;

use crate::{DocError, DocResult};

use super::Printer;

impl Printer<'_, '_, '_> {
    /// Format generic parameters.
    pub(super) fn generics(
        &self,
        generics: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> DocResult<String> {
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
    ) -> DocResult<String> {
        let parameter = self.module.view().get(parameter_id);
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
                return Err(DocError::missing("generic parameter formatting"));
            }
        };

        Ok(text)
    }

    /// Format parameter labels in declaration order.
    pub(super) fn parameter_labels(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> DocResult<Vec<String>> {
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
    ) -> DocResult<String> {
        let parameter = self.module.view().get(parameter_id);
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
    ) -> DocResult<String> {
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
    ) -> DocResult<String> {
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
    ) -> DocResult<String> {
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
