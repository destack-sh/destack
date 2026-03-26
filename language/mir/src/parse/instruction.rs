use std::str::FromStr;

use crate::{
    ArgumentSlice, AtomicRmwOperator, AtomicScope, BinaryOperator, CastOperator, Function,
    Instruction, InterfaceSlotId, LocalNodeId, MemoryOrdering, MemoryRegionSet, MemoryScope,
    MemorySemantics, TensorConvertMode, TensorConvolutionDimensionNumbers, TensorConvolutionWindow,
    TensorDotDimensionNumbers, TensorGatherDimensionNumbers, TensorReduceOperator,
    TensorScatterDimensionNumbers, TensorScatterMode, Type, UnaryOperator, Value,
    VectorConvertMode, VectorReduceOperator, VtableSlotId,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Parse an instruction.
    pub(super) fn eat_instruction(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        // optional destination
        let mut destination = None;
        let mut destination_type = None;
        let mut instruction_span = None;
        if self.peek_token(TokenType::Value) {
            let (parsed_destination, parsed_type, parsed_span) = self.parse_typed_destination()?;
            self.record_value_type(parsed_destination, parsed_type);
            self.eat_token(TokenType::Equals)?;
            destination = Some(parsed_destination);
            destination_type = Some(parsed_type);
            instruction_span = Some(parsed_span);
        }

        // opcode
        let opcode = self
            .peek()
            .cloned()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;
        let (opcode_text, opcode_start) = self.eat_opcode()?;
        let opcode_span = self.span_for_token(&opcode);
        let instruction_span = instruction_span.unwrap_or(opcode_span);

        // reject destinations on void instructions
        if destination.is_some()
            && matches!(
                opcode_text,
                "local.set"
                    | "store"
                    | "raw.drop"
                    | "stack.drop"
                    | "atomic.store"
                    | "atomic.fence"
                    | "barrier"
                    | "assume"
                    | "tensor.store"
                    | "tensor.fill"
                    | "tensor.copy"
                    | "raw.free"
            )
        {
            return Err(ParseError::invalid(
                &format!("instruction '{opcode_text}'"),
                opcode_start,
            ));
        }

        // parse the operation
        let instruction = match opcode_text {
            // local operations
            "local.set" => {
                let local = self.parse_local_ref()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::LocalSet { local, value }
            }

            // memory side effects
            "store" => {
                let pointer = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::Store { pointer, value }
            }
            "raw.drop" => {
                let value = self.parse_value()?;
                Instruction::RawDrop { value }
            }
            "stack.drop" => {
                let value = self.parse_value()?;
                Instruction::StackDrop { value }
            }
            "atomic.store" => {
                let pointer = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                let (ordering, scope, memory_scope, semantics) = self.parse_atomic_attributes()?;
                Instruction::AtomicStore {
                    pointer,
                    value,
                    ordering,
                    scope,
                    memory_scope,
                    semantics,
                }
            }
            "atomic.fence" => {
                let (ordering, scope, memory_scope, semantics) = self.parse_atomic_attributes()?;
                Instruction::AtomicFence {
                    ordering,
                    scope,
                    memory_scope,
                    semantics,
                }
            }
            "barrier" => {
                let (scope, memory_scope, semantics) = self.parse_barrier_attributes()?;
                Instruction::Barrier {
                    scope,
                    memory_scope,
                    semantics,
                }
            }
            "assume" => {
                let condition = self.parse_value()?;
                Instruction::Assume { condition }
            }

            // tensor side effects
            "tensor.store" => {
                let view = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let indices = self.parse_value_bracket_list()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                let indices = self.tree.add_arguments(&indices);
                Instruction::TensorStore {
                    view,
                    indices,
                    value,
                }
            }
            "tensor.fill" => {
                let view = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::TensorFill { view, value }
            }
            "tensor.copy" => {
                let target = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let source = self.parse_value()?;
                Instruction::TensorCopy { target, source }
            }

            // calls and intrinsics
            "call" => {
                let (function, arguments) = self.parse_direct_call_target()?;
                let arguments = self.tree.add_arguments(&arguments);
                let signature = self.parse_direct_call_signature(function)?;

                Instruction::Call {
                    destination,
                    function,
                    arguments,
                    signature,
                    effects: None,
                }
            }
            "call.virtual" => {
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_virtual_call_target()?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallVirtual {
                    destination,
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    signature,
                    effects: None,
                }
            }
            "call.interface" => {
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_interface_call_target()?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallInterface {
                    destination,
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    signature,
                    effects: None,
                }
            }
            "call.indirect" => {
                let (callee, arguments, signature) = self.parse_indirect_call_target()?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallIndirect {
                    destination,
                    callee,
                    arguments,
                    signature,
                    effects: None,
                }
            }
            _ if opcode_text.starts_with("intrinsic.") => {
                let intrinsic = self.parse_intrinsic_name(opcode_text, opcode_start)?;
                let arguments = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::Intrinsic {
                    destination,
                    intrinsic,
                    arguments,
                }
            }

            // allocation side effects
            "raw.free" => {
                let pointer = self.parse_value()?;
                Instruction::RawFree { pointer }
            }

            // result instructions
            _ => {
                let destination = destination.ok_or_else(|| {
                    ParseError::invalid(&format!("instruction '{opcode_text}'"), opcode_start)
                })?;
                let destination_type = destination_type.ok_or_else(|| {
                    ParseError::invalid(&format!("instruction '{opcode_text}'"), opcode_start)
                })?;

                match opcode_text {
                    // constant
                    "iconst" => {
                        if let Some(token) = self.peek()
                            && token.ty == TokenType::StringLiteral
                        {
                            return Err(ParseError::invalid(
                                "string constants must use globals",
                                token.start,
                            ));
                        }

                        let value = self.parse_constant_for_type(destination_type)?;
                        Instruction::Const { destination, value }
                    }

                    // binary ops
                    _ if opcode_text.parse::<BinaryOperator>().is_ok() => {
                        let operator = opcode_text.parse().unwrap();
                        let left = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value()?;
                        Instruction::Binary {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }

                    // unary ops
                    _ if opcode_text.parse::<UnaryOperator>().is_ok() => {
                        let operator = opcode_text.parse().unwrap();
                        let argument = self.parse_value()?;
                        Instruction::Unary {
                            destination,
                            operator,
                            argument,
                        }
                    }

                    // cast ops
                    _ if opcode_text.parse::<CastOperator>().is_ok() => {
                        let operator = opcode_text.parse().unwrap();
                        let argument = self.parse_value()?;
                        self.eat_token(TokenType::Arrow)?;
                        let to_type = self.parse_type()?;
                        Instruction::Cast {
                            destination,
                            operator,
                            argument,
                            to_type,
                        }
                    }

                    // selection
                    "select" => {
                        let condition = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let then_value = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let else_value = self.parse_value()?;
                        Instruction::Select {
                            destination,
                            condition,
                            then_value,
                            else_value,
                        }
                    }

                    // local operations
                    "local.get" => {
                        let local = self.parse_local_ref()?;
                        Instruction::LocalGet { destination, local }
                    }
                    "local.addr" => {
                        let local = self.parse_local_ref()?;
                        Instruction::LocalAddr {
                            destination,
                            local,
                            result_type: destination_type,
                        }
                    }

                    // global operations
                    "global.addr" => {
                        let global = self.parse_global_reference()?;
                        Instruction::GlobalAddr {
                            destination,
                            global,
                            result_type: destination_type,
                        }
                    }
                    "global.const" => {
                        let global = self.parse_global_reference()?;
                        Instruction::GlobalConst {
                            destination,
                            global,
                        }
                    }
                    "function.addr" => {
                        let function = self.parse_function_reference()?;
                        Instruction::FunctionAddr {
                            destination,
                            function,
                        }
                    }
                    "function.value" => {
                        let function = self.parse_function_reference()?;
                        self.eat_token(TokenType::Comma)?;
                        let environment = self.parse_value()?;
                        Instruction::FunctionValue {
                            destination,
                            function,
                            environment,
                        }
                    }
                    "function.environment" => Instruction::FunctionEnvironment { destination },

                    // memory operations
                    "load" => {
                        let pointer = self.parse_value()?;
                        Instruction::Load {
                            destination,
                            pointer,
                            result_type: destination_type,
                        }
                    }

                    // aggregate operations
                    "field.get" => {
                        let aggregate = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_literal()? as u32;
                        Instruction::FieldGet {
                            destination,
                            aggregate,
                            index,
                        }
                    }
                    "field.addr" => {
                        let aggregate = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_literal()? as u32;
                        Instruction::FieldAddr {
                            destination,
                            aggregate,
                            index,
                            result_type: destination_type,
                        }
                    }
                    "field.set" => {
                        let aggregate = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_literal()? as u32;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value()?;
                        Instruction::FieldSet {
                            destination,
                            aggregate,
                            index,
                            value,
                        }
                    }
                    "element.get" => {
                        let array = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value()?;
                        Instruction::ElementGet {
                            destination,
                            array,
                            index,
                        }
                    }
                    "element.addr" => {
                        let array = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value()?;
                        Instruction::ElementAddr {
                            destination,
                            array,
                            index,
                            result_type: destination_type,
                        }
                    }
                    "element.set" => {
                        let array = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value()?;
                        Instruction::ElementSet {
                            destination,
                            array,
                            index,
                            value,
                        }
                    }
                    "struct" => {
                        let ty = self.parse_type()?;
                        let fields = self.parse_call_arguments()?;
                        let fields = self.tree.add_arguments(&fields);
                        Instruction::Struct {
                            destination,
                            ty,
                            fields,
                        }
                    }
                    "tuple" => {
                        let ty = self.parse_type()?;
                        let elements = self.parse_call_arguments()?;
                        let elements = self.tree.add_arguments(&elements);
                        Instruction::Tuple {
                            destination,
                            ty,
                            elements,
                        }
                    }
                    "array" => {
                        let ty = self.parse_type()?;
                        let elements = self.parse_call_arguments()?;
                        let elements = self.tree.add_arguments(&elements);
                        Instruction::Array {
                            destination,
                            ty,
                            elements,
                        }
                    }

                    // vector operations
                    "vector.splat" => {
                        let value = self.parse_value()?;
                        Instruction::VectorSplat { destination, value }
                    }
                    "vector.extract" => {
                        let vector = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value()?;
                        Instruction::VectorExtract {
                            destination,
                            vector,
                            index,
                        }
                    }
                    "vector.insert" => {
                        let vector = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value()?;
                        Instruction::VectorInsert {
                            destination,
                            vector,
                            index,
                            value,
                        }
                    }
                    "vector.shuffle" => {
                        let left = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let mask = self.parse_u32_bracket_list()?;
                        Instruction::VectorShuffle {
                            destination,
                            left,
                            right,
                            mask,
                        }
                    }
                    "vector.select" => {
                        let mask = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let then_value = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let else_value = self.parse_value()?;
                        Instruction::VectorSelect {
                            destination,
                            mask,
                            then_value,
                            else_value,
                        }
                    }
                    "vector.reduce" => {
                        let operator = self.parse_vector_reduce_operator()?;
                        self.eat_token(TokenType::Comma)?;
                        let vector = self.parse_value()?;
                        Instruction::VectorReduce {
                            destination,
                            operator,
                            vector,
                        }
                    }
                    "vector.compare" => {
                        let operator = self.parse_compare_operator()?;
                        self.eat_token(TokenType::Comma)?;
                        let left = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value()?;
                        Instruction::VectorCompare {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }
                    "vector.convert" => {
                        let mode = self.parse_vector_convert_mode()?;
                        self.eat_token(TokenType::Comma)?;
                        let vector = self.parse_value()?;
                        Instruction::VectorConvert {
                            destination,
                            mode,
                            vector,
                        }
                    }

                    // tensor operations
                    "tensor.load" => {
                        let view = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let indices = self.parse_value_bracket_list()?;
                        let indices = self.tree.add_arguments(&indices);
                        Instruction::TensorLoad {
                            destination,
                            view,
                            indices,
                        }
                    }
                    "tensor.reshape" => {
                        let tensor = self.parse_value()?;
                        let shape = if self.eat_token_maybe(TokenType::Comma) {
                            let values = self.parse_value_bracket_list()?;
                            self.tree.add_arguments(&values)
                        } else {
                            ArgumentSlice::default()
                        };
                        Instruction::TensorReshape {
                            destination,
                            tensor,
                            shape,
                        }
                    }
                    "tensor.broadcast" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let dimensions = self.parse_u32_bracket_list()?;
                        Instruction::TensorBroadcast {
                            destination,
                            tensor,
                            dimensions,
                        }
                    }
                    "tensor.transpose" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let permutation = self.parse_u32_bracket_list()?;
                        Instruction::TensorTranspose {
                            destination,
                            tensor,
                            permutation,
                        }
                    }
                    "tensor.cast" => {
                        let tensor = self.parse_value()?;
                        Instruction::TensorCast {
                            destination,
                            tensor,
                        }
                    }
                    "tensor.view" => {
                        let view = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("offsets")?;
                        let offsets = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("sizes")?;
                        let sizes = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("strides")?;
                        let strides = self.parse_value_bracket_list()?;

                        // range payload
                        let mut arguments =
                            Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
                        arguments.extend_from_slice(&offsets);
                        arguments.extend_from_slice(&sizes);
                        arguments.extend_from_slice(&strides);
                        let arguments = self.tree.add_arguments(&arguments);

                        // range counts
                        let offsets_count = u16::try_from(offsets.len())
                            .map_err(|_| ParseError::invalid("offsets count", self.pos()))?;
                        let sizes_count = u16::try_from(sizes.len())
                            .map_err(|_| ParseError::invalid("sizes count", self.pos()))?;
                        let strides_count = u16::try_from(strides.len())
                            .map_err(|_| ParseError::invalid("strides count", self.pos()))?;

                        Instruction::TensorView {
                            destination,
                            view,
                            arguments,
                            offsets_count,
                            sizes_count,
                            strides_count,
                        }
                    }
                    "tensor.slice" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("offsets")?;
                        let offsets = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("sizes")?;
                        let sizes = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("strides")?;
                        let strides = self.parse_value_bracket_list()?;

                        // range payload
                        let mut arguments =
                            Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
                        arguments.extend_from_slice(&offsets);
                        arguments.extend_from_slice(&sizes);
                        arguments.extend_from_slice(&strides);
                        let arguments = self.tree.add_arguments(&arguments);

                        // range counts
                        let offsets_count = u16::try_from(offsets.len())
                            .map_err(|_| ParseError::invalid("offsets count", self.pos()))?;
                        let sizes_count = u16::try_from(sizes.len())
                            .map_err(|_| ParseError::invalid("sizes count", self.pos()))?;
                        let strides_count = u16::try_from(strides.len())
                            .map_err(|_| ParseError::invalid("strides count", self.pos()))?;

                        Instruction::TensorSlice {
                            destination,
                            tensor,
                            arguments,
                            offsets_count,
                            sizes_count,
                            strides_count,
                        }
                    }
                    "tensor.pad" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("value")?;
                        let value = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("low")?;
                        let low = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("high")?;
                        let high = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("interior")?;
                        let interior = self.parse_value_bracket_list()?;

                        // padding payload
                        let mut arguments =
                            Vec::with_capacity(low.len() + high.len() + interior.len());
                        arguments.extend_from_slice(&low);
                        arguments.extend_from_slice(&high);
                        arguments.extend_from_slice(&interior);
                        let arguments = self.tree.add_arguments(&arguments);

                        // padding counts
                        let low_count = u16::try_from(low.len())
                            .map_err(|_| ParseError::invalid("low padding count", self.pos()))?;
                        let high_count = u16::try_from(high.len())
                            .map_err(|_| ParseError::invalid("high padding count", self.pos()))?;
                        let interior_count = u16::try_from(interior.len()).map_err(|_| {
                            ParseError::invalid("interior padding count", self.pos())
                        })?;

                        Instruction::TensorPad {
                            destination,
                            tensor,
                            arguments,
                            low_count,
                            high_count,
                            interior_count,
                            value,
                        }
                    }
                    "tensor.concat" => {
                        let tensors = self.parse_value_bracket_list()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("axis")?;
                        let axis = self.parse_int_as_u32()?;
                        let tensors = self.tree.add_arguments(&tensors);
                        Instruction::TensorConcat {
                            destination,
                            tensors,
                            axis,
                        }
                    }
                    "tensor.compare" => {
                        let operator = self.parse_compare_operator()?;
                        self.eat_token(TokenType::Comma)?;
                        let left = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value()?;
                        Instruction::TensorCompare {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }
                    "tensor.select" => {
                        let mask = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let then_value = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let else_value = self.parse_value()?;
                        Instruction::TensorSelect {
                            destination,
                            mask,
                            then_value,
                            else_value,
                        }
                    }
                    "tensor.reduce" => {
                        let operator = self.parse_tensor_reduce_operator()?;
                        self.eat_token(TokenType::Comma)?;
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let initial = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("axes")?;
                        let axes = self.parse_u32_bracket_list()?;
                        Instruction::TensorReduce {
                            destination,
                            operator,
                            tensor,
                            initial,
                            axes,
                        }
                    }
                    "tensor.dot" => {
                        let left = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let dimensions = self.parse_tensor_dot_dimensions()?;
                        Instruction::TensorDot {
                            destination,
                            left,
                            right,
                            dimensions,
                        }
                    }
                    "tensor.convolution" => {
                        let input = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let kernel = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let dimensions = self.parse_tensor_convolution_dimensions()?;
                        let window = self.parse_tensor_convolution_window(&dimensions)?;
                        let (feature_group_count, batch_group_count) =
                            self.parse_tensor_convolution_group_counts()?;
                        Instruction::TensorConvolution {
                            destination,
                            input,
                            kernel,
                            dimensions,
                            window,
                            feature_group_count,
                            batch_group_count,
                        }
                    }
                    "tensor.gather" => {
                        let operand = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let indices = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let dimensions = self.parse_tensor_gather_dimensions()?;
                        self.eat_token(TokenType::Comma)?;
                        self.eat_named_key("slice_sizes")?;
                        let slice_sizes = self.parse_u32_bracket_list()?;
                        Instruction::TensorGather {
                            destination,
                            operand,
                            indices,
                            dimensions,
                            slice_sizes,
                        }
                    }
                    "tensor.scatter" => {
                        let operand = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let indices = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let updates = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let dimensions = self.parse_tensor_scatter_dimensions()?;
                        let mode = self
                            .parse_optional_scatter_mode()?
                            .unwrap_or(TensorScatterMode::Replace);
                        Instruction::TensorScatter {
                            destination,
                            operand,
                            indices,
                            updates,
                            dimensions,
                            mode,
                        }
                    }
                    "tensor.convert" => {
                        let mode = self.parse_tensor_convert_mode()?;
                        self.eat_token(TokenType::Comma)?;
                        let tensor = self.parse_value()?;
                        Instruction::TensorConvert {
                            destination,
                            mode,
                            tensor,
                        }
                    }

                    // allocation operations
                    "managed.alloc" => {
                        let layout = self.parse_type()?;
                        Instruction::ManagedAlloc {
                            destination,
                            layout,
                            result_type: destination_type,
                        }
                    }
                    "managed.alloc_array" => {
                        let element = self.parse_type()?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value()?;
                        Instruction::ManagedAllocArray {
                            destination,
                            element,
                            length,
                            result_type: destination_type,
                        }
                    }
                    "raw.alloc" => {
                        let layout = self.parse_type()?;
                        Instruction::RawAlloc {
                            destination,
                            layout,
                            result_type: destination_type,
                        }
                    }
                    "stack.alloc" => {
                        let layout = self.parse_type()?;
                        Instruction::StackAlloc {
                            destination,
                            layout,
                            result_type: destination_type,
                        }
                    }

                    // atomic memory operations
                    "atomic.load" => {
                        let pointer = self.parse_value()?;
                        let (ordering, scope, memory_scope, semantics) =
                            self.parse_atomic_attributes()?;
                        Instruction::AtomicLoad {
                            destination,
                            pointer,
                            result_type: destination_type,
                            ordering,
                            scope,
                            memory_scope,
                            semantics,
                        }
                    }
                    "atomic.cas" | "atomic.cas.weak" => {
                        let pointer = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let expected = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let new_value = self.parse_value()?;
                        let (ordering, scope, memory_scope, semantics) =
                            self.parse_atomic_attributes()?;
                        Instruction::AtomicCompareExchange {
                            destination,
                            pointer,
                            expected,
                            new_value,
                            is_weak: opcode_text == "atomic.cas.weak",
                            ordering,
                            scope,
                            memory_scope,
                            semantics,
                        }
                    }
                    _ if opcode_text.starts_with("atomic.rmw.") => {
                        let operator = self.parse_atomic_rmw_operator(opcode_text, opcode_start)?;
                        let pointer = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value()?;
                        let (ordering, scope, memory_scope, semantics) =
                            self.parse_atomic_attributes()?;
                        Instruction::AtomicRmw {
                            destination,
                            operator,
                            pointer,
                            value,
                            ordering,
                            scope,
                            memory_scope,
                            semantics,
                        }
                    }

                    _ => {
                        return Err(ParseError::invalid(
                            &format!("instruction '{opcode_text}'"),
                            opcode_start,
                        ));
                    }
                }
            }
        };

        // record the instruction
        let instruction_id = self.tree.insert(instruction);
        self.tree.set_span(instruction_id, instruction_span);
        Ok(instruction_id)
    }
    /// Parse a bracketed list of values.
    fn parse_value_bracket_list(&mut self) -> ParseResult<Vec<Value>> {
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_token(TokenType::CloseBracket) {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        // close the list
        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a bracketed list of u32 values.
    fn parse_u32_bracket_list(&mut self) -> ParseResult<Vec<u32>> {
        let values = self.parse_int_bracket_list()?;
        values
            .into_iter()
            .map(|value| {
                u32::try_from(value).map_err(|_| ParseError::invalid("u32 literal", self.pos()))
            })
            .collect()
    }

    /// Parse a bracketed list of u64 values.
    fn parse_u64_bracket_list(&mut self) -> ParseResult<Vec<u64>> {
        let values = self.parse_int_bracket_list()?;
        values
            .into_iter()
            .map(|value| {
                u64::try_from(value).map_err(|_| ParseError::invalid("u64 literal", self.pos()))
            })
            .collect()
    }

    /// Parse a bracketed list of integer values.
    fn parse_int_bracket_list(&mut self) -> ParseResult<Vec<i64>> {
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_token(TokenType::CloseBracket) {
            let value = self.parse_int_literal()?;
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        // close the list
        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a bracketed list of boolean values.
    fn parse_bool_bracket_list(&mut self) -> ParseResult<Vec<bool>> {
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_token(TokenType::CloseBracket) {
            let token = self.eat_token(TokenType::BoolLiteral)?;
            let value = token.text == "true";
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        // close the list
        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse one named assignment key like `name=`.
    fn eat_named_key(&mut self, name: &str) -> ParseResult<()> {
        let (key, key_start) = self.eat_assignment_key()?;
        if key != name {
            return Err(ParseError::invalid(name, key_start));
        }

        Ok(())
    }

    /// Parse one assignment key like `name=`.
    fn eat_assignment_key(&mut self) -> ParseResult<(&'a str, usize)> {
        let token = self.eat_token(TokenType::Identifier)?;
        let text = token.text;
        let start = token.start;
        self.eat_token(TokenType::Equals)?;
        Ok((text, start))
    }

    /// Parse a vector reduction operator.
    fn parse_vector_reduce_operator(&mut self) -> ParseResult<VectorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = VectorReduceOperator::parse(token.text)
            .ok_or_else(|| ParseError::invalid("vector reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a vector conversion mode.
    fn parse_vector_convert_mode(&mut self) -> ParseResult<VectorConvertMode> {
        // parse the mode token
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = VectorConvertMode::parse(token.text)
            .ok_or_else(|| ParseError::invalid("vector convert mode", token.start))?;
        Ok(mode)
    }

    /// Parse a tensor conversion mode.
    fn parse_tensor_convert_mode(&mut self) -> ParseResult<TensorConvertMode> {
        // parse the mode token
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = TensorConvertMode::parse(token.text)
            .ok_or_else(|| ParseError::invalid("tensor convert mode", token.start))?;
        Ok(mode)
    }

    /// Parse a tensor reduction operator.
    fn parse_tensor_reduce_operator(&mut self) -> ParseResult<TensorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = TensorReduceOperator::parse(token.text)
            .ok_or_else(|| ParseError::invalid("tensor reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a comparison operator for vector or tensor operations.
    fn parse_compare_operator(&mut self) -> ParseResult<BinaryOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = BinaryOperator::from_str(token.text)
            .map_err(|_| ParseError::invalid("comparison operator", token.start))?;
        Ok(operator)
    }

    /// Parse an optional tensor scatter mode.
    fn parse_optional_scatter_mode(&mut self) -> ParseResult<Option<TensorScatterMode>> {
        // check for a comma followed by the mode
        if !self.peek_token(TokenType::Comma) {
            return Ok(None);
        }
        self.eat_token(TokenType::Comma)?;
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "mode" {
            return Err(ParseError::invalid("mode", token.start));
        }
        self.eat_token(TokenType::Equals)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = TensorScatterMode::parse(token.text)
            .ok_or_else(|| ParseError::invalid("scatter mode", token.start))?;
        Ok(Some(mode))
    }

    /// Parse tensor dot dimension numbers.
    fn parse_tensor_dot_dimensions(&mut self) -> ParseResult<TensorDotDimensionNumbers> {
        // parse the header
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParen)?;

        // parse named lists
        let mut lhs_batch = None;
        let mut rhs_batch = None;
        let mut lhs_contracting = None;
        let mut rhs_contracting = None;
        while !self.peek_token(TokenType::CloseParen) {
            let (key, key_start) = self.eat_assignment_key()?;
            let list = self.parse_u32_bracket_list()?;
            match key {
                "lhs_batch" => lhs_batch = Some(list),
                "rhs_batch" => rhs_batch = Some(list),
                "lhs_contract" => lhs_contracting = Some(list),
                "rhs_contract" => rhs_contracting = Some(list),
                _ => return Err(ParseError::invalid("dot dimension key", key_start)),
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;

        // validate required keys
        let lhs_batch = lhs_batch.ok_or_else(|| ParseError::invalid("lhs_batch", self.pos()))?;
        let rhs_batch = rhs_batch.ok_or_else(|| ParseError::invalid("rhs_batch", self.pos()))?;
        let lhs_contracting =
            lhs_contracting.ok_or_else(|| ParseError::invalid("lhs_contract", self.pos()))?;
        let rhs_contracting =
            rhs_contracting.ok_or_else(|| ParseError::invalid("rhs_contract", self.pos()))?;

        Ok(TensorDotDimensionNumbers {
            lhs_batch,
            rhs_batch,
            lhs_contracting,
            rhs_contracting,
        })
    }

    /// Parse tensor convolution dimension numbers.
    fn parse_tensor_convolution_dimensions(
        &mut self,
    ) -> ParseResult<TensorConvolutionDimensionNumbers> {
        // parse the header
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParen)?;

        // parse named lists
        let mut input_batch = None;
        let mut input_feature = None;
        let mut input_spatial = None;
        let mut kernel_input_feature = None;
        let mut kernel_output_feature = None;
        let mut kernel_spatial = None;
        let mut output_batch = None;
        let mut output_feature = None;
        let mut output_spatial = None;
        while !self.peek_token(TokenType::CloseParen) {
            let (key, key_start) = self.eat_assignment_key()?;
            match key {
                "input_batch" => input_batch = Some(self.parse_int_as_u32()?),
                "input_feature" => input_feature = Some(self.parse_int_as_u32()?),
                "input_spatial" => input_spatial = Some(self.parse_u32_bracket_list()?),
                "kernel_input_feature" => kernel_input_feature = Some(self.parse_int_as_u32()?),
                "kernel_output_feature" => kernel_output_feature = Some(self.parse_int_as_u32()?),
                "kernel_spatial" => kernel_spatial = Some(self.parse_u32_bracket_list()?),
                "output_batch" => output_batch = Some(self.parse_int_as_u32()?),
                "output_feature" => output_feature = Some(self.parse_int_as_u32()?),
                "output_spatial" => output_spatial = Some(self.parse_u32_bracket_list()?),
                _ => {
                    return Err(ParseError::invalid("convolution dimension key", key_start));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;

        // validate required keys
        let input_batch =
            input_batch.ok_or_else(|| ParseError::invalid("input_batch", self.pos()))?;
        let input_feature =
            input_feature.ok_or_else(|| ParseError::invalid("input_feature", self.pos()))?;
        let input_spatial =
            input_spatial.ok_or_else(|| ParseError::invalid("input_spatial", self.pos()))?;
        let kernel_input_feature = kernel_input_feature
            .ok_or_else(|| ParseError::invalid("kernel_input_feature", self.pos()))?;
        let kernel_output_feature = kernel_output_feature
            .ok_or_else(|| ParseError::invalid("kernel_output_feature", self.pos()))?;
        let kernel_spatial =
            kernel_spatial.ok_or_else(|| ParseError::invalid("kernel_spatial", self.pos()))?;
        let output_batch =
            output_batch.ok_or_else(|| ParseError::invalid("output_batch", self.pos()))?;
        let output_feature =
            output_feature.ok_or_else(|| ParseError::invalid("output_feature", self.pos()))?;
        let output_spatial =
            output_spatial.ok_or_else(|| ParseError::invalid("output_spatial", self.pos()))?;

        Ok(TensorConvolutionDimensionNumbers {
            input_batch,
            input_feature,
            input_spatial,
            kernel_input_feature,
            kernel_output_feature,
            kernel_spatial,
            output_batch,
            output_feature,
            output_spatial,
        })
    }

    /// Parse convolution window parameters.
    fn parse_tensor_convolution_window(
        &mut self,
        dimensions: &TensorConvolutionDimensionNumbers,
    ) -> ParseResult<TensorConvolutionWindow> {
        // infer spatial rank defaults
        let spatial_rank = dimensions.input_spatial.len();
        let default_stride = vec![1; spatial_rank];
        let default_padding = vec![0; spatial_rank];
        let default_dilation = vec![1; spatial_rank];
        let default_reversal = vec![false; spatial_rank];

        // collect optional entries
        let mut strides = None;
        let mut padding_low = None;
        let mut padding_high = None;
        let mut lhs_dilation = None;
        let mut rhs_dilation = None;
        let mut window_reversal = None;

        while self.peek_token(TokenType::Comma) {
            // check if the next key belongs to group counts
            let next_key = self
                .peek_nth_token(1)
                .ok_or_else(|| ParseError::unexpected_end("convolution window key", self.pos()))?;
            if next_key.ty == TokenType::Identifier
                && (next_key.text == "feature_group" || next_key.text == "batch_group")
            {
                break;
            }

            // consume the window key
            self.eat_token(TokenType::Comma)?;
            let (key, key_start) = self.eat_assignment_key()?;
            match key {
                "strides" => strides = Some(self.parse_u64_bracket_list()?),
                "padding_low" => padding_low = Some(self.parse_u64_bracket_list()?),
                "padding_high" => padding_high = Some(self.parse_u64_bracket_list()?),
                "lhs_dilation" => lhs_dilation = Some(self.parse_u64_bracket_list()?),
                "rhs_dilation" => rhs_dilation = Some(self.parse_u64_bracket_list()?),
                "window_reversal" => window_reversal = Some(self.parse_bool_bracket_list()?),
                _ => {
                    return Err(ParseError::invalid("convolution window key", key_start));
                }
            }
        }

        Ok(TensorConvolutionWindow {
            strides: strides.unwrap_or(default_stride),
            padding_low: padding_low.unwrap_or(default_padding.clone()),
            padding_high: padding_high.unwrap_or(default_padding),
            lhs_dilation: lhs_dilation.unwrap_or(default_dilation.clone()),
            rhs_dilation: rhs_dilation.unwrap_or(default_dilation),
            window_reversal: window_reversal.unwrap_or(default_reversal),
        })
    }

    /// Parse convolution group counts.
    fn parse_tensor_convolution_group_counts(&mut self) -> ParseResult<(u32, u32)> {
        // parse optional group counts
        let mut feature_group_count = None;
        let mut batch_group_count = None;
        while self.peek_token(TokenType::Comma) {
            self.eat_token(TokenType::Comma)?;
            let (key, key_start) = self.eat_assignment_key()?;
            let value = self.parse_int_as_u32()?;
            match key {
                "feature_group" => feature_group_count = Some(value),
                "batch_group" => batch_group_count = Some(value),
                _ => {
                    return Err(ParseError::invalid("convolution group key", key_start));
                }
            }
        }

        Ok((
            feature_group_count.unwrap_or(1),
            batch_group_count.unwrap_or(1),
        ))
    }

    /// Parse tensor gather dimension numbers.
    fn parse_tensor_gather_dimensions(&mut self) -> ParseResult<TensorGatherDimensionNumbers> {
        // parse the header
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParen)?;

        // parse named lists
        let mut offset_dims = None;
        let mut collapsed_slice_dims = None;
        let mut start_index_map = None;
        let mut index_vector_dim = None;
        while !self.peek_token(TokenType::CloseParen) {
            let (key, key_start) = self.eat_assignment_key()?;
            match key {
                "offset_dims" => offset_dims = Some(self.parse_u32_bracket_list()?),
                "collapsed_slice_dims" => {
                    collapsed_slice_dims = Some(self.parse_u32_bracket_list()?);
                }
                "start_index_map" => start_index_map = Some(self.parse_u32_bracket_list()?),
                "index_vector_dim" => index_vector_dim = Some(self.parse_int_as_u32()?),
                _ => return Err(ParseError::invalid("gather dimension key", key_start)),
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;

        // validate required keys
        let offset_dims =
            offset_dims.ok_or_else(|| ParseError::invalid("offset_dims", self.pos()))?;
        let collapsed_slice_dims = collapsed_slice_dims
            .ok_or_else(|| ParseError::invalid("collapsed_slice_dims", self.pos()))?;
        let start_index_map =
            start_index_map.ok_or_else(|| ParseError::invalid("start_index_map", self.pos()))?;
        let index_vector_dim =
            index_vector_dim.ok_or_else(|| ParseError::invalid("index_vector_dim", self.pos()))?;

        Ok(TensorGatherDimensionNumbers {
            offset_dims,
            collapsed_slice_dims,
            start_index_map,
            index_vector_dim,
        })
    }

    /// Parse tensor scatter dimension numbers.
    fn parse_tensor_scatter_dimensions(&mut self) -> ParseResult<TensorScatterDimensionNumbers> {
        // parse the header
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParen)?;

        // parse named lists
        let mut update_window_dims = None;
        let mut inserted_window_dims = None;
        let mut scatter_dims_to_operand_dims = None;
        let mut index_vector_dim = None;
        while !self.peek_token(TokenType::CloseParen) {
            let (key, key_start) = self.eat_assignment_key()?;
            match key {
                "update_window_dims" => update_window_dims = Some(self.parse_u32_bracket_list()?),
                "inserted_window_dims" => {
                    inserted_window_dims = Some(self.parse_u32_bracket_list()?);
                }
                "scatter_dims_to_operand_dims" => {
                    scatter_dims_to_operand_dims = Some(self.parse_u32_bracket_list()?);
                }
                "index_vector_dim" => index_vector_dim = Some(self.parse_int_as_u32()?),
                _ => return Err(ParseError::invalid("scatter dimension key", key_start)),
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParen)?;

        // validate required keys
        let update_window_dims = update_window_dims
            .ok_or_else(|| ParseError::invalid("update_window_dims", self.pos()))?;
        let inserted_window_dims = inserted_window_dims
            .ok_or_else(|| ParseError::invalid("inserted_window_dims", self.pos()))?;
        let scatter_dims_to_operand_dims = scatter_dims_to_operand_dims
            .ok_or_else(|| ParseError::invalid("scatter_dims_to_operand_dims", self.pos()))?;
        let index_vector_dim =
            index_vector_dim.ok_or_else(|| ParseError::invalid("index_vector_dim", self.pos()))?;

        Ok(TensorScatterDimensionNumbers {
            update_window_dims,
            inserted_window_dims,
            scatter_dims_to_operand_dims,
            index_vector_dim,
        })
    }

    /// Parse an integer literal as u32.
    fn parse_int_as_u32(&mut self) -> ParseResult<u32> {
        // read the literal
        let value = self.parse_int_literal()?;
        u32::try_from(value).map_err(|_| ParseError::invalid("u32 literal", self.pos()))
    }

    /// Parse one direct call target and arguments.
    pub(super) fn parse_direct_call_target(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Function>, Vec<Value>)> {
        let function = self.parse_function_reference()?;
        let arguments = self.parse_call_arguments()?;
        Ok((function, arguments))
    }

    /// Parse an optional direct call signature.
    fn parse_direct_call_signature(
        &mut self,
        function: LocalNodeId<Function>,
    ) -> ParseResult<LocalNodeId<Type>> {
        if self.eat_token_maybe(TokenType::Arrow) {
            return self.parse_type();
        }

        let function = self.tree.get(function);
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect();

        Ok(self.intern_type(Type::FunctionPointer {
            parameters,
            result: function.return_type,
        }))
    }

    /// Parse one virtual call target and signature.
    pub(super) fn parse_virtual_call_target(
        &mut self,
    ) -> ParseResult<(
        Value,
        LocalNodeId<Type>,
        VtableSlotId,
        Vec<Value>,
        LocalNodeId<Type>,
    )> {
        let receiver = self.parse_value()?;
        self.eat_token(TokenType::Comma)?;
        let declaring_type = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let slot_id = VtableSlotId::new(self.parse_int_literal()? as u32);
        let arguments = self.parse_call_arguments()?;
        self.eat_token(TokenType::Arrow)?;
        let signature = self.parse_type()?;

        Ok((receiver, declaring_type, slot_id, arguments, signature))
    }

    /// Parse one interface call target and signature.
    pub(super) fn parse_interface_call_target(
        &mut self,
    ) -> ParseResult<(
        Value,
        LocalNodeId<Type>,
        InterfaceSlotId,
        Vec<Value>,
        LocalNodeId<Type>,
    )> {
        let receiver = self.parse_value()?;
        self.eat_token(TokenType::Comma)?;
        let declaring_type = self.parse_type()?;
        self.eat_token(TokenType::Comma)?;
        let slot_id = InterfaceSlotId::new(self.parse_int_literal()? as u32);
        let arguments = self.parse_call_arguments()?;
        self.eat_token(TokenType::Arrow)?;
        let signature = self.parse_type()?;

        Ok((receiver, declaring_type, slot_id, arguments, signature))
    }

    /// Parse one indirect call target and signature.
    pub(super) fn parse_indirect_call_target(
        &mut self,
    ) -> ParseResult<(Value, Vec<Value>, LocalNodeId<Type>)> {
        let callee = self.parse_value()?;
        let arguments = self.parse_call_arguments()?;
        self.eat_token(TokenType::Arrow)?;
        let signature = self.parse_type()?;

        Ok((callee, arguments, signature))
    }

    /// Parse one atomic ordering, scope, memory scope, and semantics suffix.
    fn parse_atomic_attributes(
        &mut self,
    ) -> ParseResult<(MemoryOrdering, AtomicScope, MemoryScope, MemorySemantics)> {
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("ordering")?;
        let ordering = self.parse_memory_ordering()?;
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("scope")?;
        let scope = self.parse_atomic_scope()?;
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("memory_scope")?;
        let memory_scope = self.parse_memory_scope()?;
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("semantics")?;
        let semantics = self.parse_memory_semantics()?;

        Ok((ordering, scope, memory_scope, semantics))
    }

    /// Parse one barrier scope, memory scope, and semantics suffix.
    fn parse_barrier_attributes(
        &mut self,
    ) -> ParseResult<(AtomicScope, MemoryScope, MemorySemantics)> {
        self.eat_named_key("scope")?;
        let scope = self.parse_atomic_scope()?;
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("memory_scope")?;
        let memory_scope = self.parse_memory_scope()?;
        self.eat_token(TokenType::Comma)?;

        self.eat_named_key("semantics")?;
        let semantics = self.parse_memory_semantics()?;

        Ok((scope, memory_scope, semantics))
    }

    /// Parse one memory ordering like `seq_cst`.
    fn parse_memory_ordering(&mut self) -> ParseResult<MemoryOrdering> {
        let token_start = self.pos();
        self.eat_token(TokenType::Identifier)?
            .text
            .parse::<MemoryOrdering>()
            .map_err(|_| ParseError::invalid("memory ordering", token_start))
    }

    /// Parse one atomic scope like `device`.
    fn parse_atomic_scope(&mut self) -> ParseResult<AtomicScope> {
        let token_start = self.pos();
        self.eat_token(TokenType::Identifier)?
            .text
            .parse::<AtomicScope>()
            .map_err(|_| ParseError::invalid("atomic scope", token_start))
    }

    /// Parse one memory scope like `device`.
    fn parse_memory_scope(&mut self) -> ParseResult<MemoryScope> {
        let token_start = self.pos();
        self.eat_token(TokenType::Identifier)?
            .text
            .parse::<MemoryScope>()
            .map_err(|_| ParseError::invalid("memory scope", token_start))
    }

    /// Parse memory semantics for atomic operations.
    fn parse_memory_semantics(&mut self) -> ParseResult<MemorySemantics> {
        // semantics state
        let mut locations = MemoryRegionSet::NONE;
        let mut has_location = false;
        let mut is_location_locked = false;
        let mut is_volatile = false;
        let mut is_make_available = false;
        let mut is_make_visible = false;
        let is_list = self.eat_token_maybe(TokenType::OpenBracket);

        // parse one or more semantics items
        loop {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("memory semantics", self.pos()))?;
            if !matches!(
                token.ty,
                TokenType::Identifier | TokenType::Global | TokenType::Local
            ) {
                return Err(ParseError::unexpected(
                    "memory semantics",
                    token.ty,
                    token.start,
                ));
            }

            let token_text = token.text.to_string();
            let token_start = token.start;
            self.bump();

            // flags and locations
            match token_text.as_str() {
                "volatile" => {
                    is_volatile = true;
                }
                "make_available" => {
                    is_make_available = true;
                }
                "make_visible" => {
                    is_make_visible = true;
                }
                _ => {
                    let location = self.parse_memory_location(&token_text, token_start)?;
                    if location == MemoryRegionSet::ANY || location == MemoryRegionSet::NONE {
                        if has_location && !is_location_locked {
                            return Err(ParseError::new(
                                "memory semantics cannot mix any/none with other locations",
                                token_start,
                            ));
                        }

                        locations = location;
                        has_location = true;
                        is_location_locked = true;
                    } else {
                        if is_location_locked {
                            return Err(ParseError::new(
                                "memory semantics cannot mix any/none with other locations",
                                token_start,
                            ));
                        }

                        if !has_location {
                            locations = MemoryRegionSet::NONE;
                            has_location = true;
                        }

                        locations.insert(location);
                    }
                }
            }

            // single values stop after one item
            if !is_list {
                break;
            }

            // lists stop before the closing bracket
            if !self.eat_token_maybe(TokenType::Comma) || self.peek_token(TokenType::CloseBracket) {
                break;
            }
        }

        // close the list
        if is_list {
            self.eat_token(TokenType::CloseBracket)?;
        }

        // default the location set when omitted
        if !has_location {
            locations = MemoryRegionSet::ANY;
        }

        Ok(MemorySemantics::with_flags(
            locations,
            is_volatile,
            is_make_available,
            is_make_visible,
        ))
    }

    /// Parse one atomic read modify write opcode suffix.
    fn parse_atomic_rmw_operator(
        &self,
        text: &str,
        start: usize,
    ) -> ParseResult<AtomicRmwOperator> {
        let Some(operator_text) = text.strip_prefix("atomic.rmw.") else {
            return Err(ParseError::invalid("atomic.rmw opcode", start));
        };

        AtomicRmwOperator::parse(operator_text)
            .ok_or_else(|| ParseError::invalid("atomic rmw operator", start))
    }
}
