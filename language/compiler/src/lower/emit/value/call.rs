use destack_dir::{Expression, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a call expression to its result value and type.
    pub(crate) fn lower_call_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: &LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
        static_arguments: &Option<Vec<LocalNodeId<dir::Argument>>>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        if static_arguments.is_some() {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "static arguments are not supported".to_string(),
            })?;
        }

        // check for resolution on the call expression
        // extract target symbol if static resolution (to avoid borrow issues)
        let resolution = self.get_resolution(expression_id);
        let static_target = match resolution {
            Some(Resolution::Static { candidate, .. }) => Some(candidate.target_symbol),
            _ => None,
        };
        let resolution_receiver = resolution.and_then(|resolution| resolution.receiver());

        // resolve target function and receiver based on resolution or syntax
        let (function_id, receiver_value) = match static_target {
            // static resolution: use target_symbol from candidate
            Some(target_symbol) => {
                // check if this is a method call (left is Member)
                let left_expr = self.dir_tree.get(*left);
                let receiver_value = if resolution_receiver.is_some()
                    && let Expression::Member {
                        left: receiver_id,
                        static_arguments: member_static_args,
                        ..
                    } = left_expr
                {
                    if member_static_args.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "static arguments on method calls are not supported"
                                .to_string(),
                        })?;
                    }
                    // skip evaluation for namespace receivers, they are compile-time only
                    if self.receiver_is_namespace_reference(*receiver_id) {
                        None
                    } else {
                        let (receiver_value, _) = self.lower_value_expression(*receiver_id)?;
                        Some(receiver_value)
                    }
                } else {
                    None
                };

                // look up function by target symbol
                let function_id =
                    *self
                        .functions_by_symbol
                        .get(&target_symbol)
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "missing function for resolved target symbol".to_string(),
                        })?;

                (function_id, receiver_value)
            }

            // no resolution: fall back to syntax-based dispatch for direct calls
            _ => {
                let left_expr = self.dir_tree.get(*left);
                match left_expr {
                    // method call requires Resolution from Analyze
                    Expression::Member { name, .. } => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: format!(
                                "method call '{}' missing Resolution (Analyze issue)",
                                self.strings.get(*name).as_str()
                            ),
                        })?;
                    }

                    // direct function call
                    Expression::LocalReference { target_symbol, .. }
                    | Expression::ModuleReference { target_symbol, .. }
                    | Expression::GlobalReference { target_symbol, .. } => {
                        let function_id =
                            *self.functions_by_symbol.get(target_symbol).ok_or_else(|| {
                                LowerError::UnsupportedConstruct {
                                    node: expression_id
                                        .into_global_any(self.module_id)
                                        .into_anchored(Some(self.profile)),
                                    message: "missing function symbol".to_string(),
                                }
                            })?;
                        (function_id, None)
                    }

                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "unsupported call expression target".to_string(),
                        })?;
                    }
                }
            }
        };

        // build arguments: receiver (if method call) + declared arguments
        let mut arguments =
            Vec::with_capacity(dynamic_arguments.len() + receiver_value.is_some() as usize);
        if let Some(receiver) = receiver_value {
            arguments.push(receiver);
        }
        for argument_id in dynamic_arguments {
            let argument = self.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "unsupported non-positional argument".to_string(),
                })?;
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            arguments.push(value);
        }

        // get result type
        let result_type = self.mir_type_for_expression(expression_id)?;

        // emit call with metadata when we have a static resolution
        let value = if static_target.is_some() {
            let signature = result_type;
            let metadata = mir::CallMetadata::direct(function_id, signature);
            self.builder
                .call_with_metadata(function_id, arguments, metadata)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "call returned no value".to_string(),
                })?
        }
        // fall back to plain call
        else {
            self.builder.call(function_id, arguments).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "call returned no value".to_string(),
                }
            })?
        };

        Ok((value, result_type))
    }
}
