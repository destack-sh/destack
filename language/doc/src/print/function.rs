use destack_dir as dir;

use crate::{DocError, DocResult};

use super::{FormattedSignature, Printer};

impl Printer<'_, '_, '_> {
    /// Format one function signature type.
    pub(super) fn function_type(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> DocResult<String> {
        let parameters = self.function_parameters(function, parameter_names)?;
        let return_type = self.function_return_type(function)?;
        let prefix = match (function.asynchrony, function.is_construct) {
            (dir::Asynchrony::Async, _) => "async ",
            (dir::Asynchrony::Sync, true) => "new ",
            (dir::Asynchrony::Sync, false) => "",
        };

        Ok(format!("{prefix}({parameters}) => {return_type}"))
    }

    /// Format parameters carried by one function type.
    fn function_parameters(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> DocResult<String> {
        let parameters = self.types().parameters(function.parameters);
        if let Some(parameter_names) = parameter_names
            && parameters.len() != parameter_names.len()
        {
            return Err(DocError::invalid(format!(
                "callable parameter names: expected {}, found {}",
                parameters.len(),
                parameter_names.len()
            )));
        }

        // format parameters in declaration order
        let mut formatted = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let name = parameter_names.map(|names| names[index].trim_start_matches("..."));
            formatted.push(self.parameter_type(parameter, name)?);
        }

        Ok(formatted.join(", "))
    }

    /// Format the return type carried by one function type.
    fn function_return_type(&self, function: &dir::FunctionSignatureType) -> DocResult<String> {
        match function.return_type {
            Some(type_id) => self.global_type(type_id),
            None => Ok("void".to_string()),
        }
    }

    /// Format one function declaration.
    pub(super) fn function_declaration(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        declaration_prefix: &str,
    ) -> DocResult<FormattedSignature> {
        let phase_prefix = function_phase_prefix(signature.phase);
        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };
        let generics = self.generics(&signature.generic_parameters)?;
        let parameters = self
            .parameter_labels(signature.this_parameter, &signature.parameters)?
            .join(", ");
        let return_type = self.return_type(signature.return_type)?;

        let prefix = format!("{declaration_prefix}{phase_prefix}{async_prefix}function ");
        let suffix = format!("{generics}({parameters}){return_type}");

        Ok(FormattedSignature::named(prefix, name, suffix))
    }

    /// Format one callable signature and retain its declared name interval.
    pub(crate) fn named_call_signature(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> DocResult<FormattedSignature> {
        let phase_prefix = function_phase_prefix(signature.phase);
        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };
        let generics = self.generics(&signature.generic_parameters)?;
        let this_parameter = if include_this {
            signature.this_parameter
        } else {
            None
        };
        let parameters = self
            .parameter_labels(this_parameter, &signature.parameters)?
            .join(", ");
        let return_type = self.return_type(signature.return_type)?;
        let prefix = format!("{phase_prefix}{async_prefix}");
        let suffix = format!("{generics}({parameters}){return_type}");

        Ok(FormattedSignature::named(prefix, name, suffix))
    }

    /// Format an anonymous call or construct signature from its authored type member.
    pub(super) fn type_member_signature(
        &self,
        member: &dir::TypeMember,
    ) -> DocResult<FormattedSignature> {
        let (prefix, generics, receiver, parameters, return_type) = match member {
            dir::TypeMember::CallSignature { signature } => (
                "",
                &signature.generic_parameters,
                signature.this_parameter,
                &signature.parameters,
                signature.return_type,
            ),
            dir::TypeMember::ConstructSignature { signature } => (
                if signature.is_abstract {
                    "abstract new "
                } else {
                    "new "
                },
                &signature.generic_parameters,
                None,
                &signature.parameters,
                signature.return_type,
            ),
            _ => return Err(DocError::invalid("callable type member")),
        };

        // retain authored generic parameters, receiver, and argument names
        let generics = self.generics(generics)?;
        let parameters = self.parameter_labels(receiver, parameters)?.join(", ");
        let return_type = self.return_type(return_type)?;

        Ok(FormattedSignature::plain(format!(
            "{prefix}{generics}({parameters}){return_type}"
        )))
    }

    /// Format one optional return type.
    fn return_type(
        &self,
        return_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> DocResult<String> {
        let Some(return_node) = return_type else {
            return Ok(String::new());
        };

        let node_id = return_node.into_global_any(self.module.module_id());
        let type_text = self.node_type(node_id)?;

        Ok(format!(": {type_text}"))
    }
}

/// Format one function phase.
fn function_phase_prefix(phase: dir::FunctionPhase) -> &'static str {
    match phase {
        dir::FunctionPhase::Normal => "",
        dir::FunctionPhase::Const => "const ",
    }
}
