use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::intrinsic::IntrinsicCall;
use crate::{CompilerError, CompilerResult};

/// One execution context operation named by an intrinsic.
pub(in crate::lower) enum ContextIntrinsic {
    /// A load of the current execution context.
    Current,
    /// A replacement of the current execution context, yielding the previous one.
    Replace,
    /// An extension of one execution context with a variable value.
    Bind,
    /// A load of one variable value from an execution context.
    Get,
}

impl ContextIntrinsic {
    /// Return the context operation one intrinsic name denotes.
    pub(in crate::lower) fn from_name(name: &str) -> Option<Self> {
        let denoted = match name {
            "context.current" => Self::Current,
            "context.replace" => Self::Replace,
            "context.bind" => Self::Bind,
            "context.get" => Self::Get,
            _ => return None,
        };

        Some(denoted)
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one execution context operation.
    pub(in crate::lower) fn lower_context_intrinsic(
        &mut self,
        operation: ContextIntrinsic,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let result = self.lower_type(call.resolution.return_type)?;

        // emit the context operation the name denotes
        match operation {
            ContextIntrinsic::Current => Ok(Some(self.builder.context_current(result))),
            ContextIntrinsic::Replace => {
                let context = self.argument_value(call, 0)?;

                Ok(Some(self.builder.context_replace(context, result)))
            }
            ContextIntrinsic::Bind => {
                let context = self.argument_value(call, 0)?;
                let variable = self.argument_value(call, 1)?;
                let value = self.argument_value(call, 2)?;
                let node = self.context_node_type(context, variable, value)?;

                Ok(Some(
                    self.builder
                        .context_bind(context, variable, value, node, result),
                ))
            }
            ContextIntrinsic::Get => {
                let context = self.argument_value(call, 0)?;
                let variable = self.argument_value(call, 1)?;
                let default = self.argument_value(call, 2)?;
                let node = self.context_node_type(context, variable, default)?;

                Ok(Some(
                    self.builder
                        .context_get(context, variable, default, node, result),
                ))
            }
        }
    }

    /// Build the context node type binding one value to one variable.
    fn context_node_type(
        &mut self,
        context: mir::Value,
        variable: mir::Value,
        value: mir::Value,
    ) -> CompilerResult<mir::TypeId> {
        // read the type of each bound value
        let parent = self.value_type(context, "context")?;
        let variable = self.value_type(variable, "context variable")?;
        let value = self.value_type(value, "context value")?;

        // name the three node slots in their physical order
        let slots = [("parent", parent), ("variable", variable), ("value", value)];
        let mut fields = Vec::with_capacity(slots.len());
        for (name, ty) in slots {
            let name = self.lower.strings.intern(name);
            let field = self.builder.tree_mut().intern_field(mir::Field {
                name: Some(name),
                ty,
                attributes: Vec::new(),
            });
            fields.push(field);
        }

        // intern the node as a managed struct
        Ok(self
            .builder
            .tree_mut()
            .intern_type(mir::Type::Struct { fields }))
    }

    /// Return the defined type of one lowered value.
    fn value_type(&self, value: mir::Value, role: &str) -> CompilerResult<mir::TypeId> {
        self.builder
            .value_type(value)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("an undefined {role} value"),
            })
    }
}
