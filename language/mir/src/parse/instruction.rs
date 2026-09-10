use crate::source::TokenType;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::{
    AddressKind, AtomicAccess, AtomicRmwOperator, BinaryOperator, Call, Callee, CastOperator,
    CompareExchangeAccess, ConvertMode, Copy, CounterId, DispatchSlot, ExecutionScope, FenceAccess,
    GenericParameterDomain, Instruction, LayoutMeasure, LocalNodeId, MemoryOrdering, SamplerId,
    StorageSet, TypeId, UnaryOperator, Value, VectorReduceOperator,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse an instruction.
    pub(super) fn parse_instruction(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        // whole instruction
        let instruction_start = self.pos();

        // optional destination
        let mut destination: Option<Value> = None;
        let mut destination_type = None;
        let mut destination_span = None;
        let mut destination_type_span = None;
        if self.is_value_definition_start() {
            let (parsed_destination, parsed_type, parsed_span, parsed_type_span) =
                self.parse_typed_destination_parts()?;
            self.record_value_type(parsed_destination, parsed_type)?;
            self.eat_token(TokenType::Equal)?;
            destination = Some(parsed_destination);
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
                    && (matches!(
                        self.tree.source_text(token.span),
                        "null" | "undefined" | "inf" | "-inf" | "NaN" | "witness"
                    ) || LayoutMeasure::from_keyword(self.tree.source_text(token.span))
                        .is_some()
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
                )?;

                return Ok(id);
            }
        }

        // ordered source segments
        let mut segment_spans = Vec::new();

        // opcode
        let opcode = self
            .peek()
            .cloned()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;
        let (opcode_text, opcode_start) = self.parse_opcode()?;
        let opcode_text = opcode_text.as_str();
        let opcode_span = opcode.span;
        segment_spans.push(opcode_span);

        // reject destinations on void instructions
        if destination.is_some()
            && matches!(
                opcode_text,
                "local.set"
                    | "store"
                    | "drop"
                    | "release"
                    | "hold"
                    | "barrier.write"
                    | "atomic.store"
                    | "atomic.fence"
                    | "assume"
                    | "poll"
                    | "breakpoint"
                    | "profile.increment"
                    | "profile.sample"
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

            // calls and intrinsics
            "call" => {
                let (callee, arguments, signature) =
                    self.parse_direct_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);

                Instruction::Call {
                    destination,
                    call: Call::new(callee, arguments, signature),
                }
            }
            "call.witness" => {
                let (callee, arguments, signature) =
                    self.parse_witness_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);

                Instruction::Call {
                    destination,
                    call: Call::new(callee, arguments, signature),
                }
            }
            "call.virtual" => {
                let (receiver, class, slot, arguments, signature) =
                    self.parse_virtual_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);
                Instruction::Call {
                    destination,
                    call: Call::new(
                        Callee::Virtual {
                            receiver,
                            class,
                            slot,
                        },
                        arguments,
                        signature,
                    ),
                }
            }
            "call.dynamic" => {
                let (receiver, constraint, slot, arguments, signature) =
                    self.parse_dynamic_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);
                Instruction::Call {
                    destination,
                    call: Call::new(
                        Callee::Dynamic {
                            receiver,
                            constraint,
                            slot,
                        },
                        arguments,
                        signature,
                    ),
                }
            }
            "call.indirect" => {
                let (callee, arguments, signature) =
                    self.parse_indirect_call_target_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);
                Instruction::Call {
                    destination,
                    call: Call::new(Callee::Indirect { value: callee }, arguments, signature),
                }
            }
            "drop" => {
                let value = self.parse_value_segment(&mut segment_spans)?;

                Instruction::Drop { value }
            }

            // allocation protocol
            "release" => {
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Release { value }
            }

            // collector protocol
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

            // atomic memory operations
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

            // assumptions and hints
            "assume" => {
                let condition = self.parse_value_segment(&mut segment_spans)?;
                Instruction::Assume { condition }
            }

            // profile instrumentation
            "profile.increment" => {
                let counter =
                    CounterId(self.parse_profile_id_segment(&mut segment_spans, "counter")?);
                Instruction::ProfileIncrement { counter }
            }
            "profile.sample" => {
                let sampler =
                    SamplerId(self.parse_profile_id_segment(&mut segment_spans, "sampler")?);
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value_segment(&mut segment_spans)?;
                Instruction::ProfileSample { sampler, value }
            }

            // runtime control
            "poll" => Instruction::Poll,

            // debug control
            "breakpoint" => Instruction::Breakpoint,

            // intrinsics
            _ if opcode_text.starts_with("intrinsic.") => {
                let intrinsic = self.parse_intrinsic_name(opcode_text, opcode_start)?;
                let arguments = self.parse_call_argument_segments(&mut segment_spans)?;
                let arguments = self.tree.add_values(&arguments);
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
                        let left = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let right = self.parse_value_segment(&mut segment_spans)?;
                        let operator = opcode_text
                            .parse()
                            .map_err(|_| ParseError::invalid("binary operator", opcode_start))?;
                        Instruction::Binary {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }

                    // unary ops
                    _ if opcode_text.parse::<UnaryOperator>().is_ok() => {
                        let argument = self.parse_value_segment(&mut segment_spans)?;
                        let operator = opcode_text
                            .parse()
                            .map_err(|_| ParseError::invalid("unary operator", opcode_start))?;
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
                            to_type,
                        }
                    }

                    "copy" => {
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Copy { destination, value }
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
                    "local.get" | "local.get.copy" => {
                        let copy = if opcode_text.ends_with(".copy") {
                            Copy::Yes
                        } else {
                            Copy::No
                        };
                        let local = self.parse_local_segment(&mut segment_spans)?;
                        Instruction::LocalGet {
                            copy,
                            destination,
                            local,
                        }
                    }
                    "local.address" | "local.project" => {
                        let kind = address_kind(opcode_text);
                        let local = self.parse_local_segment(&mut segment_spans)?;
                        Instruction::LocalAddr {
                            destination,
                            local,
                            result_type: destination_type,
                            kind,
                        }
                    }
                    // global operations
                    "global.address" | "global.project" => {
                        let kind = address_kind(opcode_text);
                        let global = self.parse_global_segment(&mut segment_spans)?;
                        Instruction::GlobalAddr {
                            destination,
                            global,
                            result_type: destination_type,
                            kind,
                        }
                    }
                    "function.address" => {
                        let (function, arguments, span) = self.parse_function_reference_part()?;
                        segment_spans.push(span);
                        Instruction::FunctionAddr {
                            destination,
                            function,
                            arguments,
                        }
                    }
                    "function.bind" => {
                        let (function, arguments, span) = self.parse_function_reference_part()?;
                        segment_spans.push(span);
                        self.eat_token(TokenType::Comma)?;
                        let environment = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::FunctionBind {
                            destination,
                            function,
                            arguments,
                            environment,
                        }
                    }
                    "function.environment" => {
                        let function = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::FunctionEnvironment {
                            destination,
                            function,
                        }
                    }
                    "function.environment.current" => {
                        Instruction::FunctionEnvironmentCurrent { destination }
                    }

                    // execution contexts
                    "context.current" => Instruction::ContextCurrent { destination },
                    "context.replace" => {
                        let context = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ContextReplace {
                            destination,
                            context,
                        }
                    }
                    "context.bind" => {
                        let context = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let variable = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let node_type = self.parse_type_segment(&mut segment_spans)?;
                        Instruction::ContextBind {
                            destination,
                            context,
                            variable,
                            value,
                            node_type,
                            result_type: destination_type,
                        }
                    }
                    "context.get" => {
                        let context = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let variable = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let default = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let node_type = self.parse_type_segment(&mut segment_spans)?;
                        Instruction::ContextGet {
                            destination,
                            context,
                            variable,
                            default,
                            node_type,
                            result_type: destination_type,
                        }
                    }

                    // memory operations
                    "load" | "load.copy" => {
                        let copy = if opcode_text.ends_with(".copy") {
                            Copy::Yes
                        } else {
                            Copy::No
                        };
                        let pointer = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::Load {
                            copy,
                            destination,
                            pointer,
                            result_type: destination_type,
                        }
                    }

                    // aggregate construction
                    "aggregate" => {
                        let values = self.parse_call_argument_segments(&mut segment_spans)?;
                        let values = self.tree.add_values(&values);
                        Instruction::Aggregate {
                            destination,
                            values,
                        }
                    }

                    // aggregate projection
                    "field.get" | "field.get.copy" => {
                        let copy = if opcode_text.ends_with(".copy") {
                            Copy::Yes
                        } else {
                            Copy::No
                        };
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let field = self.parse_int_segment(&mut segment_spans)?;
                        let field = u32::try_from(field)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        Instruction::FieldGet {
                            copy,
                            destination,
                            aggregate,
                            field,
                        }
                    }
                    "field.set" => {
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let field = self.parse_int_segment(&mut segment_spans)?;
                        let field = u32::try_from(field)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::FieldSet {
                            destination,
                            aggregate,
                            field,
                            value,
                        }
                    }
                    "field.address" | "field.project" => {
                        let kind = address_kind(opcode_text);
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let field = self.parse_int_segment(&mut segment_spans)?;
                        let field = u32::try_from(field)
                            .map_err(|_| ParseError::invalid("field index", self.pos()))?;
                        Instruction::FieldAddr {
                            destination,
                            aggregate,
                            field,
                            result_type: destination_type,
                            kind,
                        }
                    }
                    "element.get" | "element.get.copy" => {
                        let copy = if opcode_text.ends_with(".copy") {
                            Copy::Yes
                        } else {
                            Copy::No
                        };
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("element index", self.pos()))?;
                        Instruction::ElementGet {
                            copy,
                            destination,
                            aggregate,
                            index,
                        }
                    }
                    "element.set" => {
                        let aggregate = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_int_segment(&mut segment_spans)?;
                        let index = u32::try_from(index)
                            .map_err(|_| ParseError::invalid("element index", self.pos()))?;
                        self.eat_token(TokenType::Comma)?;
                        let value = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ElementSet {
                            destination,
                            aggregate,
                            index,
                            value,
                        }
                    }
                    "element.address" | "element.project" => {
                        let kind = address_kind(opcode_text);
                        let base = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let index = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::ElementAddr {
                            destination,
                            base,
                            index,
                            result_type: destination_type,
                            kind,
                        }
                    }

                    // variant construction and projection
                    "variant.new" => {
                        let case = self.parse_int_segment(&mut segment_spans)?;
                        let case = u32::try_from(case)
                            .map_err(|_| ParseError::invalid("case index", self.pos()))?;
                        let payload = match self.peek_is(TokenType::Comma) {
                            true => {
                                self.eat_token(TokenType::Comma)?;

                                Some(self.parse_value_segment(&mut segment_spans)?)
                            }
                            false => None,
                        };
                        Instruction::VariantNew {
                            destination,
                            case,
                            payload,
                            result_type: destination_type,
                        }
                    }
                    "variant.tag" => {
                        let variant = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::VariantTag {
                            destination,
                            variant,
                        }
                    }
                    "variant.tag.load" => {
                        let variant = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::VariantTagLoad {
                            destination,
                            variant,
                        }
                    }
                    "variant.payload" | "variant.payload.copy" => {
                        let copy = if opcode_text.ends_with(".copy") {
                            Copy::Yes
                        } else {
                            Copy::No
                        };
                        let variant = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let case = self.parse_int_segment(&mut segment_spans)?;
                        let case = u32::try_from(case)
                            .map_err(|_| ParseError::invalid("case index", self.pos()))?;
                        Instruction::VariantPayload {
                            copy,
                            destination,
                            variant,
                            case,
                        }
                    }
                    "variant.payload.address" | "variant.payload.project" => {
                        let kind = address_kind(opcode_text);
                        let variant = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let case = self.parse_int_segment(&mut segment_spans)?;
                        let case = u32::try_from(case)
                            .map_err(|_| ParseError::invalid("case index", self.pos()))?;
                        Instruction::VariantPayloadAddr {
                            destination,
                            variant,
                            case,
                            result_type: destination_type,
                            kind,
                        }
                    }

                    // slice descriptors
                    "slice.view" => {
                        let source = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let start = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::SliceView {
                            destination,
                            source,
                            start,
                            length,
                            result_type: destination_type,
                        }
                    }
                    "slice.length" => {
                        let slice = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::SliceLength { destination, slice }
                    }

                    // dynamic values
                    "dynamic.bind" => {
                        let payload = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let concrete = self.parse_type_segment(&mut segment_spans)?;
                        Instruction::DynamicBind {
                            destination,
                            payload,
                            concrete,
                        }
                    }
                    "dynamic.payload" => {
                        let dynamic = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::DynamicPayload {
                            destination,
                            dynamic,
                            result_type: destination_type,
                        }
                    }
                    "dynamic.type" => {
                        let dynamic = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::DynamicType {
                            destination,
                            dynamic,
                        }
                    }
                    "dynamic.read" => {
                        let dynamic = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let slot = self.parse_int_segment(&mut segment_spans)?;
                        let slot = u32::try_from(slot)
                            .map_err(|_| ParseError::invalid("dispatch slot", self.pos()))?;
                        Instruction::DynamicRead {
                            destination,
                            dynamic,
                            slot: DispatchSlot(slot),
                            result_type: destination_type,
                        }
                    }
                    "dynamic.find" => {
                        let dynamic = self.parse_value_segment(&mut segment_spans)?;
                        self.eat_token(TokenType::Comma)?;
                        let key = self.parse_value_segment(&mut segment_spans)?;
                        Instruction::DynamicFind {
                            destination,
                            dynamic,
                            key,
                            result_type: destination_type,
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
                        let mask = self.tree.add_indices(&mask);
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
                        let (operator, left, right) = self.parse_comparison()?;
                        Instruction::VectorCompare {
                            destination,
                            operator,
                            left,
                            right,
                        }
                    }
                    "vector.convert" => {
                        let mode = self.parse_convert_mode("vector convert mode")?;
                        self.eat_token(TokenType::Comma)?;
                        let vector = self.parse_value()?;
                        Instruction::VectorConvert {
                            destination,
                            mode,
                            vector,
                        }
                    }

                    // allocation operations
                    "new.zeroed" => {
                        let storage_type = self.parse_type()?;
                        Instruction::NewZeroed {
                            destination,
                            storage_type,
                            result_type: destination_type,
                        }
                    }
                    "new.uninit" => {
                        let storage_type = self.parse_type()?;
                        Instruction::NewUninit {
                            destination,
                            storage_type,
                            result_type: destination_type,
                        }
                    }
                    "new.complete" => {
                        let value = self.parse_value()?;
                        Instruction::NewComplete {
                            destination,
                            value,
                            result_type: destination_type,
                        }
                    }
                    "new.slice.zeroed" => {
                        let element = self.parse_type()?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value()?;
                        Instruction::NewSliceZeroed {
                            destination,
                            element,
                            length,
                            result_type: destination_type,
                        }
                    }
                    "new.slice.uninit" => {
                        let element = self.parse_type()?;
                        self.eat_token(TokenType::Comma)?;
                        let length = self.parse_value()?;
                        Instruction::NewSliceUninit {
                            destination,
                            element,
                            length,
                            result_type: destination_type,
                        }
                    }
                    // atomic memory operations
                    "atomic.load" => {
                        let pointer = self.parse_value()?;
                        let access = self.parse_atomic_access()?;
                        Instruction::AtomicLoad {
                            destination,
                            pointer,
                            result_type: destination_type,
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
        )?;
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
    ) -> ParseResult<()> {
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

        // ordered source segments
        self.set_segment_spans(instruction_id, segment_spans)?;

        Ok(())
    }

    /// Parse one profile identifier.
    fn parse_profile_id_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
        name: &str,
    ) -> ParseResult<u32> {
        let keyword = self.eat_token(TokenType::Identifier)?;
        segment_spans.push(keyword.span);
        if self.tree.source_text(keyword.span) != name {
            return Err(ParseError::invalid(name, self.pos()));
        }

        let open = self.eat_token(TokenType::OpenParenthesis)?;
        segment_spans.push(open.span);

        let number = self.eat_token(TokenType::Integer)?;
        segment_spans.push(number.span);
        let value = self
            .tree
            .source_text(number.span)
            .parse::<u32>()
            .map_err(|_| ParseError::invalid(name, self.pos()))?;

        let close = self.eat_token(TokenType::CloseParenthesis)?;
        segment_spans.push(close.span);

        Ok(value)
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

    /// Parse a bracketed list of integer values.
    fn parse_int_bracket_list(&mut self) -> ParseResult<Vec<i128>> {
        // open the list
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();

        // read values
        while !self.peek_is(TokenType::CloseBracket) {
            let value = self.parse_int_literal()?;
            values.push(value);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        // close the list
        self.eat_token(TokenType::CloseBracket)?;
        Ok(values)
    }

    /// Parse a vector reduction operator.
    fn parse_vector_reduce_operator(&mut self) -> ParseResult<VectorReduceOperator> {
        // parse the operator token
        let token = self.eat_token(TokenType::Identifier)?;
        let operator = VectorReduceOperator::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid("vector reduce operator", token.start()))?;
        Ok(operator)
    }

    /// Parse one numeric conversion policy.
    fn parse_convert_mode(&mut self, expected: &'static str) -> ParseResult<ConvertMode> {
        // parse the mode token
        let token = self.eat_token(TokenType::Identifier)?;
        let mode = ConvertMode::parse(self.tree.source_text(token.span))
            .ok_or_else(|| ParseError::invalid(expected, token.start()))?;

        Ok(mode)
    }

    /// Parse one vector comparison.
    fn parse_comparison(&mut self) -> ParseResult<(BinaryOperator, Value, Value)> {
        let token = self.eat_token(TokenType::Identifier)?;
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_value()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_value()?;
        let operator = self
            .tree
            .source_text(token.span)
            .parse()
            .map_err(|_| ParseError::invalid("binary operator", token.start()))?;

        Ok((operator, left, right))
    }

    /// Parse one direct call target and arguments.
    pub(super) fn parse_direct_call_target(&mut self) -> ParseResult<(Callee, Vec<Value>, TypeId)> {
        let mut segment_spans = Vec::new();
        self.parse_direct_call_target_segments(&mut segment_spans)
    }

    /// Parse one direct call target, arguments, and signature with source segments.
    pub(super) fn parse_direct_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(Callee, Vec<Value>, TypeId)> {
        let (function, generic_arguments, span) = self.parse_function_reference_part()?;
        segment_spans.push(span);
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_call_signature_segment(segment_spans)?;
        let callee = Callee::Direct {
            function,
            arguments: generic_arguments,
        };

        Ok((callee, arguments, signature))
    }

    /// Parse one witness call target and arguments.
    pub(super) fn parse_witness_call_target(
        &mut self,
    ) -> ParseResult<(Callee, Vec<Value>, TypeId)> {
        let mut segment_spans = Vec::new();
        self.parse_witness_call_target_segments(&mut segment_spans)
    }

    /// Parse one witness call target, arguments, and signature with source segments.
    pub(super) fn parse_witness_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(Callee, Vec<Value>, TypeId)> {
        // read the receiver type, the interface, and the requirement it names
        let receiver = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let interface = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let (requirement, generic_arguments, span) = self.parse_function_reference_part()?;
        if !generic_arguments.is_empty() {
            return Err(ParseError::invalid(
                "witness requirement",
                span.start as usize,
            ));
        }
        segment_spans.push(span);

        // read the call arguments and the explicit signature
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_call_signature_segment(segment_spans)?;
        let callee = Callee::Witness {
            receiver: TypeId::from(receiver),
            interface: TypeId::from(interface),
            requirement,
        };

        Ok((callee, arguments, signature))
    }

    /// Parse one virtual call target and signature.
    pub(super) fn parse_virtual_call_target(
        &mut self,
    ) -> ParseResult<(Value, TypeId, DispatchSlot, Vec<Value>, TypeId)> {
        let mut segment_spans = Vec::new();
        self.parse_virtual_call_target_segments(&mut segment_spans)
    }

    /// Parse one virtual call target and signature with source segments.
    pub(super) fn parse_virtual_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(Value, TypeId, DispatchSlot, Vec<Value>, TypeId)> {
        let receiver = self.parse_value_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let class = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let slot = self.parse_int_segment(segment_spans)?;
        let slot = u32::try_from(slot)
            .map_err(|_| ParseError::invalid("virtual dispatch slot", self.pos()))?;
        let slot = DispatchSlot::new(slot);
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_call_signature_segment(segment_spans)?;

        Ok((receiver, class, slot, arguments, signature))
    }

    /// Parse one dynamic call target and signature.
    pub(super) fn parse_dynamic_call_target(
        &mut self,
    ) -> ParseResult<(Value, TypeId, DispatchSlot, Vec<Value>, TypeId)> {
        let mut segment_spans = Vec::new();
        self.parse_dynamic_call_target_segments(&mut segment_spans)
    }

    /// Parse one dynamic call target and signature with source segments.
    pub(super) fn parse_dynamic_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(Value, TypeId, DispatchSlot, Vec<Value>, TypeId)> {
        let receiver = self.parse_value_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let constraint = self.parse_type_segment(segment_spans)?;
        self.eat_token(TokenType::Comma)?;
        let slot = self.parse_int_segment(segment_spans)?;
        let slot =
            u32::try_from(slot).map_err(|_| ParseError::invalid("dynamic slot", self.pos()))?;
        let slot = DispatchSlot::new(slot);
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_call_signature_segment(segment_spans)?;

        Ok((receiver, constraint, slot, arguments, signature))
    }

    /// Parse one indirect call target and signature.
    pub(super) fn parse_indirect_call_target(
        &mut self,
    ) -> ParseResult<(Value, Vec<Value>, TypeId)> {
        let mut segment_spans = Vec::new();
        self.parse_indirect_call_target_segments(&mut segment_spans)
    }

    /// Parse one indirect call target and signature with source segments.
    pub(super) fn parse_indirect_call_target_segments(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<(Value, Vec<Value>, TypeId)> {
        let callee = self.parse_value_segment(segment_spans)?;
        let arguments = self.parse_call_argument_segments(segment_spans)?;
        let signature = self.parse_call_signature_segment(segment_spans)?;

        Ok((callee, arguments, signature))
    }

    /// Parse one call signature.
    fn parse_call_signature_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<TypeId> {
        let signature_start = self.pos();
        self.eat_token(TokenType::Colon)?;

        self.parse_lifetime_scope(|parser, lifetimes| {
            let signature = parser.parse_signature(lifetimes)?;
            let signature_span = parser.span_from_parse_start(signature_start);
            segment_spans.push(signature_span);

            Ok(signature)
        })
    }

    /// Parse one atomic access suffix.
    fn parse_atomic_access(&mut self) -> ParseResult<AtomicAccess> {
        self.eat_token(TokenType::Comma)?;

        let ordering = self.parse_memory_ordering()?;
        let mut access = AtomicAccess::ordered(ordering);

        while self.eat_token_if(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic access", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                access.scope = self.parse_execution_scope_clause()?;
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
        let mut failure_ordering = None;

        while self.eat_token_if(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic compare exchange", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                success.scope = self.parse_execution_scope_clause()?;
            } else if token_text == "failure" {
                self.eat_token(TokenType::Identifier)?;
                self.eat_token(TokenType::OpenParenthesis)?;
                failure_ordering = Some(self.parse_memory_ordering()?);
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

        while self.eat_token_if(TokenType::Comma) {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("atomic fence", self.pos()))?;
            let token_text = self.tree.source_text(token.span).to_string();

            if token_text == "scope" {
                access.scope = self.parse_execution_scope_clause()?;
            } else if token_text == "storage" {
                access.storage = self.parse_storage_clause()?;
            } else {
                return Err(ParseError::invalid("atomic fence", self.pos()));
            }
        }

        Ok(access)
    }

    /// Parse one `scope(...)` execution clause.
    fn parse_execution_scope_clause(&mut self) -> ParseResult<ExecutionScope> {
        self.eat_token(TokenType::Identifier)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let scope = self.parse_execution_scope()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(scope)
    }

    /// Parse one `storage(...)` fence clause.
    fn parse_storage_clause(&mut self) -> ParseResult<StorageSet> {
        self.eat_token(TokenType::Identifier)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let storage = self.parse_storage_set()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(storage)
    }

    /// Parse one memory ordering like `sequentiallyConsistent`.
    fn parse_memory_ordering(&mut self) -> ParseResult<MemoryOrdering> {
        let token_start = self.pos();
        let token = self.eat_token(TokenType::Identifier)?;
        let text = self.tree.source_text(token.span);
        if let Some(ordering) = MemoryOrdering::from_name(text) {
            return Ok(ordering);
        }
        match self.generic_parameter(text) {
            Some((index, parameter))
                if matches!(parameter.domain, GenericParameterDomain::Value { .. }) =>
            {
                Ok(MemoryOrdering::Parameter(index))
            }
            _ => Err(ParseError::invalid("memory ordering", token_start)),
        }
    }

    /// Parse one execution scope like `device`.
    fn parse_execution_scope(&mut self) -> ParseResult<ExecutionScope> {
        let token_start = self.pos();
        let token = self.eat_token(TokenType::Identifier)?;
        self.tree
            .source_text(token.span)
            .parse::<ExecutionScope>()
            .map_err(|_| ParseError::invalid("execution scope", token_start))
    }

    /// Parse a storage set for one fence.
    fn parse_storage_set(&mut self) -> ParseResult<StorageSet> {
        let mut storage = StorageSet::NONE;
        let mut has_storage = false;
        let mut is_space_locked = false;
        let is_list = self.eat_token_if(TokenType::OpenBracket);

        // parse one or more storage names
        loop {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("storage set", self.pos()))?;
            if !matches!(
                self.token_type(token),
                TokenType::Identifier | TokenType::Global | TokenType::Local | TokenType::Shared
            ) {
                return Err(ParseError::unexpected(
                    "storage set",
                    self.token_type(token),
                    token.start(),
                ));
            }

            let token_text = self.tree.source_text(token.span).to_string();
            let token_start = token.start();
            self.bump();

            // combine storage names
            let space = self.parse_storage(&token_text, token_start)?;
            if space == StorageSet::ANY || space == StorageSet::NONE {
                if has_storage && !is_space_locked {
                    return Err(ParseError::new(
                        "storage set cannot mix any/none with other storage",
                        token_start,
                    ));
                }

                storage = space;
                has_storage = true;
                is_space_locked = true;
            } else {
                if is_space_locked {
                    return Err(ParseError::new(
                        "storage set cannot mix any/none with other storage",
                        token_start,
                    ));
                }

                storage.insert(space);
                has_storage = true;
            }

            // single values stop after one item
            if !is_list {
                break;
            }

            // lists stop before the closing bracket
            if !self.eat_token_if(TokenType::Comma) || self.peek_is(TokenType::CloseBracket) {
                break;
            }
        }

        if is_list {
            self.eat_token(TokenType::CloseBracket)?;
        }

        if !has_storage {
            storage = StorageSet::ANY;
        }

        Ok(storage)
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

        operator_text
            .parse()
            .map_err(|_| ParseError::invalid("atomic rmw operator", start))
    }
}

/// Read the address kind spelled by one address mnemonic's final segment.
fn address_kind(opcode: &str) -> AddressKind {
    opcode
        .rsplit('.')
        .next()
        .and_then(AddressKind::from_mnemonic)
        .unwrap_or_default()
}
