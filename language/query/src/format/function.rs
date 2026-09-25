use tspp_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format the parameter labels stored by one callable type.
    pub(crate) fn callable_parameter_labels(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<Vec<Option<String>>> {
        self.module.read_signature(
            type_id,
            |function, module| {
                let formatter = Formatter::new(module, self.program).with_reopening(self.reopening);
                let parameters = formatter.types()?.parameters(function.parameters);
                let strings = formatter.module.strings();
                let labels = parameters
                    .iter()
                    .map(|parameter| {
                        parameter.name.map(|name| {
                            let rest = if parameter.is_rest { "..." } else { "" };
                            let optional = if parameter.is_optional { "?" } else { "" };

                            format!("{rest}{}{optional}", strings.get(name))
                        })
                    })
                    .collect();

                Ok(labels)
            },
            self.program,
        )
    }

    /// Format one callable type with optional parameter names.
    pub(crate) fn callable_type(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        self.module.read_signature(
            type_id,
            |function, module| {
                let formatter = Formatter::new(module, self.program).with_reopening(self.reopening);

                formatter.function_type(function, parameter_names)
            },
            self.program,
        )
    }

    /// Format one callable as a completion label suffix.
    pub(crate) fn callable_suffix(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        self.module.read_signature(
            type_id,
            |function, module| {
                let formatter = Formatter::new(module, self.program).with_reopening(self.reopening);

                formatter.function_suffix(function, parameter_names)
            },
            self.program,
        )
    }

    /// Format one named callable signature.
    pub(crate) fn callable_signature(
        &self,
        name: &str,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<String> {
        self.module.read_signature(
            type_id,
            |function, module| {
                let formatter = Formatter::new(module, self.program).with_reopening(self.reopening);
                let parameters = formatter.function_parameters(function, None)?;
                let return_type = formatter.function_return_type(function)?;
                let prefix = match function.asynchrony {
                    dir::Asynchrony::Sync => "",
                    dir::Asynchrony::Async => "async ",
                };

                Ok(format!("{prefix}{name}({parameters}): {return_type}"))
            },
            self.program,
        )
    }

    /// Format one function signature type.
    pub(super) fn function_type(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        let parameters = self.function_parameters(function, parameter_names)?;
        let return_type = self.function_return_type(function)?;
        let prefix = match (function.asynchrony, function.is_construct) {
            (dir::Asynchrony::Async, _) => "async ",
            (dir::Asynchrony::Sync, true) => "new ",
            (dir::Asynchrony::Sync, false) => "",
        };

        Ok(format!("{prefix}({parameters}) => {return_type}"))
    }

    /// Format one function type as a completion label suffix.
    fn function_suffix(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        let parameters = self.function_parameters(function, parameter_names)?;
        let return_type = self.function_return_type(function)?;

        Ok(format!("({parameters}): {return_type}"))
    }

    /// Format parameters carried by one function type.
    fn function_parameters(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: Option<&[String]>,
    ) -> QueryResult<String> {
        let parameters = self.types()?.parameters(function.parameters);
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
        let mut formatted = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let name = parameter_names.map(|names| names[index].trim_start_matches("..."));
            formatted.push(self.parameter_type(parameter, name)?);
        }

        Ok(formatted.join(", "))
    }

    /// Format the return type carried by one function type.
    fn function_return_type(&self, function: &dir::FunctionSignatureType) -> QueryResult<String> {
        match function.return_type {
            Some(type_id) => self.global_type(type_id),
            None => Ok("void".to_string()),
        }
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
        dir::FunctionPhase::Const => "const ",
    }
}
