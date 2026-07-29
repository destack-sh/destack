use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::{Formatter, formatted};

impl Formatter<'_, '_, '_> {
    /// Format one callable type with parameter names.
    pub(crate) fn callable_type(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: &[String],
    ) -> QueryResult<Option<String>> {
        self.query.read_type(type_id, |type_value, owner| {
            let formatter = Formatter::new(owner, self.query);
            match type_value {
                dir::Type::FunctionSignature(function) => formatter
                    .function_type(owner.types().signature(*function), Some(parameter_names)),
                dir::Type::Function(function) => {
                    formatter.callable_type(function.signature, parameter_names)
                }
                dir::Type::FunctionPointer(function) => {
                    formatter.callable_type(function.signature, parameter_names)
                }
                _ => formatter.local_type(type_value),
            }
        })
    }

    /// Format one function signature type.
    pub(super) fn function_type(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<Option<String>> {
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
            let name = match parameter_names {
                Some(names) => names[index].as_str(),
                None => "_",
            };
            let parameter =
                formatted!(self.parameter_type(parameter, name.trim_start_matches("...")));
            formatted_parameters.push(parameter);
        }
        let parameters = formatted_parameters.join(", ");

        // format the return and callable modifiers
        let return_type = match function.return_type {
            Some(type_id) => formatted!(self.global_type(type_id)),
            None => "void".to_string(),
        };
        let prefix = match function.asynchrony {
            dir::Asynchrony::Sync => "",
            dir::Asynchrony::Async => "async ",
        };

        Ok(Some(format!("{prefix}({parameters}) => {return_type}")))
    }

    /// Format one method signature with its declaration modifiers.
    pub(crate) fn method_signature(
        &self,
        signature: &dir::FunctionSignature,
        is_static: bool,
    ) -> QueryResult<Option<String>> {
        let Some(mut text) = self.call_signature("", signature, false)? else {
            return Ok(None);
        };
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

        Ok(Some(format!("{static_prefix}{role_prefix}{text}")))
    }

    /// Format one function declaration.
    pub(super) fn function_declaration(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        declaration_prefix: &str,
    ) -> QueryResult<Option<String>> {
        let phase_prefix = function_phase_prefix(signature.phase);
        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };
        let generics = formatted!(self.generics(&signature.generic_parameters));
        let parameters =
            formatted!(self.parameter_labels(signature.this_parameter, &signature.parameters))
                .join(", ");
        let return_type = formatted!(self.return_type(signature.return_type));

        Ok(Some(format!(
            "{declaration_prefix}{phase_prefix}{async_prefix}function \
             {name}{generics}({parameters}){return_type}"
        )))
    }

    /// Format one callable signature.
    pub(crate) fn call_signature(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> QueryResult<Option<String>> {
        let phase_prefix = function_phase_prefix(signature.phase);
        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };
        let generics = formatted!(self.generics(&signature.generic_parameters));
        let this_parameter = if include_this {
            signature.this_parameter
        } else {
            None
        };
        let parameters =
            formatted!(self.parameter_labels(this_parameter, &signature.parameters)).join(", ");
        let return_type = formatted!(self.return_type(signature.return_type));

        Ok(Some(format!(
            "{phase_prefix}{async_prefix}{name}{generics}({parameters}){return_type}"
        )))
    }

    /// Format one optional return type.
    fn return_type(
        &self,
        return_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<Option<String>> {
        let Some(return_node) = return_type else {
            return Ok(Some(String::new()));
        };

        let node_id = return_node.into_global_any(self.module.module_id());
        let type_text = formatted!(self.node_type(node_id));

        Ok(Some(format!(": {type_text}")))
    }
}

/// Format one function phase.
fn function_phase_prefix(phase: dir::FunctionPhase) -> &'static str {
    match phase {
        dir::FunctionPhase::Normal => "",
        dir::FunctionPhase::Comptime => "comptime ",
    }
}
