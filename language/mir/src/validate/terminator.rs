use std::collections::{HashMap, HashSet};

use crate::{
    Block, Function, LocalNodeId, Mutability, NodeType, ReferenceKind, SwitchCase, Terminator,
    TrapKind, Type, Value,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate a terminator and its successor edges.
    pub(super) fn validate_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        // check uses
        for value in terminator.uses() {
            self.ensure_defined(value, ValidateAnchor::node(block_id), defined_values)?;
        }

        // check return value presence
        if let Terminator::Return { value } = terminator {
            let returns_void = matches!(self.tree.get(function.return_type), Type::Void);

            // reject return values for void functions
            if returns_void && value.is_some() {
                return Err(ValidateError::ReturnValueNotAllowedForVoid {
                    anchor: ValidateAnchor::node(block_id),
                });
            }

            // reject missing return values for non void functions
            if !returns_void && value.is_none() {
                return Err(ValidateError::ReturnValueRequiredForNonVoid {
                    anchor: ValidateAnchor::node(block_id),
                });
            }
        }

        // check successors and argument counts
        match terminator {
            Terminator::Return { .. } | Terminator::Unreachable => {
                // no successors to validate
            }
            Terminator::Throw { value } => {
                let thrown_type_id = self.value_type_or_error(
                    function,
                    *value,
                    ValidateAnchor::node(block_id),
                    "throw",
                )?;
                let thrown_type = self.tree.get(thrown_type_id);

                let Type::Reference {
                    kind: ReferenceKind::Managed,
                    ..
                } = thrown_type
                else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "throw requires a managed reference payload".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };
            }
            Terminator::Trap { kind, payload } => match kind {
                TrapKind::Abort => {
                    if payload.is_some() {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "trap abort does not accept a payload".to_string(),
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }
                TrapKind::Panic => {
                    let Some(payload) = payload else {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "trap panic requires a payload".to_string(),
                            anchor: ValidateAnchor::node(block_id),
                        });
                    };

                    let payload_type_id = self.value_type_or_error(
                        function,
                        *payload,
                        ValidateAnchor::node(block_id),
                        "trap panic",
                    )?;
                    let payload_type = self.tree.get(payload_type_id);

                    let Type::Reference {
                        kind: ReferenceKind::Managed,
                        mutability: Mutability::Immutable,
                        is_nullable: false,
                        ..
                    } = payload_type
                    else {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message:
                                "trap panic requires a non null readonly managed reference payload"
                                    .to_string(),
                            anchor: ValidateAnchor::node(block_id),
                        });
                    };
                }
            },
            Terminator::Jump { target, arguments } => {
                // validate jump arguments
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
                // validate branch arguments
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
                // validate check successors
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
                // validate switch successors
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
                // validate yield resume arguments
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
                if arguments.len() != callee.parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: callee.parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                self.validate_call_continuations(
                    block_id,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    callee.return_type,
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
                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call.indirect signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                if let Some(env) = env {
                    let env_type_id = self.value_type_or_error(
                        function,
                        *env,
                        ValidateAnchor::node(block_id),
                        "call.indirect env",
                    )?;
                    let env_type = self.tree.get(env_type_id);
                    if !matches!(env_type, Type::Reference { .. }) {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "call.indirect env must be a reference type".to_string(),
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }

                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                self.validate_call_continuations(
                    block_id,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    *result,
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
                self.ensure_node_type(
                    NodeType::Type,
                    declaring_type.id,
                    ValidateAnchor::node(block_id),
                )?;

                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                self.validate_virtual_dispatch_slot(
                    *declaring_type,
                    *slot_id,
                    ValidateAnchor::node(block_id),
                )?;

                self.validate_call_continuations(
                    block_id,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    *result,
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
                self.ensure_node_type(
                    NodeType::Type,
                    declaring_type.id,
                    ValidateAnchor::node(block_id),
                )?;

                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                self.validate_interface_dispatch_slot(
                    *declaring_type,
                    *slot_id,
                    ValidateAnchor::node(block_id),
                )?;

                self.validate_call_continuations(
                    block_id,
                    *normal_target,
                    normal_arguments,
                    *unwind_target,
                    unwind_arguments,
                    *result,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::TailCall {
                function: callee_id,
                arguments,
            } => {
                // validate tail call signatures
                self.ensure_node_type(
                    NodeType::Function,
                    callee_id.id,
                    ValidateAnchor::node(block_id),
                )?;

                // load the callee signature
                let callee = self.tree.get(*callee_id);

                // reject mismatched argument counts
                if arguments.len() != callee.parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: callee.parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(callee.return_type), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(ValidateError::TailCallReturnTypeMismatch {
                        anchor: ValidateAnchor::node(block_id),
                    });
                }
            }
            Terminator::TailCallIndirect {
                arguments,
                signature,
                env,
                ..
            } => {
                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tailcall.indirect signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                if let Some(env) = env {
                    let env_type_id = self.value_type_or_error(
                        function,
                        *env,
                        ValidateAnchor::node(block_id),
                        "tailcall.indirect env",
                    )?;
                    let env_type = self.tree.get(env_type_id);
                    if !matches!(env_type, Type::Reference { .. }) {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "tailcall.indirect env must be a reference type".to_string(),
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }

                // reject mismatched argument counts
                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(*result), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(ValidateError::TailCallReturnTypeMismatch {
                        anchor: ValidateAnchor::node(block_id),
                    });
                }
            }
            Terminator::TailCallVirtual {
                arguments,
                declaring_type,
                slot_id,
                signature,
                ..
            } => {
                self.ensure_node_type(
                    NodeType::Type,
                    declaring_type.id,
                    ValidateAnchor::node(block_id),
                )?;

                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tailcall signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                // reject mismatched argument counts
                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(*result), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(ValidateError::TailCallReturnTypeMismatch {
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                // validate dispatch metadata
                self.validate_virtual_dispatch_slot(
                    *declaring_type,
                    *slot_id,
                    ValidateAnchor::node(block_id),
                )?;
            }
            Terminator::TailCallInterface {
                arguments,
                declaring_type,
                slot_id,
                signature,
                ..
            } => {
                self.ensure_node_type(
                    NodeType::Type,
                    declaring_type.id,
                    ValidateAnchor::node(block_id),
                )?;

                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tailcall signature is not a function type".to_string(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                };

                if arguments.len() != parameters.len() {
                    return Err(ValidateError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(*result), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(ValidateError::TailCallReturnTypeMismatch {
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                self.validate_interface_dispatch_slot(
                    *declaring_type,
                    *slot_id,
                    ValidateAnchor::node(block_id),
                )?;
            }
        }

        Ok(())
    }

    /// Validate normal and unwind continuations for a call terminator.
    fn validate_call_continuations(
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
        if !block_ids.contains(&normal_target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: normal_target,
                anchor: ValidateAnchor::node(source_block),
            });
        }

        let normal_block = self.tree.get(normal_target);
        let expects_result = !matches!(self.tree.get(result_type), Type::Void);
        let expected_normal_arguments = normal_block
            .parameters
            .len()
            .saturating_sub(usize::from(expects_result));
        if normal_arguments.len() != expected_normal_arguments {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(normal_target, block_order),
                expected: expected_normal_arguments,
                got: normal_arguments.len(),
                anchor: ValidateAnchor::node(source_block),
            });
        }

        if expects_result {
            let Some(result_parameter) = normal_block.parameters.first() else {
                return Err(ValidateError::BlockArgumentCountMismatch {
                    block_label: self.block_label(normal_target, block_order),
                    expected: 1,
                    got: 0,
                    anchor: ValidateAnchor::node(source_block),
                });
            };

            if result_parameter.ty != result_type {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "call normal continuation result type mismatch".to_string(),
                    anchor: ValidateAnchor::node(source_block),
                });
            }
        }

        if !block_ids.contains(&unwind_target) {
            return Err(ValidateError::UnknownBlockTarget {
                block_id: unwind_target,
                anchor: ValidateAnchor::node(source_block),
            });
        }

        let unwind_block = self.tree.get(unwind_target);
        let expected_unwind_arguments = unwind_block.parameters.len().saturating_sub(1);
        if unwind_arguments.len() != expected_unwind_arguments {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(unwind_target, block_order),
                expected: expected_unwind_arguments,
                got: unwind_arguments.len(),
                anchor: ValidateAnchor::node(source_block),
            });
        }

        let Some(exception_parameter) = unwind_block.parameters.first() else {
            return Err(ValidateError::BlockArgumentCountMismatch {
                block_label: self.block_label(unwind_target, block_order),
                expected: 1,
                got: 0,
                anchor: ValidateAnchor::node(source_block),
            });
        };
        let exception_type = self.tree.get(exception_parameter.ty);
        let Type::Reference {
            kind: ReferenceKind::Managed,
            ..
        } = exception_type
        else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call unwind continuation requires a managed exception parameter"
                    .to_string(),
                anchor: ValidateAnchor::node(source_block),
            });
        };

        Ok(())
    }

    /// Validate block argument counts for a target.
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
}
