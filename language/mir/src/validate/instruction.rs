use std::collections::HashSet;

use crate::{
    AddressSpace, ArgumentSlice, Constant, Function, Instruction, Local, LocalNodeId, Mutability,
    NodeType, ReferenceKind, TensorDimension, Type, Value,
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
                    signature,
                    effects,
                    declared_target,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    if let Some(target) = declared_target {
                        self.validate_call_signature_matches_function(
                            ValidateAnchor::node(instruction_id),
                            *signature,
                            *target,
                        )?;
                    }
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                Instruction::CallInterface {
                    destination,
                    signature,
                    effects,
                    declared_target,
                    ..
                } => {
                    self.validate_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    if let Some(target) = declared_target {
                        self.validate_call_signature_matches_function(
                            ValidateAnchor::node(instruction_id),
                            *signature,
                            *target,
                        )?;
                    }
                    self.validate_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
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

        Ok(())
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
        // skip node type checks when disabled
        if !self.options.validate_node_types {
            return Ok(());
        }

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
        // skip argument slice checks when disabled
        if !self.options.validate_argument_slices {
            return Ok(());
        }

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
        // skip local reference checks when disabled
        if !self.options.validate_local_references {
            return Ok(());
        }

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
        // skip node checks when disabled
        if !self.options.validate_node_types {
            return Ok(());
        }

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
            Instruction::CallVirtual {
                declared_target, ..
            }
            | Instruction::CallInterface {
                declared_target, ..
            } => {
                // ensure declared targets resolve
                if let Some(target) = declared_target {
                    self.ensure_node_type(NodeType::Function, target.id, anchor)?;
                }
            }
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
        // skip type shape checks when disabled
        if !self.options.validate_type_shapes {
            return Ok(());
        }

        // prepare anchor for shape validation
        let anchor = ValidateAnchor::node(instruction_id);

        // validate aggregate constructor shapes
        match instruction {
            Instruction::Struct { ty, fields, .. } => {
                // resolve struct layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Struct { fields, .. } => fields.len(),
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "struct",
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
                let (kind, address_space, is_nullable) = match destination_type {
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

                if !matches!(address_space, AddressSpace::Generic | AddressSpace::Heap) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "null constant requires a generic or heap address space"
                            .to_string(),
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
                self.validate_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Raw),
                    None,
                    anchor,
                )?;
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
            Instruction::FieldAddr { result_type, .. }
            | Instruction::ElementAddr { result_type, .. } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(*result_type, None, None, None, anchor)?;
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
            && *pointee != expected
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
