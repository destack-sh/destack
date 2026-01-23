use crate::{
    ArgumentSlice, AtomicScope, BinaryOperator, CastOperator, CheckConstraint, CheckTarget,
    DispatchTableId, Function, Instruction, Intrinsic, LocalNodeId, MemoryLocationSet,
    MemoryOrdering,
    MemoryScope, MemorySemantics, SwitchCase, Terminator, TensorConvolutionDimensionNumbers,
    TensorConvolutionWindow, TensorDotDimensionNumbers, TensorGatherDimensionNumbers,
    TensorReduceOperator, TensorScatterDimensionNumbers, TensorScatterMode, Type, UnaryOperator,
    Value, VectorReduceOperator,
};

use super::constant::{parse_intrinsic_name, parse_memory_location};
use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Parse an instruction.
    ///
    /// Instructions come in two forms:
    /// - With destination: `vN = opcode ...`
    /// - Without destination: `opcode ...`
    pub(super) fn eat_instruction(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        // check if this is an instruction with destination (vN = ...)
        let has_destination = self.peek_token(TokenType::Value);
        if has_destination {
            self.eat_instruction_with_destination()
        } else {
            self.eat_instruction_without_destination()
        }
    }

    /// Parse an instruction that produces a value: `vN = opcode ...`
    fn eat_instruction_with_destination(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        let (destination, destination_type, destination_span) = self.parse_typed_destination()?;
        self.record_value_type(destination, destination_type);
        self.eat_token(TokenType::Equals)?;

        // opcode can be an identifier or the `struct` keyword
        let (opcode_text, opcode_start) = self.eat_opcode()?;

        let instruction = match opcode_text {
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
                let result_type = destination_type;
                Instruction::LocalAddr {
                    destination,
                    local,
                    result_type,
                }
            }

            // global operations
            "global.addr" => {
                let global = self.parse_global_reference()?;
                let result_type = destination_type;
                Instruction::GlobalAddr {
                    destination,
                    global,
                    result_type,
                }
            }
            "global.const" => {
                let global = self.parse_global_reference()?;
                Instruction::GlobalConst {
                    destination,
                    global,
                }
            }

            // memory operations
            "load" => {
                let pointer = self.parse_value()?;
                let result_type = destination_type;
                Instruction::Load {
                    destination,
                    pointer,
                    result_type,
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
                let result_type = destination_type;
                Instruction::FieldAddr {
                    destination,
                    aggregate,
                    index,
                    result_type,
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
                let result_type = destination_type;
                Instruction::ElementAddr {
                    destination,
                    array,
                    index,
                    result_type,
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
                let args = self.parse_call_arguments()?;
                let fields = self.tree.add_arguments(&args);
                Instruction::Struct {
                    destination,
                    ty,
                    fields,
                }
            }
            "tuple" => {
                let ty = self.parse_type()?;
                let args = self.parse_call_arguments()?;
                let elements = self.tree.add_arguments(&args);
                Instruction::Tuple {
                    destination,
                    ty,
                    elements,
                }
            }
            "array" => {
                let ty = self.parse_type()?;
                let args = self.parse_call_arguments()?;
                let elements = self.tree.add_arguments(&args);
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
            "tensor.slice" => {
                let tensor = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let offsets = self.parse_named_value_list("offsets")?;
                self.eat_token(TokenType::Comma)?;
                let sizes = self.parse_named_value_list("sizes")?;
                self.eat_token(TokenType::Comma)?;
                let strides = self.parse_named_value_list("strides")?;
                let arguments = self.pack_tensor_ranges(&offsets, &sizes, &strides)?;
                let offsets_count = self.parse_u16_count(offsets.len(), "offsets count")?;
                let sizes_count = self.parse_u16_count(sizes.len(), "sizes count")?;
                let strides_count = self.parse_u16_count(strides.len(), "strides count")?;
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
                let value = self.parse_named_value("value")?;
                self.eat_token(TokenType::Comma)?;
                let low = self.parse_named_value_list("low")?;
                self.eat_token(TokenType::Comma)?;
                let high = self.parse_named_value_list("high")?;
                self.eat_token(TokenType::Comma)?;
                let interior = self.parse_named_value_list("interior")?;
                let arguments = self.pack_tensor_padding(&low, &high, &interior)?;
                let low_count = self.parse_u16_count(low.len(), "low padding count")?;
                let high_count = self.parse_u16_count(high.len(), "high padding count")?;
                let interior_count =
                    self.parse_u16_count(interior.len(), "interior padding count")?;
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
                let axis = self.parse_named_u32("axis")?;
                let tensors = self.tree.add_arguments(&tensors);
                Instruction::TensorConcat {
                    destination,
                    tensors,
                    axis,
                }
            }
            "tensor.reduce" => {
                let operator = self.parse_tensor_reduce_operator()?;
                self.eat_token(TokenType::Comma)?;
                let tensor = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let initial = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let axes = self.parse_named_u32_list("axes")?;
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
                let slice_sizes = self.parse_named_u32_list("slice_sizes")?;
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
                    .parse_optional_named_scatter_mode()?
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
                let tensor = self.parse_value()?;
                Instruction::TensorConvert { destination, tensor }
            }

            // function calls
            "call" => {
                let function = self.parse_function_reference()?;
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                let signature = if self.eat_token_maybe(TokenType::Arrow) {
                    self.parse_type()?
                } else {
                    self.signature_type_for_function(function)?
                };
                Instruction::Call {
                    destination: Some(destination),
                    function,
                    arguments,
                    signature,
                    effects: None,
                }
            }
            "call.virtual" => {
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallVirtual {
                    destination: Some(destination),
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                    effects: None,
                }
            }
            "call.interface" => {
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallInterface {
                    destination: Some(destination),
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                    effects: None,
                }
            }
            "call.indirect" => {
                let callee = self.parse_value()?;
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallIndirect {
                    destination: Some(destination),
                    callee,
                    arguments,
                    signature,
                    effects: None,
                }
            }

            // allocation operations
            "managed.alloc" => {
                let layout = self.parse_type()?;
                let result_type = destination_type;
                Instruction::ManagedAlloc {
                    destination,
                    layout,
                    result_type,
                }
            }
            "managed.alloc_array" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                let result_type = destination_type;
                Instruction::ManagedAllocArray {
                    destination,
                    element,
                    length,
                    result_type,
                }
            }
            "raw.alloc" => {
                let layout = self.parse_type()?;
                let result_type = destination_type;
                Instruction::RawAlloc {
                    destination,
                    layout,
                    result_type,
                }
            }
            "stack.alloc" => {
                let layout = self.parse_type()?;
                let result_type = destination_type;
                Instruction::StackAlloc {
                    destination,
                    layout,
                    result_type,
                }
            }

            // intrinsics: intrinsic.{name}(args) or intrinsic.{name}(args, ordering)
            _ if opcode_text.starts_with("intrinsic.") => {
                let intrinsic = parse_intrinsic_name(opcode_text, opcode_start)?;
                let (args, ordering, scope, memory_scope, semantics) =
                    self.parse_intrinsic_arguments(intrinsic)?;
                let arguments = self.tree.add_arguments(&args);
                Instruction::Intrinsic {
                    destination: Some(destination),
                    intrinsic,
                    arguments,
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
        };

        let instruction_id = self.tree.insert(instruction);
        self.tree.set_span(instruction_id, destination_span);
        Ok(instruction_id)
    }

    /// Parse an instruction without a destination: `opcode ...`
    fn eat_instruction_without_destination(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        let file_id = self.file_id;
        let opcode = self.eat_token(TokenType::Identifier)?;
        let opcode_start = opcode.start;
        let opcode_text = opcode.text;
        let opcode_span = Self::span_for_token(file_id, opcode);

        let instruction = match opcode_text {
            // local operations
            "local.set" => {
                let local = self.parse_local_ref()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::LocalSet { local, value }
            }

            // memory operations
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
            "assume" => {
                let condition = self.parse_value()?;
                Instruction::Assume { condition }
            }

            // tensor operations
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

            // void calls
            "call" => {
                let function = self.parse_function_reference()?;
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                let signature = if self.eat_token_maybe(TokenType::Arrow) {
                    self.parse_type()?
                } else {
                    self.signature_type_for_function(function)?
                };
                Instruction::Call {
                    destination: None,
                    function,
                    arguments,
                    signature,
                    effects: None,
                }
            }
            "call.virtual" => {
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallVirtual {
                    destination: None,
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                    effects: None,
                }
            }
            "call.interface" => {
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallInterface {
                    destination: None,
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                    effects: None,
                }
            }
            "call.indirect" => {
                let callee = self.parse_value()?;
                let args = self.parse_call_arguments()?;
                let arguments = self.tree.add_arguments(&args);
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Instruction::CallIndirect {
                    destination: None,
                    callee,
                    arguments,
                    signature,
                    effects: None,
                }
            }

            // allocation operations (no destination)
            "raw.free" => {
                let pointer = self.parse_value()?;
                Instruction::RawFree { pointer }
            }

            // void intrinsics: intrinsic.{name}(args) or intrinsic.{name}(args, ordering)
            _ if opcode_text.starts_with("intrinsic.") => {
                let intrinsic = parse_intrinsic_name(opcode_text, opcode_start)?;
                let (args, ordering, scope, memory_scope, semantics) =
                    self.parse_intrinsic_arguments(intrinsic)?;
                let arguments = self.tree.add_arguments(&args);
                Instruction::Intrinsic {
                    destination: None,
                    intrinsic,
                    arguments,
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
        };

        let instruction_id = self.tree.insert(instruction);
        self.tree.set_span(instruction_id, opcode_span);
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
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_token(TokenType::CloseBracket) {
            let value = self.parse_int_literal()?;
            let value = u32::try_from(value)
                .map_err(|_| ParseError::invalid("u32 literal", self.pos()))?;
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        // close the list
        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a bracketed list of u64 values.
    fn parse_u64_bracket_list(&mut self) -> ParseResult<Vec<u64>> {
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_token(TokenType::CloseBracket) {
            let value = self.parse_int_literal()?;
            let value = u64::try_from(value)
                .map_err(|_| ParseError::invalid("u64 literal", self.pos()))?;
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

    /// Parse a named value list like `name=[v0, v1]`.
    fn parse_named_value_list(&mut self, name: &str) -> ParseResult<Vec<Value>> {
        // validate the list name
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != name {
            return Err(ParseError::invalid(name, token.start));
        }
        self.eat_token(TokenType::Equals)?;

        // parse the list
        self.parse_value_bracket_list()
    }

    /// Parse a named u32 list like `name=[0, 1]`.
    fn parse_named_u32_list(&mut self, name: &str) -> ParseResult<Vec<u32>> {
        // validate the list name
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != name {
            return Err(ParseError::invalid(name, token.start));
        }
        self.eat_token(TokenType::Equals)?;

        // parse the list
        self.parse_u32_bracket_list()
    }

    /// Parse a named u32 value like `axis=0`.
    fn parse_named_u32(&mut self, name: &str) -> ParseResult<u32> {
        // validate the name
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != name {
            return Err(ParseError::invalid(name, token.start));
        }
        self.eat_token(TokenType::Equals)?;

        // parse the literal
        let value = self.parse_int_literal()?;
        u32::try_from(value).map_err(|_| ParseError::invalid("u32 literal", self.pos()))
    }

    /// Parse a named value like `value=v0`.
    fn parse_named_value(&mut self, name: &str) -> ParseResult<Value> {
        // validate the name
        let token = self.eat_token(TokenType::Identifier)?;
        if token.text != name {
            return Err(ParseError::invalid(name, token.start));
        }
        self.eat_token(TokenType::Equals)?;

        // parse the value
        self.parse_value()
    }

    /// Parse a vector reduction operator.
    fn parse_vector_reduce_operator(&mut self) -> ParseResult<VectorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = VectorReduceOperator::parse(token.text)
            .ok_or_else(|| ParseError::invalid("vector reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse a tensor reduction operator.
    fn parse_tensor_reduce_operator(&mut self) -> ParseResult<TensorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = TensorReduceOperator::parse(token.text)
            .ok_or_else(|| ParseError::invalid("tensor reduce operator", token.start))?;
        Ok(operator)
    }

    /// Parse an optional tensor scatter mode.
    fn parse_optional_named_scatter_mode(&mut self) -> ParseResult<Option<TensorScatterMode>> {
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
            let (key_text, key_start) = {
                let key = self.eat_token(TokenType::Identifier)?;
                (key.text.to_string(), key.start)
            };
            self.eat_token(TokenType::Equals)?;
            let list = self.parse_u32_bracket_list()?;
            match key_text.as_str() {
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
            let (key_text, key_start) = {
                let key = self.eat_token(TokenType::Identifier)?;
                (key.text.to_string(), key.start)
            };
            self.eat_token(TokenType::Equals)?;
            match key_text.as_str() {
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
                    return Err(ParseError::invalid(
                        "convolution dimension key",
                        key_start,
                    ));
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
            let next_key = self.peek_nth_token(1).ok_or_else(|| {
                ParseError::unexpected_end("convolution window key", self.pos())
            })?;
            if next_key.ty == TokenType::Identifier
                && (next_key.text == "feature_group" || next_key.text == "batch_group")
            {
                break;
            }

            // consume the window key
            self.eat_token(TokenType::Comma)?;
            let (key_text, key_start) = {
                let token = self.eat_token(TokenType::Identifier)?;
                (token.text.to_string(), token.start)
            };
            self.eat_token(TokenType::Equals)?;
            match key_text.as_str() {
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
            let (key_text, key_start) = {
                let token = self.eat_token(TokenType::Identifier)?;
                (token.text.to_string(), token.start)
            };
            self.eat_token(TokenType::Equals)?;
            let value = self.parse_int_as_u32()?;
            match key_text.as_str() {
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
            let (key_text, key_start) = {
                let key = self.eat_token(TokenType::Identifier)?;
                (key.text.to_string(), key.start)
            };
            self.eat_token(TokenType::Equals)?;
            match key_text.as_str() {
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
            let (key_text, key_start) = {
                let key = self.eat_token(TokenType::Identifier)?;
                (key.text.to_string(), key.start)
            };
            self.eat_token(TokenType::Equals)?;
            match key_text.as_str() {
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

    /// Pack tensor range lists into a single argument slice.
    fn pack_tensor_ranges(
        &mut self,
        offsets: &[Value],
        sizes: &[Value],
        strides: &[Value],
    ) -> ParseResult<ArgumentSlice> {
        // build the packed argument list
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(offsets);
        values.extend_from_slice(sizes);
        values.extend_from_slice(strides);

        Ok(self.tree.add_arguments(&values))
    }

    /// Pack tensor padding lists into a single argument slice.
    fn pack_tensor_padding(
        &mut self,
        low: &[Value],
        high: &[Value],
        interior: &[Value],
    ) -> ParseResult<ArgumentSlice> {
        // build the packed argument list
        let mut values = Vec::with_capacity(low.len() + high.len() + interior.len());
        values.extend_from_slice(low);
        values.extend_from_slice(high);
        values.extend_from_slice(interior);

        Ok(self.tree.add_arguments(&values))
    }

    /// Convert a list length into u16 for instruction metadata.
    fn parse_u16_count(&mut self, count: usize, context: &str) -> ParseResult<u16> {
        u16::try_from(count).map_err(|_| ParseError::invalid(context, self.pos()))
    }

    /// Parse an integer literal as u32.
    fn parse_int_as_u32(&mut self) -> ParseResult<u32> {
        // read the literal
        let value = self.parse_int_literal()?;
        u32::try_from(value).map_err(|_| ParseError::invalid("u32 literal", self.pos()))
    }

    /// Parse a terminator.
    pub(super) fn parse_terminator(&mut self) -> ParseResult<Terminator> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("terminator", self.pos()))?;

        match token.ty {
            TokenType::Return => {
                self.bump();
                let value = if self.peek_token(TokenType::Value) {
                    Some(self.parse_value()?)
                } else {
                    None
                };
                Ok(Terminator::Return { value })
            }

            TokenType::Jump => {
                self.bump();
                let target = self.parse_block_ref()?;
                let arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                Ok(Terminator::Jump { target, arguments })
            }

            TokenType::Branch => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let then_target = self.parse_block_ref()?;
                let then_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                self.eat_token(TokenType::Comma)?;
                let else_target = self.parse_block_ref()?;
                let else_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                Ok(Terminator::Branch {
                    condition,
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                })
            }

            TokenType::Check => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let constraint = self.parse_check_kind()?;
                self.eat_token(TokenType::Comma)?;
                let success = self.parse_check_target()?;
                self.eat_token(TokenType::Comma)?;
                let failure = self.parse_check_target()?;
                Ok(Terminator::Check {
                    condition,
                    constraint,
                    success,
                    failure,
                })
            }

            TokenType::Switch => {
                self.bump();
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let default = self.parse_block_ref()?;
                let default_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };

                // parse cases: value => blockN(args), ...
                let mut cases = Vec::new();
                while self.eat_token_maybe(TokenType::Comma) {
                    let case_value = self.parse_int_literal()?;
                    self.eat_token(TokenType::FatArrow)?;
                    let target = self.parse_block_ref()?;
                    let arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                        let args = self.parse_value_list()?;
                        self.eat_token(TokenType::CloseParen)?;
                        args
                    } else {
                        Vec::new()
                    };
                    cases.push(SwitchCase {
                        value: case_value,
                        target,
                        arguments,
                    });
                }

                Ok(Terminator::Switch {
                    value,
                    default,
                    default_arguments,
                    cases,
                })
            }

            TokenType::Yield => {
                self.bump();
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let resume = self.parse_block_ref()?;
                let resume_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                Ok(Terminator::Yield {
                    value,
                    resume,
                    resume_arguments,
                })
            }

            TokenType::Unreachable => {
                self.bump();
                Ok(Terminator::Unreachable)
            }

            TokenType::TailCall => {
                self.bump();
                // tailcall @func(args...)
                let function = self.parse_function_reference()?;
                let arguments = self.parse_call_arguments()?;
                Ok(Terminator::TailCall {
                    function,
                    arguments,
                })
            }

            TokenType::TailCallIndirect => {
                self.bump();
                // tailcall.indirect callee(args...) -> signature
                let callee = self.parse_value()?;
                let arguments = self.parse_call_arguments()?;
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Ok(Terminator::TailCallIndirect {
                    callee,
                    arguments,
                    signature,
                })
            }
            TokenType::TailCallVirtual => {
                self.bump();
                // tailcall.virtual receiver, declaring_type, slot_id[, @target](args...) -> signature
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let arguments = self.parse_call_arguments()?;
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Ok(Terminator::TailCallVirtual {
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                })
            }
            TokenType::TailCallInterface => {
                self.bump();
                // tailcall.interface receiver, declaring_type, slot_id[, @target](args...) -> signature
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let declaring_type = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let slot_id = self.parse_int_literal()? as u32;
                let declared_target = if self.eat_token_maybe(TokenType::Comma) {
                    Some(self.parse_function_reference()?)
                } else {
                    None
                };
                let arguments = self.parse_call_arguments()?;
                self.eat_token(TokenType::Arrow)?;
                let signature = self.parse_type()?;
                Ok(Terminator::TailCallInterface {
                    receiver,
                    arguments,
                    declaring_type,
                    slot_id,
                    declared_target,
                    signature,
                })
            }

            _ => Err(ParseError::unexpected("terminator", token.ty, token.start)),
        }
    }

    /// Parse a check kind and its operands.
    fn parse_check_kind(&mut self) -> ParseResult<CheckConstraint> {
        // parse the kind identifier
        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("check kind", self.pos()))?;
        let kind_text = kind_token.text;
        let kind_start = kind_token.start;
        match kind_token.ty {
            TokenType::Identifier | TokenType::TypeName | TokenType::Type => {
                self.bump();
            }
            _ => {
                return Err(ParseError::unexpected(
                    "check kind",
                    kind_token.ty,
                    kind_start,
                ));
            }
        }

        // split the kind into segments
        let mut parts = kind_text.split('.');
        let head = parts.next().unwrap_or_default();

        // dispatch on the kind head
        match head {
            "bounds" => {
                // parse signedness
                let signedness = parts.next().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "signed" => true,
                    "unsigned" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse bounds operands
                let index = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let collection = self.parse_value()?;

                Ok(CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    is_signed,
                })
            }
            "null" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse the null checked value
                let value = self.parse_value()?;
                Ok(CheckConstraint::Null { value })
            }
            "div_zero" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse the divisor
                let divisor = self.parse_value()?;
                Ok(CheckConstraint::DivZero { divisor })
            }
            "type" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse type tag operands
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_type()?;

                Ok(CheckConstraint::Type { value, expected })
            }
            "union" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse union tag operands
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_int_literal()?;
                let expected = u64::try_from(expected)
                    .map_err(|_| ParseError::invalid("check union tag", kind_start))?;

                Ok(CheckConstraint::Union { value, expected })
            }
            "vtable" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse vtable operands
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_type()?;

                Ok(CheckConstraint::Vtable { receiver, expected })
            }
            "itab" => {
                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse itab operands
                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_int_literal()?;
                let expected = u32::try_from(expected)
                    .map_err(|_| ParseError::invalid("check itab id", kind_start))?;

                Ok(CheckConstraint::Itab {
                    receiver,
                    expected: DispatchTableId::new(expected),
                })
            }

            "shift" => {
                // parse signedness
                let signedness = parts.next().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "signed" => true,
                    "unsigned" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse shift operands
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let bit_width = self.parse_int_literal()?;
                let bit_width = u8::try_from(bit_width)
                    .map_err(|_| ParseError::invalid("check bit width", kind_start))?;

                Ok(CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                })
            }
            "narrow" => {
                // parse signedness
                let signedness = parts.next().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "signed" => true,
                    "unsigned" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse narrow operands
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let to_width = self.parse_int_literal()?;
                let to_width = u8::try_from(to_width)
                    .map_err(|_| ParseError::invalid("check width", kind_start))?;

                Ok(CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                })
            }
            "overflow" => {
                // parse signedness and operator
                let signedness = parts.next().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let operator_text = parts.next().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "signed" => true,
                    "unsigned" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };
                let operator = operator_text.parse::<BinaryOperator>().map_err(|_| {
                    ParseError::invalid(&format!("check operator '{operator_text}'"), kind_start)
                })?;

                // reject extra segments
                if parts.next().is_some() {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                // parse overflow operands
                let left = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let right = self.parse_value()?;

                Ok(CheckConstraint::Overflow {
                    operator,
                    left,
                    right,
                    is_signed,
                })
            }
            _ => Err(ParseError::invalid(
                &format!("check kind '{kind_text}'"),
                kind_start,
            )),
        }
    }

    /// Parse a check target with optional block arguments.
    fn parse_check_target(&mut self) -> ParseResult<CheckTarget> {
        // parse the target block
        let target = self.parse_block_ref()?;

        // parse optional arguments
        let arguments = if self.eat_token_maybe(TokenType::OpenParen) {
            let args = self.parse_value_list()?;
            self.eat_token(TokenType::CloseParen)?;
            args
        } else {
            Vec::new()
        };

        Ok(CheckTarget { target, arguments })
    }

    /// Build a function pointer type for a direct call signature.
    fn signature_type_for_function(
        &mut self,
        function: LocalNodeId<Function>,
    ) -> ParseResult<LocalNodeId<Type>> {
        let function = self.tree.get(function);
        let parameters = function.parameters.iter().map(|param| param.ty).collect();
        Ok(self.tree.insert_type(Type::FunctionPointer {
            parameters,
            result: function.return_type,
        }))
    }

    /// Parse intrinsic arguments: (values...) or (values..., ordering) for atomics.
    fn parse_intrinsic_arguments(
        &mut self,
        intrinsic: Intrinsic,
    ) -> ParseResult<(
        Vec<Value>,
        Option<MemoryOrdering>,
        Option<AtomicScope>,
        Option<MemoryScope>,
        Option<MemorySemantics>,
    )> {
        self.eat_token(TokenType::OpenParen)?;

        let mut values = Vec::new();
        let mut ordering = None;
        let mut scope = None;
        let mut memory_scope = None;
        let mut semantics = None;

        // parse values and potentially an ordering at the end
        loop {
            // check for closing paren
            if self.peek_token(TokenType::CloseParen) {
                break;
            }

            // try to parse a value
            if self.peek_token(TokenType::Value) {
                values.push(self.parse_value()?);
            } else if self.peek_token(TokenType::Identifier) {
                let key = self.eat_token(TokenType::Identifier)?;
                let key_text = key.text.to_string();
                let key_start = key.start;
                self.eat_token(TokenType::Equals)?;
                match key_text.as_str() {
                    "ordering" => {
                        ordering = Some(
                            self.eat_token(TokenType::Identifier)?
                                .text
                                .parse::<MemoryOrdering>()
                                .map_err(|_| ParseError::invalid("memory ordering", key_start))?,
                        );
                    }
                    "scope" => {
                        scope = Some(
                            self.eat_token(TokenType::Identifier)?
                                .text
                                .parse::<AtomicScope>()
                                .map_err(|_| ParseError::invalid("atomic scope", key_start))?,
                        );
                    }
                    "memory_scope" => {
                        memory_scope = Some(
                            self.eat_token(TokenType::Identifier)?
                                .text
                                .parse::<MemoryScope>()
                                .map_err(|_| ParseError::invalid("memory scope", key_start))?,
                        );
                    }
                    "semantics" => {
                        semantics = Some(self.parse_memory_semantics()?);
                    }
                    _ => {
                        return Err(ParseError::invalid("intrinsic argument", key_start));
                    }
                }
            } else {
                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::unexpected_end("intrinsic argument", self.pos()))?;
                return Err(ParseError::unexpected(
                    "intrinsic argument",
                    token.ty,
                    token.start,
                ));
            }

            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParen)?;
        let requires_ordering = intrinsic.requires_ordering();
        let requires_semantics = intrinsic.requires_memory_semantics();

        if requires_ordering && ordering.is_none() {
            return Err(ParseError::invalid(
                "atomic intrinsic requires ordering",
                self.pos(),
            ));
        }

        if requires_semantics {
            if scope.is_none() {
                return Err(ParseError::invalid(
                    "atomic intrinsic requires scope",
                    self.pos(),
                ));
            }
            if memory_scope.is_none() {
                return Err(ParseError::invalid(
                    "atomic intrinsic requires memory scope",
                    self.pos(),
                ));
            }
            if semantics.is_none() {
                return Err(ParseError::invalid(
                    "atomic intrinsic requires semantics",
                    self.pos(),
                ));
            }
        }

        if !requires_ordering && ordering.is_some() {
            return Err(ParseError::invalid(
                "ordering only allowed for atomic intrinsics",
                self.pos(),
            ));
        }

        if !requires_semantics && (scope.is_some() || memory_scope.is_some() || semantics.is_some())
        {
            return Err(ParseError::invalid(
                "atomic scope and semantics only allowed for atomic intrinsics",
                self.pos(),
            ));
        }
        Ok((values, ordering, scope, memory_scope, semantics))
    }

    /// Parse memory semantics for atomic operations.
    fn parse_memory_semantics(&mut self) -> ParseResult<MemorySemantics> {
        // semantics state
        let mut locations = MemoryLocationSet::NONE;
        let mut has_location = false;
        let mut is_location_locked = false;
        let mut is_volatile = false;
        let mut is_make_available = false;
        let mut is_make_visible = false;

        // parse a single semantics identifier
        let mut parse_item = |text: &str, start: usize| -> ParseResult<()> {
            match text {
                "volatile" => {
                    is_volatile = true;
                    Ok(())
                }
                "make_available" => {
                    is_make_available = true;
                    Ok(())
                }
                "make_visible" => {
                    is_make_visible = true;
                    Ok(())
                }
                _ => {
                    let location = parse_memory_location(text, start)?;
                    if location == MemoryLocationSet::ANY || location == MemoryLocationSet::NONE {
                        if has_location && !is_location_locked {
                            return Err(ParseError::new(
                                "memory semantics cannot mix any/none with other locations",
                                start,
                            ));
                        }
                        locations = location;
                        has_location = true;
                        is_location_locked = true;
                        return Ok(());
                    }
                    if is_location_locked {
                        return Err(ParseError::new(
                            "memory semantics cannot mix any/none with other locations",
                            start,
                        ));
                    }
                    if !has_location {
                        locations = MemoryLocationSet::NONE;
                        has_location = true;
                    }
                    locations.insert(location);
                    Ok(())
                }
            }
        };

        // parse the semantics list or single value
        if self.eat_token_maybe(TokenType::OpenBracket) {
            while !self.peek_token(TokenType::CloseBracket) {
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
                parse_item(&token_text, token_start)?;
                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }
            self.eat_token(TokenType::CloseBracket)?;
        } else {
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
            parse_item(&token_text, token_start)?;
        }

        // default the location set when omitted
        if !has_location {
            locations = MemoryLocationSet::ANY;
        }

        Ok(MemorySemantics::with_flags(
            locations,
            is_volatile,
            is_make_available,
            is_make_visible,
        ))
    }
}
