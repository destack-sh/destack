use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format one callable type with optional parameter names.
    pub(crate) fn callable_type(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        self.query.read_type(type_id, |type_value, owner| {
            let formatter = Formatter::new(owner, self.query);
            match type_value {
                dir::Type::FunctionSignature(function) => {
                    formatter.function_type(owner.types().signature(*function), parameter_names)
                }
                dir::Type::Function(function) => {
                    formatter.callable_type(function.signature, parameter_names)
                }
                dir::Type::FunctionPointer(function) => {
                    formatter.callable_type(function.signature, parameter_names)
                }
                _ => Err(QueryError::invalid(format!("callable type: {type_id:?}"))),
            }
        })
    }

    /// Format one named callable signature.
    pub(crate) fn callable_signature(
        &self,
        name: &str,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<String> {
        self.query.read_type(type_id, |type_value, owner| {
            let formatter = Formatter::new(owner, self.query);
            let function = match type_value {
                dir::Type::FunctionSignature(function) => owner.types().signature(*function),
                dir::Type::Function(function) => {
                    return formatter.callable_signature(name, function.signature);
                }
                dir::Type::FunctionPointer(function) => {
                    return formatter.callable_signature(name, function.signature);
                }
                _ => {
                    return Err(QueryError::invalid(format!(
                        "callable signature type: {type_id:?}"
                    )));
                }
            };
            let parameters = owner.types().parameters(function.parameters);
            let mut formatted_parameters = Vec::with_capacity(parameters.len());

            // format positional constructor parameters in declaration order
            for parameter in parameters {
                formatted_parameters.push(formatter.parameter_type(parameter, None)?);
            }
            let parameters = formatted_parameters.join(", ");
            let return_type = match function.return_type {
                Some(return_type) => formatter.global_type(return_type)?,
                None => "void".to_string(),
            };
            let prefix = match function.asynchrony {
                dir::Asynchrony::Sync => "",
                dir::Asynchrony::Async => "async ",
            };

            Ok(format!("{prefix}{name}({parameters}): {return_type}"))
        })
    }

    /// Format one function signature type.
    pub(super) fn function_type(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        let parameters = self.module.types().parameters(function.parameters);
        if let Some(parameter_names) = parameter_names
            && parameters.len() != parameter_names.len()
        {
            return Err(QueryError::invalid(format!(
                "callable parameter names: expected {}, found {}",
                parameters.len(),
                parameter_names.len()
            )));
        }

        // format parameters in declaration order
        let mut formatted_parameters = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let name = parameter_names.map(|names| names[index].trim_start_matches("..."));
            let parameter = self.parameter_type(parameter, name)?;
            formatted_parameters.push(parameter);
        }
        let parameters = formatted_parameters.join(", ");

        // format the return and callable modifiers
        let return_type = match function.return_type {
            Some(type_id) => self.global_type(type_id)?,
            None => "void".to_string(),
        };
        let prefix = match function.asynchrony {
            dir::Asynchrony::Sync => "",
            dir::Asynchrony::Async => "async ",
        };

        Ok(format!("{prefix}({parameters}) => {return_type}"))
    }

    /// Format one method signature with its declaration modifiers.
    pub(crate) fn method_signature(
        &self,
        signature: &dir::FunctionSignature,
        is_static: bool,
    ) -> QueryResult<String> {
        let mut text = self.call_signature("", signature, false)?;
        if matches!(signature.role, Some(dir::FunctionRole::Setter))
            && signature.return_type.is_none()
        {
            text.push_str(": void");
        }

        let static_prefix = if is_static { "static " } else { "" };
        let role_prefix = match signature.role {
            Some(dir::FunctionRole::Getter) => "get ",
            Some(dir::FunctionRole::Setter) => "set ",
            _ => "",
        };

        Ok(format!("{static_prefix}{role_prefix}{text}"))
    }

    /// Format one function declaration.
    pub(super) fn function_declaration(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        declaration_prefix: &str,
    ) -> QueryResult<String> {
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

        Ok(format!(
            "{declaration_prefix}{phase_prefix}{async_prefix}function \
             {name}{generics}({parameters}){return_type}"
        ))
    }

    /// Format one callable signature.
    pub(crate) fn call_signature(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> QueryResult<String> {
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

        Ok(format!(
            "{phase_prefix}{async_prefix}{name}{generics}({parameters}){return_type}"
        ))
    }

    /// Format one optional return type.
    fn return_type(
        &self,
        return_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<String> {
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
        dir::FunctionPhase::Comptime => "comptime ",
    }
}
