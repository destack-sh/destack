use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
    /// Emit one function parameter.
    fn emit_parameter(
        &mut self,
        source: dir::LocalNodeId<dir::Parameter>,
    ) -> Result<js::LocalNodeId<js::Parameter>, EmitError> {
        let parameter = self.tree.get(source);
        match parameter {
            // emit a named parameter
            dir::Parameter::Named { name, default, .. } => {
                let name = self.emit_binding_identifier(*name, source)?;
                let default = default
                    .map(|default| self.emit_expression(default))
                    .transpose()?;
                let parameter = js::Parameter::Named { name, default };
                let parameter = self.insert_from_source(parameter, source);

                Ok(parameter)
            }
            // emit a pattern parameter
            dir::Parameter::Pattern {
                pattern, default, ..
            } => {
                let pattern = self.emit_pattern(*pattern)?;
                let default = default
                    .map(|default| self.emit_expression(default))
                    .transpose()?;
                let parameter = js::Parameter::Pattern { pattern, default };

                Ok(self.insert_from_source(parameter, source))
            }
            // reject rest parameters
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
                Err(self.internal_error(
                    "rest parameter reached ordinary parameter emission".to_string(),
                ))
            }
            // reject parser error slots
            dir::Parameter::Error => Err(self.unhandled(
                source.into_global_any(self.module),
                Some("parameter error slots cannot reach JavaScript emission".to_string()),
            )),
        }
    }

    /// Emit one function rest parameter.
    fn emit_rest_parameter(
        &mut self,
        source: dir::LocalNodeId<dir::Parameter>,
    ) -> Result<js::LocalNodeId<js::Pattern>, EmitError> {
        let parameter = self.tree.get(source);
        match parameter {
            // emit a named rest parameter
            dir::Parameter::VariadicNamed { name, .. } => {
                let identifier = self.emit_binding_identifier(*name, source)?;
                let pattern = js::Pattern::Binding { identifier };
                let pattern = self.insert_from_source(pattern, source);

                Ok(pattern)
            }
            // emit a pattern rest parameter
            dir::Parameter::VariadicPattern { pattern, .. } => self.emit_pattern(*pattern),
            // reject ordinary parameters
            _ => Err(self
                .internal_error("ordinary parameter reached rest parameter emission".to_string())),
        }
    }

    /// Emit a function signature from DIR into JavaScript.
    pub(crate) fn emit_function_signature(
        &mut self,
        function_signature: &dir::FunctionSignature,
    ) -> Result<js::FunctionSignature, EmitError> {
        let asynchrony = self.asynchrony(function_signature.asynchrony);
        let mut parameters = Vec::with_capacity(function_signature.parameters.len());
        let mut rest = None;

        // emit parameters in source order
        for parameter in function_signature.parameters.iter().copied() {
            match self.tree.get(parameter) {
                // emit the final rest parameter
                dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
                    if rest.is_some() {
                        return Err(self.internal_error(
                            "function has multiple JavaScript rest parameters".to_string(),
                        ));
                    }

                    // store the rest parameter
                    rest = Some(self.emit_rest_parameter(parameter)?);
                }
                // reject parameters after the rest parameter
                _ if rest.is_some() => {
                    return Err(
                        self.internal_error("JavaScript rest parameter is not final".to_string())
                    );
                }
                // emit an ordinary parameter
                _ => parameters.push(self.emit_parameter(parameter)?),
            }
        }

        Ok(js::FunctionSignature {
            asynchrony,
            parameters,
            rest,
            is_generator: function_signature.is_generator,
        })
    }
}
