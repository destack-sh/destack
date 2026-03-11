use std::collections::HashSet;

use crate::{
    AllocationMode, ArgumentSlice, CastOperator, Constant, Function, Instruction, Intrinsic, Local,
    LocalNodeId, Mutability, NodeType, ReferenceKind, TensorDimension, Type, Value,
    compute_type_layout,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate an instruction uses only defined values.
    pub(super) fn validate_instruction(
        &self,
        function: &Function,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        locals: &HashSet<LocalNodeId<Local>>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        // check inline uses
        for value in instruction.uses() {
            self.ensure_defined(value, ValidateAnchor::node(instruction_id), defined_values)?;
        }

        // check external argument uses
        if let Some(slice) = instruction.argument_slice() {
            // validate the slice bounds
            self.validate_argument_slice(slice, instruction_id)?;

            // ensure slice arguments are defined
            let arguments = self.tree.get_arguments(slice);
            for &value in arguments {
                self.ensure_defined(value, ValidateAnchor::node(instruction_id), defined_values)?;
            }

            // validate call signatures and effects
            match instruction {
                Instruction::Call {
                    destination,
                    function,
                    signature,
                    effects,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    self.validate_call_signature_matches_function(
                        ValidateAnchor::node(instruction_id),
                        *signature,
                        *function,
                    )?;
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                Instruction::CallVirtual {
                    destination,
                    declaring_type,
                    slot_id,
                    signature,
                    effects,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                    self.validate_virtual_dispatch_slot(
                        *declaring_type,
                        *slot_id,
                        ValidateAnchor::node(instruction_id),
                    )?;
                }
                Instruction::CallInterface {
                    destination,
                    declaring_type,
                    slot_id,
                    signature,
                    effects,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                    self.validate_interface_dispatch_slot(
                        *declaring_type,
                        *slot_id,
                        ValidateAnchor::node(instruction_id),
                    )?;
                }
                Instruction::CallIndirect {
                    destination,
                    signature,
                    effects,
                    env,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    if let Some(env) = env {
                        let env_type_id = self.value_type_or_error(
                            function,
                            *env,
                            ValidateAnchor::node(instruction_id),
                            "call.indirect env",
                        )?;
                        let env_type = self.tree.get(env_type_id);
                        if !matches!(env_type, Type::Reference { .. }) {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "call.indirect env must be a reference type".to_string(),
                                anchor: ValidateAnchor::node(instruction_id),
                            });
                        }
                    }
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                _ => {}
            }
        }

        // validate instruction node references
        self.validate_instruction_nodes(instruction, instruction_id)?;

        // validate local references
        self.validate_local_reference(instruction, locals, instruction_id)?;

        // validate instruction shape invariants
        self.validate_instruction_shapes(instruction, instruction_id)?;

        // validate vector and tensor instruction semantics
        self.validate_vector_tensor_ops(function, instruction, instruction_id)?;

        // validate inline types
        self.validate_instruction_inline_types(function, instruction, instruction_id)?;

        // validate function allocation policy
        self.validate_allocation_mode(function, instruction, instruction_id)?;

        Ok(())
    }

    /// Validate function allocation policy against allocation instructions.
    fn validate_allocation_mode(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // classify allocation instructions
        let allocation_instruction = match instruction {
            Instruction::ManagedAlloc { .. } => Some("managed.alloc"),
            Instruction::ManagedAllocArray { .. } => Some("managed.alloc_array"),
            Instruction::RawAlloc { .. } => Some("raw.alloc"),
            Instruction::StackAlloc { .. } => Some("stack.alloc"),
            _ => None,
        };
        let Some(allocation_instruction) = allocation_instruction else {
            return Ok(());
        };

        // enforce allocation mode policy
        let violation = match function.allocation {
            AllocationMode::Any => None,
            AllocationMode::NoManaged => matches!(
                instruction,
                Instruction::ManagedAlloc { .. } | Instruction::ManagedAllocArray { .. }
            )
            .then_some("no_managed forbids managed allocations"),
            AllocationMode::StackOnly => matches!(
                instruction,
                Instruction::ManagedAlloc { .. }
                    | Instruction::ManagedAllocArray { .. }
                    | Instruction::RawAlloc { .. }
            )
            .then_some("stack_only forbids non-stack allocations"),
        };
        let Some(violation) = violation else {
            return Ok(());
        };

        Err(ValidateError::MetadataInvariantViolation {
            message: format!(
                "allocation mode violation: '{allocation_instruction}' is invalid because {violation}"
            ),
            anchor: ValidateAnchor::node(instruction_id),
        })
    }

    /// Resolve a value type for validation.
    pub(super) fn value_type_or_error(
        &self,
        function: &Function,
        value: Value,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        // look up the value type
        let value_type = function.value_type(value).ok_or_else(|| {
            ValidateError::MetadataInvariantViolation {
                message: format!("missing value type for {label}"),
                anchor,
            }
        })?;

        Ok(value_type)
    }

    /// Check whether two tensor shapes are compatible for casts.
    fn tensor_shapes_compatible(
        &self,
        left: &[TensorDimension],
        right: &[TensorDimension],
    ) -> bool {
        // reject mismatched ranks
        if left.len() != right.len() {
            return false;
        }

        // allow dynamic to match any dimension
        left.iter()
            .zip(right)
            .all(|(left_dim, right_dim)| match (left_dim, right_dim) {
                (TensorDimension::Static(left_size), TensorDimension::Static(right_size)) => {
                    left_size == right_size
                }
                _ => true,
            })
    }

    /// Validate vector and tensor instruction invariants.
    fn validate_vector_tensor_ops(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // prepare anchor for error reporting
        let anchor = ValidateAnchor::node(instruction_id);

        match instruction {
            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                // resolve operand types
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "binary left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "binary right")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "binary result")?;

                // validate elementwise vector/tensor operations
                match self.tree.get(left_type_id) {
                    Type::Vector {
                        element: left_element,
                        lanes: left_lanes,
                        ..
                    } => {
                        let right_type = self.tree.get(right_type_id);
                        let destination_type = self.tree.get(destination_type_id);
                        let (right_element, right_lanes) = match right_type {
                            Type::Vector { element, lanes, .. } => (element, lanes),
                            other => {
                                return Err(ValidateError::MetadataInvariantViolation {
                                    message: format!(
                                        "vector binary expects vector operands, found {}",
                                        self.type_kind(other)
                                    ),
                                    anchor,
                                });
                            }
                        };
                        let (dest_element, dest_lanes) = match destination_type {
                            Type::Vector { element, lanes, .. } => (element, lanes),
                            other => {
                                return Err(ValidateError::MetadataInvariantViolation {
                                    message: format!(
                                        "vector binary result must be a vector, found {}",
                                        self.type_kind(other)
                                    ),
                                    anchor,
                                });
                            }
                        };

                        if left_element != right_element
                            || left_lanes != right_lanes
                            || left_element != dest_element
                            || left_lanes != dest_lanes
                        {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "vector binary operands and result must match".to_string(),
                                anchor,
                            });
                        }

                        if operator.is_comparison() {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "vector comparisons must use vector.compare".to_string(),
                                anchor,
                            });
                        }
                    }
                    Type::Tensor {
                        element: left_element,
                        shape: left_shape,
                        layout: left_layout,
                        ..
                    } => {
                        let right_type = self.tree.get(right_type_id);
                        let destination_type = self.tree.get(destination_type_id);
                        let (right_element, right_shape, right_layout) = match right_type {
                            Type::Tensor {
                                element,
                                shape,
                                layout,
                                ..
                            } => (element, shape, layout),
                            other => {
                                return Err(ValidateError::MetadataInvariantViolation {
                                    message: format!(
                                        "tensor binary expects tensor operands, found {}",
                                        self.type_kind(other)
                                    ),
                                    anchor,
                                });
                            }
                        };
                        let (dest_element, dest_shape, dest_layout) = match destination_type {
                            Type::Tensor {
                                element,
                                shape,
                                layout,
                                ..
                            } => (element, shape, layout),
                            other => {
                                return Err(ValidateError::MetadataInvariantViolation {
                                    message: format!(
                                        "tensor binary result must be a tensor, found {}",
                                        self.type_kind(other)
                                    ),
                                    anchor,
                                });
                            }
                        };

                        if left_element != right_element
                            || left_shape != right_shape
                            || left_layout != right_layout
                            || left_element != dest_element
                            || left_shape != dest_shape
                            || left_layout != dest_layout
                        {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "tensor binary operands and result must match".to_string(),
                                anchor,
                            });
                        }

                        if operator.is_comparison() {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "tensor comparisons must use tensor.compare".to_string(),
                                anchor,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Instruction::Unary {
                destination,
                argument,
                ..
            } => {
                let argument_type_id =
                    self.value_type_or_error(function, *argument, anchor, "unary argument")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "unary result")?;

                match self.tree.get(argument_type_id) {
                    Type::Vector { .. } => {
                        if argument_type_id != destination_type_id {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "vector unary result must match operand type".to_string(),
                                anchor,
                            });
                        }
                    }
                    Type::Tensor { .. } => {
                        if argument_type_id != destination_type_id {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "tensor unary result must match operand type".to_string(),
                                anchor,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                // reject non comparison operators
                if !operator.is_comparison() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                // resolve operand types
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "vector.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "vector.compare right")?;

                // resolve operand vector layouts
                let left_type = self.tree.get(left_type_id);
                let right_type = self.tree.get(right_type_id);
                let (left_element, left_lanes) = match left_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (right_element, right_lanes) = match right_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched vector operands
                if left_element != right_element || left_lanes != right_lanes {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare expects matching vector operand types".to_string(),
                        anchor,
                    });
                }

                // resolve destination type
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.compare result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let (result_element, result_lanes) = match destination_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare result must be a vector, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched lane counts
                if *result_lanes != *left_lanes {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare result must match lane count".to_string(),
                        anchor,
                    });
                }

                // reject non boolean result element types
                let result_element_type = self.tree.get(*result_element);
                if !matches!(result_element_type, Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare result element type must be bool".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::VectorConvert {
                destination,
                vector,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *vector, anchor, "vector.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.convert result",
                )?;

                // resolve vector types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let source_lanes = match source_type {
                    Type::Vector { lanes, .. } => lanes,
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.convert expects vector source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let destination_lanes = match destination_type {
                    Type::Vector { lanes, .. } => lanes,
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.convert expects vector result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched lane counts
                if source_lanes != destination_lanes {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.convert must preserve lane count".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                // reject non comparison operators
                if !operator.is_comparison() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                // resolve operand types
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "tensor.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "tensor.compare right")?;

                // resolve operand tensor layouts
                let left_type = self.tree.get(left_type_id);
                let right_type = self.tree.get(right_type_id);
                let (left_element, left_shape, left_layout) = match left_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (right_element, right_shape, right_layout) = match right_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched tensor operands
                if left_element != right_element
                    || left_shape != right_shape
                    || left_layout != right_layout
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare expects matching tensor operand types".to_string(),
                        anchor,
                    });
                }

                // resolve destination type
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.compare result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let (result_element, result_shape, result_layout) = match destination_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare result must be a tensor, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched shapes or layouts
                if result_shape != left_shape || result_layout != left_layout {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare result must match tensor shape and layout"
                            .to_string(),
                        anchor,
                    });
                }

                // reject non boolean result element types
                let result_element_type = self.tree.get(*result_element);
                if !matches!(result_element_type, Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare result element type must be bool".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                let mask_type_id =
                    self.value_type_or_error(function, *mask, anchor, "vector.select mask")?;
                let then_type_id =
                    self.value_type_or_error(function, *then_value, anchor, "vector.select then")?;
                let else_type_id =
                    self.value_type_or_error(function, *else_value, anchor, "vector.select else")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.select result",
                )?;

                let mask_type = self.tree.get(mask_type_id);
                let (mask_element, mask_lanes) = match mask_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.select mask must be a vector, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                if !matches!(self.tree.get(*mask_element), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.select mask element type must be bool".to_string(),
                        anchor,
                    });
                }

                let then_type = self.tree.get(then_type_id);
                let else_type = self.tree.get(else_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (then_element, then_lanes) = match then_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.select expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (else_element, else_lanes) = match else_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.select expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (dest_element, dest_lanes) = match destination_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "vector.select result must be a vector, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                if then_element != else_element
                    || then_lanes != else_lanes
                    || then_element != dest_element
                    || then_lanes != dest_lanes
                    || then_lanes != mask_lanes
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.select operands must match mask lane count".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                let mask_type_id =
                    self.value_type_or_error(function, *mask, anchor, "tensor.select mask")?;
                let then_type_id =
                    self.value_type_or_error(function, *then_value, anchor, "tensor.select then")?;
                let else_type_id =
                    self.value_type_or_error(function, *else_value, anchor, "tensor.select else")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.select result",
                )?;

                let mask_type = self.tree.get(mask_type_id);
                let (mask_element, mask_shape, mask_layout) = match mask_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.select mask must be a tensor, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                if !matches!(self.tree.get(*mask_element), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.select mask element type must be bool".to_string(),
                        anchor,
                    });
                }

                let then_type = self.tree.get(then_type_id);
                let else_type = self.tree.get(else_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (then_element, then_shape, then_layout) = match then_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.select expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (else_element, else_shape, else_layout) = match else_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.select expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (dest_element, dest_shape, dest_layout) = match destination_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.select result must be a tensor, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                if then_element != else_element
                    || then_shape != else_shape
                    || then_layout != else_layout
                    || then_element != dest_element
                    || then_shape != dest_shape
                    || then_layout != dest_layout
                    || then_shape != mask_shape
                    || then_layout != mask_layout
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.select operands must match mask shape".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorConvert {
                destination,
                tensor,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.convert result",
                )?;

                // resolve tensor types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (source_shape, source_layout) = match source_type {
                    Type::Tensor { shape, layout, .. } => (shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.convert expects tensor source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (destination_shape, destination_layout) = match destination_type {
                    Type::Tensor { shape, layout, .. } => (shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.convert expects tensor result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched shapes or layouts
                if source_shape != destination_shape || source_layout != destination_layout {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.convert must preserve shape and layout".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorCast {
                destination,
                tensor,
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.cast source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.cast result")?;

                // resolve tensor types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (source_element, source_shape, source_layout) = match source_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.cast expects tensor source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (destination_element, destination_shape, destination_layout) =
                    match destination_type {
                        Type::Tensor {
                            element,
                            shape,
                            layout,
                            ..
                        } => (element, shape, layout),
                        other => {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: format!(
                                    "tensor.cast expects tensor result, found {}",
                                    self.type_kind(other)
                                ),
                                anchor,
                            });
                        }
                    };

                // reject element or layout mismatches
                if source_element != destination_element
                    || source_layout != destination_layout
                    || !self.tensor_shapes_compatible(source_shape, destination_shape)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.cast requires compatible shapes and matching layout"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorView {
                destination,
                view,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *view, anchor, "tensor.view source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.view result")?;

                // resolve view types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (
                    source_kind,
                    source_address_space,
                    source_mutability,
                    source_element,
                    source_shape,
                ) = match source_type {
                    Type::TensorReference {
                        kind,
                        address_space,
                        mutability,
                        element,
                        shape,
                        ..
                    } => (kind, address_space, mutability, element, shape),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.view expects tensor reference source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (
                    destination_kind,
                    destination_address_space,
                    destination_mutability,
                    destination_element,
                    destination_shape,
                ) = match destination_type {
                    Type::TensorReference {
                        kind,
                        address_space,
                        mutability,
                        element,
                        shape,
                        ..
                    } => (kind, address_space, mutability, element, shape),
                    other => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.view expects tensor reference result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject incompatible reference kinds
                if source_kind != destination_kind
                    || source_address_space != destination_address_space
                    || source_element != destination_element
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view requires matching reference kind, address space, and element type"
                            .to_string(),
                        anchor,
                    });
                }

                // reject mutability upgrades
                if matches!(source_mutability, Mutability::Immutable)
                    && matches!(destination_mutability, Mutability::Mutable)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view cannot increase mutability".to_string(),
                        anchor,
                    });
                }

                // reject mismatched ranks
                if source_shape.len() != destination_shape.len() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view requires matching ranks".to_string(),
                        anchor,
                    });
                }

                // reject mismatched view argument counts
                let expected = destination_shape.len();
                if *offsets_count as usize != expected
                    || *sizes_count as usize != expected
                    || *strides_count as usize != expected
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "tensor.view requires offsets, sizes, and strides for each dimension"
                                .to_string(),
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Ensure a value is defined within the function.
    pub(super) fn ensure_defined(
        &self,
        value: Value,
        anchor: ValidateAnchor,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        // ensure the value was defined
        if !defined_values.contains(&value) {
            return Err(ValidateError::UseOfUndefinedValue { value, anchor });
        }

        Ok(())
    }

    /// Ensure a node id points at the expected node type.
    pub(super) fn ensure_node_type(
        &self,
        expected: NodeType,
        node_id: u32,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // resolve the node type
        let found = self
            .tree
            .node_type_by_node_id
            .get(node_id as usize)
            .copied();

        // reject mismatched node types
        if found != Some(expected) {
            return Err(ValidateError::InvalidNodeReference {
                expected,
                found,
                node_id,
                anchor,
            });
        }

        Ok(())
    }

    /// Ensure an argument slice is within the argument buffer.
    pub(super) fn validate_argument_slice(
        &self,
        slice: ArgumentSlice,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // compute slice bounds
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        let len = self.tree.instruction_arguments.len();

        // reject out of bounds slices
        if end > len {
            return Err(ValidateError::ArgumentSliceOutOfBounds {
                start: slice.start,
                count: slice.count,
                len,
                anchor: ValidateAnchor::node(instruction_id),
            });
        }

        Ok(())
    }

    /// Validate local references are defined within the function.
    fn validate_local_reference(
        &self,
        instruction: &Instruction,
        locals: &HashSet<LocalNodeId<Local>>,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // validate local references
        match instruction {
            Instruction::LocalGet { local, .. }
            | Instruction::LocalAddr { local, .. }
            | Instruction::LocalSet { local, .. } => {
                // reject locals not owned by the function
                if !locals.contains(local) {
                    return Err(ValidateError::LocalReferenceNotInFunction {
                        local_id: *local,
                        anchor: ValidateAnchor::node(instruction_id),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate instruction node references.
    fn validate_instruction_nodes(
        &self,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // prepare anchor for node checks
        let anchor = ValidateAnchor::node(instruction_id);

        // validate instruction node references
        match instruction {
            Instruction::LocalGet { local, .. } | Instruction::LocalSet { local, .. } => {
                // ensure local ids resolve
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
            }
            Instruction::LocalAddr {
                local, result_type, ..
            } => {
                // ensure local ids resolve
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
            }
            Instruction::GlobalAddr { global, .. } | Instruction::GlobalConst { global, .. } => {
                // ensure global ids resolve
                self.ensure_node_type(NodeType::Global, global.id, anchor)?;
            }
            Instruction::FunctionAddr { function, .. } => {
                // ensure function ids resolve
                self.ensure_node_type(NodeType::Function, function.id, anchor)?;
            }
            Instruction::Call { function, .. } => {
                // ensure function ids resolve
                self.ensure_node_type(NodeType::Function, function.id, anchor)?;
            }
            Instruction::CallVirtual { .. } | Instruction::CallInterface { .. } => {}
            Instruction::Cast { to_type, .. }
            | Instruction::Struct { ty: to_type, .. }
            | Instruction::Tuple { ty: to_type, .. }
            | Instruction::Array { ty: to_type, .. }
            | Instruction::ManagedAlloc {
                layout: to_type, ..
            }
            | Instruction::ManagedAllocArray {
                element: to_type, ..
            }
            | Instruction::RawAlloc {
                layout: to_type, ..
            }
            | Instruction::StackAlloc {
                layout: to_type, ..
            } => {
                // ensure type ids resolve
                self.ensure_node_type(NodeType::Type, to_type.id, anchor)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate aggregate constructor shapes.
    fn validate_instruction_shapes(
        &self,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // prepare anchor for shape validation
        let anchor = ValidateAnchor::node(instruction_id);

        // validate aggregate constructor shapes
        match instruction {
            Instruction::Struct { ty, fields, .. } => {
                // resolve struct layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Struct { fields, .. } => fields.len(),
                    Type::FunctionValue { .. } => 2,
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "struct or fnvalue",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = fields.len();

                // reject mismatched field counts
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::Tuple { ty, elements, .. } => {
                // resolve tuple layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Tuple { elements, .. } => elements.len(),
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "tuple",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = elements.len();

                // reject mismatched element counts
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::Array { ty, elements, .. } => {
                // resolve array layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Array { length, .. } => usize::try_from(*length).unwrap_or(usize::MAX),
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "array",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = elements.len();

                // reject mismatched element counts
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorSlice {
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *offsets_count as usize + *sizes_count as usize + *strides_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorView {
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *offsets_count as usize + *sizes_count as usize + *strides_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorPad {
                arguments,
                low_count,
                high_count,
                interior_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *low_count as usize + *high_count as usize + *interior_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate inline result and signature types.
    fn validate_instruction_inline_types(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // prepare anchor for error reporting
        let anchor = ValidateAnchor::node(instruction_id);

        // validate pointer-producing result types
        match instruction {
            Instruction::Const { destination, value } => {
                let Constant::Null = value else {
                    return Ok(());
                };

                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "null constant")?;
                let destination_type = self.tree.get(destination_type_id);
                let (kind, _address_space, is_nullable) = match destination_type {
                    Type::Reference {
                        kind,
                        address_space,
                        is_nullable,
                        ..
                    } => (*kind, *address_space, *is_nullable),
                    _ => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "null constant requires a reference type".to_string(),
                            anchor,
                        });
                    }
                };

                if !is_nullable {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "null constant requires a nullable reference type".to_string(),
                        anchor,
                    });
                }

                if !matches!(
                    kind,
                    ReferenceKind::Managed
                        | ReferenceKind::Owned
                        | ReferenceKind::Borrowed
                        | ReferenceKind::Raw
                ) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "null constant requires a managed or raw reference kind"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::GlobalAddr {
                global,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                let global_decl = self.tree.get(*global);
                self.validate_reference_result_type(
                    *result_type,
                    Some(global_decl.ty),
                    Some(ReferenceKind::Raw),
                    Some(global_decl.mutability),
                    anchor,
                )?;
            }
            Instruction::FunctionAddr {
                destination,
                function: target,
            } => {
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "function.addr result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let Type::FunctionPointer { parameters, result } = destination_type else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.addr result must be a function pointer".to_string(),
                        anchor,
                    });
                };

                let target_function = self.tree.get(*target);
                if target_function.parameters.len() != parameters.len() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.addr result parameter count mismatch".to_string(),
                        anchor,
                    });
                }

                for (parameter, signature_type) in
                    target_function.parameters.iter().zip(parameters.iter())
                {
                    if parameter.ty != *signature_type {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function.addr result parameter types mismatch".to_string(),
                            anchor,
                        });
                    }
                }

                if target_function.return_type != *result {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.addr result return type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::FunctionEnv { destination } => {
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "function.env result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                if !matches!(destination_type, Type::Reference { .. }) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.env result must be a reference type".to_string(),
                        anchor,
                    });
                }

                let env_type = function.closure_env_type.ok_or_else(|| {
                    ValidateError::MetadataInvariantViolation {
                        message: "function.env requires a closure_env type".to_string(),
                        anchor,
                    }
                })?;
                if env_type != destination_type_id {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.env type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::ManagedAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Managed),
                    None,
                    anchor,
                )?;
            }
            Instruction::ManagedAllocArray {
                element,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    *result_type,
                    Some(*element),
                    Some(ReferenceKind::Managed),
                    None,
                    anchor,
                )?;
            }
            Instruction::RawAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                let Type::Reference { kind, pointee, .. } = self.tree.get(*result_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "pointer-producing instruction result type is not a reference"
                            .to_string(),
                        anchor,
                    });
                };

                if *pointee != *layout {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "pointer-producing instruction result type mismatches pointee"
                            .to_string(),
                        anchor,
                    });
                }

                if !matches!(kind, ReferenceKind::Raw | ReferenceKind::Owned) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "pointer-producing instruction result type has wrong reference kind"
                                .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::StackAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Raw),
                    None,
                    anchor,
                )?;
            }
            Instruction::AtomicLoad {
                destination,
                pointer,
                result_type,
                ..
            } => {
                let pointer_type =
                    self.value_type_or_error(function, *pointer, anchor, "atomic.load pointer")?;
                let destination_type =
                    self.value_type_or_error(function, *destination, anchor, "atomic.load result")?;

                let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.load pointer must be a reference type".to_string(),
                        anchor,
                    });
                };

                if !self.types_equivalent(*pointee, *result_type)
                    || !self.types_equivalent(destination_type, *result_type)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.load result type must match the pointer pointee"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::AtomicStore { pointer, value, .. } => {
                let pointer_type =
                    self.value_type_or_error(function, *pointer, anchor, "atomic.store pointer")?;
                let value_type =
                    self.value_type_or_error(function, *value, anchor, "atomic.store value")?;

                let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.store pointer must be a reference type".to_string(),
                        anchor,
                    });
                };

                if !self.is_store_compatible_type(value_type, *pointee) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.store value type must match the pointer pointee"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                ..
            } => {
                let pointer_type = self.value_type_or_error(
                    function,
                    *pointer,
                    anchor,
                    "atomic.compare_exchange pointer",
                )?;
                let expected_type = self.value_type_or_error(
                    function,
                    *expected,
                    anchor,
                    "atomic.compare_exchange expected",
                )?;
                let new_value_type = self.value_type_or_error(
                    function,
                    *new_value,
                    anchor,
                    "atomic.compare_exchange new value",
                )?;
                let destination_type = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "atomic.compare_exchange result",
                )?;

                let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.compare_exchange pointer must be a reference type"
                            .to_string(),
                        anchor,
                    });
                };

                if !self.types_equivalent(expected_type, *pointee)
                    || !self.types_equivalent(new_value_type, *pointee)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.compare_exchange values must match the pointer pointee"
                            .to_string(),
                        anchor,
                    });
                }

                let Type::Tuple { elements, .. } = self.tree.get(destination_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "atomic.compare_exchange result must be a tuple of (old_value, bool)"
                                .to_string(),
                        anchor,
                    });
                };

                if elements.len() != 2
                    || !self.types_equivalent(elements[0], *pointee)
                    || !matches!(self.tree.get(elements[1]), Type::Boolean)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "atomic.compare_exchange result must be a tuple of (old_value, bool)"
                                .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::AtomicRmw {
                destination,
                pointer,
                value,
                ..
            } => {
                let pointer_type =
                    self.value_type_or_error(function, *pointer, anchor, "atomic.rmw pointer")?;
                let value_type =
                    self.value_type_or_error(function, *value, anchor, "atomic.rmw value")?;
                let destination_type =
                    self.value_type_or_error(function, *destination, anchor, "atomic.rmw result")?;

                let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.rmw pointer must be a reference type".to_string(),
                        anchor,
                    });
                };

                if !self.types_equivalent(value_type, *pointee)
                    || !self.types_equivalent(destination_type, *pointee)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "atomic.rmw value and result must match the pointer pointee"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::AtomicFence { .. } | Instruction::Barrier { .. } => {}
            Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let aggregate_type =
                    self.value_type_or_error(function, *aggregate, anchor, "field.get aggregate")?;
                let destination_type = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "field.get destination",
                )?;

                let field_type =
                    self.field_type_for_aggregate(aggregate_type, *index, anchor, "field.get")?;
                if !self.types_equivalent(destination_type, field_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "field.get destination type mismatches projected field type"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let aggregate_type =
                    self.value_type_or_error(function, *aggregate, anchor, "field.set aggregate")?;
                let destination_type = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "field.set destination",
                )?;
                let value_type =
                    self.value_type_or_error(function, *value, anchor, "field.set value")?;

                if !self.types_equivalent(destination_type, aggregate_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "field.set destination type mismatches aggregate type".to_string(),
                        anchor,
                    });
                }

                let field_type =
                    self.field_type_for_aggregate(aggregate_type, *index, anchor, "field.set")?;
                if !self.is_store_compatible_type(value_type, field_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "field.set value type mismatches projected field type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::FieldAddr {
                aggregate,
                index,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;

                let aggregate_type =
                    self.value_type_or_error(function, *aggregate, anchor, "field.addr aggregate")?;
                let _ =
                    self.field_type_for_aggregate(aggregate_type, *index, anchor, "field.addr")?;
                self.validate_reference_result_type(*result_type, None, None, None, anchor)?;
            }
            Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let array_type =
                    self.value_type_or_error(function, *array, anchor, "element.get array")?;
                let destination_type = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "element.get destination",
                )?;
                let index_type =
                    self.value_type_or_error(function, *index, anchor, "element.get index")?;

                let element_type =
                    self.element_type_for_array(array_type, anchor, "element.get")?;
                if !self.types_equivalent(destination_type, element_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.get destination type mismatches array element type"
                            .to_string(),
                        anchor,
                    });
                }

                if !matches!(
                    self.tree.get(index_type),
                    Type::Int { .. } | Type::Isize | Type::Usize
                ) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.get index must be an integer type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let array_type =
                    self.value_type_or_error(function, *array, anchor, "element.set array")?;
                let destination_type = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "element.set destination",
                )?;
                let index_type =
                    self.value_type_or_error(function, *index, anchor, "element.set index")?;
                let value_type =
                    self.value_type_or_error(function, *value, anchor, "element.set value")?;

                if !self.types_equivalent(destination_type, array_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.set destination type mismatches array type".to_string(),
                        anchor,
                    });
                }

                let element_type =
                    self.element_type_for_array(array_type, anchor, "element.set")?;
                if !self.is_store_compatible_type(value_type, element_type) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.set value type mismatches array element type".to_string(),
                        anchor,
                    });
                }

                if !matches!(
                    self.tree.get(index_type),
                    Type::Int { .. } | Type::Isize | Type::Usize
                ) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.set index must be an integer type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::ElementAddr {
                array,
                index,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;

                let array_type =
                    self.value_type_or_error(function, *array, anchor, "element.addr array")?;
                let index_type =
                    self.value_type_or_error(function, *index, anchor, "element.addr index")?;
                let _ = self.element_type_for_array(array_type, anchor, "element.addr")?;
                self.validate_reference_result_type(*result_type, None, None, None, anchor)?;

                if !matches!(
                    self.tree.get(index_type),
                    Type::Int { .. } | Type::Isize | Type::Usize
                ) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "element.addr index must be an integer type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                let argument_type =
                    self.value_type_or_error(function, *argument, anchor, "cast argument")?;
                self.validate_cast_legality(*operator, argument_type, *to_type, anchor)?;
            }
            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                let arguments = self.tree.get_arguments(*arguments);
                match intrinsic {
                    Intrinsic::AddrSpaceCast => {
                        if arguments.len() != 1 {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "addrspace.cast requires one argument".to_string(),
                                anchor,
                            });
                        }
                        let Some(destination) = destination else {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "addrspace.cast requires a destination value".to_string(),
                                anchor,
                            });
                        };

                        let source_type = self.value_type_or_error(
                            function,
                            arguments[0],
                            anchor,
                            "addrspace.cast source",
                        )?;
                        let destination_type = self.value_type_or_error(
                            function,
                            *destination,
                            anchor,
                            "addrspace.cast destination",
                        )?;
                        let source = self.tree.get(source_type);
                        let destination = self.tree.get(destination_type);

                        match (source, destination) {
                            (
                                Type::Reference {
                                    kind: source_kind,
                                    mutability: source_mutability,
                                    pointee: source_pointee,
                                    ..
                                },
                                Type::Reference {
                                    kind: destination_kind,
                                    mutability: destination_mutability,
                                    pointee: destination_pointee,
                                    ..
                                },
                            ) => {
                                if source_kind != destination_kind
                                    || source_mutability != destination_mutability
                                    || source_pointee != destination_pointee
                                {
                                    return Err(ValidateError::MetadataInvariantViolation {
                                        message: "addrspace.cast requires matching reference kind, mutability, and pointee".to_string(),
                                        anchor,
                                    });
                                }
                            }
                            (
                                Type::TensorReference {
                                    kind: source_kind,
                                    mutability: source_mutability,
                                    element: source_element,
                                    shape: source_shape,
                                    layout: source_layout,
                                    ..
                                },
                                Type::TensorReference {
                                    kind: destination_kind,
                                    mutability: destination_mutability,
                                    element: destination_element,
                                    shape: destination_shape,
                                    layout: destination_layout,
                                    ..
                                },
                            ) => {
                                if source_kind != destination_kind
                                    || source_mutability != destination_mutability
                                    || source_element != destination_element
                                    || source_shape != destination_shape
                                    || source_layout != destination_layout
                                {
                                    return Err(ValidateError::MetadataInvariantViolation {
                                        message: "addrspace.cast requires matching tensor reference kind, mutability, element, shape, and layout".to_string(),
                                        anchor,
                                    });
                                }
                            }
                            _ => {
                                return Err(ValidateError::MetadataInvariantViolation {
                                    message:
                                        "addrspace.cast requires reference or tensor reference types"
                                            .to_string(),
                                    anchor,
                                });
                            }
                        }
                    }
                    Intrinsic::PtrOffsetFrom => {
                        if arguments.len() != 2 {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "ptr_offset_from requires two pointer arguments"
                                    .to_string(),
                                anchor,
                            });
                        }
                        let Some(destination) = destination else {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "ptr_offset_from requires a destination value".to_string(),
                                anchor,
                            });
                        };

                        let left_type = self.value_type_or_error(
                            function,
                            arguments[0],
                            anchor,
                            "ptr_offset_from left",
                        )?;
                        let right_type = self.value_type_or_error(
                            function,
                            arguments[1],
                            anchor,
                            "ptr_offset_from right",
                        )?;
                        let destination_type = self.value_type_or_error(
                            function,
                            *destination,
                            anchor,
                            "ptr_offset_from destination",
                        )?;
                        let left = self.tree.get(left_type);
                        let right = self.tree.get(right_type);
                        let destination = self.tree.get(destination_type);

                        if !matches!(
                            left,
                            Type::Reference {
                                kind: ReferenceKind::Raw,
                                ..
                            } | Type::TensorReference {
                                kind: ReferenceKind::Raw,
                                ..
                            }
                        ) || !matches!(
                            right,
                            Type::Reference {
                                kind: ReferenceKind::Raw,
                                ..
                            } | Type::TensorReference {
                                kind: ReferenceKind::Raw,
                                ..
                            }
                        ) {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "ptr_offset_from requires raw pointer arguments"
                                    .to_string(),
                                anchor,
                            });
                        }

                        if !matches!(destination, Type::Int { .. } | Type::Isize | Type::Usize) {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "ptr_offset_from destination must be an integer type"
                                    .to_string(),
                                anchor,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Instruction::Call { signature, .. } | Instruction::CallIndirect { signature, .. } => {
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                let ty = self.tree.get(*signature);
                if !matches!(ty, Type::FunctionPointer { .. }) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::CallVirtual {
                declaring_type,
                signature,
                ..
            }
            | Instruction::CallInterface {
                declaring_type,
                signature,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                let ty = self.tree.get(*signature);
                if !matches!(ty, Type::FunctionPointer { .. }) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Resolve the projected field type for a struct, tuple, or function value aggregate.
    fn field_type_for_aggregate(
        &self,
        aggregate_type: LocalNodeId<Type>,
        index: u32,
        anchor: ValidateAnchor,
        operation: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        let index = index as usize;
        match self.tree.get(aggregate_type) {
            Type::Struct { fields, .. } => {
                let Some(field_id) = fields.get(index) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{operation} field index {index} out of bounds for struct with {} fields",
                            fields.len()
                        ),
                        anchor,
                    });
                };

                Ok(self.tree.get(*field_id).ty)
            }
            Type::Tuple { elements, .. } => {
                let Some(element_type) = elements.get(index) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{operation} field index {index} out of bounds for tuple with {} elements",
                            elements.len()
                        ),
                        anchor,
                    });
                };

                Ok(*element_type)
            }
            Type::FunctionValue {
                signature,
                environment,
            } => match index {
                0 => Ok(*signature),
                1 => Ok(*environment),
                _ => Err(ValidateError::MetadataInvariantViolation {
                    message: format!(
                        "{operation} field index {index} out of bounds for fnvalue with 2 fields"
                    ),
                    anchor,
                }),
            },
            Type::Reference { pointee, .. } => match self.tree.get(*pointee) {
                Type::Struct { fields, .. } => {
                    let Some(field_id) = fields.get(index) else {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "{operation} field index {index} out of bounds for struct with {} fields",
                                fields.len()
                            ),
                            anchor,
                        });
                    };

                    Ok(self.tree.get(*field_id).ty)
                }
                Type::Tuple { elements, .. } => {
                    let Some(element_type) = elements.get(index) else {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "{operation} field index {index} out of bounds for tuple with {} elements",
                                elements.len()
                            ),
                            anchor,
                        });
                    };

                    Ok(*element_type)
                }
                Type::FunctionValue {
                    signature,
                    environment,
                } => match index {
                    0 => Ok(*signature),
                    1 => Ok(*environment),
                    _ => Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{operation} field index {index} out of bounds for fnvalue with 2 fields"
                        ),
                        anchor,
                    }),
                },
                _ => {
                    if index != 0 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: format!(
                                "{operation} field index {index} out of bounds for scalar pointee"
                            ),
                            anchor,
                        });
                    }

                    Ok(*pointee)
                }
            },
            _ => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{operation} expects a struct, tuple, or fnvalue aggregate"),
                anchor,
            }),
        }
    }

    /// Resolve the element type for an array or pointer aggregate.
    fn element_type_for_array(
        &self,
        array_type: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        operation: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        match self.tree.get(array_type) {
            Type::Array { element, .. } => Ok(*element),
            Type::Reference { pointee, .. } => match self.tree.get(*pointee) {
                Type::Array { element, .. } => Ok(*element),
                _ => Ok(*pointee),
            },
            Type::TensorReference { element, .. } => Ok(*element),
            _ => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{operation} expects an array aggregate"),
                anchor,
            }),
        }
    }

    /// Check whether a stored value type is compatible with a projected slot type.
    fn is_store_compatible_type(
        &self,
        value_type: LocalNodeId<Type>,
        slot_type: LocalNodeId<Type>,
    ) -> bool {
        if self.types_equivalent(value_type, slot_type) {
            return true;
        }

        match (self.tree.get(value_type), self.tree.get(slot_type)) {
            (
                Type::Reference {
                    pointee: value_pointee,
                    ..
                },
                Type::Reference {
                    pointee: slot_pointee,
                    ..
                },
            ) => self.types_equivalent(*value_pointee, *slot_pointee),
            (
                Type::TensorReference {
                    element: value_element,
                    shape: value_shape,
                    layout: value_layout,
                    ..
                },
                Type::TensorReference {
                    element: slot_element,
                    shape: slot_shape,
                    layout: slot_layout,
                    ..
                },
            ) => {
                value_element == slot_element
                    && value_shape == slot_shape
                    && value_layout == slot_layout
            }
            (
                Type::Reference {
                    pointee: value_pointee,
                    ..
                },
                _,
            ) => self.types_equivalent(*value_pointee, slot_type),
            (
                _,
                Type::Reference {
                    pointee: slot_pointee,
                    ..
                },
            ) => self.types_equivalent(value_type, *slot_pointee),
            (
                Type::TensorReference {
                    element: value_element,
                    shape: value_shape,
                    layout: value_layout,
                    ..
                },
                Type::Tensor {
                    element: slot_element,
                    shape: slot_shape,
                    layout: slot_layout,
                    ..
                },
            ) => {
                value_element == slot_element
                    && value_shape == slot_shape
                    && value_layout == slot_layout
            }
            (
                Type::Tensor {
                    element: value_element,
                    shape: value_shape,
                    layout: value_layout,
                    ..
                },
                Type::TensorReference {
                    element: slot_element,
                    shape: slot_shape,
                    layout: slot_layout,
                    ..
                },
            ) => {
                value_element == slot_element
                    && value_shape == slot_shape
                    && value_layout == slot_layout
            }
            _ => false,
        }
    }

    /// Check whether two type ids are structurally equivalent.
    fn types_equivalent(
        &self,
        left_type: LocalNodeId<Type>,
        right_type: LocalNodeId<Type>,
    ) -> bool {
        let mut seen_pairs = HashSet::new();
        self.types_equivalent_inner(left_type, right_type, &mut seen_pairs)
    }

    /// Compare two type ids recursively while tolerating distinct but equivalent node ids.
    fn types_equivalent_inner(
        &self,
        left_type: LocalNodeId<Type>,
        right_type: LocalNodeId<Type>,
        seen_pairs: &mut HashSet<(u32, u32)>,
    ) -> bool {
        if left_type == right_type {
            return true;
        }

        if !seen_pairs.insert((left_type.id, right_type.id)) {
            return true;
        }

        match (self.tree.get(left_type), self.tree.get(right_type)) {
            (Type::Void, Type::Void)
            | (Type::Boolean, Type::Boolean)
            | (Type::Isize, Type::Isize)
            | (Type::Usize, Type::Usize)
            | (Type::TypeDescriptor, Type::TypeDescriptor)
            | (Type::TypeId, Type::TypeId) => true,
            (
                Type::Int {
                    width: left_width,
                    is_signed: left_signed,
                },
                Type::Int {
                    width: right_width,
                    is_signed: right_signed,
                },
            ) => left_width == right_width && left_signed == right_signed,
            (Type::Float { width: left_width }, Type::Float { width: right_width }) => {
                left_width == right_width
            }
            (
                Type::Reference {
                    kind: left_kind,
                    address_space: left_space,
                    mutability: left_mutability,
                    pointee: left_pointee,
                    is_nullable: left_nullable,
                },
                Type::Reference {
                    kind: right_kind,
                    address_space: right_space,
                    mutability: right_mutability,
                    pointee: right_pointee,
                    is_nullable: right_nullable,
                },
            ) => {
                left_kind == right_kind
                    && left_space == right_space
                    && left_mutability == right_mutability
                    && left_nullable == right_nullable
                    && self.types_equivalent_inner(*left_pointee, *right_pointee, seen_pairs)
            }
            (
                Type::Array {
                    element: left_element,
                    length: left_length,
                    copyability: left_copyability,
                },
                Type::Array {
                    element: right_element,
                    length: right_length,
                    copyability: right_copyability,
                },
            ) => {
                left_length == right_length
                    && left_copyability == right_copyability
                    && self.types_equivalent_inner(*left_element, *right_element, seen_pairs)
            }
            (
                Type::Tuple {
                    elements: left_elements,
                    copyability: left_copyability,
                },
                Type::Tuple {
                    elements: right_elements,
                    copyability: right_copyability,
                },
            ) => {
                left_copyability == right_copyability
                    && left_elements.len() == right_elements.len()
                    && left_elements.iter().zip(right_elements).all(
                        |(left_element, right_element)| {
                            self.types_equivalent_inner(*left_element, *right_element, seen_pairs)
                        },
                    )
            }
            (
                Type::Struct {
                    fields: left_fields,
                    copyability: left_copyability,
                },
                Type::Struct {
                    fields: right_fields,
                    copyability: right_copyability,
                },
            ) => {
                left_copyability == right_copyability
                    && left_fields.len() == right_fields.len()
                    && left_fields
                        .iter()
                        .zip(right_fields)
                        .all(|(left_field, right_field)| {
                            let left_field = self.tree.get(*left_field);
                            let right_field = self.tree.get(*right_field);

                            left_field.name == right_field.name
                                && self.types_equivalent_inner(
                                    left_field.ty,
                                    right_field.ty,
                                    seen_pairs,
                                )
                        })
            }
            (
                Type::Newtype {
                    inner: left_inner,
                    copyability: left_copyability,
                },
                Type::Newtype {
                    inner: right_inner,
                    copyability: right_copyability,
                },
            ) => {
                left_copyability == right_copyability
                    && self.types_equivalent_inner(*left_inner, *right_inner, seen_pairs)
            }
            (
                Type::Vector {
                    element: left_element,
                    lanes: left_lanes,
                    copyability: left_copyability,
                },
                Type::Vector {
                    element: right_element,
                    lanes: right_lanes,
                    copyability: right_copyability,
                },
            ) => {
                left_lanes == right_lanes
                    && left_copyability == right_copyability
                    && self.types_equivalent_inner(*left_element, *right_element, seen_pairs)
            }
            (
                Type::Tensor {
                    element: left_element,
                    shape: left_shape,
                    layout: left_layout,
                    copyability: left_copyability,
                },
                Type::Tensor {
                    element: right_element,
                    shape: right_shape,
                    layout: right_layout,
                    copyability: right_copyability,
                },
            ) => {
                left_shape == right_shape
                    && left_layout == right_layout
                    && left_copyability == right_copyability
                    && self.types_equivalent_inner(*left_element, *right_element, seen_pairs)
            }
            (
                Type::TensorReference {
                    kind: left_kind,
                    address_space: left_space,
                    mutability: left_mutability,
                    element: left_element,
                    shape: left_shape,
                    layout: left_layout,
                    is_nullable: left_nullable,
                },
                Type::TensorReference {
                    kind: right_kind,
                    address_space: right_space,
                    mutability: right_mutability,
                    element: right_element,
                    shape: right_shape,
                    layout: right_layout,
                    is_nullable: right_nullable,
                },
            ) => {
                left_kind == right_kind
                    && left_space == right_space
                    && left_mutability == right_mutability
                    && left_shape == right_shape
                    && left_layout == right_layout
                    && left_nullable == right_nullable
                    && self.types_equivalent_inner(*left_element, *right_element, seen_pairs)
            }
            (
                Type::FunctionPointer {
                    parameters: left_parameters,
                    result: left_result,
                },
                Type::FunctionPointer {
                    parameters: right_parameters,
                    result: right_result,
                },
            ) => {
                left_parameters.len() == right_parameters.len()
                    && left_parameters.iter().zip(right_parameters).all(
                        |(left_parameter, right_parameter)| {
                            self.types_equivalent_inner(
                                *left_parameter,
                                *right_parameter,
                                seen_pairs,
                            )
                        },
                    )
                    && self.types_equivalent_inner(*left_result, *right_result, seen_pairs)
            }
            (
                Type::FunctionValue {
                    signature: left_signature,
                    environment: left_environment,
                },
                Type::FunctionValue {
                    signature: right_signature,
                    environment: right_environment,
                },
            ) => {
                self.types_equivalent_inner(*left_signature, *right_signature, seen_pairs)
                    && self.types_equivalent_inner(
                        *left_environment,
                        *right_environment,
                        seen_pairs,
                    )
            }
            _ => false,
        }
    }

    /// Validate cast operator legality for canonical storage representations.
    fn validate_cast_legality(
        &self,
        operator: CastOperator,
        argument_type: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let argument_storage_type = self.storage_type_id(argument_type);
        let result_storage_type = self.storage_type_id(result_type);
        let argument = self.tree.get(argument_storage_type);
        let result = self.tree.get(result_storage_type);
        let argument_integer_width = self.integer_bit_width(argument);
        let result_integer_width = self.integer_bit_width(result);
        let argument_is_integer = argument_integer_width.is_some();
        let result_is_integer = result_integer_width.is_some();
        let argument_float_width = self.float_bit_width(argument);
        let result_float_width = self.float_bit_width(result);
        let argument_is_float = matches!(argument, Type::Float { .. });
        let result_is_float = matches!(result, Type::Float { .. });
        let argument_is_pointer = self.pointer_bit_width(argument).is_some();
        let result_is_pointer = self.pointer_bit_width(result).is_some();
        let argument_is_raw_pointer = self.is_raw_pointer_like(argument);

        match operator {
            CastOperator::PointerToInt => {
                if !argument_is_raw_pointer || !result_is_integer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "ptr_to_int requires raw pointer source and integer destination"
                            .to_string(),
                        anchor,
                    });
                }

                if let Some(result_width) = result_integer_width
                    && result_width < self.tree.pointer_bits()
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "ptr_to_int destination must be at least pointer width"
                            .to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::IntToPointer => {
                if !argument_is_integer || !result_is_pointer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "int_to_ptr requires integer source and pointer destination"
                            .to_string(),
                        anchor,
                    });
                }

                if let Some(argument_width) = argument_integer_width
                    && argument_width < self.tree.pointer_bits()
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "int_to_ptr source must be at least pointer width".to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::Bitcast => {
                if argument_is_pointer != result_is_pointer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "bitcast cannot cast between pointer and non pointer categories"
                            .to_string(),
                        anchor,
                    });
                }

                let argument_layout = compute_type_layout(
                    self.tree,
                    argument_storage_type,
                    self.tree.pointer_bytes(),
                );
                let result_layout =
                    compute_type_layout(self.tree, result_storage_type, self.tree.pointer_bytes());
                if argument_layout.size != result_layout.size {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "bitcast requires equal storage size, got {} and {} bytes",
                            argument_layout.size, result_layout.size
                        ),
                        anchor,
                    });
                }
            }
            CastOperator::Truncate => {
                if !argument_is_integer || !result_is_integer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "integer cast requires integer source and destination".to_string(),
                        anchor,
                    });
                }

                if let (Some(argument_width), Some(result_width)) =
                    (argument_integer_width, result_integer_width)
                    && result_width > argument_width
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "trunc requires destination integer no wider than source"
                            .to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::ZeroExtend | CastOperator::SignExtend => {
                if !argument_is_integer || !result_is_integer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "integer cast requires integer source and destination".to_string(),
                        anchor,
                    });
                }

                if let (Some(argument_width), Some(result_width)) =
                    (argument_integer_width, result_integer_width)
                    && result_width < argument_width
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "integer extend requires destination no narrower than source"
                            .to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::FloatToSignedInt
            | CastOperator::FloatToUnsignedInt
            | CastOperator::FloatToSignedIntSaturating
            | CastOperator::FloatToUnsignedIntSaturating => {
                if !argument_is_float || !result_is_integer {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "float to int cast requires float source and integer destination"
                            .to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::SignedIntToFloat | CastOperator::UnsignedIntToFloat => {
                if !argument_is_integer || !result_is_float {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "int to float cast requires integer source and float destination"
                            .to_string(),
                        anchor,
                    });
                }
            }
            CastOperator::FloatTruncate | CastOperator::FloatExtend => {
                if !argument_is_float || !result_is_float {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "float cast requires float source and destination".to_string(),
                        anchor,
                    });
                }

                if let (Some(argument_width), Some(result_width)) =
                    (argument_float_width, result_float_width)
                {
                    if matches!(operator, CastOperator::FloatTruncate)
                        && result_width > argument_width
                    {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "fnarrow requires destination float no wider than source"
                                .to_string(),
                            anchor,
                        });
                    }
                    if matches!(operator, CastOperator::FloatExtend)
                        && result_width < argument_width
                    {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "fwiden requires destination float no narrower than source"
                                .to_string(),
                            anchor,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Resolve the canonical storage type by unwrapping transparent newtype wrappers.
    fn storage_type_id(&self, mut type_id: LocalNodeId<Type>) -> LocalNodeId<Type> {
        loop {
            match self.tree.get(type_id) {
                Type::Newtype { inner, .. } => {
                    type_id = *inner;
                }
                _ => return type_id,
            }
        }
    }

    /// Return integer bit width for integer-like MIR types.
    fn integer_bit_width(&self, ty: &Type) -> Option<u16> {
        match ty {
            Type::Int { width, .. } => Some(*width),
            Type::Isize | Type::Usize => Some(self.tree.pointer_bits()),
            _ => None,
        }
    }

    /// Return float bit width for float MIR types.
    fn float_bit_width(&self, ty: &Type) -> Option<u16> {
        match ty {
            Type::Float { width } => Some(*width),
            _ => None,
        }
    }

    /// Return pointer bit width for pointer-like MIR types.
    fn pointer_bit_width(&self, ty: &Type) -> Option<u16> {
        match ty {
            Type::Reference { .. }
            | Type::TensorReference { .. }
            | Type::FunctionPointer { .. } => Some(self.tree.pointer_bits()),
            _ => None,
        }
    }

    /// Return whether a type is raw-pointer-like for integer and pointer casts.
    fn is_raw_pointer_like(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Reference {
                kind: ReferenceKind::Raw,
                ..
            } | Type::TensorReference {
                kind: ReferenceKind::Raw,
                ..
            } | Type::FunctionPointer { .. }
        )
    }

    /// Validate a reference result type.
    fn validate_reference_result_type(
        &self,
        result_type: LocalNodeId<Type>,
        expected_pointee: Option<LocalNodeId<Type>>,
        expected_kind: Option<ReferenceKind>,
        expected_mutability: Option<Mutability>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let Type::Reference {
            kind,
            mutability,
            pointee,
            ..
        } = self.tree.get(result_type)
        else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type is not a reference".to_string(),
                anchor,
            });
        };

        if let Some(expected) = expected_pointee
            && !self.types_equivalent(*pointee, expected)
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type mismatches pointee".to_string(),
                anchor,
            });
        }

        if let Some(expected) = expected_kind
            && *kind != expected
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type has wrong reference kind"
                    .to_string(),
                anchor,
            });
        }

        if let Some(expected) = expected_mutability
            && *mutability != expected
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type has wrong mutability"
                    .to_string(),
                anchor,
            });
        }

        Ok(())
    }
}
