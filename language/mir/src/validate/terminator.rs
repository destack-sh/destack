use std::collections::{HashMap, HashSet};

use crate::{
    Block, Function, LocalNodeId, Mutability, NodeType, ReferenceKind, SwitchCase, Terminator,
    TrapKind, Type, Value,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate one terminator and its successor contracts.
    pub(super) fn validate_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        // validate terminator inputs
        self.validate_terminator_uses(block_id, terminator, defined_values)?;
        self.validate_return_terminator(function, block_id, terminator)?;

        // validate terminator-specific structure
        match terminator {
            Terminator::Return { .. } | Terminator::Unreachable => {}
            Terminator::Throw { value } => {
                self.validate_throw_terminator(function, block_id, *value)?;
            }
            Terminator::Trap { kind, payload } => {
                self.validate_trap_terminator(function, block_id, *kind, *payload)?;
            }
            Terminator::Jump { target, arguments } => {
                self.validate_block_arguments(
                    block_id,
                    *target,
                    arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                self.validate_block_arguments(
                    block_id,
                    *then_target,
                    then_arguments,
                    block_ids,
                    block_order,
                )?;
                self.validate_block_arguments(
                    block_id,
                    *else_target,
                    else_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Check {
                success, failure, ..
            } => {
                self.validate_block_arguments(
                    block_id,
                    success.target,
                    &success.arguments,
                    block_ids,
                    block_order,
                )?;
                self.validate_block_arguments(
                    block_id,
                    failure.target,
                    &failure.arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                self.validate_block_arguments(
                    block_id,
                    *default,
                    default_arguments,
                    block_ids,
                    block_order,
                )?;
                self.validate_switch_cases(block_id, cases, block_ids, block_order)?;
            }
            Terminator::Yield {
                resume,
                resume_arguments,
                ..
            } => {
                self.validate_resume_arguments(
                    block_id,
                    *resume,
                    resume_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Call {
                function: callee_id,
                arguments,
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
            } => {
                self.ensure_node_type(
                    NodeType::Function,
                    callee_id.id,
                    ValidateAnchor::node(block_id),
                )?;
                let callee = self.tree.get(*callee_id);

                self.validate_regular_call(
                    block_id,
                    arguments.len(),
                    callee.parameters.len(),
                    callee.return_type,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::CallIndirect {
                signature,
                env,
                arguments,
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                let (parameters, result) =
                    self.function_pointer_signature(*signature, anchor, "call.indirect signature")?;
                self.validate_optional_environment(function, *env, block_id, "call.indirect env")?;

                self.validate_regular_call(
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::CallVirtual {
                declaring_type,
                slot_id,
                signature,
                arguments,
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_virtual_dispatch_slot(*declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(*signature, anchor, "call signature")?;

                self.validate_regular_call(
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::CallInterface {
                declaring_type,
                slot_id,
                signature,
                arguments,
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_interface_dispatch_slot(*declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(*signature, anchor, "call signature")?;

                self.validate_regular_call(
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::TailCall {
                function: callee_id,
                arguments,
            } => {
                self.ensure_node_type(
                    NodeType::Function,
                    callee_id.id,
                    ValidateAnchor::node(block_id),
                )?;
                let callee = self.tree.get(*callee_id);

                self.validate_tail_call(
                    function,
                    block_id,
                    arguments.len(),
                    callee.parameters.len(),
                    callee.return_type,
                )?;
            }
            Terminator::TailCallIndirect {
                arguments,
                signature,
                env,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                let (parameters, result) = self.function_pointer_signature(
                    *signature,
                    anchor,
                    "tailcall.indirect signature",
                )?;
                self.validate_optional_environment(
                    function,
                    *env,
                    block_id,
                    "tailcall.indirect env",
                )?;

                self.validate_tail_call(
                    function,
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
            Terminator::TailCallVirtual {
                arguments,
                declaring_type,
                slot_id,
                signature,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_virtual_dispatch_slot(*declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(*signature, anchor, "tailcall signature")?;

                self.validate_tail_call(
                    function,
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
            Terminator::TailCallInterface {
                arguments,
                declaring_type,
                slot_id,
                signature,
                ..
            } => {
                let anchor = ValidateAnchor::node(block_id);
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_interface_dispatch_slot(*declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(*signature, anchor, "tailcall signature")?;

                self.validate_tail_call(
                    function,
                    block_id,
                    arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
        }

        Ok(())
    }

    /// Validate the values used by one terminator.
    fn validate_terminator_uses(
        &self,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        for value in terminator.uses() {
            self.ensure_defined(value, ValidateAnchor::node(block_id), defined_values)?;
        }

        Ok(())
    }

    /// Validate the return contract for one terminator.
    fn validate_return_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
    ) -> ValidateResult<()> {
        let Terminator::Return { value } = terminator else {
            return Ok(());
        };

        let returns_void = matches!(self.tree.get(function.return_type), Type::Void);
        if returns_void && value.is_some() {
            return Err(ValidateError::ReturnValueNotAllowedForVoid {
                anchor: ValidateAnchor::node(block_id),
            });
        }

        if !returns_void && value.is_none() {
            return Err(ValidateError::ReturnValueRequiredForNonVoid {
                anchor: ValidateAnchor::node(block_id),
            });
        }

        Ok(())
    }

    /// Validate a throw terminator payload.
    fn validate_throw_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        value: Value,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(block_id);
        let thrown_type_id = self.value_type_or_error(function, value, anchor, "throw")?;
        let thrown_type = self.tree.get(thrown_type_id);

        let Type::Reference {
            kind: ReferenceKind::Managed,
            ..
        } = thrown_type
        else {
            return Err(self.metadata_error(anchor, "throw requires a managed reference payload"));
        };

        Ok(())
    }

    /// Validate a trap terminator payload.
    fn validate_trap_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        kind: TrapKind,
        payload: Option<Value>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(block_id);

        match kind {
            TrapKind::Abort => {
                if payload.is_some() {
                    return Err(self.metadata_error(anchor, "trap abort does not accept a payload"));
                }
            }
            TrapKind::Panic => {
                let Some(payload) = payload else {
                    return Err(self.metadata_error(anchor, "trap panic requires a payload"));
                };

                let payload_type_id =
                    self.value_type_or_error(function, payload, anchor, "trap panic")?;
                let payload_type = self.tree.get(payload_type_id);

                let Type::Reference {
                    kind: ReferenceKind::Managed,
                    mutability: Mutability::Immutable,
                    is_nullable: false,
                    ..
                } = payload_type
                else {
                    return Err(self.metadata_error(
                        anchor,
                        "trap panic requires a non null readonly managed reference payload",
                    ));
                };
            }
        }

        Ok(())
    }

    /// Validate a direct call terminator.
    fn validate_block_arguments(
        &self,
        source_block: LocalNodeId<Block>,
        target: LocalNodeId<Block>,
        arguments: &[Value],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        // ensure target exists
        if !block_ids.contains(&target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: target,
                anchor: ValidateAnchor::node(source_block),
            });
        }

        // ensure argument count matches parameters
        let block = self.tree.get(target);
        if arguments.len() != block.parameters.len() {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(target, block_order),
                expected: block.parameters.len(),
                got: arguments.len(),
                anchor: ValidateAnchor::node(source_block),
            });
        }

        Ok(())
    }

    /// Validate resume arguments for a yield terminator.
    fn validate_resume_arguments(
        &self,
        source_block: LocalNodeId<Block>,
        resume: LocalNodeId<Block>,
        resume_arguments: &[Value],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        // ensure resume target exists
        if !block_ids.contains(&resume) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: resume,
                anchor: ValidateAnchor::node(source_block),
            });
        }

        // ensure resume arguments align with the resume block parameters
        let block = self.tree.get(resume);
        if resume_arguments.len() + 1 != block.parameters.len() {
            return Err(ValidateError::ResumeArgumentCountMismatch {
                block_label: self.block_label(resume, block_order),
                expected: block.parameters.len().saturating_sub(1),
                got: resume_arguments.len(),
                anchor: ValidateAnchor::node(source_block),
            });
        }

        Ok(())
    }

    /// Validate switch case values and arguments.
    fn validate_switch_cases(
        &self,
        source_block: LocalNodeId<Block>,
        cases: &[SwitchCase],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        // track case values
        let mut seen = HashSet::new();

        for case in cases {
            // reject duplicate case values
            if !seen.insert(case.value) {
                return Err(ValidateError::DuplicateSwitchCaseValue {
                    value: case.value,
                    anchor: ValidateAnchor::node(source_block),
                });
            }

            // validate case arguments
            self.validate_block_arguments(
                source_block,
                case.target,
                &case.arguments,
                block_ids,
                block_order,
            )?;
        }

        Ok(())
    }

    /// Validate one regular call terminator.
    fn validate_regular_call(
        &self,
        block_id: LocalNodeId<Block>,
        argument_count: usize,
        parameter_count: usize,
        result: LocalNodeId<Type>,
        normal_target: LocalNodeId<Block>,
        normal_arguments: &[Value],
        unwind_target: LocalNodeId<Block>,
        unwind_arguments: &[Value],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(block_id);

        self.validate_call_argument_count(anchor, argument_count, parameter_count)?;
        self.validate_call_continuations(
            block_id,
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
            result,
            block_ids,
            block_order,
        )?;

        Ok(())
    }

    /// Validate one tail call terminator.
    fn validate_tail_call(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        argument_count: usize,
        parameter_count: usize,
        result: LocalNodeId<Type>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(block_id);

        self.validate_call_argument_count(anchor, argument_count, parameter_count)?;
        self.validate_tail_call_return_kind(function.return_type, result, anchor)?;

        Ok(())
    }

    /// Validate one function-pointer signature and return its shape.
    pub(super) fn function_pointer_signature(
        &self,
        signature: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<(&[LocalNodeId<Type>], LocalNodeId<Type>)> {
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(self.metadata_error(anchor, format!("{label} is not a function type")));
        };

        self.ensure_node_type(NodeType::Type, result.id, anchor)?;
        for &parameter in parameters {
            self.ensure_node_type(NodeType::Type, parameter.id, anchor)?;
        }

        Ok((parameters.as_slice(), *result))
    }

    /// Validate one optional call environment.
    pub(super) fn validate_optional_environment(
        &self,
        function: &Function,
        env: Option<Value>,
        block_id: LocalNodeId<Block>,
        label: &'static str,
    ) -> ValidateResult<()> {
        let Some(env) = env else {
            return Ok(());
        };

        let anchor = ValidateAnchor::node(block_id);
        self.ensure_reference_value(function, env, anchor, label)?;

        Ok(())
    }

    /// Validate one call argument count.
    pub(super) fn validate_call_argument_count(
        &self,
        anchor: ValidateAnchor,
        actual: usize,
        expected: usize,
    ) -> ValidateResult<()> {
        if actual != expected {
            return Err(ValidateError::CallArgumentCountMismatch {
                expected,
                got: actual,
                anchor,
            });
        }

        Ok(())
    }

    /// Validate one tail-call return kind contract.
    pub(super) fn validate_tail_call_return_kind(
        &self,
        caller_result: LocalNodeId<Type>,
        callee_result: LocalNodeId<Type>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let caller_returns_void = matches!(self.tree.get(caller_result), Type::Void);
        let callee_returns_void = matches!(self.tree.get(callee_result), Type::Void);
        if caller_returns_void != callee_returns_void {
            return Err(ValidateError::TailCallReturnTypeMismatch { anchor });
        }

        Ok(())
    }

    /// Validate normal and unwind continuations for a call terminator.
    pub(super) fn validate_call_continuations(
        &self,
        source_block: LocalNodeId<Block>,
        normal_target: LocalNodeId<Block>,
        normal_arguments: &[Value],
        unwind_target: LocalNodeId<Block>,
        unwind_arguments: &[Value],
        result_type: LocalNodeId<Type>,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(source_block);

        if !block_ids.contains(&normal_target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: normal_target,
                anchor,
            });
        }

        let normal_block = self.tree.get(normal_target);
        let expects_result = !matches!(self.tree.get(result_type), Type::Void);

        if expects_result && normal_block.parameters.is_empty() {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(normal_target, block_order),
                expected: 1,
                got: 0,
                anchor,
            });
        }

        if expects_result && normal_block.parameters[0].ty != result_type {
            return Err(self.metadata_error(
                anchor,
                "call normal continuation result parameter type must match the callee result",
            ));
        }

        let expected_normal_arguments = normal_block
            .parameters
            .len()
            .saturating_sub(usize::from(expects_result));
        if normal_arguments.len() != expected_normal_arguments {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(normal_target, block_order),
                expected: expected_normal_arguments,
                got: normal_arguments.len(),
                anchor,
            });
        }

        if !block_ids.contains(&unwind_target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: unwind_target,
                anchor,
            });
        }

        let unwind_block = self.tree.get(unwind_target);
        if unwind_block.parameters.is_empty() {
            return Err(self.metadata_error(
                anchor,
                "call unwind continuation requires a managed exception parameter",
            ));
        }

        let unwind_parameter_type = self.tree.get(unwind_block.parameters[0].ty);
        let Type::Reference {
            kind: ReferenceKind::Managed,
            mutability: Mutability::Immutable,
            is_nullable: false,
            ..
        } = unwind_parameter_type
        else {
            return Err(self.metadata_error(
                anchor,
                "call unwind continuation requires a managed exception parameter",
            ));
        };

        if unwind_arguments.len() + 1 != unwind_block.parameters.len() {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(unwind_target, block_order),
                expected: unwind_block.parameters.len().saturating_sub(1),
                got: unwind_arguments.len(),
                anchor,
            });
        }

        Ok(())
    }
}
