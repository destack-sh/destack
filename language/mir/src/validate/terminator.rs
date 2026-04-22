use std::collections::{HashMap, HashSet};

use crate::{
    Block, BlockTarget, Function, IntegerReference, LocalNodeId, Mutability, NodeType,
    ReferenceKind, SwitchCase, Terminator, TrapKind, Type, TypeReference, Value, ValueReference,
    function_signature_parts,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate one terminator and its successor contracts.
    pub(super) fn validate_terminator(
        &self,
        function: &Function,
        _block_id: LocalNodeId<Block>,
        terminator_id: LocalNodeId<Terminator>,
        terminator: &Terminator,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(terminator_id);

        // recovered syntax
        if matches!(terminator, Terminator::Error) {
            return Err(ValidateError::RecoveredSyntaxNode {
                kind: "terminator",
                anchor,
            });
        }

        // validate terminator inputs
        self.validate_terminator_uses(anchor, terminator, defined_values)?;
        self.validate_return_terminator(function, anchor, terminator)?;

        // validate terminator-specific structure
        match terminator {
            Terminator::Error => unreachable!("recovered terminator should have returned above"),
            Terminator::Return { .. } | Terminator::Unreachable => {}
            Terminator::Throw { value } => {
                self.validate_throw_terminator(function, anchor, *value)?;
            }
            Terminator::Trap { kind, payload } => {
                self.validate_trap_terminator(function, anchor, *kind, *payload)?;
            }
            Terminator::Jump { target } => {
                self.validate_block_arguments(anchor, target, block_ids, block_order)?;
            }
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                self.validate_block_arguments(anchor, then_target, block_ids, block_order)?;
                self.validate_block_arguments(anchor, else_target, block_ids, block_order)?;
            }
            Terminator::Check {
                success, failure, ..
            } => {
                self.validate_block_arguments(anchor, success, block_ids, block_order)?;
                self.validate_block_arguments(anchor, failure, block_ids, block_order)?;
            }
            Terminator::Switch { default, cases, .. } => {
                self.validate_block_arguments(anchor, default, block_ids, block_order)?;
                self.validate_switch_cases(anchor, cases, block_ids, block_order)?;
            }
            Terminator::Yield { resume, .. } => {
                self.validate_resume_arguments(anchor, resume, block_ids, block_order)?;
            }
            Terminator::Invoke {
                function: callee_id,
                call,
                normal_target,
                unwind_target,
            } => {
                let callee_id =
                    self.require_function_reference(*callee_id, anchor, "invoke callee")?;
                self.ensure_node_type(NodeType::Function, callee_id.id, anchor)?;
                self.validate_direct_call_environment(anchor, callee_id)?;
                self.validate_call_signature_matches_function(anchor, call.signature, callee_id)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "call signature")?;

                self.validate_regular_call(
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                    normal_target,
                    unwind_target,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let (parameters, result) = self.indirect_call_signature(
                    call.signature,
                    anchor,
                    "call.indirect signature",
                )?;
                self.validate_indirect_callee_signature(
                    function,
                    *callee,
                    call.signature,
                    anchor,
                    "call.indirect callee",
                )?;

                self.validate_regular_call(
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                    normal_target,
                    unwind_target,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::InvokeVirtual {
                declaring_type,
                slot_id,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "invoke virtual type")?;
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_virtual_dispatch_slot(declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "call signature")?;

                self.validate_regular_call(
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                    normal_target,
                    unwind_target,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::InvokeInterface {
                declaring_type,
                slot_id,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "invoke interface type")?;
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_interface_dispatch_slot(declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "call signature")?;

                self.validate_regular_call(
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                    normal_target,
                    unwind_target,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::TailCall {
                function: callee_id,
                call,
            } => {
                let callee_id =
                    self.require_function_reference(*callee_id, anchor, "tail call callee")?;
                self.ensure_node_type(NodeType::Function, callee_id.id, anchor)?;
                self.validate_direct_call_environment(anchor, callee_id)?;
                self.validate_call_signature_matches_function(anchor, call.signature, callee_id)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "tailCall signature")?;

                self.validate_tail_call(
                    function,
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
            Terminator::TailCallIndirect { callee, call, .. } => {
                let (parameters, result) = self.indirect_call_signature(
                    call.signature,
                    anchor,
                    "tailCall.indirect signature",
                )?;
                self.validate_indirect_callee_signature(
                    function,
                    *callee,
                    call.signature,
                    anchor,
                    "tailCall.indirect callee",
                )?;

                self.validate_tail_call(
                    function,
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
            Terminator::TailCallVirtual {
                declaring_type,
                slot_id,
                call,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "tail call virtual type")?;
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_virtual_dispatch_slot(declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "tailCall signature")?;

                self.validate_tail_call(
                    function,
                    anchor,
                    call.arguments.len(),
                    parameters.len(),
                    result,
                )?;
            }
            Terminator::TailCallInterface {
                declaring_type,
                slot_id,
                call,
                ..
            } => {
                let declaring_type = self.require_type_reference(
                    *declaring_type,
                    anchor,
                    "tail call interface type",
                )?;
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.validate_interface_dispatch_slot(declaring_type, *slot_id, anchor)?;
                let (parameters, result) =
                    self.function_pointer_signature(call.signature, anchor, "tailCall signature")?;

                self.validate_tail_call(
                    function,
                    anchor,
                    call.arguments.len(),
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
        anchor: ValidateAnchor,
        terminator: &Terminator,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        for value in terminator.uses() {
            let value = self.require_value_reference(value, anchor, "terminator operand")?;
            self.ensure_defined(value, anchor, defined_values)?;
        }

        Ok(())
    }

    /// Validate the return contract for one terminator.
    fn validate_return_terminator(
        &self,
        function: &Function,
        anchor: ValidateAnchor,
        terminator: &Terminator,
    ) -> ValidateResult<()> {
        let Terminator::Return { value } = terminator else {
            return Ok(());
        };

        let return_type =
            self.require_type_reference(function.return_type, anchor, "function return type")?;
        let returns_void = matches!(self.tree.get(return_type), Type::Void);
        if returns_void && value.is_some() {
            return Err(ValidateError::ReturnValueNotAllowedForVoid { anchor });
        }

        if !returns_void && value.is_none() {
            return Err(ValidateError::ReturnValueRequiredForNonVoid { anchor });
        }

        Ok(())
    }

    /// Validate a throw terminator payload.
    fn validate_throw_terminator(
        &self,
        function: &Function,
        anchor: ValidateAnchor,
        value: ValueReference,
    ) -> ValidateResult<()> {
        let value = self.require_value_reference(value, anchor, "throw payload")?;
        let thrown_type_id = self.value_type_or_error(function, value.into(), anchor, "throw")?;
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
        anchor: ValidateAnchor,
        kind: TrapKind,
        payload: Option<ValueReference>,
    ) -> ValidateResult<()> {
        match kind {
            TrapKind::Abort => {
                if payload.is_some() {
                    return Err(self.metadata_error(anchor, "trap.abort does not accept a payload"));
                }
            }
            TrapKind::Panic => {
                let Some(payload) = payload else {
                    return Err(self.metadata_error(anchor, "trap.panic requires a payload"));
                };
                let payload = self.require_value_reference(payload, anchor, "trap payload")?;

                let payload_type_id =
                    self.value_type_or_error(function, payload.into(), anchor, "trap.panic")?;
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
                        "trap.panic requires a non null readonly managed reference payload",
                    ));
                };
            }
        }

        Ok(())
    }

    /// Validate a direct call terminator.
    fn validate_block_arguments(
        &self,
        anchor: ValidateAnchor,
        target: &BlockTarget,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        let argument_count = target.arguments.len();
        let target = self.require_block_reference(target.block, anchor, "block target")?;

        // ensure target exists
        if !block_ids.contains(&target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: target,
                anchor,
            });
        }

        // ensure argument count matches parameters
        let block = self.tree.get(target);
        if argument_count != block.parameters.len() {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(target, block_order),
                expected: block.parameters.len(),
                got: argument_count,
                anchor,
            });
        }

        Ok(())
    }

    /// Validate resume arguments for a yield terminator.
    fn validate_resume_arguments(
        &self,
        anchor: ValidateAnchor,
        resume: &BlockTarget,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        let resume_arguments = resume.arguments.len();
        let resume = self.require_block_reference(resume.block, anchor, "resume target")?;

        // ensure resume target exists
        if !block_ids.contains(&resume) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: resume,
                anchor,
            });
        }

        // ensure resume arguments align with the resume block parameters
        let block = self.tree.get(resume);
        if resume_arguments + 1 != block.parameters.len() {
            return Err(ValidateError::ResumeArgumentCountMismatch {
                block_label: self.block_label(resume, block_order),
                expected: block.parameters.len().saturating_sub(1),
                got: resume_arguments,
                anchor,
            });
        }

        Ok(())
    }

    /// Validate switch case values and arguments.
    fn validate_switch_cases(
        &self,
        anchor: ValidateAnchor,
        cases: &[SwitchCase],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        // track case values
        let mut seen = HashSet::new();

        for case in cases {
            let IntegerReference::Integer(case_value) = case.value else {
                return Err(self.metadata_error(anchor, "switch case value is not concrete"));
            };

            // reject duplicate case values
            if !seen.insert(case_value) {
                return Err(ValidateError::DuplicateSwitchCaseValue {
                    value: case_value,
                    anchor,
                });
            }

            // validate case arguments
            self.validate_block_arguments(anchor, &case.target, block_ids, block_order)?;
        }

        Ok(())
    }

    /// Validate one regular call terminator.
    fn validate_regular_call(
        &self,
        anchor: ValidateAnchor,
        argument_count: usize,
        parameter_count: usize,
        result: LocalNodeId<Type>,
        normal_target: &BlockTarget,
        unwind_target: &BlockTarget,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        self.validate_call_argument_count(anchor, argument_count, parameter_count)?;
        self.validate_call_continuations(
            anchor,
            normal_target,
            unwind_target,
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
        anchor: ValidateAnchor,
        argument_count: usize,
        parameter_count: usize,
        result: LocalNodeId<Type>,
    ) -> ValidateResult<()> {
        self.validate_call_argument_count(anchor, argument_count, parameter_count)?;
        let caller_result =
            self.require_type_reference(function.return_type, anchor, "tail call return type")?;
        self.validate_tail_call_return_kind(caller_result, result, anchor)?;

        Ok(())
    }

    /// Validate one indirect-call signature and return its shape.
    pub(super) fn indirect_call_signature(
        &self,
        signature: TypeReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<(&[TypeReference], LocalNodeId<Type>)> {
        let signature = self.require_type_reference(signature, anchor, label)?;
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let signature = match self.tree.get(signature) {
            Type::FunctionSignature { .. } => signature,
            Type::Closure { signature, .. } => {
                self.require_type_reference(*signature, anchor, label)?
            }
            _ => {
                return Err(self.metadata_error(anchor, format!("{label} is not a function type")));
            }
        };

        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let Some((parameters, result)) = function_signature_parts(self.tree.get(signature)) else {
            return Err(self.metadata_error(anchor, format!("{label} is not a function type")));
        };

        let result = self.require_type_reference(result, anchor, "call result type")?;
        self.ensure_node_type(NodeType::Type, result.id, anchor)?;
        for &parameter in parameters {
            let parameter =
                self.require_type_reference(parameter, anchor, "call parameter type")?;
            self.ensure_node_type(NodeType::Type, parameter.id, anchor)?;
        }

        Ok((parameters, result))
    }

    /// Validate one plain function-pointer signature and return its shape.
    pub(super) fn function_pointer_signature(
        &self,
        signature: TypeReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<(&[TypeReference], LocalNodeId<Type>)> {
        let signature = self.require_type_reference(signature, anchor, label)?;
        if matches!(self.tree.get(signature), Type::Closure { .. }) {
            return Err(self.metadata_error(anchor, format!("{label} is not a function pointer")));
        }

        self.indirect_call_signature(signature.into(), anchor, label)
    }

    /// Validate one indirect callee against the declared call abi.
    pub(super) fn validate_indirect_callee_signature(
        &self,
        function: &Function,
        callee: ValueReference,
        signature: TypeReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<()> {
        let callee = self.require_value_reference(callee, anchor, label)?;
        let signature = self.require_type_reference(signature, anchor, label)?;
        let actual_type = self.value_type_or_error(function, callee.into(), anchor, label)?;
        let actual_signature = match self.tree.get(actual_type) {
            Type::FunctionPointer { signature } => {
                self.require_type_reference(*signature, anchor, "indirect callee signature")?
            }
            Type::Closure { signature } => {
                self.require_type_reference(*signature, anchor, "indirect callee signature")?
            }
            _ => actual_type,
        };

        if !self.types_equivalent(actual_signature, signature) {
            return Err(self.metadata_error(anchor, format!("{label} type mismatch")));
        }

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

    /// Validate success and exception continuations for a call terminator.
    pub(super) fn validate_call_continuations(
        &self,
        anchor: ValidateAnchor,
        normal_target: &BlockTarget,
        unwind_target: &BlockTarget,
        result_type: LocalNodeId<Type>,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> ValidateResult<()> {
        let normal_arguments = normal_target.arguments.len();
        let unwind_arguments = unwind_target.arguments.len();
        let normal_target =
            self.require_block_reference(normal_target.block, anchor, "call success target")?;

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

        if expects_result {
            let normal_result = self.require_type_reference(
                normal_block.parameters[0].ty,
                anchor,
                "call success continuation parameter type",
            )?;

            if normal_result != result_type {
                return Err(self.metadata_error(
                    anchor,
                    "call success continuation result parameter type must match the callee result",
                ));
            }
        }

        let expected_normal_arguments = normal_block
            .parameters
            .len()
            .saturating_sub(usize::from(expects_result));
        if normal_arguments != expected_normal_arguments {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(normal_target, block_order),
                expected: expected_normal_arguments,
                got: normal_arguments,
                anchor,
            });
        }

        let unwind_target =
            self.require_block_reference(unwind_target.block, anchor, "call unwind target")?;
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
                "call exception continuation requires a managed exception parameter",
            ));
        }

        let unwind_parameter_type = self.require_type_reference(
            unwind_block.parameters[0].ty,
            anchor,
            "call unwind continuation parameter type",
        )?;
        let unwind_parameter_type = self.tree.get(unwind_parameter_type);
        let Type::Reference {
            kind: ReferenceKind::Managed,
            mutability: Mutability::Immutable,
            is_nullable: false,
            ..
        } = unwind_parameter_type
        else {
            return Err(self.metadata_error(
                anchor,
                "call exception continuation requires a managed exception parameter",
            ));
        };

        if unwind_arguments + 1 != unwind_block.parameters.len() {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(unwind_target, block_order),
                expected: unwind_block.parameters.len().saturating_sub(1),
                got: unwind_arguments,
                anchor,
            });
        }

        Ok(())
    }
}
