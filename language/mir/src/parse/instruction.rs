use crate::source::TokenType;
use std::str::FromStr;

use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::{
    ArgumentSlice, AtomicAccess, AtomicRmwOperator, BinaryOperator, Call, CastOperator,
    CompareExchangeAccess, DispatchSlot, FenceAccess, FunctionReference, Instruction, LocalNodeId,
    MemoryFlags, MemoryOrdering, MemoryScope, Place, PlaceOrigin, PlaceProjection, SpaceSet,
    SyncScope, TensorConvertMode, TensorConvolutionDimensionNumbers, TensorConvolutionWindow,
    TensorDotDimensionNumbers, TensorGatherDimensionNumbers, TensorIndexReduceOperator,
    TensorIndexTieBreak, TensorReduceOperator, TensorScatterDimensionNumbers, TensorScatterMode,
    Type, TypeReference, UnaryOperator, ValueReference, VectorConvertMode, VectorReduceOperator,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse an instruction.
    pub(super) fn eat_instruction(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        // whole instruction
        let instruction_start = self.pos();

        // optional destination
        let mut destination: Option<ValueReference> = None;
        let mut destination_type = None;
        let mut destination_span = None;
        let mut destination_type_span = None;
        if self.is_value_definition_start() {
            let (parsed_destination, parsed_type, parsed_span, parsed_type_span) =
                self.parse_typed_destination_parts()?;
            self.record_value_type(parsed_destination, parsed_type);
            self.eat_token(TokenType::Equal)?;
            destination = Some(parsed_destination.into());
            destination_type = Some(parsed_type);
            destination_span = Some(parsed_span);
            destination_type_span = Some(parsed_type_span);
        }

        // destination driven literal sugar
        if let Some(destination) = destination
            && let Some(destination_type) = destination_type
        {
            // direct constant literal
            if let Some(token) = self.peek()
                && (matches!(
                    self.token_type(token),
                    TokenType::BooleanLiteral
                        | TokenType::Integer
                        | TokenType::Float
                        | TokenType::Character
                ) || (self.token_type(token) == TokenType::Identifier
                    && (self.tree.source_text(token.span) == "null"
                        || self
                            .parse_float_constant(self.tree.source_text(token.span))
                            .is_some())))
            {
                let value = self.parse_constant_for_type(destination_type)?;
                let instruction = Instruction::Const { destination, value };
                let id = self.tree.insert(instruction);
                self.apply_instruction_spans(
                    id,
                    self.span_from_parse_start(instruction_start),
                    destination_span,
                    destination_type_span,
                    &[],
                );
                return Ok(id);
            }
        }

        // ordered source parts
        let mut segment_spans = Vec::new();

        // opcode
        let opcode = self
            .peek()
            .cloned()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;
        let (opcode_text, opcode_start) = self.eat_opcode()?;
        let opcode_text = opcode_text.as_str();
        let opcode_span = opcode.span;
        segment_spans.push(opcode_span);

        // reject destinations on void instructions
        if destination.is_some()
            && matches!(
                opcode_text,
                "local.set"
                    | "store"
                    | "free"
                    | "drop"
                    | "barrier.write"
                    | "atomic.store"
                    | "atomic.fence"
                    | "assume"
                    | "tensor.store"
                    | "tensor.fill"
                    | "tensor.copy"
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
                let local = self.parse_local_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::LocalSet { local, value }
            }

            // memory side effects
            "store" => {
                let pointer = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Store { pointer, value }
            }
            "unpin" => {
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Unpin { value }
            }
            "drop" => {
                let (place, span) = self.parse_place_part()?;
                segment_spans.push(span);
                Instruction::Drop { place }
            }
            "free" => {
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Free { value }
            }
            "barrier.write" => {
                let object = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let offset = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let byte_len = self.parse_value_segment(&mut segment_spans)?;
                Instruction::BarrierWrite {
                    object,
                    offset,
                    byte_len,
                }
            }
            "atomic.store" => {
                let pointer = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                let access = self.parse_atomic_access()?;
                Instruction::AtomicStore {
                    pointer,
                    value,
                    access,
                }
            }
            "atomic.fence" => {
                let access = self.parse_fence_access()?;
                Instruction::AtomicFence { access }
            }
            "assume" => {
                let condition = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Assume { condition }
            }

            // tensor side effects
            "tensor.store" => {
                let view = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let indices = self.parse_value_bracket_list_segments(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                let indices = self.tree.add_arguments(&indices);
                Instruction::TensorStore {
                    view,
                    indices,
                    value,
                }
            }
            "tensor.fill" => {
                let view = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::TensorFill { view, value }
            }
            "tensor.copy" => {
                let target = self.parse_value_segment(&mut segment_spans)?;
                self.eat_token(TokenType::Comma)?;
                let source = self.parse_value_segment(&mut segment_spans)?;
                Instruction::TensorCopy { target, source }
            }

            // calls and intrinsics
            "call" => {
                let (function, arguments, signature) =
                    self.parse_direct_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_arguments(&arguments);

                Instruction::Call {
                    destination,
                    function,
                    call: Call::new(arguments, signature),
                }
            }
            "call.virtual" => {
                let (receiver, class, slot, arguments, signature) =
                    self.parse_class_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallVirtual {
                    destination,
                    receiver,
                    class,
                    slot,
                    call: Call::new(arguments, signature),
                }
            }
            "call.dynamic" => {
                let (receiver, constraint, slot, arguments, signature) =
                    self.parse_dynamic_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallDynamic {
                    destination,
                    receiver,
                    constraint,
                    slot,
                    call: Call::new(arguments, signature),
                }
            }
            "call.indirect" => {
                let (callee, arguments, signature) =
                    self.parse_indirect_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::CallIndirect {
                    destination,
                    callee,
                    call: Call::new(arguments, signature),
                }
            }
            _ if opcode_text.starts_with("intrinsic.") => {
                let intrinsic = self.parse_intrinsic_name(opcode_text, opcode_start)?;
                let arguments = self.parse_call_argument_segments(&mut segment_spans)?;
                let arguments = self.tree.add_arguments(&arguments);
                Instruction::Intrinsic {
                    destination,
                    intrinsic,
                    arguments,
                }
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
                    // binary ops
                    _ if opcode_text.parse::<BinaryOperator>().is_ok() => {
                        let operator = opcode_text
                            .parse()
                            .map_err(|_| ParseError::invalid("binary operator", opcode_start))?;
                        let left = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Binary {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }

                    // unary ops
                    _ if opcode_text.parse::<UnaryOperator>().is_ok() => {
                        let operator = opcode_text
                            .parse()
                            .map_err(|_| ParseError::invalid("unary operator", opcode_start))?;
                        let argument = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Unary {
                            destination,
                            operator,
                            argument,
                        }
                    }

                    // cast ops
                    _ if opcode_text.parse::<CastOperator>().is_ok() => {
                        let operator = opcode_text
                            .parse()
                            .map_err(|_| ParseError::invalid("cast operator", opcode_start))?;
                        let argument = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Arrow)?;
                        let to_type = self.parse_type_segment(&mut segment_spans)?;
                        Instruction::Cast {
                            destination,
                            operator,
                            argument,
                            to_type: to_type.into(),
                        }
                    }

                    // selection
                    "select" => {
                        let condition = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let then_value = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let else_value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Select {
                            destination,
                            condition,
                            then_value,
                            else_value,
                        }
                    }

                    // local operations
                    "local.get" => {
                        let local = self.parse_local_segment(&mut segment_spans)?;
                        Instruction::LocalGet { destination, local }
                    }
                    "local.address" => {
                        let local = self.parse_local_segment(&mut segment_spans)?;
                        Instruction::LocalAddr {
                            destination,
                            local,
                            result_type: destination_type.into(),
                        }
                    }
                    "pin" => {
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Pin {
                            destination,
                            value,
                            result_type: destination_type.into(),
                        }
                    }

                    // global operations
                    "global.address" => {
                        let global = self.parse_global_segment(&mut segment_spans)?;
                        Instruction::GlobalAddr {
                            destination,
                            global,
                            result_type: destination_type.into(),
                        }
                    }
                    "function.address" => {
                        let function = self.parse_function_segment(&mut segment_spans)?;
                        Instruction::FunctionAddr {
                            destination,
                            function,
                        }
                    }
                    "closure.bind" => {
                        let function = self.parse_function_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let environment = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ClosureBind {
                            destination,
                            function,
                            environment,
                        }
                    }
                    "closure.environment" => Instruction::ClosureEnvironment { destination },

                    // memory operations
                    "load" => {
                        let pointer = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Load {
                            destination,
                            pointer,
                            result_type: destination_type.into(),
                        }
                    }

                    // aggregate operations
                    "field.get" => {
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        Instruction::FieldGet {
                            destination,
                            aggregate,
                            index,
                        }
                    }
                    "field.address" => {
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        Instruction::FieldAddr {
                            destination,
                            aggregate,
                            index,
                            result_type: destination_type.into(),
                        }
                    }
                    "field.set" => {
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::FieldSet {
                            destination,
                            aggregate,
                            index,
                            value,
                        }
                    }
                    "element.get" => {
                        let array = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("element index", self.pos()))?;
                        Instruction::ElementGet {
                            destination,
                            array,
                            index,
                        }
                    }
                    "element.address" => {
                        let array = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ElementAddr {
                            destination,
                            array,
                            index,
                            result_type: destination_type.into(),
                        }
                    }
                    "slice" => {
                        let source = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let start = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Slice {
                            destination,
                            source,
                            start,
                            length,
                            result_type: destination_type.into(),
                        }
                    }
                    "element.set" => {
                        let array = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("element index", self.pos()))?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ElementSet {
                            destination,
                            array,
                            index,
                            value,
                        }
                    }
                    "struct" => {
                        let ty = self.parse_type_segment(&mut segment_spans)?;
                        let fields = self.parse_call_argument_segments(&mut segment_spans)?;
                        let fields = self.tree.add_arguments(&fields);
                        Instruction::Struct {
                            destination,
                            ty: ty.into(),
                            fields,
                        }
                    }
                    "tuple" => {
                        let ty = self.parse_type_segment(&mut segment_spans)?;
                        let elements = self.parse_call_argument_segments(&mut segment_spans)?;
                        let elements = self.tree.add_arguments(&elements);
                        Instruction::Tuple {
                            destination,
                            ty: ty.into(),
                            elements,
                        }
                    }
                    "array" => {
                        let ty = self.parse_type_segment(&mut segment_spans)?;
                        let elements = self.parse_call_argument_segments(&mut segment_spans)?;
                        let elements = self.tree.add_arguments(&elements);
                        Instruction::Array {
                            destination,
                            ty: ty.into(),
                            elements,
                        }
                    }
                    // vector operations
                    "vector.splat" => {
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::VectorSplat { destination, value }
                    }
                    "vector.extract" => {
                        let vector = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::VectorExtract {
                            destination,
                            vector,
                            index,
                        }
                    }
                    "vector.insert" => {
                        let vector = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
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
                    "tensor.splat" => {
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::TensorSplat { destination, value }
                    }
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
                    "tensor.extract" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let indices = self.parse_value_bracket_list()?;
                        let indices = self.tree.add_arguments(&indices);
                        Instruction::TensorExtract {
                            destination,
                            tensor,
                            indices,
                        }
                    }
                    "tensor.reshape" => {
                        let tensor = self.parse_value()?;
                        let shape = if self.eat_token_maybe(TokenType::Comma) {
                            let values = self.parse_named_value_group("shape")?;
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
                        let dimensions = self.parse_named_u32_group("dimensions")?;
                        Instruction::TensorBroadcast {
                            destination,
                            tensor,
                            dimensions,
                        }
                    }
                    "tensor.transpose" => {
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let permutation = self.parse_named_u32_group("permutation")?;
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
                        let offsets = self.parse_named_value_group("offsets")?;
                        self.eat_token(TokenType::Comma)?;
                        let sizes = self.parse_named_value_group("sizes")?;
                        self.eat_token(TokenType::Comma)?;
                        let strides = self.parse_named_value_group("strides")?;

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
                        let offsets = self.parse_named_value_group("offsets")?;
                        self.eat_token(TokenType::Comma)?;
                        let sizes = self.parse_named_value_group("sizes")?;
                        self.eat_token(TokenType::Comma)?;
                        let strides = self.parse_named_value_group("strides")?;

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
                        let value = self.parse_named_single_value_group("value")?;
                        self.eat_token(TokenType::Comma)?;
                        let low = self.parse_named_value_group("low")?;
                        self.eat_token(TokenType::Comma)?;
                        let high = self.parse_named_value_group("high")?;
                        self.eat_token(TokenType::Comma)?;
                        let interior = self.parse_named_value_group("interior")?;

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
                        let tensors = self.parse_named_value_group("tensors")?;
                        self.eat_token(TokenType::Comma)?;
                        let axis = self.parse_named_single_u32_group("axis")?;
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
                        let axes = self.parse_named_u32_group("axes")?;
                        Instruction::TensorReduce {
                            destination,
                            operator,
                            tensor,
                            initial,
                            axes,
                        }
                    }
                    "tensor.indexReduce" => {
                        let operator = self.parse_tensor_index_reduce_operator()?;
                        self.eat_token(TokenType::Comma)?;
                        let tensor = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let axis = self.parse_named_single_u32_group("axis")?;
                        let tie_break = self
                            .parse_optional_tensor_index_tie_break()?
                            .unwrap_or(TensorIndexTieBreak::First);
                        Instruction::TensorIndexReduce {
                            destination,
                            operator,
                            tensor,
                            axis,
                            tie_break,
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
                        let slice_sizes = self.parse_named_u32_group("sliceSizes")?;
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
                    "new.zeroed" => {
                        let layout = self.parse_type()?;
                        Instruction::NewZeroed {
                            destination,
                            layout: layout.into(),
                            result_type: destination_type.into(),
                        }
                    }
                    "new.uninit" => {
                        let layout = self.parse_type()?;
                        Instruction::NewUninit {
                            destination,
                            layout: layout.into(),
                            result_type: destination_type.into(),
                        }
                    }
                    "new.complete" => {
                        let value = self.parse_value()?;
                        Instruction::NewComplete {
                            destination,
                            value,
                            result_type: destination_type.into(),
                        }
                    }
                    "new.slice.zeroed" => {
                        let element = self.parse_type()?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value()?;
                        Instruction::NewSliceZeroed {
                            destination,
                            element: element.into(),
                            length,
                            result_type: destination_type.into(),
                        }
                    }
                    "new.slice.uninit" => {
                        let element = self.parse_type()?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value()?;
                        Instruction::NewSliceUninit {
                            destination,
                            element: element.into(),
                            length,
                            result_type: destination_type.into(),
                        }
                    }
                    "frame.alloc.zeroed" => {
                        let layout = self.parse_type()?;
                        Instruction::FrameAllocZeroed {
                            destination,
                            layout: layout.into(),
                            result_type: destination_type.into(),
                        }
                    }
                    "frame.alloc.uninit" => {
                        let layout = self.parse_type()?;
                        Instruction::FrameAllocUninit {
                            destination,
                            layout: layout.into(),
                            result_type: destination_type.into(),
                        }
                    }

                    // atomic memory operations
                    "atomic.load" => {
                        let pointer = self.parse_value()?;
                        let access = self.parse_atomic_access()?;
                        Instruction::AtomicLoad {
                            destination,
                            pointer,
                            result_type: destination_type.into(),
                            access,
                        }
                    }
                    "atomic.cas" | "atomic.cas.weak" => {
                        let pointer = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let expected = self.parse_value()?;
                        self.eat_token(TokenType::Comma)?;
                        let new_value = self.parse_value()?;
                        let access = self.parse_atomic_compare_exchange_access()?;
                        Instruction::AtomicCompareExchange {
                            destination,
                            pointer,
                            expected,
                            new_value,
                            is_weak: opcode_text == "atomic.cas.weak",
                            access,
                        }
                    }
                    _ if opcode_text.starts_with("atomic.rmw.") => {
                        let operator = self.parse_atomic_rmw_operator(opcode_text, opcode_start)?;
                        let pointer = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        let access = self.parse_atomic_access()?;
                        Instruction::AtomicRmw {
                            destination,
                            operator,
                            pointer,
                            value,
                            access,
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
        self.apply_instruction_spans(
            instruction_id,
            self.span_from_parse_start(instruction_start),
            destination_span,
            destination_type_span,
            &segment_spans,
        );
        Ok(instruction_id)
    }

    /// Apply source ownership spans to one parsed instruction.
    fn apply_instruction_spans(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction_span: Span,
        main_span: Option<Span>,
        type_span: Option<Span>,
        segment_spans: &[Span],
    ) {
        // enclosing span
        self.tree.set_text_span(instruction_id, instruction_span);

        // focal span
        if let Some(main_span) = main_span.or_else(|| segment_spans.first().copied()) {
            self.tree.set_main_span(instruction_id, main_span);
        }

        // destination type
        if let Some(type_span) = type_span {
            self.tree.set_side_span(
                instruction_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                type_span,
            );
        }

        // ordered source parts
        self.set_segment_spans(instruction_id, segment_spans);
    }

    /// Parse a bracketed list of values and append each element as one source segment.
    fn parse_value_bracket_list(&mut self) -> ParseResult<Vec<ValueReference>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        while !self.peek_token(TokenType::CloseBracket) {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a bracketed list of values and append each element as one source segment.
    fn parse_value_bracket_list_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<Vec<ValueReference>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        while !self.peek_token(TokenType::CloseBracket) {
            values.push(self.parse_value_segment(segment_spans)?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a parenthesized list of values.
    fn parse_value_paren_list(&mut self) -> ParseResult<Vec<ValueReference>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        while !self.peek_token(TokenType::CloseParenthesis) {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;
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

    /// Parse a parenthesized list of u32 values.
    fn parse_u32_paren_list(&mut self) -> ParseResult<Vec<u32>> {
        let values = self.parse_int_paren_list()?;
        values
            .into_iter()
            .map(|value| {
                u32::try_from(value).map_err(|_| ParseError::invalid("u32 literal", self.pos()))
            })
            .collect()
    }

    /// Parse a parenthesized list of u64 values.
    fn parse_u64_paren_list(&mut self) -> ParseResult<Vec<u64>> {
        let values = self.parse_int_paren_list()?;
        values
            .into_iter()
            .map(|value| {
                u64::try_from(value).map_err(|_| ParseError::invalid("u64 literal", self.pos()))
            })
            .collect()
    }

    /// Parse a bracketed list of integer values.
    fn parse_int_bracket_list(&mut self) -> ParseResult<Vec<i128>> {
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

    /// Parse a parenthesized list of integer values.
    fn parse_int_paren_list(&mut self) -> ParseResult<Vec<i128>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        while !self.peek_token(TokenType::CloseParenthesis) {
            let value = self.parse_int_literal()?;
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(values)
    }

    /// Parse a parenthesized list of boolean values.
    fn parse_bool_paren_list(&mut self) -> ParseResult<Vec<bool>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        while !self.peek_token(TokenType::CloseParenthesis) {
            let token = self.eat_token(TokenType::BooleanLiteral)?;
            let value = self.tree.source_text(token.span) == "true";
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(values)
    }

    /// Parse one named value group like `name(v0, v1)`.
    fn parse_named_value_group(&mut self, name: &str) -> ParseResult<Vec<ValueReference>> {
        self.eat_named_group(name)?;
        self.parse_value_paren_list()
    }

    /// Parse one named single value group like `name(v0)`.
    fn parse_named_single_value_group(&mut self, name: &str) -> ParseResult<ValueReference> {
        let values = self.parse_named_value_group(name)?;
        let [value] = values.as_slice() else {
            return Err(ParseError::invalid(name, self.pos()));
        };

        Ok(*value)
    }

    /// Parse one named u32 group like `name(0, 1)`.
    fn parse_named_u32_group(&mut self, name: &str) -> ParseResult<Vec<u32>> {
        self.eat_named_group(name)?;
        self.parse_u32_paren_list()
    }

    /// Parse one named single u32 group like `name(0)`.
    fn parse_named_single_u32_group(&mut self, name: &str) -> ParseResult<u32> {
        let values = self.parse_named_u32_group(name)?;
        let [value] = values.as_slice() else {
            return Err(ParseError::invalid(name, self.pos()));
        };

        Ok(*value)
    }

    /// Parse one MIR place.
    fn parse_place_part(&mut self) -> ParseResult<(Place, Span)> {
        let start = self.pos();
        if !self.eat_identifier_text("place") {
            let (value, span) = self.parse_value_reference_part()?;

            return Ok((Place::value(value), span));
        }

        self.eat_token(TokenType::OpenParenthesis)?;
        let origin = self.parse_place_origin()?;
        let mut place = Place::new(origin);

        while self.eat_token_maybe(TokenType::Comma) {
            let projection = self.parse_place_projection()?;
            place.push(projection);
        }

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok((place, self.span_from_parse_start(start)))
    }

    /// Parse one MIR place origin.
    fn parse_place_origin(&mut self) -> ParseResult<PlaceOrigin> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("place origin", self.pos()))?;

        match self.token_type(token) {
            TokenType::Value => {
                let (value, _) = self.parse_value_reference_part()?;

                Ok(PlaceOrigin::Value(value))
            }
            TokenType::LocalReference => {
                let (local, _) = self.parse_local_ref_part()?;

                Ok(PlaceOrigin::Local(local))
            }
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                if let Some(value) = self.value_name_map.get(&name).copied() {
                    self.bump();

                    return Ok(PlaceOrigin::Value(value.into()));
                }

                let (global, _) = self.parse_global_reference_part()?;

                Ok(PlaceOrigin::Global(global))
            }
            _ => Err(ParseError::unexpected_token("place origin", token)),
        }
    }

    /// Parse one MIR place projection.
    fn parse_place_projection(&mut self) -> ParseResult<PlaceProjection> {
        if self.eat_identifier_text("field") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let index = self.parse_int_literal()?;
            let index =
                u32::try_from(index).map_err(|_| ParseError::invalid("place index", self.pos()))?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(PlaceProjection::Field { index });
        }

        if self.eat_identifier_text("element") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let index = self.parse_int_literal()?;
            let index =
                u32::try_from(index).map_err(|_| ParseError::invalid("place index", self.pos()))?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(PlaceProjection::Element { index });
        }

        if self.eat_identifier_text("index") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let index = self.parse_value()?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(PlaceProjection::Index { index });
        }

        if self.eat_identifier_text("range") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let start = self.parse_value()?;
            self.eat_token(TokenType::Comma)?;
            let length = self.parse_value()?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(PlaceProjection::Range { start, length });
        }

        Err(ParseError::invalid("place projection", self.pos()))
    }

    /// Parse one named group header like `name(`.
    fn eat_named_group(&mut self, name: &str) -> ParseResult<()> {
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != name {
            return Err(ParseError::invalid(name, token.start));
        }

        Ok(())
    }

    /// Parse a vector reduction operator.
    fn parse_vector_reduce_operator(&mut self) -> ParseResult<VectorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = VectorReduceOperator::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("vector reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a vector conversion mode.
    fn parse_vector_convert_mode(&mut self) -> ParseResult<VectorConvertMode> {
        // parse the mode token
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = VectorConvertMode::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("vector convert mode", token.start))?;
        Ok(mode)
    }

    /// Parse a tensor conversion mode.
    fn parse_tensor_convert_mode(&mut self) -> ParseResult<TensorConvertMode> {
        // parse the mode token
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = TensorConvertMode::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("tensor convert mode", token.start))?;
        Ok(mode)
    }

    /// Parse a tensor reduction operator.
    fn parse_tensor_reduce_operator(&mut self) -> ParseResult<TensorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = TensorReduceOperator::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("tensor reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a tensor index reduction operator.
    fn parse_tensor_index_reduce_operator(&mut self) -> ParseResult<TensorIndexReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = TensorIndexReduceOperator::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("tensor index reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a comparison operator for vector or tensor operations.
    fn parse_compare_operator(&mut self) -> ParseResult<BinaryOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = BinaryOperator::from_str(self.tree.source_text(token.span))
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
        if self.tree.source_text(token.span) != "mode" {
            return Err(ParseError::invalid("mode", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let mode_text = self.tree.source_text(token.span).to_string();
        let mode_start = token.start;
        self.eat_token(TokenType::CloseParenthesis)?;
        let mode = TensorScatterMode::parse(&mode_text)
            .ok_or_else(|| ParseError::invalid("scatter mode", mode_start))?;
        Ok(Some(mode))
    }

    /// Parse an optional tensor index reduction tie break.
    fn parse_optional_tensor_index_tie_break(
        &mut self,
    ) -> ParseResult<Option<TensorIndexTieBreak>> {
        // check for a comma followed by the tie break
        if !self.peek_token(TokenType::Comma) {
            return Ok(None);
        }
        self.eat_token(TokenType::Comma)?;
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != "tieBreak" {
            return Err(ParseError::invalid("tieBreak", token.start));
        }

        // parse the named value
        self.eat_token(TokenType::OpenParenthesis)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let tie_break_text = self.tree.source_text(token.span).to_string();
        let tie_break_start = token.start;
        self.eat_token(TokenType::CloseParenthesis)?;
        let tie_break = TensorIndexTieBreak::parse(&tie_break_text)
            .ok_or_else(|| ParseError::invalid("tensor index reduce tie break", tie_break_start))?;
        Ok(Some(tie_break))
    }

    /// Parse tensor dot dimension numbers.
    fn parse_tensor_dot_dimensions(&mut self) -> ParseResult<TensorDotDimensionNumbers> {
        // parse the header
        let token = self.eat_token(TokenType::Identifier)?;
        if self.tree.source_text(token.span) != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;

        // parse named lists
        let mut lhs_batch = None;
        let mut rhs_batch = None;
        let mut lhs_contracting = None;
        let mut rhs_contracting = None;
        while !self.peek_token(TokenType::CloseParenthesis) {
            let key_token = self.eat_token(TokenType::Identifier)?;
            let key_text = self.tree.source_text(key_token.span).to_string();
            let key_start = key_token.start;
            let list = self.parse_u32_paren_list()?;
            match key_text.as_str() {
                "lhsBatch" => lhs_batch = Some(list),
                "rhsBatch" => rhs_batch = Some(list),
                "lhsContract" => lhs_contracting = Some(list),
                "rhsContract" => rhs_contracting = Some(list),
                _ => return Err(ParseError::invalid("dot dimension key", key_start)),
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        // validate required keys
        let lhs_batch = lhs_batch.ok_or_else(|| ParseError::invalid("lhsBatch", self.pos()))?;
        let rhs_batch = rhs_batch.ok_or_else(|| ParseError::invalid("rhsBatch", self.pos()))?;
        let lhs_contracting =
            lhs_contracting.ok_or_else(|| ParseError::invalid("lhsContract", self.pos()))?;
        let rhs_contracting =
            rhs_contracting.ok_or_else(|| ParseError::invalid("rhsContract", self.pos()))?;

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
        if self.tree.source_text(token.span) != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;

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
        while !self.peek_token(TokenType::CloseParenthesis) {
            let key_token = self.eat_token(TokenType::Identifier)?;
            match self.tree.source_text(key_token.span) {
                "inputBatch" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    input_batch = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "inputFeature" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    input_feature = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "inputSpatial" => input_spatial = Some(self.parse_u32_paren_list()?),
                "kernelInputFeature" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    kernel_input_feature = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "kernelOutputFeature" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    kernel_output_feature = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "kernelSpatial" => kernel_spatial = Some(self.parse_u32_paren_list()?),
                "outputBatch" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    output_batch = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "outputFeature" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    output_feature = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                "outputSpatial" => output_spatial = Some(self.parse_u32_paren_list()?),
                _ => {
                    return Err(ParseError::invalid(
                        "convolution dimension key",
                        key_token.start,
                    ));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        // validate required keys
        let input_batch =
            input_batch.ok_or_else(|| ParseError::invalid("inputBatch", self.pos()))?;
        let input_feature =
            input_feature.ok_or_else(|| ParseError::invalid("inputFeature", self.pos()))?;
        let input_spatial =
            input_spatial.ok_or_else(|| ParseError::invalid("inputSpatial", self.pos()))?;
        let kernel_input_feature = kernel_input_feature
            .ok_or_else(|| ParseError::invalid("kernelInputFeature", self.pos()))?;
        let kernel_output_feature = kernel_output_feature
            .ok_or_else(|| ParseError::invalid("kernelOutputFeature", self.pos()))?;
        let kernel_spatial =
            kernel_spatial.ok_or_else(|| ParseError::invalid("kernelSpatial", self.pos()))?;
        let output_batch =
            output_batch.ok_or_else(|| ParseError::invalid("outputBatch", self.pos()))?;
        let output_feature =
            output_feature.ok_or_else(|| ParseError::invalid("outputFeature", self.pos()))?;
        let output_spatial =
            output_spatial.ok_or_else(|| ParseError::invalid("outputSpatial", self.pos()))?;

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
        // derive omitted window defaults from the declared spatial rank
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

        if self.eat_token_maybe(TokenType::Comma) {
            let token = self.eat_token(TokenType::Identifier)?;
            if self.tree.source_text(token.span) != "window" {
                return Err(ParseError::invalid("window", token.start));
            }
            self.eat_token(TokenType::OpenParenthesis)?;

            while !self.peek_token(TokenType::CloseParenthesis) {
                let key_token = self.eat_token(TokenType::Identifier)?;
                match self.tree.source_text(key_token.span) {
                    "strides" => strides = Some(self.parse_u64_paren_list()?),
                    "paddingLow" => padding_low = Some(self.parse_u64_paren_list()?),
                    "paddingHigh" => padding_high = Some(self.parse_u64_paren_list()?),
                    "lhsDilation" => lhs_dilation = Some(self.parse_u64_paren_list()?),
                    "rhsDilation" => rhs_dilation = Some(self.parse_u64_paren_list()?),
                    "windowReversal" => window_reversal = Some(self.parse_bool_paren_list()?),
                    _ => {
                        return Err(ParseError::invalid(
                            "convolution window key",
                            key_token.start,
                        ));
                    }
                }

                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            self.eat_token(TokenType::CloseParenthesis)?;
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

        if self.eat_token_maybe(TokenType::Comma) {
            let token = self.eat_token(TokenType::Identifier)?;
            if self.tree.source_text(token.span) != "groups" {
                return Err(ParseError::invalid("groups", token.start));
            }
            self.eat_token(TokenType::OpenParenthesis)?;

            while !self.peek_token(TokenType::CloseParenthesis) {
                let key_token = self.eat_token(TokenType::Identifier)?;
                let key_text = self.tree.source_text(key_token.span).to_string();
                let key_start = key_token.start;
                let value = {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    let value = self.parse_int_as_u32()?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                    value
                };

                match key_text.as_str() {
                    "feature" => feature_group_count = Some(value),
                    "batch" => batch_group_count = Some(value),
                    _ => return Err(ParseError::invalid("convolution group key", key_start)),
                }

                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            self.eat_token(TokenType::CloseParenthesis)?;
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
        if self.tree.source_text(token.span) != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;

        // parse named lists
        let mut offset_dims = None;
        let mut collapsed_slice_dims = None;
        let mut start_index_map = None;
        let mut index_vector_dim = None;
        while !self.peek_token(TokenType::CloseParenthesis) {
            let key_token = self.eat_token(TokenType::Identifier)?;
            match self.tree.source_text(key_token.span) {
                "offsetDims" => offset_dims = Some(self.parse_u32_paren_list()?),
                "collapsedSliceDims" => collapsed_slice_dims = Some(self.parse_u32_paren_list()?),
                "startIndexMap" => start_index_map = Some(self.parse_u32_paren_list()?),
                "indexVectorDim" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    index_vector_dim = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                _ => return Err(ParseError::invalid("gather dimension key", key_token.start)),
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        // validate required keys
        let offset_dims =
            offset_dims.ok_or_else(|| ParseError::invalid("offsetDims", self.pos()))?;
        let collapsed_slice_dims = collapsed_slice_dims
            .ok_or_else(|| ParseError::invalid("collapsedSliceDims", self.pos()))?;
        let start_index_map =
            start_index_map.ok_or_else(|| ParseError::invalid("startIndexMap", self.pos()))?;
        let index_vector_dim =
            index_vector_dim.ok_or_else(|| ParseError::invalid("indexVectorDim", self.pos()))?;

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
        if self.tree.source_text(token.span) != "dims" {
            return Err(ParseError::invalid("dims", token.start));
        }
        self.eat_token(TokenType::OpenParenthesis)?;

        // parse named lists
        let mut update_window_dims = None;
        let mut inserted_window_dims = None;
        let mut scatter_dims_to_operand_dims = None;
        let mut index_vector_dim = None;
        while !self.peek_token(TokenType::CloseParenthesis) {
            let key_token = self.eat_token(TokenType::Identifier)?;
            match self.tree.source_text(key_token.span) {
                "updateWindowDims" => update_window_dims = Some(self.parse_u32_paren_list()?),
                "insertedWindowDims" => inserted_window_dims = Some(self.parse_u32_paren_list()?),
                "scatterDimsToOperandDims" => {
                    scatter_dims_to_operand_dims = Some(self.parse_u32_paren_list()?);
                }
                "indexVectorDim" => {
                    self.eat_token(TokenType::OpenParenthesis)?;
                    index_vector_dim = Some(self.parse_int_as_u32()?);
                    self.eat_token(TokenType::CloseParenthesis)?;
                }
                _ => {
                    return Err(ParseError::invalid(
                        "scatter dimension key",
                        key_token.start,
                    ));
                }
            }
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        // validate required keys
        let update_window_dims = update_window_dims
            .ok_or_else(|| ParseError::invalid("updateWindowDims", self.pos()))?;
        let inserted_window_dims = inserted_window_dims
            .ok_or_else(|| ParseError::invalid("insertedWindowDims", self.pos()))?;
        let scatter_dims_to_operand_dims = scatter_dims_to_operand_dims
            .ok_or_else(|| ParseError::invalid("scatterDimsToOperandDims", self.pos()))?;
        let index_vector_dim =
            index_vector_dim.ok_or_else(|| ParseError::invalid("indexVectorDim", self.pos()))?;

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
    ) -> ParseResult<(FunctionReference, Vec<ValueReference>, TypeReference)> {
        let mut segment_spans = Vec::new();
        self.parse_direct_call_target_segments(&mut segment_spans)
    }

    /// Parse one direct call target and arguments with source segments.
    pub(super) fn parse_direct_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(FunctionReference, Vec<ValueReference>, TypeReference)> {
        let function = self.parse_function_segment(segment_spans)?;
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_required_call_signature_segment(segment_spans)?;
        Ok((function, arguments, signature))
    }

    /// Parse one virtual call target and signature.
    pub(super) fn parse_class_call_target(
        &mut self,
    ) -> ParseResult<(
        ValueReference,
        TypeReference,
        DispatchSlot,
        Vec<ValueReference>,
        TypeReference,
    )> {
        let mut segment_spans = Vec::new();
        self.parse_class_call_target_segments(&mut segment_spans)
    }

    /// Parse one virtual call target and signature with source segments.
    pub(super) fn parse_class_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(
        ValueReference,
        TypeReference,
        DispatchSlot,
        Vec<ValueReference>,
        TypeReference,
    )> {
        let receiver = self.parse_value_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let class = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let slot = self.parse_int_segment(segment_spans)?;
        let slot = u32::try_from(slot)
            .map_err(|_| ParseError::invalid("virtual dispatch slot", self.pos()))?;
        let slot = DispatchSlot::new(slot);
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_required_call_signature_segment(segment_spans)?;

        Ok((receiver, class.into(), slot, arguments, signature))
    }

    /// Parse one dynamic call target and signature.
    pub(super) fn parse_dynamic_call_target(
        &mut self,
    ) -> ParseResult<(
        ValueReference,
        TypeReference,
        DispatchSlot,
        Vec<ValueReference>,
        TypeReference,
    )> {
        let mut segment_spans = Vec::new();
        self.parse_dynamic_call_target_segments(&mut segment_spans)
    }

    /// Parse one dynamic call target and signature with source segments.
    pub(super) fn parse_dynamic_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(
        ValueReference,
        TypeReference,
        DispatchSlot,
        Vec<ValueReference>,
        TypeReference,
    )> {
        let receiver = self.parse_value_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let constraint = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let slot = self.parse_int_segment(segment_spans)?;
        let slot =
            u32::try_from(slot).map_err(|_| ParseError::invalid("dynamic slot", self.pos()))?;
        let slot = DispatchSlot::new(slot);
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_required_call_signature_segment(segment_spans)?;

        Ok((receiver, constraint.into(), slot, arguments, signature))
    }

    /// Parse one indirect call target and signature.
    pub(super) fn parse_indirect_call_target(
        &mut self,
    ) -> ParseResult<(ValueReference, Vec<ValueReference>, TypeReference)> {
        let mut segment_spans = Vec::new();
        self.parse_indirect_call_target_segments(&mut segment_spans)
    }

    /// Parse one indirect call target and signature with source segments.
    pub(super) fn parse_indirect_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(ValueReference, Vec<ValueReference>, TypeReference)> {
        let callee = self.parse_value_segment(segment_spans)?;
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_required_call_signature_segment(segment_spans)?;

        Ok((callee, arguments, signature))
    }

    /// Parse one required call signature.
    fn parse_required_call_signature_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<TypeReference> {
        let signature_start = self.pos();
        self.eat_token(TokenType::Colon)?;
        self.eat_token(TokenType::OpenParenthesis)?;

        let mut parameters = Vec::new();
        while !self.peek_token(TokenType::CloseParenthesis) {
            parameters.push(self.parse_type()?.into());
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;
        self.eat_token(TokenType::Arrow)?;
        let result = self.parse_type()?;
        let signature_span = self.span_from_parse_start(signature_start);
        segment_spans.push(signature_span);
        let borrow_obligations = self.parse_borrow_obligations()?;

        Ok(self
            .intern_type(Type::FunctionSignature {
                parameters,
                result: result.into(),
                borrow_obligations,
            })?
            .into())
    }

    /// Parse one atomic access suffix.
    fn parse_atomic_access(&mut self) -> ParseResult<AtomicAccess> {
        self.eat_token(TokenType::Comma)?;

        let ordering = self.parse_memory_ordering()?;
        let mut access = AtomicAccess::ordered(ordering);

        while self.eat_token_maybe(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic access", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                access.scope = self.parse_sync_scope_clause()?;
            } else if token_text == "volatile" {
                self.eat_token(TokenType::Identifier)?;
                access.is_volatile = true;
            } else {
                return Err(ParseError::invalid("atomic access", self.pos()));
            }
        }

        Ok(access)
    }

    /// Parse one compare exchange access suffix.
    fn parse_atomic_compare_exchange_access(&mut self) -> ParseResult<CompareExchangeAccess> {
        self.eat_token(TokenType::Comma)?;

        let ordering = self.parse_memory_ordering()?;
        let mut success = AtomicAccess::ordered(ordering);
        let mut failure_ordering = CompareExchangeAccess::default_failure_ordering(ordering);

        while self.eat_token_maybe(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic compare exchange", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                success.scope = self.parse_sync_scope_clause()?;
            } else if token_text == "volatile" {
                self.eat_token(TokenType::Identifier)?;
                success.is_volatile = true;
            } else if token_text == "failure" {
                self.eat_token(TokenType::Identifier)?;
                self.eat_token(TokenType::OpenParenthesis)?;
                failure_ordering = self.parse_memory_ordering()?;
                self.eat_token(TokenType::CloseParenthesis)?;
            } else {
                return Err(ParseError::invalid(
                    "atomic compare exchange access",
                    self.pos(),
                ));
            }
        }

        Ok(CompareExchangeAccess::new(success, failure_ordering))
    }

    /// Parse one fence access suffix.
    fn parse_fence_access(&mut self) -> ParseResult<FenceAccess> {
        let ordering = self.parse_memory_ordering()?;
        let mut access = FenceAccess::ordered(ordering);

        while self.eat_token_maybe(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic fence", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                access.scope = self.parse_sync_scope_clause()?;
            } else if token_text == "memory" {
                access.memory_scope = self.parse_memory_scope_clause()?;
            } else {
                access.flags = self.parse_memory_flags()?;
            }
        }

        Ok(access)
    }

    /// Parse one `scope(...)` synchronization clause.
    fn parse_sync_scope_clause(&mut self) -> ParseResult<SyncScope> {
        self.eat_token(TokenType::Identifier)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let scope = self.parse_sync_scope()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(scope)
    }

    /// Parse one `memory(...)` fence clause.
    fn parse_memory_scope_clause(&mut self) -> ParseResult<MemoryScope> {
        self.eat_token(TokenType::Identifier)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let scope = self.parse_memory_scope()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(scope)
    }

    /// Parse one memory ordering like `sequentiallyConsistent`.
    fn parse_memory_ordering(&mut self) -> ParseResult<MemoryOrdering> {
        let token_start = self.pos();
        let token = self.eat_token(TokenType::Identifier)?;
        self.tree
            .source_text(token.span)
            .parse::<MemoryOrdering>()
            .map_err(|_| ParseError::invalid("memory ordering", token_start))
    }

    /// Parse one synchronization scope like `device`.
    fn parse_sync_scope(&mut self) -> ParseResult<SyncScope> {
        let token_start = self.pos();
        let token = self.eat_token(TokenType::Identifier)?;
        self.tree
            .source_text(token.span)
            .parse::<SyncScope>()
            .map_err(|_| ParseError::invalid("synchronization scope", token_start))
    }

    /// Parse one memory scope like `device`.
    fn parse_memory_scope(&mut self) -> ParseResult<MemoryScope> {
        let token_start = self.pos();
        let token = self.eat_token(TokenType::Identifier)?;
        self.tree
            .source_text(token.span)
            .parse::<MemoryScope>()
            .map_err(|_| ParseError::invalid("memory scope", token_start))
    }

    /// Parse memory flags for one fence.
    fn parse_memory_flags(&mut self) -> ParseResult<MemoryFlags> {
        // flags state
        let mut spaces = SpaceSet::NONE;
        let mut has_space = false;
        let mut is_space_locked = false;
        let mut makes_available = false;
        let mut makes_visible = false;
        let is_list = self.eat_token_maybe(TokenType::OpenBracket);

        // parse one or more flag items
        loop {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("memory flags", self.pos()))?;
            if !matches!(
                self.token_type(token),
                TokenType::Identifier | TokenType::Global | TokenType::Local
            ) {
                return Err(ParseError::unexpected(
                    "memory flags",
                    self.token_type(token),
                    token.start,
                ));
            }

            let token_text = self.tree.source_text(token.span).to_string();
            let token_start = token.start;
            self.bump();

            // flags and spaces
            match token_text.as_str() {
                "makeAvailable" => {
                    makes_available = true;
                }
                "makeVisible" => {
                    makes_visible = true;
                }
                _ => {
                    let space = self.parse_memory_space(&token_text, token_start)?;
                    if space == SpaceSet::ANY || space == SpaceSet::NONE {
                        if has_space && !is_space_locked {
                            return Err(ParseError::new(
                                "memory flags cannot mix any/none with other spaces",
                                token_start,
                            ));
                        }

                        spaces = space;
                        has_space = true;
                        is_space_locked = true;
                    } else {
                        if is_space_locked {
                            return Err(ParseError::new(
                                "memory flags cannot mix any/none with other spaces",
                                token_start,
                            ));
                        }

                        if !has_space {
                            spaces = SpaceSet::NONE;
                            has_space = true;
                        }

                        spaces.insert(space);
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

        // default the space set when omitted
        if !has_space {
            spaces = SpaceSet::ANY;
        }

        Ok(MemoryFlags::with_flags(
            spaces,
            makes_available,
            makes_visible,
        ))
    }

    /// Parse one atomic read-modify-write opcode suffix.
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
