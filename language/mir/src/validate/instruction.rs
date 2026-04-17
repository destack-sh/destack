use std::collections::HashSet;

use crate::{
    AddressSpace, AllocationMode, ArgumentSlice, CastOperator, Constant, Function, Instruction,
    Intrinsic, Local, LocalNodeId, Mutability, NodeType, ReferenceKind, TensorDimension,
    TensorLayout, Type, TypeReference, Value, ValueReference, compute_type_layout,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate one instruction against function-local invariants.
    pub(super) fn validate_instruction(
        &self,
        function: &Function,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        locals: &HashSet<LocalNodeId<Local>>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);

        // recovered syntax
        if matches!(instruction, Instruction::Error) {
            return Err(ValidateError::RecoveredSyntaxNode {
                kind: "instruction",
                anchor,
            });
        }

        // uses
        self.validate_instruction_uses(instruction, anchor, defined_values)?;

        // call metadata
        self.validate_instruction_call(function, instruction_id, instruction)?;

        // operation rules
        self.validate_instruction_operation(function, instruction, instruction_id, locals)?;

        // allocation policy
        self.validate_instruction_allocation(function, instruction, instruction_id)?;

        Ok(())
    }

    /// Validate the SSA uses embedded in one instruction.
    fn validate_instruction_uses(
        &self,
        instruction: &Instruction,
        anchor: ValidateAnchor,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        for value in instruction.uses() {
            let value = self.require_value_reference(value, anchor, "instruction operand")?;
            self.ensure_defined(value, anchor, defined_values)?;
        }

        Ok(())
    }

    /// Validate call-site metadata attached to one instruction.
    fn validate_instruction_call(
        &self,
        function: &Function,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);

        // shared call metadata
        let Some((destination, call)) = (match instruction {
            Instruction::Call {
                destination, call, ..
            }
            | Instruction::CallVirtual {
                destination, call, ..
            }
            | Instruction::CallInterface {
                destination, call, ..
            }
            | Instruction::CallIndirect {
                destination, call, ..
            } => Some((*destination, call)),
            _ => None,
        }) else {
            return Ok(());
        };

        self.validate_argument_slice(call.arguments, instruction_id)?;
        let arguments = self.tree.get_arguments(call.arguments);

        self.validate_call_signature(instruction_id, destination, arguments.len(), call.signature)?;
        self.validate_call_metadata(
            instruction_id,
            call.memory_effect.as_ref(),
            call.behavior.as_ref(),
            call.argument_attributes.as_slice(),
            &call.return_attribute,
            arguments.len(),
        )?;

        // per-call-kind checks
        match instruction {
            Instruction::Call {
                function: callee, ..
            } => {
                let callee = self.require_function_reference(*callee, anchor, "call callee")?;
                self.validate_call_signature_matches_function(anchor, call.signature, callee)?;
                self.validate_direct_call_environment(anchor, callee)?;
            }
            Instruction::CallVirtual {
                declaring_type,
                slot_id,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "virtual call type")?;
                self.validate_virtual_dispatch_slot(declaring_type, *slot_id, anchor)?;
            }
            Instruction::CallInterface {
                declaring_type,
                slot_id,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "interface call type")?;
                self.validate_interface_dispatch_slot(declaring_type, *slot_id, anchor)?;
            }
            Instruction::CallIndirect { callee, .. } => {
                self.validate_indirect_callee_signature(
                    function,
                    *callee,
                    call.signature,
                    anchor,
                    "call.indirect callee",
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate function allocation policy against allocation instructions.
    fn validate_instruction_allocation(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> ValidateResult<()> {
        // classify allocation instructions
        let allocation_instruction = match instruction {
            Instruction::ManagedAlloc { .. } => Some("managed.alloc"),
            Instruction::ManagedAllocArray { .. } => Some("managed.allocArray"),
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
            .then_some("noManaged forbids managed allocations"),
            AllocationMode::StackOnly => matches!(
                instruction,
                Instruction::ManagedAlloc { .. }
                    | Instruction::ManagedAllocArray { .. }
                    | Instruction::RawAlloc { .. }
            )
            .then_some("stackOnly forbids non-stack allocations"),
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
        value: ValueReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        let value = self.require_value_reference(value, anchor, label)?;

        // look up the value type
        let value_type = function.value_type(value).ok_or_else(|| {
            ValidateError::MetadataInvariantViolation {
                message: format!("missing value type for {label}"),
                anchor,
            }
        })?;

        Ok(value_type)
    }

    /// Validate one external argument slice.
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

    /// Resolve one vector type.
    fn vector_type(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<(LocalNodeId<Type>, u32)> {
        let vector_type = self.tree.get(type_id);

        match vector_type {
            Type::Vector { element, lanes, .. } => {
                let Some(element) = self.concrete_type_reference(*element) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!("{message}, found vector with non concrete element type"),
                        anchor,
                    });
                };

                Ok((element, *lanes))
            }
            other => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{message}, found {}", self.type_kind(other)),
                anchor,
            }),
        }
    }

    /// Resolve one tensor type.
    fn tensor_type(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<(LocalNodeId<Type>, &[TensorDimension], &TensorLayout)> {
        let tensor_type = self.tree.get(type_id);

        match tensor_type {
            Type::Tensor {
                element,
                shape,
                layout,
                ..
            } => {
                let Some(element) = self.concrete_type_reference(*element) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!("{message}, found tensor with non concrete element type"),
                        anchor,
                    });
                };

                Ok((element, shape, layout))
            }
            other => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{message}, found {}", self.type_kind(other)),
                anchor,
            }),
        }
    }

    /// Resolve one tensor reference type.
    fn tensor_reference_type(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<(
        ReferenceKind,
        AddressSpace,
        Mutability,
        LocalNodeId<Type>,
        &[TensorDimension],
        &TensorLayout,
    )> {
        let tensor_type = self.tree.get(type_id);

        match tensor_type {
            Type::TensorReference {
                kind,
                address_space,
                mutability,
                element,
                shape,
                layout,
                ..
            } => {
                let Some(element) = self.concrete_type_reference(*element) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{message}, found tensor reference with non concrete element type"
                        ),
                        anchor,
                    });
                };

                Ok((*kind, *address_space, *mutability, element, shape, layout))
            }
            other => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{message}, found {}", self.type_kind(other)),
                anchor,
            }),
        }
    }

    /// Resolve one reference type.
    fn reference_type(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<(ReferenceKind, Mutability, LocalNodeId<Type>, bool)> {
        let reference_type = self.tree.get(type_id);

        match reference_type {
            Type::Reference {
                kind,
                mutability,
                pointee,
                is_nullable,
                ..
            } => {
                let Some(pointee) = self.concrete_type_reference(*pointee) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{message}, found reference with non concrete pointee type"
                        ),
                        anchor,
                    });
                };

                Ok((*kind, *mutability, pointee, *is_nullable))
            }
            _ => Err(ValidateError::MetadataInvariantViolation {
                message: message.to_string(),
                anchor,
            }),
        }
    }

    /// Resolve one reference pointee type.
    fn reference_pointee_type_or_error(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        let (_kind, _mutability, pointee, _is_nullable) =
            self.reference_type(type_id, anchor, message)?;

        Ok(pointee)
    }

    /// Ensure one type is integer-like.
    fn expect_integer_like_type(
        &self,
        type_id: LocalNodeId<Type>,
        anchor: ValidateAnchor,
        message: &'static str,
    ) -> ValidateResult<()> {
        if !matches!(
            self.tree.get(type_id),
            Type::Int { .. } | Type::Isize | Type::Usize
        ) {
            return Err(ValidateError::MetadataInvariantViolation {
                message: message.to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate one instruction operation.
    fn validate_instruction_operation(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
        locals: &HashSet<LocalNodeId<Local>>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);

        // embedded node references
        match instruction {
            Instruction::LocalGet { local, .. } | Instruction::LocalSet { local, .. } => {
                let local = self.require_local_reference(*local, anchor, "local reference")?;
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
            }
            Instruction::LocalAddr {
                local, result_type, ..
            } => {
                let local = self.require_local_reference(*local, anchor, "local reference")?;
                let result_type =
                    self.require_type_reference(*result_type, anchor, "local address type")?;
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
            }
            Instruction::GlobalAddr { global, .. } | Instruction::GlobalConst { global, .. } => {
                let global = self.require_global_reference(*global, anchor, "global reference")?;
                self.ensure_node_type(NodeType::Global, global.id, anchor)?;
            }
            Instruction::FunctionAddr { function, .. } | Instruction::Call { function, .. } => {
                let function =
                    self.require_function_reference(*function, anchor, "function reference")?;
                self.ensure_node_type(NodeType::Function, function.id, anchor)?;
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
                let to_type =
                    self.require_type_reference(*to_type, anchor, "instruction type reference")?;
                self.ensure_node_type(NodeType::Type, to_type.id, anchor)?;
            }
            _ => {}
        }

        // local ownership
        match instruction {
            Instruction::LocalGet { local, .. }
            | Instruction::LocalAddr { local, .. }
            | Instruction::LocalSet { local, .. } => {
                let local = self.require_local_reference(*local, anchor, "local reference")?;
                if !locals.contains(&local) {
                    return Err(ValidateError::LocalReferenceNotInFunction {
                        local_id: local,
                        anchor,
                    });
                }
            }
            _ => {}
        }

        // aggregate form
        match instruction {
            Instruction::Struct { ty, fields, .. } => {
                let ty = self.require_type_reference(*ty, anchor, "struct type")?;
                let expected = match self.tree.get(ty) {
                    Type::Struct { fields, .. } => fields.len(),
                    Type::Closure { .. } => 2,
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "struct or callable",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };

                if expected != fields.len() {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got: fields.len(),
                        anchor,
                    });
                }
            }
            Instruction::Tuple { ty, elements, .. } => {
                let ty = self.require_type_reference(*ty, anchor, "tuple type")?;
                let expected = match self.tree.get(ty) {
                    Type::Tuple { elements, .. } => elements.len(),
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "tuple",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };

                if expected != elements.len() {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got: elements.len(),
                        anchor,
                    });
                }
            }
            Instruction::Array { ty, elements, .. } => {
                let ty = self.require_type_reference(*ty, anchor, "array type")?;
                let expected = match self.tree.get(ty) {
                    Type::Array { length, .. } => usize::try_from(*length).unwrap_or(usize::MAX),
                    other => {
                        return Err(ValidateError::AggregateTypeMismatch {
                            expected: "array",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };

                if expected != elements.len() {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got: elements.len(),
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
            }
            | Instruction::TensorView {
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                let expected =
                    *offsets_count as usize + *sizes_count as usize + *strides_count as usize;
                if expected != arguments.len() {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got: arguments.len(),
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
                let expected =
                    *low_count as usize + *high_count as usize + *interior_count as usize;
                if expected != arguments.len() {
                    return Err(ValidateError::AggregateArgumentCountMismatch {
                        expected,
                        got: arguments.len(),
                        anchor,
                    });
                }
            }
            _ => {}
        }

        // typed operations
        match instruction {
            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "binary left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "binary right")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "binary result")?;

                match self.tree.get(left_type_id) {
                    Type::Vector { .. } => {
                        let left_vector = self.vector_type(
                            left_type_id,
                            anchor,
                            "vector binary expects vector operands",
                        )?;
                        let right_vector = self.vector_type(
                            right_type_id,
                            anchor,
                            "vector binary expects vector operands",
                        )?;
                        let destination_vector = self.vector_type(
                            destination_type_id,
                            anchor,
                            "vector binary result must be a vector",
                        )?;

                        if left_vector.0 != right_vector.0
                            || left_vector.1 != right_vector.1
                            || left_vector.0 != destination_vector.0
                            || left_vector.1 != destination_vector.1
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
                    Type::Tensor { .. } => {
                        let left_tensor = self.tensor_type(
                            left_type_id,
                            anchor,
                            "tensor binary expects tensor operands",
                        )?;
                        let right_tensor = self.tensor_type(
                            right_type_id,
                            anchor,
                            "tensor binary expects tensor operands",
                        )?;
                        let destination_tensor = self.tensor_type(
                            destination_type_id,
                            anchor,
                            "tensor binary result must be a tensor",
                        )?;

                        if left_tensor.0 != right_tensor.0
                            || left_tensor.1 != right_tensor.1
                            || left_tensor.2 != right_tensor.2
                            || left_tensor.0 != destination_tensor.0
                            || left_tensor.1 != destination_tensor.1
                            || left_tensor.2 != destination_tensor.2
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
                if !operator.is_comparison() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "vector.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "vector.compare right")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.compare result",
                )?;

                let left_vector = self.vector_type(
                    left_type_id,
                    anchor,
                    "vector.compare expects vector operands",
                )?;
                let right_vector = self.vector_type(
                    right_type_id,
                    anchor,
                    "vector.compare expects vector operands",
                )?;
                let result_vector = self.vector_type(
                    destination_type_id,
                    anchor,
                    "vector.compare result must be a vector",
                )?;

                if left_vector.0 != right_vector.0 || left_vector.1 != right_vector.1 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare expects matching vector operand types".to_string(),
                        anchor,
                    });
                }

                if result_vector.1 != left_vector.1 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare result must match lane count".to_string(),
                        anchor,
                    });
                }

                if !matches!(self.tree.get(result_vector.0), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.compare result element type must be boolean".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::VectorConvert {
                destination,
                vector,
                ..
            } => {
                let source_type_id =
                    self.value_type_or_error(function, *vector, anchor, "vector.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.convert result",
                )?;

                let source_vector = self.vector_type(
                    source_type_id,
                    anchor,
                    "vector.convert expects vector source",
                )?;
                let destination_vector = self.vector_type(
                    destination_type_id,
                    anchor,
                    "vector.convert expects vector result",
                )?;

                if source_vector.1 != destination_vector.1 {
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
                if !operator.is_comparison() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "tensor.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "tensor.compare right")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.compare result",
                )?;

                let left_tensor = self.tensor_type(
                    left_type_id,
                    anchor,
                    "tensor.compare expects tensor operands",
                )?;
                let right_tensor = self.tensor_type(
                    right_type_id,
                    anchor,
                    "tensor.compare expects tensor operands",
                )?;
                let result_tensor = self.tensor_type(
                    destination_type_id,
                    anchor,
                    "tensor.compare result must be a tensor",
                )?;

                if left_tensor.0 != right_tensor.0
                    || left_tensor.1 != right_tensor.1
                    || left_tensor.2 != right_tensor.2
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare expects matching tensor operand types".to_string(),
                        anchor,
                    });
                }

                if result_tensor.1 != left_tensor.1 || result_tensor.2 != left_tensor.2 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare result must match tensor shape and layout"
                            .to_string(),
                        anchor,
                    });
                }

                if !matches!(self.tree.get(result_tensor.0), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.compare result element type must be boolean".to_string(),
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

                let mask_vector =
                    self.vector_type(mask_type_id, anchor, "vector.select mask must be a vector")?;
                let then_vector = self.vector_type(
                    then_type_id,
                    anchor,
                    "vector.select expects vector operands",
                )?;
                let else_vector = self.vector_type(
                    else_type_id,
                    anchor,
                    "vector.select expects vector operands",
                )?;
                let destination_vector = self.vector_type(
                    destination_type_id,
                    anchor,
                    "vector.select result must be a vector",
                )?;

                if !matches!(self.tree.get(mask_vector.0), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "vector.select mask element type must be boolean".to_string(),
                        anchor,
                    });
                }

                if then_vector.0 != else_vector.0
                    || then_vector.1 != else_vector.1
                    || then_vector.0 != destination_vector.0
                    || then_vector.1 != destination_vector.1
                    || then_vector.1 != mask_vector.1
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

                let mask_tensor =
                    self.tensor_type(mask_type_id, anchor, "tensor.select mask must be a tensor")?;
                let then_tensor = self.tensor_type(
                    then_type_id,
                    anchor,
                    "tensor.select expects tensor operands",
                )?;
                let else_tensor = self.tensor_type(
                    else_type_id,
                    anchor,
                    "tensor.select expects tensor operands",
                )?;
                let destination_tensor = self.tensor_type(
                    destination_type_id,
                    anchor,
                    "tensor.select result must be a tensor",
                )?;

                if !matches!(self.tree.get(mask_tensor.0), Type::Boolean) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.select mask element type must be boolean".to_string(),
                        anchor,
                    });
                }

                if then_tensor.0 != else_tensor.0
                    || then_tensor.1 != else_tensor.1
                    || then_tensor.2 != else_tensor.2
                    || then_tensor.0 != destination_tensor.0
                    || then_tensor.1 != destination_tensor.1
                    || then_tensor.2 != destination_tensor.2
                    || then_tensor.1 != mask_tensor.1
                    || then_tensor.2 != mask_tensor.2
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
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.convert result",
                )?;

                let source_tensor = self.tensor_type(
                    source_type_id,
                    anchor,
                    "tensor.convert expects tensor source",
                )?;
                let destination_tensor = self.tensor_type(
                    destination_type_id,
                    anchor,
                    "tensor.convert expects tensor result",
                )?;

                if source_tensor.1 != destination_tensor.1
                    || source_tensor.2 != destination_tensor.2
                {
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
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.cast source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.cast result")?;

                let source_tensor =
                    self.tensor_type(source_type_id, anchor, "tensor.cast expects tensor source")?;
                let destination_tensor = self.tensor_type(
                    destination_type_id,
                    anchor,
                    "tensor.cast expects tensor result",
                )?;

                if source_tensor.0 != destination_tensor.0
                    || source_tensor.2 != destination_tensor.2
                    || !self.tensor_shapes_compatible(source_tensor.1, destination_tensor.1)
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
                let source_type_id =
                    self.value_type_or_error(function, *view, anchor, "tensor.view source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.view result")?;

                let source_view = self.tensor_reference_type(
                    source_type_id,
                    anchor,
                    "tensor.view expects tensor reference source",
                )?;
                let destination_view = self.tensor_reference_type(
                    destination_type_id,
                    anchor,
                    "tensor.view expects tensor reference result",
                )?;

                if source_view.0 != destination_view.0
                    || source_view.1 != destination_view.1
                    || source_view.3 != destination_view.3
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view requires matching reference kind, address space, and element type"
                            .to_string(),
                        anchor,
                    });
                }

                if matches!(source_view.2, Mutability::Immutable)
                    && matches!(destination_view.2, Mutability::Mutable)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view cannot increase mutability".to_string(),
                        anchor,
                    });
                }

                if source_view.4.len() != destination_view.4.len() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "tensor.view requires matching ranks".to_string(),
                        anchor,
                    });
                }

                let expected = destination_view.4.len();
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
            Instruction::Const { destination, value } => {
                let Constant::Null = value else {
                    return Ok(());
                };

                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "null constant")?;
                let reference_type = self.reference_type(
                    destination_type_id,
                    anchor,
                    "null constant requires a reference type",
                )?;
                let (kind, _mutability, _pointee, is_nullable) = reference_type;

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
                let global = self.require_global_reference(*global, anchor, "global reference")?;
                let result_type =
                    self.require_type_reference(*result_type, anchor, "global address type")?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                let global_decl = self.tree.get(global);
                let global_type =
                    self.require_type_reference(global_decl.ty, anchor, "global type")?;
                self.validate_reference_result_type(
                    result_type,
                    Some(global_type),
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
                    "function.address result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let Type::FunctionPointer { parameters, result } = destination_type else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.address result must be a function pointer".to_string(),
                        anchor,
                    });
                };

                let target =
                    self.require_function_reference(*target, anchor, "function address")?;
                let target_function = self.tree.get(target);
                if target_function.environment.is_some() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.address cannot target a function with an environment"
                            .to_string(),
                        anchor,
                    });
                }

                if target_function.parameters.len() != parameters.len() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.address result parameter count mismatch".to_string(),
                        anchor,
                    });
                }

                for (parameter, signature_type) in
                    target_function.parameters.iter().zip(parameters.iter())
                {
                    if parameter.ty != *signature_type {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function.address result parameter types mismatch".to_string(),
                            anchor,
                        });
                    }
                }

                if target_function.return_type != *result {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.address result return type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::FunctionBind {
                destination,
                function: target,
                environment,
            } => {
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "function.bind result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let Type::Closure { signature, .. } = destination_type else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.bind result must be a callable value".to_string(),
                        anchor,
                    });
                };
                let signature =
                    self.require_type_reference(*signature, anchor, "function bind signature")?;
                let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.bind signature must be a function pointer".to_string(),
                        anchor,
                    });
                };

                let target =
                    self.require_function_reference(*target, anchor, "function bind target")?;
                let target_function = self.tree.get(target);
                let target_environment = target_function.environment.ok_or_else(|| {
                    ValidateError::MetadataInvariantViolation {
                        message: "function.bind target requires an environment".to_string(),
                        anchor,
                    }
                })?;
                let target_environment = self.require_type_reference(
                    target_environment,
                    anchor,
                    "function bind environment type",
                )?;

                let actual_environment_type = self.value_type_or_error(
                    function,
                    *environment,
                    anchor,
                    "function.bind environment",
                )?;
                if !self.types_equivalent(actual_environment_type, target_environment) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.bind environment type mismatch".to_string(),
                        anchor,
                    });
                }

                if target_function.parameters.len() != parameters.len() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.bind result parameter count mismatch".to_string(),
                        anchor,
                    });
                }

                for (parameter, signature_type) in
                    target_function.parameters.iter().zip(parameters.iter())
                {
                    if parameter.ty != *signature_type {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function.bind result parameter types mismatch".to_string(),
                            anchor,
                        });
                    }
                }

                if target_function.return_type != *result {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.bind result return type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::FunctionEnvironment { destination } => {
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "function.environment result",
                )?;
                self.reference_type(
                    destination_type_id,
                    anchor,
                    "function.environment result must be a reference type",
                )?;

                let environment = function.environment.ok_or_else(|| {
                    ValidateError::MetadataInvariantViolation {
                        message: "function.environment requires an environment".to_string(),
                        anchor,
                    }
                })?;
                if environment != destination_type_id.into() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "function.environment type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::ManagedAlloc {
                layout,
                result_type,
                ..
            } => {
                let layout =
                    self.require_type_reference(*layout, anchor, "managed.alloc layout")?;
                let result_type =
                    self.require_type_reference(*result_type, anchor, "managed.alloc result type")?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    result_type,
                    Some(layout),
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
                let element = self.require_type_reference(
                    *element,
                    anchor,
                    "managed.allocArray element type",
                )?;
                let result_type = self.require_type_reference(
                    *result_type,
                    anchor,
                    "managed.allocArray result type",
                )?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    result_type,
                    Some(element),
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
                let layout = self.require_type_reference(*layout, anchor, "raw.alloc layout")?;
                let result_type =
                    self.require_type_reference(*result_type, anchor, "raw.alloc result type")?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                let reference_type = self.reference_type(
                    result_type,
                    anchor,
                    "pointer-producing instruction result type is not a reference",
                )?;
                let (kind, _mutability, pointee, _is_nullable) = reference_type;

                if pointee != layout {
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
                let layout = self.require_type_reference(*layout, anchor, "stack.alloc layout")?;
                let result_type =
                    self.require_type_reference(*result_type, anchor, "stack.alloc result type")?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.validate_reference_result_type(
                    result_type,
                    Some(layout),
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

                let pointee_type = self.reference_pointee_type_or_error(
                    pointer_type,
                    anchor,
                    "atomic.load pointer must be a reference type",
                )?;

                let result_type =
                    self.require_type_reference(*result_type, anchor, "atomic.load result type")?;

                if !self.types_equivalent(pointee_type, result_type)
                    || !self.types_equivalent(destination_type, result_type)
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

                let pointee_type = self.reference_pointee_type_or_error(
                    pointer_type,
                    anchor,
                    "atomic.store pointer must be a reference type",
                )?;

                if !self.is_store_compatible_type(value_type, pointee_type) {
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

                let pointee_type = self.reference_pointee_type_or_error(
                    pointer_type,
                    anchor,
                    "atomic.compare_exchange pointer must be a reference type",
                )?;

                if !self.types_equivalent(expected_type, pointee_type)
                    || !self.types_equivalent(new_value_type, pointee_type)
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
                            "atomic.compare_exchange result must be a tuple of (old_value, boolean)"
                                .to_string(),
                        anchor,
                    });
                };

                let Some(first) = self.concrete_type_reference(elements[0]) else {
                    return Err(self.metadata_error(
                        anchor,
                        "atomic.compare_exchange result tuple element type is not concrete",
                    ));
                };
                let Some(second) = self.concrete_type_reference(elements[1]) else {
                    return Err(self.metadata_error(
                        anchor,
                        "atomic.compare_exchange result tuple element type is not concrete",
                    ));
                };

                if elements.len() != 2
                    || !self.types_equivalent(first, pointee_type)
                    || !matches!(self.tree.get(second), Type::Boolean)
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "atomic.compare_exchange result must be a tuple of (old_value, boolean)"
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

                let pointee_type = self.reference_pointee_type_or_error(
                    pointer_type,
                    anchor,
                    "atomic.rmw pointer must be a reference type",
                )?;

                if !self.types_equivalent(value_type, pointee_type)
                    || !self.types_equivalent(destination_type, pointee_type)
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

                let field_type = self.field_type_for_projection_target(
                    aggregate_type,
                    *index as usize,
                    anchor,
                    "field.get",
                )?;
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

                let field_type = self.field_type_for_projection_target(
                    aggregate_type,
                    *index as usize,
                    anchor,
                    "field.set",
                )?;
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
                let result_type =
                    self.require_type_reference(*result_type, anchor, "field.address result type")?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;

                let aggregate_type = self.value_type_or_error(
                    function,
                    *aggregate,
                    anchor,
                    "field.address aggregate",
                )?;
                self.field_type_for_projection_target(
                    aggregate_type,
                    *index as usize,
                    anchor,
                    "field.address",
                )?;
                self.validate_reference_result_type(result_type, None, None, None, anchor)?;
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

                self.expect_integer_like_type(
                    index_type,
                    anchor,
                    "element.get index must be an integer type",
                )?;
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

                self.expect_integer_like_type(
                    index_type,
                    anchor,
                    "element.set index must be an integer type",
                )?;
            }
            Instruction::ElementAddr {
                array,
                index,
                result_type,
                ..
            } => {
                let result_type = self.require_type_reference(
                    *result_type,
                    anchor,
                    "element.address result type",
                )?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;

                let array_type =
                    self.value_type_or_error(function, *array, anchor, "element.address array")?;
                let index_type =
                    self.value_type_or_error(function, *index, anchor, "element.address index")?;
                self.element_type_for_array(array_type, anchor, "element.address")?;
                self.validate_reference_result_type(result_type, None, None, None, anchor)?;

                self.expect_integer_like_type(
                    index_type,
                    anchor,
                    "element.address index must be an integer type",
                )?;
            }
            Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                let argument_type =
                    self.value_type_or_error(function, *argument, anchor, "cast argument")?;
                let to_type = self.require_type_reference(*to_type, anchor, "cast result type")?;
                self.validate_cast_legality(*operator, argument_type, to_type, anchor)?;
            }
            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                self.validate_intrinsic_operation(
                    function,
                    *destination,
                    *intrinsic,
                    self.tree.get_arguments(*arguments),
                    anchor,
                )?;
            }
            Instruction::Call { call, .. } => {
                let signature =
                    self.require_type_reference(call.signature, anchor, "call signature")?;
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                if !matches!(self.tree.get(signature), Type::FunctionPointer { .. }) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::CallIndirect { call, .. } => {
                let signature =
                    self.require_type_reference(call.signature, anchor, "call signature")?;
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                if !matches!(
                    self.tree.get(signature),
                    Type::FunctionPointer { .. } | Type::Closure { .. }
                ) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::CallVirtual {
                declaring_type,
                call,
                ..
            }
            | Instruction::CallInterface {
                declaring_type,
                call,
                ..
            } => {
                let declaring_type =
                    self.require_type_reference(*declaring_type, anchor, "call declaring type")?;
                let signature =
                    self.require_type_reference(call.signature, anchor, "call signature")?;
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                if !matches!(self.tree.get(signature), Type::FunctionPointer { .. }) {
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

    /// Validate one intrinsic operation.
    fn validate_intrinsic_operation(
        &self,
        function: &Function,
        destination: Option<ValueReference>,
        intrinsic: Intrinsic,
        arguments: &[ValueReference],
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        match intrinsic {
            Intrinsic::AddressSpaceCast => {
                self.validate_addr_space_cast_intrinsic(function, destination, arguments, anchor)?;
            }
            Intrinsic::PointerOffsetFrom => {
                self.validate_ptr_offset_from_intrinsic(function, destination, arguments, anchor)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate the `space.cast` intrinsic.
    fn validate_addr_space_cast_intrinsic(
        &self,
        function: &Function,
        destination: Option<ValueReference>,
        arguments: &[ValueReference],
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // arity and destination
        if arguments.len() != 1 {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "space.cast requires one argument".to_string(),
                anchor,
            });
        }

        let destination = destination.ok_or_else(|| ValidateError::MetadataInvariantViolation {
            message: "space.cast requires a destination value".to_string(),
            anchor,
        })?;

        // source and destination types
        let source_type =
            self.value_type_or_error(function, arguments[0], anchor, "space.cast source")?;
        let destination_type =
            self.value_type_or_error(function, destination, anchor, "space.cast destination")?;

        // reference forms
        if let (Ok(source), Ok(destination)) = (
            self.reference_type(
                source_type,
                anchor,
                "space.cast requires reference or tensor reference types",
            ),
            self.reference_type(
                destination_type,
                anchor,
                "space.cast requires reference or tensor reference types",
            ),
        ) {
            if source.0 != destination.0 || source.1 != destination.1 || source.2 != destination.2 {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "space.cast requires matching reference kind, mutability, and pointee"
                        .to_string(),
                    anchor,
                });
            }

            return Ok(());
        }

        // tensor reference forms
        if let (Ok(source), Ok(destination)) = (
            self.tensor_reference_type(
                source_type,
                anchor,
                "space.cast requires reference or tensor reference types",
            ),
            self.tensor_reference_type(
                destination_type,
                anchor,
                "space.cast requires reference or tensor reference types",
            ),
        ) {
            if source.0 != destination.0
                || source.2 != destination.2
                || source.3 != destination.3
                || source.4 != destination.4
                || source.5 != destination.5
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "space.cast requires matching tensor reference kind, mutability, element, shape, and layout".to_string(),
                    anchor,
                });
            }

            return Ok(());
        }

        Err(ValidateError::MetadataInvariantViolation {
            message: "space.cast requires reference or tensor reference types".to_string(),
            anchor,
        })
    }

    /// Validate the `ptrOffsetFrom` intrinsic.
    fn validate_ptr_offset_from_intrinsic(
        &self,
        function: &Function,
        destination: Option<ValueReference>,
        arguments: &[ValueReference],
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // arity and destination
        if arguments.len() != 2 {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "ptrOffsetFrom requires two pointer arguments".to_string(),
                anchor,
            });
        }

        let destination = destination.ok_or_else(|| ValidateError::MetadataInvariantViolation {
            message: "ptrOffsetFrom requires a destination value".to_string(),
            anchor,
        })?;

        // operand and destination types
        let left_type =
            self.value_type_or_error(function, arguments[0], anchor, "ptrOffsetFrom left")?;
        let right_type =
            self.value_type_or_error(function, arguments[1], anchor, "ptrOffsetFrom right")?;
        let destination_type =
            self.value_type_or_error(function, destination, anchor, "ptrOffsetFrom destination")?;

        // raw pointer inputs
        if !self.is_raw_pointer_like(self.tree.get(left_type))
            || !self.is_raw_pointer_like(self.tree.get(right_type))
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "ptrOffsetFrom requires raw pointer arguments".to_string(),
                anchor,
            });
        }

        // integer destination
        self.expect_integer_like_type(
            destination_type,
            anchor,
            "ptrOffsetFrom destination must be an integer type",
        )?;

        Ok(())
    }

    /// Resolve one projected field type from a concrete projection target.
    fn field_type_for_projection_target(
        &self,
        target_type: LocalNodeId<Type>,
        index: usize,
        anchor: ValidateAnchor,
        operation: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        match self.tree.get(target_type) {
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

                self.concrete_type_reference(self.tree.get(*field_id).ty)
                    .ok_or_else(|| {
                        self.metadata_error(
                            anchor,
                            format!("{operation} field type is not concrete"),
                        )
                    })
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

                self.concrete_type_reference(*element_type).ok_or_else(|| {
                    self.metadata_error(
                        anchor,
                        format!("{operation} tuple element type is not concrete"),
                    )
                })
            }
            Type::Closure { .. } => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{operation} does not support callable"),
                anchor,
            }),
            Type::Reference { pointee, .. } => {
                let Some(pointee_type) = self.concrete_type_reference(*pointee) else {
                    return Err(self.metadata_error(
                        anchor,
                        format!("{operation} pointee type is not concrete"),
                    ));
                };
                let pointee = self.tree.get(pointee_type);

                match pointee {
                    Type::Struct { .. } | Type::Tuple { .. } => self
                        .field_type_for_projection_target(pointee_type, index, anchor, operation),
                    Type::Closure { .. } => Err(ValidateError::MetadataInvariantViolation {
                        message: format!("{operation} does not support callable"),
                        anchor,
                    }),
                    _ if index == 0 => Ok(pointee_type),
                    _ => Err(ValidateError::MetadataInvariantViolation {
                        message: format!(
                            "{operation} field index {index} out of bounds for scalar pointee"
                        ),
                        anchor,
                    }),
                }
            }
            _ => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{operation} expects a struct or tuple aggregate"),
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
        match self.array_element_type(array_type) {
            Some(element_type) => Ok(element_type),
            _ => Err(ValidateError::MetadataInvariantViolation {
                message: format!("{operation} expects an array aggregate"),
                anchor,
            }),
        }
    }

    /// Resolve one projected element type if the type supports indexing.
    fn array_element_type(&self, type_id: LocalNodeId<Type>) -> Option<LocalNodeId<Type>> {
        match self.tree.get(type_id) {
            Type::Array { element, .. } => self.concrete_type_reference(*element),
            Type::Reference { pointee, .. } => match self.concrete_type_reference(*pointee) {
                Some(pointee) => match self.tree.get(pointee) {
                    Type::Array { element, .. } => self.concrete_type_reference(*element),
                    _ => Some(pointee),
                },
                None => None,
            },
            Type::TensorReference { element, .. } => self.concrete_type_reference(*element),
            _ => None,
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

        if let (Some(value_pointee), Some(slot_pointee)) = (
            self.reference_pointee_type(value_type),
            self.reference_pointee_type(slot_type),
        ) {
            return self.types_equivalent(value_pointee, slot_pointee);
        }

        if let Some(value_pointee) = self.reference_pointee_type(value_type) {
            return self.types_equivalent(value_pointee, slot_type);
        }

        if let Some(slot_pointee) = self.reference_pointee_type(slot_type) {
            return self.types_equivalent(value_type, slot_pointee);
        }

        if let (Some(value_tensor), Some(slot_tensor)) = (
            self.tensor_storage_type(value_type),
            self.tensor_storage_type(slot_type),
        ) {
            return value_tensor.0 == slot_tensor.0
                && value_tensor.1 == slot_tensor.1
                && value_tensor.2 == slot_tensor.2;
        }

        false
    }

    /// Resolve one reference pointee type.
    fn reference_pointee_type(&self, type_id: LocalNodeId<Type>) -> Option<LocalNodeId<Type>> {
        match self.tree.get(type_id) {
            Type::Reference { pointee, .. } => self.concrete_type_reference(*pointee),
            _ => None,
        }
    }

    /// Resolve one tensor-shaped storage type.
    fn tensor_storage_type(
        &self,
        type_id: LocalNodeId<Type>,
    ) -> Option<(LocalNodeId<Type>, &[TensorDimension], &TensorLayout)> {
        match self.tree.get(type_id) {
            Type::Tensor {
                element,
                shape,
                layout,
                ..
            } => Some((self.concrete_type_reference(*element)?, shape, layout)),
            Type::TensorReference {
                element,
                shape,
                layout,
                ..
            } => Some((self.concrete_type_reference(*element)?, shape, layout)),
            _ => None,
        }
    }

    /// Resolve one concrete nested type reference.
    fn concrete_type_reference(&self, ty: TypeReference) -> Option<LocalNodeId<Type>> {
        match ty {
            TypeReference::Type(ty) => Some(ty),
            TypeReference::Missing | TypeReference::Error => None,
        }
    }

    /// Check whether two type ids are structurally equivalent.
    pub(super) fn types_equivalent(
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
                    && match (
                        self.concrete_type_reference(*left_pointee),
                        self.concrete_type_reference(*right_pointee),
                    ) {
                        (Some(left_pointee), Some(right_pointee)) => {
                            self.types_equivalent_inner(left_pointee, right_pointee, seen_pairs)
                        }
                        _ => false,
                    }
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
                    && match (
                        self.concrete_type_reference(*left_element),
                        self.concrete_type_reference(*right_element),
                    ) {
                        (Some(left_element), Some(right_element)) => {
                            self.types_equivalent_inner(left_element, right_element, seen_pairs)
                        }
                        _ => false,
                    }
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
                        |(left_element, right_element)| match (
                            self.concrete_type_reference(*left_element),
                            self.concrete_type_reference(*right_element),
                        ) {
                            (Some(left_element), Some(right_element)) => {
                                self.types_equivalent_inner(left_element, right_element, seen_pairs)
                            }
                            _ => false,
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
                                && match (
                                    self.concrete_type_reference(left_field.ty),
                                    self.concrete_type_reference(right_field.ty),
                                ) {
                                    (Some(left_field_type), Some(right_field_type)) => self
                                        .types_equivalent_inner(
                                            left_field_type,
                                            right_field_type,
                                            seen_pairs,
                                        ),
                                    _ => false,
                                }
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
                    && match (
                        self.concrete_type_reference(*left_inner),
                        self.concrete_type_reference(*right_inner),
                    ) {
                        (Some(left_inner), Some(right_inner)) => {
                            self.types_equivalent_inner(left_inner, right_inner, seen_pairs)
                        }
                        _ => false,
                    }
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
                    && match (
                        self.concrete_type_reference(*left_element),
                        self.concrete_type_reference(*right_element),
                    ) {
                        (Some(left_element), Some(right_element)) => {
                            self.types_equivalent_inner(left_element, right_element, seen_pairs)
                        }
                        _ => false,
                    }
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
                    && match (
                        self.concrete_type_reference(*left_element),
                        self.concrete_type_reference(*right_element),
                    ) {
                        (Some(left_element), Some(right_element)) => {
                            self.types_equivalent_inner(left_element, right_element, seen_pairs)
                        }
                        _ => false,
                    }
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
                    && match (
                        self.concrete_type_reference(*left_element),
                        self.concrete_type_reference(*right_element),
                    ) {
                        (Some(left_element), Some(right_element)) => {
                            self.types_equivalent_inner(left_element, right_element, seen_pairs)
                        }
                        _ => false,
                    }
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
                        |(left_parameter, right_parameter)| match (
                            self.concrete_type_reference(*left_parameter),
                            self.concrete_type_reference(*right_parameter),
                        ) {
                            (Some(left_parameter), Some(right_parameter)) => self
                                .types_equivalent_inner(
                                    left_parameter,
                                    right_parameter,
                                    seen_pairs,
                                ),
                            _ => false,
                        },
                    )
                    && match (
                        self.concrete_type_reference(*left_result),
                        self.concrete_type_reference(*right_result),
                    ) {
                        (Some(left_result), Some(right_result)) => {
                            self.types_equivalent_inner(left_result, right_result, seen_pairs)
                        }
                        _ => false,
                    }
            }
            (
                Type::Closure {
                    signature: left_signature,
                },
                Type::Closure {
                    signature: right_signature,
                },
            ) => match (
                self.concrete_type_reference(*left_signature),
                self.concrete_type_reference(*right_signature),
            ) {
                (Some(left_signature), Some(right_signature)) => {
                    self.types_equivalent_inner(left_signature, right_signature, seen_pairs)
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Validate reference result types for pointer-producing instructions.
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

        if let Some(expected) = expected_pointee {
            let Some(pointee) = self.concrete_type_reference(*pointee) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "pointer-producing instruction result pointee is not concrete"
                        .to_string(),
                    anchor,
                });
            };

            if !self.types_equivalent(pointee, expected) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "pointer-producing instruction result type mismatches pointee"
                        .to_string(),
                    anchor,
                });
            }
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
                        message: "cast.bit cannot cast between pointer and non pointer categories"
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
                            "cast.bit requires equal storage size, got {} and {} bytes",
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
                        message: "cast.truncate requires destination integer no wider than source"
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
                    let Some(inner) = self.concrete_type_reference(*inner) else {
                        return type_id;
                    };
                    type_id = inner;
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
}
