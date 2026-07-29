use std::collections::HashMap;

use destack_artifact::{FramePoint, MirOptimized, Point};
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::{RegisterAllocation, RegisterAllocator, TypeEmitter};

/// One bytecode function emission and its object sections.
#[derive(Debug)]
pub(crate) struct FunctionEmission {
    /// The encoded function body and object-relative references.
    pub(crate) body: bytecode::FunctionBody,
    /// Logical and physical frame states in coordinate order.
    pub(crate) frames: Vec<FrameEmission>,
}

/// One logical frame state and its physical bytecode registers.
#[derive(Debug)]
pub(crate) struct FrameEmission {
    /// The object-local logical coordinate.
    pub(crate) point: FramePoint,
    /// Object-local types in acquisition order.
    pub(crate) types: Vec<mir::TypeId>,
    /// Physical register spans in acquisition order.
    pub(crate) registers: Vec<bytecode::RegisterSpan>,
}

/// Emit one MIR function into relocatable bytecode.
#[derive(Debug)]
pub(crate) struct FunctionEmitter<'a> {
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Object-local type identities.
    types: &'a TypeEmitter<'a>,
    /// Object-local identity assignments.
    object: &'a ObjectEmitter,
    /// Module owning the function.
    module: ModuleId,
    /// MIR function declaration.
    function: &'a mir::Function,
    /// Object-local function identity.
    function_id: mir::FunctionId,
    /// Physical instruction builder.
    builder: bytecode::FunctionBuilder,
    /// Fixed register ranges keyed by MIR value identity.
    ranges: Vec<Option<bytecode::RegisterSpan>>,
    /// Permanent register ranges keyed by MIR local identity.
    locals: HashMap<mir::LocalId, bytecode::RegisterSpan>,
    /// Reusable exact-type scratch ranges.
    scratches: Vec<(bytecode::ValueType, bytecode::RegisterSpan)>,
    /// Reusable contiguous outgoing call registers.
    arguments: Vec<ArgumentRegisters>,
    /// CFG-correct MIR liveness used to materialize frame states.
    liveness: mir::FunctionLiveness,
    /// Frame states in emitted coordinate order.
    frames: Vec<FrameEmission>,
    /// Branch labels keyed by MIR block identity.
    blocks: HashMap<mir::BlockId, bytecode::Label>,
    /// Deferred blocks emitted after the main blocks.
    stubs: Vec<Stub>,
    /// Next dense branch label.
    next_label: u32,
}

/// One deferred bytecode block.
#[derive(Debug)]
enum Stub {
    /// One parallel block argument transfer.
    Transfer {
        /// Branch label entering this transfer.
        label: bytecode::Label,
        /// MIR edge target and arguments.
        target: mir::BlockTarget,
        /// Target parameters receiving the explicit edge arguments.
        parameters: Vec<mir::Value>,
    },
    /// One propagated panic unwind.
    Unwind {
        /// Branch label entering this propagation block.
        label: bytecode::Label,
    },
}

/// One reusable outgoing call register partition.
#[derive(Debug)]
struct ArgumentRegisters {
    /// Logical argument types in call order.
    types: Vec<bytecode::ValueType>,
    /// Complete contiguous outgoing register range.
    range: bytecode::RegisterSpan,
}

impl ArgumentRegisters {
    /// Return whether this partition has one exact argument type sequence.
    fn matches(&self, types: &[bytecode::ValueType]) -> bool {
        self.types == types
    }
}

impl<'a> FunctionEmitter<'a> {
    /// Create one function emitter and assign fixed register ranges.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
        types: &'a TypeEmitter<'a>,
        function_id: mir::FunctionId,
        function: &'a mir::Function,
    ) -> Result<Self, EmitError> {
        let body = function
            .body
            .as_ref()
            .ok_or_else(|| Self::invalid(module, "missing body"))?;
        let blocks = body
            .blocks()
            .iter()
            .enumerate()
            .map(|(index, block)| (*block, bytecode::Label(index as u32)))
            .collect();
        let registers = RegisterAllocator::new(module, optimized, function, types)?.build()?;
        let RegisterAllocation {
            values: ranges,
            locals,
            liveness,
        } = registers;
        let mut builder = bytecode::FunctionBuilder::new();

        // reserve every fixed value range before instructions are emitted
        for range in ranges.iter().flatten() {
            builder
                .reserve(*range)
                .map_err(|error| Self::invalid(module, &error.to_string()))?;
        }

        Ok(Self {
            optimized,
            types,
            object,
            module,
            function,
            function_id,
            builder,
            ranges,
            locals,
            scratches: Vec::new(),
            arguments: Vec::new(),
            liveness,
            frames: Vec::new(),
            blocks,
            stubs: Vec::new(),
            next_label: body.blocks().len() as u32,
        })
    }

    /// Emit the complete function definition.
    pub(crate) fn emit(mut self) -> Result<FunctionEmission, EmitError> {
        let body = self
            .function
            .body
            .as_ref()
            .ok_or_else(|| self.invalid_input("missing body"))?;

        // retain the complete callable input before coroutine execution begins
        if self.function.coroutine.is_some() {
            let frame = self.entry_frame()?;
            self.frames.push(frame);
        }

        // emit blocks in the operation order shared by object metadata
        let mut blocks = body.blocks().to_vec();
        self.object.order_blocks(&mut blocks);
        for block_id in blocks {
            let label = self.block_label(block_id)?;
            self.define(label)?;
            let block = self.optimized.tree.get(block_id);

            for (index, instruction_id) in block.instructions.iter().enumerate() {
                let instruction = self.optimized.tree.get(*instruction_id);
                let operation = self.builder.begin_operation();
                self.emit_instruction(*instruction_id, instruction)?;
                let frame = self.instruction_frame(block_id, index, operation)?;
                self.frames.push(frame);
            }

            let operation = self.builder.begin_operation();
            self.emit_terminator(self.optimized.tree.get(block.terminator))?;
            let frame = self.terminator_frame(block_id, operation)?;
            self.frames.push(frame);
        }

        // emit deferred transfer and unwind blocks in label order
        for stub in std::mem::take(&mut self.stubs) {
            match stub {
                Stub::Transfer {
                    label,
                    target,
                    parameters,
                } => {
                    self.define(label)?;
                    self.emit_transfer(&target, &parameters)?;
                }
                Stub::Unwind { label } => {
                    self.define(label)?;
                    self.emit_empty(bytecode::Opcode::UNWIND_RESUME)?;
                }
            }
        }

        let module = self.module;
        let body = self
            .builder
            .build()
            .map_err(|error| Self::invalid(module, &error.to_string()))?;

        Ok(FunctionEmission {
            body,
            frames: self.frames,
        })
    }

    /// Build one coroutine's initial frame state.
    fn entry_frame(&self) -> Result<FrameEmission, EmitError> {
        let entry = self
            .function
            .entry()
            .ok_or_else(|| self.invalid_input("missing function entry block"))?;
        let parameters = &self.optimized.tree.get(entry).parameters;
        let mut types =
            Vec::with_capacity(parameters.len() + usize::from(self.function.environment.is_some()));
        let mut registers = Vec::with_capacity(types.capacity());

        // retain the hidden callable environment first
        if let Some(environment) = self.function.environment {
            let ty = self.types.register_type(environment)?;
            types.push(environment);
            registers.push(bytecode::RegisterSpan::new(
                bytecode::RegisterId(0),
                ty.word_count(),
            ));
        }

        // retain every explicit argument in calling order
        for parameter in parameters {
            types.push(parameter.ty);
            registers.push(self.register(parameter.value)?);
        }

        Ok(FrameEmission {
            point: FramePoint::entry(self.function_id),
            types,
            registers,
        })
    }

    /// Build one frame state before a MIR instruction.
    fn instruction_frame(
        &self,
        block: mir::BlockId,
        index: usize,
        operation: u32,
    ) -> Result<FrameEmission, EmitError> {
        let mut values = self
            .liveness
            .value_live_before_instruction(&self.optimized.tree, block, index)
            .into_iter()
            .collect::<Vec<_>>();
        values.sort_unstable();
        let mut locals = self
            .liveness
            .local_live_before_instruction(&self.optimized.tree, block, index)
            .into_iter()
            .collect::<Vec<_>>();
        locals.sort_unstable();

        self.frame(operation, &values, &locals)
    }

    /// Build one frame state before a MIR terminator.
    fn terminator_frame(
        &self,
        block: mir::BlockId,
        operation: u32,
    ) -> Result<FrameEmission, EmitError> {
        let mut values = self
            .liveness
            .value_live_before_terminator(&self.optimized.tree, block)
            .into_iter()
            .collect::<Vec<_>>();
        values.sort_unstable();
        let mut locals = self
            .liveness
            .local_live_before_terminator(block)
            .into_iter()
            .collect::<Vec<_>>();
        locals.sort_unstable();

        self.frame(operation, &values, &locals)
    }

    /// Build one frame state from live MIR values and locals.
    fn frame(
        &self,
        operation: u32,
        values: &[mir::Value],
        locals: &[mir::LocalId],
    ) -> Result<FrameEmission, EmitError> {
        let entry = self
            .function
            .entry()
            .ok_or_else(|| self.invalid_input("missing function entry block"))?;
        let parameters = &self.optimized.tree.get(entry).parameters;
        let mut types = Vec::new();
        let mut registers = Vec::new();

        // retain the hidden environment before every source value
        if let Some(environment) = self.function.environment {
            let ty = self.types.register_type(environment)?;
            types.push(environment);
            registers.push(bytecode::RegisterSpan::new(
                bytecode::RegisterId(0),
                ty.word_count(),
            ));
        }

        // retain live function parameters in calling order
        for parameter in parameters {
            if !values.contains(&parameter.value) {
                continue;
            }
            let range = self.register(parameter.value)?;
            types.push(parameter.ty);
            registers.push(range);
        }

        // retain live locals in declaration order
        for &local in locals {
            let ty = self.optimized.tree.get(local).ty;
            let range = self.local(local)?;
            types.push(ty);
            registers.push(range);
        }

        // retain remaining live SSA values in creation order
        for &value in values {
            if parameters.iter().any(|parameter| parameter.value == value) {
                continue;
            }
            let ty = self
                .function
                .value_type(value)
                .ok_or_else(|| self.invalid_input("missing live value type"))?;
            let range = self.register(value)?;
            types.push(ty);
            registers.push(range);
        }

        Ok(FrameEmission {
            point: FramePoint::operation(Point::new(self.function_id, operation)),
            types,
            registers,
        })
    }

    /// Emit one MIR instruction.
    fn emit_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> Result<(), EmitError> {
        match instruction {
            mir::Instruction::Const { destination, value } => {
                self.emit_constant(*destination, value)
            }
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => self.emit_binary(*destination, *operator, *left, *right),
            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => self.emit_select(*destination, *condition, *then_value, *else_value),
            mir::Instruction::LocalGet { destination, local } => {
                self.emit_local_get(*destination, *local)
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => self.emit_local_address(*destination, *local),
            mir::Instruction::LocalSet { local, value } => self.emit_local_set(*local, *value),
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.emit_global_address(*destination, *global),
            mir::Instruction::Load {
                destination,
                pointer,
                result_type,
            } => self.emit_load(*destination, *pointer, *result_type),
            mir::Instruction::Store { pointer, value } => self.emit_store(*pointer, *value),
            mir::Instruction::Aggregate {
                destination,
                values,
            } => self.emit_aggregate(*destination, *values),
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => self.emit_field_get(*destination, *aggregate, *field),
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                field,
                value,
            } => self.emit_field_set(*destination, *aggregate, *field, *value),
            mir::Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => self.emit_element_get(*destination, *aggregate, *index),
            mir::Instruction::ElementSet {
                destination,
                aggregate,
                index,
                value,
            } => self.emit_element_set(*destination, *aggregate, *index, *value),
            mir::Instruction::VariantNew {
                destination,
                case,
                payload,
                result_type,
            } => self.emit_variant_new(*destination, *case, *payload, *result_type),
            mir::Instruction::VariantTag {
                destination,
                variant,
            } => self.emit_variant_tag(*destination, *variant),
            mir::Instruction::VariantPayload {
                destination,
                variant,
                case,
            } => self.emit_variant_payload(*destination, *variant, *case),
            mir::Instruction::Call { destination, call } => {
                self.emit_call(*destination, call, bytecode::Opcode::CALL)
            }
            mir::Instruction::ContinuationNew {
                destination,
                function,
                arguments,
            } => self.emit_continuation_new(*destination, *function, *arguments),
            mir::Instruction::ContinuationDestroy { continuation } => {
                self.emit_continuation_destroy(*continuation)
            }
            mir::Instruction::WaiterQueue {
                destination,
                waiter,
                value,
            } => self.emit_waiter_queue(*destination, *waiter, *value),
            mir::Instruction::WaiterCancel {
                destination,
                waiter,
            } => self.emit_waiter_cancel(*destination, *waiter),
            mir::Instruction::TaskResolve { destination, value } => {
                self.emit_task_resolve(*destination, *value)
            }
            mir::Instruction::TaskStart {
                destination,
                continuation,
            } => self.emit_task_start(*destination, *continuation),
            mir::Instruction::TaskPark { task, waiter } => self.emit_task_park(*task, *waiter),
            mir::Instruction::TaskCancel { task } => {
                self.emit_task(bytecode::Opcode::TASK_CANCEL, *task)
            }
            mir::Instruction::TaskDetach { task } => {
                self.emit_task(bytecode::Opcode::TASK_DETACH, *task)
            }
            mir::Instruction::Drop { value } => self.emit_drop(*value),
            mir::Instruction::NewZeroed {
                destination,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Value,
                bytecode::Initialization::Zeroed,
                None,
            ),
            mir::Instruction::NewUninit {
                destination,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Value,
                bytecode::Initialization::Uninit,
                None,
            ),
            mir::Instruction::NewComplete {
                destination, value, ..
            } => self.emit_new_complete(*destination, *value),
            mir::Instruction::NewSliceZeroed {
                destination,
                length,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Zeroed,
                Some(*length),
            ),
            mir::Instruction::NewSliceUninit {
                destination,
                length,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Uninit,
                Some(*length),
            ),
            mir::Instruction::Breakpoint => self.emit_empty(bytecode::Opcode::BREAKPOINT),

            mir::Instruction::Error => Err(self.invalid_input("invalid instruction")),

            mir::Instruction::Unary { .. } | mir::Instruction::Cast { .. } => {
                Err(self.unsupported("scalar conversion"))
            }
            mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. } => {
                Err(self.unsupported("function value"))
            }
            mir::Instruction::FieldAddr { .. } | mir::Instruction::ElementAddr { .. } => {
                Err(self.unsupported("address projection"))
            }
            mir::Instruction::SliceView { .. } | mir::Instruction::SliceLength { .. } => {
                Err(self.unsupported("slice operation"))
            }
            mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. } => Err(self.unsupported("dynamic operation")),
            mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorSelect { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. } => Err(self.unsupported("vector operation")),
            mir::Instruction::TensorSplat { .. }
            | mir::Instruction::TensorLoad { .. }
            | mir::Instruction::TensorExtract { .. }
            | mir::Instruction::TensorStore { .. }
            | mir::Instruction::TensorFill { .. }
            | mir::Instruction::TensorCopy { .. }
            | mir::Instruction::TensorReshape { .. }
            | mir::Instruction::TensorBroadcast { .. }
            | mir::Instruction::TensorTranspose { .. }
            | mir::Instruction::TensorCast { .. }
            | mir::Instruction::TensorView { .. }
            | mir::Instruction::TensorSlice { .. }
            | mir::Instruction::TensorPad { .. }
            | mir::Instruction::TensorConcat { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorReduce { .. }
            | mir::Instruction::TensorIndexReduce { .. }
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorConvert { .. } => Err(self.unsupported("tensor operation")),
            mir::Instruction::Free { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::BarrierWrite { .. } => {
                Err(self.unsupported("memory ownership operation"))
            }
            mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. } => Err(self.unsupported("atomic operation")),
            mir::Instruction::Assume { .. } => Err(self.unsupported("assumption")),
            mir::Instruction::ProfileIncrement { counter } => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::PROFILE_INCREMENT);
                instruction.counter(bytecode::CounterId(counter.0));

                self.encode(instruction, &[])
            }
            mir::Instruction::ProfileSample { sampler, value } => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::PROFILE_SAMPLE);
                instruction.sampler(bytecode::SamplerId(sampler.0));
                instruction.register(self.word(*value)?);

                self.encode(instruction, &[])
            }
            mir::Instruction::Intrinsic { .. } => Err(self.unsupported("machine intrinsic")),
        }
    }

    /// Emit one MIR terminator.
    fn emit_terminator(&mut self, terminator: &mir::Terminator) -> Result<(), EmitError> {
        match terminator {
            mir::Terminator::Return { value } => {
                let range = value.map(|value| self.register(value)).transpose()?;
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::RETURN);
                instruction.span(range.unwrap_or_else(bytecode::RegisterSpan::empty));

                self.encode(instruction, &[])
            }
            mir::Terminator::Jump { target } => {
                let label = self.edge_label(terminator, target)?;
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::JUMP);
                instruction.branch(label);

                self.encode(instruction, &[])
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let then_label = self.edge_label(terminator, then_target)?;
                let else_label = self.edge_label(terminator, else_target)?;
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::BRANCH);
                instruction.register(self.word(*condition)?);
                instruction.branch(then_label);
                instruction.branch(else_label);

                self.encode(instruction, &[])
            }
            mir::Terminator::Await {
                park,
                value,
                resume,
                cancel,
                unwind,
            } => self.emit_await(terminator, *park, *value, resume, cancel, unwind.as_ref()),
            mir::Terminator::Yield {
                value,
                resume,
                complete,
                unwind,
            } => self.emit_yield(terminator, *value, resume, complete, unwind.as_ref()),
            mir::Terminator::ContinuationResume {
                continuation,
                value,
                yielded,
                returned,
                unwind,
            } => self.emit_continuation_transfer(
                terminator,
                bytecode::Opcode::CONTINUATION_RESUME,
                *continuation,
                *value,
                yielded,
                returned,
                unwind.as_ref(),
            ),
            mir::Terminator::ContinuationComplete {
                continuation,
                value,
                yielded,
                returned,
                unwind,
            } => self.emit_continuation_transfer(
                terminator,
                bytecode::Opcode::CONTINUATION_COMPLETE,
                *continuation,
                *value,
                yielded,
                returned,
                unwind.as_ref(),
            ),
            mir::Terminator::Panic { payload } => {
                let opcode = if payload.is_some() {
                    bytecode::Opcode::PANIC_VALUE
                } else {
                    bytecode::Opcode::PANIC
                };
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                if let Some(payload) = payload {
                    let ty = self
                        .function
                        .value_type(*payload)
                        .ok_or_else(|| self.invalid_input("missing panic payload type"))?;
                    let ty = self.types.type_id(ty)?;
                    instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
                    instruction.span(self.register(*payload)?);
                }

                self.encode(instruction, &[])
            }
            mir::Terminator::UnwindResume => self.emit_empty(bytecode::Opcode::UNWIND_RESUME),
            mir::Terminator::Unreachable => self.emit_empty(bytecode::Opcode::UNREACHABLE),

            mir::Terminator::Error => Err(self.invalid_input("invalid terminator")),
            mir::Terminator::Check { .. } => Err(self.unsupported("checked branch")),
            mir::Terminator::Switch { .. } | mir::Terminator::VariantSwitch { .. } => {
                Err(self.unsupported("switch"))
            }
            mir::Terminator::Invoke { .. } | mir::Terminator::TailCall { .. } => {
                Err(self.unsupported("call terminator"))
            }
            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => {
                Err(self.unsupported("fallible allocation"))
            }
            mir::Terminator::Abort { .. } => Err(self.unsupported("abort")),
        }
    }

    /// Emit one asynchronous suspension.
    fn emit_await(
        &mut self,
        terminator: &mir::Terminator,
        park: mir::FunctionId,
        value: mir::Value,
        resume: &mir::BlockTarget,
        cancel: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let definitions = self.successor_definitions(terminator, resume)?;
        let park = self.types.function_id(park)?;
        let resume = self.edge_label(terminator, resume)?;
        let cancel = self.edge_label(terminator, cancel)?;
        let unwind = self.unwind_label(terminator, unwind)?;

        // encode the selected park implementation and consumed awaitable
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::AWAIT);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, park.0);
        instruction.span(self.register(value)?);
        instruction.branch(resume);
        instruction.branch(cancel);
        instruction.branch(unwind);

        self.encode(instruction, &definitions)
    }

    /// Emit one generator suspension.
    fn emit_yield(
        &mut self,
        terminator: &mir::Terminator,
        value: mir::Value,
        resume: &mir::BlockTarget,
        complete: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let mut definitions = self.successor_definitions(terminator, resume)?;
        definitions.extend(self.successor_definitions(terminator, complete)?);
        let resume = self.edge_label(terminator, resume)?;
        let complete = self.edge_label(terminator, complete)?;
        let unwind = self.unwind_label(terminator, unwind)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::YIELD);
        instruction.span(self.register(value)?);
        instruction.branch(resume);
        instruction.branch(complete);
        instruction.branch(unwind);

        self.encode(instruction, &definitions)
    }

    /// Emit one scalar MIR binary operation.
    fn emit_binary(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<(), EmitError> {
        let scalar = self
            .register_type(left)?
            .scalar_type()
            .ok_or_else(|| self.invalid_input("unsupported binary value type"))?;
        let operation = match operator {
            mir::BinaryOperator::Add => bytecode::IntegerOperation::Add,
            mir::BinaryOperator::Subtract => bytecode::IntegerOperation::Subtract,
            mir::BinaryOperator::Multiply => bytecode::IntegerOperation::Multiply,
            mir::BinaryOperator::SignedDivide | mir::BinaryOperator::UnsignedDivide => {
                bytecode::IntegerOperation::Divide
            }
            mir::BinaryOperator::SignedRemainder | mir::BinaryOperator::UnsignedRemainder => {
                bytecode::IntegerOperation::Remainder
            }
            mir::BinaryOperator::And => bytecode::IntegerOperation::And,
            mir::BinaryOperator::Or => bytecode::IntegerOperation::Or,
            mir::BinaryOperator::Xor => bytecode::IntegerOperation::Xor,
            mir::BinaryOperator::ShiftLeft => bytecode::IntegerOperation::ShiftLeft,
            mir::BinaryOperator::ArithmeticShiftRight | mir::BinaryOperator::LogicalShiftRight => {
                bytecode::IntegerOperation::ShiftRight
            }
            mir::BinaryOperator::Equal => bytecode::IntegerOperation::Equal,
            mir::BinaryOperator::NotEqual => bytecode::IntegerOperation::NotEqual,
            mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => {
                bytecode::IntegerOperation::LessThan
            }
            mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => {
                bytecode::IntegerOperation::LessEqual
            }
            mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => {
                bytecode::IntegerOperation::GreaterThan
            }
            mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => {
                bytecode::IntegerOperation::GreaterEqual
            }
            _ => return self.emit_float_binary(destination, operator, left, right, scalar),
        };
        let opcode = bytecode::Opcode::integer(operation, scalar)
            .ok_or_else(|| self.invalid_input("unsupported integer operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(left)?);
        instruction.register(self.word(right)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one floating-point MIR binary operation.
    fn emit_float_binary(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        scalar: bytecode::Scalar,
    ) -> Result<(), EmitError> {
        let operation = match operator {
            mir::BinaryOperator::FloatAdd => bytecode::FloatOperation::Add,
            mir::BinaryOperator::FloatSubtract => bytecode::FloatOperation::Subtract,
            mir::BinaryOperator::FloatMultiply => bytecode::FloatOperation::Multiply,
            mir::BinaryOperator::FloatDivide => bytecode::FloatOperation::Divide,
            mir::BinaryOperator::FloatEqual => bytecode::FloatOperation::Equal,
            mir::BinaryOperator::FloatNotEqual => bytecode::FloatOperation::NotEqual,
            mir::BinaryOperator::FloatLessThan => bytecode::FloatOperation::LessThan,
            mir::BinaryOperator::FloatLessEqual => bytecode::FloatOperation::LessEqual,
            mir::BinaryOperator::FloatGreaterThan => bytecode::FloatOperation::GreaterThan,
            mir::BinaryOperator::FloatGreaterEqual => bytecode::FloatOperation::GreaterEqual,
            _ => return Err(self.invalid_input("unsupported binary operator")),
        };
        let opcode = bytecode::Opcode::float(operation, scalar)
            .ok_or_else(|| self.invalid_input("unsupported float operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(left)?);
        instruction.register(self.word(right)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one scalar MIR constant.
    fn emit_constant(
        &mut self,
        destination: mir::Value,
        constant: &mir::Constant,
    ) -> Result<(), EmitError> {
        let result = self.register(destination)?;
        let ty = self.register_type(destination)?;
        let instruction = match constant {
            mir::Constant::Null => {
                bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_NULL)
            }
            mir::Constant::Undefined => {
                bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_UNDEFINED)
            }
            mir::Constant::Zeroed => {
                bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_ZEROED)
            }
            mir::Constant::Uninit => return Err(self.unsupported("uninitialized register value")),
            mir::Constant::Boolean { value } => {
                let mut instruction = bytecode::InstructionBuilder::new(
                    bytecode::Opcode::constant(bytecode::Scalar::Boolean),
                );
                instruction.u64(u64::from(*value));

                instruction
            }
            mir::Constant::Int { value, width, .. } if *width == 128 => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_INT128);
                instruction.u128(*value as u128);

                instruction
            }
            mir::Constant::UInt { value, width } if *width == 128 => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_UINT128);
                instruction.u128(*value);

                instruction
            }
            mir::Constant::Int { value, .. } => self.scalar_constant(ty, *value as u64)?,
            mir::Constant::UInt { value, .. } => self.scalar_constant(ty, *value as u64)?,
            mir::Constant::Float { bits, .. } => self.scalar_constant(ty, *bits)?,
            mir::Constant::Char { value } => self.scalar_constant(ty, u64::from(*value as u32))?,
        };

        self.encode(instruction, &[(result, ty)])
    }

    /// Emit one scalar literal bit pattern.
    fn scalar_constant(
        &self,
        ty: bytecode::ValueType,
        bits: u64,
    ) -> Result<bytecode::InstructionBuilder, EmitError> {
        let scalar = ty
            .scalar_type()
            .ok_or_else(|| self.invalid_input("non-scalar constant"))?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::constant(scalar));
        instruction.u64(bits);

        Ok(instruction)
    }

    /// Emit one scalar or range selection.
    fn emit_select(
        &mut self,
        destination: mir::Value,
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> Result<(), EmitError> {
        let result = self.register(destination)?;
        let then_value = self.register(then_value)?;
        let else_value = self.register(else_value)?;
        let opcode = if result.word_count == 1 {
            bytecode::Opcode::SELECT
        } else {
            bytecode::Opcode::SELECT_RANGE
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(condition)?);
        if result.word_count == 1 {
            instruction.register(then_value.start);
            instruction.register(else_value.start);
        } else {
            instruction.span(then_value);
            instruction.span(else_value);
        }
        let ty = self.register_type(destination)?;

        self.encode(instruction, &[(result, ty)])
    }

    /// Emit one relocatable global reference.
    fn emit_global_address(
        &mut self,
        destination: mir::Value,
        global: mir::GlobalId,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::GLOBAL_ADDRESS);
        let global = self.types.global_id(global)?;
        instruction.relocation(bytecode::RelocationTag::GLOBAL, global.0);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Copy one MIR local into an SSA value range.
    fn emit_local_get(
        &mut self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let source = self.local(local)?;
        let destination = self.register(destination)?;

        self.emit_move(source, destination, ty)
    }

    /// Return one MIR local's stable frame reference.
    fn emit_local_address(
        &mut self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> Result<(), EmitError> {
        let local = self.local(local)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FRAME_ADDRESS);
        instruction.span(local);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Copy one MIR value into a local register range.
    fn emit_local_set(&mut self, local: mir::LocalId, value: mir::Value) -> Result<(), EmitError> {
        let source = self.register(value)?;
        let destination = self.local(local)?;
        let ty = self.register_type(value)?;

        self.emit_move(source, destination, ty)
    }

    /// Emit one scalar or packed value load.
    fn emit_load(
        &mut self,
        destination: mir::Value,
        reference: mir::Value,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let pointer = self.materialize_pointer(reference)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::memory(
                bytecode::MemoryOperation::Load,
                scalar,
            ));
            instruction.register(pointer);

            instruction
        } else {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::LOAD);
            instruction.register(pointer);
            instruction.u32(self.types.byte_len(result_type)?);

            instruction
        };
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one scalar or packed value store.
    fn emit_store(&mut self, reference: mir::Value, value: mir::Value) -> Result<(), EmitError> {
        let ty = self.register_type(value)?;
        let pointer = self.materialize_pointer(reference)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::memory(
                bytecode::MemoryOperation::Store,
                scalar,
            ));
            instruction.register(pointer);
            instruction.register(self.word(value)?);

            instruction
        } else {
            let value_type = self
                .function
                .value_type(value)
                .ok_or_else(|| self.invalid_input("missing stored value type"))?;
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::STORE);
            instruction.register(pointer);
            instruction.span(self.register(value)?);
            instruction.u32(self.types.byte_len(value_type)?);

            instruction
        };

        self.encode(instruction, &[])
    }

    /// Materialize one relative MIR reference as an ephemeral machine pointer.
    fn materialize_pointer(
        &mut self,
        reference: mir::Value,
    ) -> Result<bytecode::RegisterId, EmitError> {
        let ty = self.value_type(reference)?;
        let mir::Type::Reference { space, .. } = self.optimized.tree.get(ty) else {
            return Err(self.invalid_input("memory access requires a reference"));
        };
        let opcode = match space {
            mir::Space::Local => bytecode::Opcode::POINTER_LOCAL,
            mir::Space::Shared => bytecode::Opcode::POINTER_SHARED,
            mir::Space::Frame => bytecode::Opcode::POINTER_FRAME,
            mir::Space::Static => bytecode::Opcode::POINTER_GLOBAL,
        };
        let pointer = self.scratch(bytecode::ValueType::pointer())?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(reference)?);

        self.encode(instruction, &[(pointer, bytecode::ValueType::pointer())])?;

        Ok(pointer.start)
    }

    /// Emit one direct allocation operation.
    fn emit_new(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        result_type: mir::TypeId,
        kind: bytecode::NewKind,
        initialization: bytecode::Initialization,
        length: Option<mir::Value>,
    ) -> Result<(), EmitError> {
        let point = self.object.instruction_point(instruction_id);
        let allocation = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.invalid_input("missing allocation site"))?;
        let result = self.types.register_type(result_type)?;
        let reference = match kind {
            bytecode::NewKind::Value => result.reference_type(),
            bytecode::NewKind::Slice => result.slice_reference(),
        }
        .ok_or_else(|| self.invalid_input("allocation result is not a reference"))?;
        let operation = bytecode::New {
            space: reference.space(),
            ownership: reference.kind(),
            kind,
            initialization,
            is_fallible: false,
        };
        let opcode = bytecode::Opcode::new(operation)
            .ok_or_else(|| self.invalid_input("invalid allocation operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);
        if let Some(length) = length {
            instruction.register(self.word(length)?);
        }
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Complete one initialization token without runtime work.
    fn emit_new_complete(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let source = self.register(value)?;
        let destination = self.register(destination)?;

        self.emit_move(source, destination, ty)
    }

    /// Emit one generated destructor call.
    fn emit_drop(&mut self, value: mir::Value) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let destructor = self
            .optimized
            .drops
            .destructor(ty)
            .ok_or_else(|| self.invalid_input("missing destructor"))?;
        let destructor = self.types.function_id(destructor)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DROP);
        instruction.span(self.register(value)?);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, destructor.0);

        self.encode(instruction, &[])
    }

    /// Construct one packed aggregate from logical source values.
    fn emit_aggregate(
        &mut self,
        destination: mir::Value,
        values: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        let ty = self
            .function
            .value_type(destination)
            .ok_or_else(|| self.invalid_input("missing aggregate type"))?;
        let values = self.optimized.tree.get_values(values);
        let placements = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let registers = self.register(*value)?;
                let (byte_offset, byte_len) = self.types.placement(ty, index as u32)?;

                Ok(bytecode::Placement::new(registers, byte_offset, byte_len))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::AGGREGATE);
        instruction
            .placements(&placements)
            .map_err(|error| self.bytecode_error(error))?;
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Read one source-ordered field from a packed value.
    fn emit_field_get(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let field = self.types.field(aggregate_type, field)?;

        self.emit_extract(destination, aggregate, field.offset, field.size)
    }

    /// Replace one source-ordered field inside a packed value.
    fn emit_field_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let field = self.types.field(aggregate_type, field)?;

        self.emit_insert(destination, aggregate, field.offset, field.size, value)
    }

    /// Read one fixed element from a packed value.
    fn emit_element_get(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let (byte_offset, byte_len) = self.types.element(aggregate_type, index)?;

        self.emit_extract(destination, aggregate, byte_offset, byte_len)
    }

    /// Replace one fixed element inside a packed value.
    fn emit_element_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let (byte_offset, byte_len) = self.types.element(aggregate_type, index)?;

        self.emit_insert(destination, aggregate, byte_offset, byte_len, value)
    }

    /// Read one exact byte range from a packed value.
    fn emit_extract(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        byte_offset: u32,
        byte_len: u32,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::EXTRACT);
        instruction.span(self.register(aggregate)?);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Replace one exact byte range inside a packed value.
    fn emit_insert(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        byte_offset: u32,
        byte_len: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::INSERT);
        instruction.span(self.register(aggregate)?);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);
        instruction.span(self.register(value)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Construct one packed variant case.
    fn emit_variant_new(
        &mut self,
        destination: mir::Value,
        case: u32,
        payload: Option<mir::Value>,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::VARIANT_NEW);
        let result_type = self.types.type_id(result_type)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, result_type.0);
        instruction.u32(case);
        let payload = payload
            .map(|payload| self.register(payload))
            .transpose()?
            .unwrap_or_else(bytecode::RegisterSpan::empty);
        instruction.span(payload);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Read one packed variant's logical discriminant.
    fn emit_variant_tag(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self
            .function
            .value_type(variant)
            .ok_or_else(|| self.invalid_input("missing variant type"))?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::VARIANT_TAG);
        instruction.span(self.register(variant)?);
        let ty = self.types.type_id(ty)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, ty.0);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Read one statically selected variant payload.
    fn emit_variant_payload(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
        case: u32,
    ) -> Result<(), EmitError> {
        let variant_type = self.value_type(variant)?;
        let (byte_offset, byte_len) = self.types.variant(variant_type, case)?;

        self.emit_extract(destination, variant, byte_offset, byte_len)
    }

    /// Emit one direct call instruction.
    fn emit_call(
        &mut self,
        destination: Option<mir::Value>,
        call: &mir::Call,
        opcode: bytecode::Opcode,
    ) -> Result<(), EmitError> {
        let mir::Callee::Direct { function } = call.callee else {
            return Err(self.invalid_input("unsupported call target"));
        };
        let function = self.types.function_id(function)?;
        let arguments = self.optimized.tree.get_values(call.arguments);
        let arguments = self.emit_arguments(arguments)?;

        // anchor the logical Program point to the call after argument moves
        self.builder
            .anchor_operation()
            .map_err(|error| self.bytecode_error(error))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
        instruction.span(arguments);
        let definitions = destination
            .map(|value| self.definition(value))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();

        self.encode(instruction, &definitions)
    }

    /// Emit one ready continuation construction.
    fn emit_continuation_new(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
        arguments: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        let function = self.types.function_id(function)?;
        let arguments = self.optimized.tree.get_values(arguments);
        let arguments = self.emit_arguments(arguments)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTINUATION_NEW);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
        instruction.span(arguments);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit destruction of one continuation.
    fn emit_continuation_destroy(&mut self, continuation: mir::Value) -> Result<(), EmitError> {
        let mut instruction =
            bytecode::InstructionBuilder::new(bytecode::Opcode::CONTINUATION_DESTROY);
        instruction.register(self.word(continuation)?);

        self.encode(instruction, &[])
    }

    /// Emit one asynchronous waiter settlement.
    fn emit_waiter_queue(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let ty = self.types.type_id(ty)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::WAITER_QUEUE);
        instruction.register(self.word(waiter)?);
        instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
        instruction.span(self.register(value)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one asynchronous waiter cancellation.
    fn emit_waiter_cancel(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::WAITER_CANCEL);
        instruction.register(self.word(waiter)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one already completed task.
    fn emit_task_resolve(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let ty = self.types.type_id(ty)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_RESOLVE);
        instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
        instruction.span(self.register(value)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit one eager task start.
    fn emit_task_start(
        &mut self,
        destination: mir::Value,
        continuation: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_START);
        instruction.register(self.word(continuation)?);
        let definition = self.definition(destination)?;

        self.encode(instruction, &[definition])
    }

    /// Emit parking one waiter on a task.
    fn emit_task_park(&mut self, task: mir::Value, waiter: mir::Value) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_PARK);
        instruction.register(self.word(task)?);
        instruction.register(self.word(waiter)?);

        self.encode(instruction, &[])
    }

    /// Emit one scalar task operation.
    fn emit_task(&mut self, opcode: bytecode::Opcode, task: mir::Value) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(task)?);

        self.encode(instruction, &[])
    }

    /// Emit one continuation control transfer.
    fn emit_continuation_transfer(
        &mut self,
        terminator: &mir::Terminator,
        opcode: bytecode::Opcode,
        continuation: mir::Value,
        value: mir::Value,
        yielded: &mir::BlockTarget,
        returned: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let mut definitions = self.successor_definitions(terminator, yielded)?;
        definitions.extend(self.successor_definitions(terminator, returned)?);
        let yielded = self.edge_label(terminator, yielded)?;
        let returned = self.edge_label(terminator, returned)?;
        let unwind = self.unwind_label(terminator, unwind)?;

        // encode both completion paths and their direct destinations
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(continuation)?);
        instruction.span(self.register(value)?);
        instruction.branch(yielded);
        instruction.branch(returned);
        instruction.branch(unwind);

        self.encode(instruction, &definitions)
    }

    /// Move scattered values into one reusable contiguous call register partition.
    fn emit_arguments(
        &mut self,
        arguments: &[mir::Value],
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        let sources = arguments
            .iter()
            .map(|value| self.register(*value))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(registers) = Self::pack(&sources) {
            return Ok(registers);
        }
        let types = arguments
            .iter()
            .map(|value| self.register_type(*value))
            .collect::<Result<Vec<_>, _>>()?;

        // reuse a partition with the exact physical argument sequence
        let existing = self
            .arguments
            .iter()
            .position(|registers| registers.matches(&types))
            .map(|index| self.arguments[index].range);
        let range = if let Some(range) = existing {
            range
        } else {
            let registers = types
                .iter()
                .copied()
                .map(|ty| self.append_registers(ty))
                .collect::<Result<Vec<_>, _>>()?;

            Self::pack(&registers).ok_or_else(|| self.invalid_input("outgoing call registers"))?
        };
        let mut target = range.start.0;

        // copy each logical argument before entering the callee
        for (source, ty) in sources.into_iter().zip(types.iter().copied()) {
            let registers =
                bytecode::RegisterSpan::new(bytecode::RegisterId(target), ty.word_count());
            self.emit_move(source, registers, ty)?;
            target += ty.word_count();
        }

        // retain newly allocated registers for later calls with this sequence
        if existing.is_none() {
            self.arguments.push(ArgumentRegisters { types, range });
        }

        Ok(range)
    }

    /// Pack adjacent register ranges into one physical window.
    fn pack(ranges: &[bytecode::RegisterSpan]) -> Option<bytecode::RegisterSpan> {
        let Some(first) = ranges.first() else {
            return Some(bytecode::RegisterSpan::empty());
        };
        let mut end = u32::from(first.start.0);
        for range in ranges {
            if u32::from(range.start.0) != end {
                return None;
            }
            end += u32::from(range.word_count);
        }
        let word_count = u16::try_from(end - u32::from(first.start.0)).ok()?;

        Some(bytecode::RegisterSpan::new(first.start, word_count))
    }

    /// Emit one deferred block argument transfer.
    fn emit_transfer(
        &mut self,
        target: &mir::BlockTarget,
        parameters: &[mir::Value],
    ) -> Result<(), EmitError> {
        let arguments = target.arguments(&self.optimized.tree);
        if arguments.len() != parameters.len() {
            return Err(self.invalid_input("block argument count"));
        }

        let mut moves = arguments
            .iter()
            .zip(parameters)
            .map(|(argument, parameter)| {
                Ok((
                    self.register(*argument)?,
                    self.register(*parameter)?,
                    self.register_type(*argument)?,
                ))
            })
            .collect::<Result<Vec<_>, EmitError>>()?;
        moves.retain(|(source, destination, _)| source != destination);

        // emit acyclic moves first and preserve one source to break each cycle
        while !moves.is_empty() {
            let ready = moves.iter().position(|(_, destination, _)| {
                !moves.iter().any(|(source, _, _)| source == destination)
            });
            if let Some(index) = ready {
                let (source, destination, ty) = moves.remove(index);
                self.emit_move(source, destination, ty)?;
            } else {
                let source = moves[0].0;
                let ty = moves[0].2;
                let scratch = self.scratch(ty)?;
                self.emit_move(source, scratch, ty)?;
                for (candidate, _, _) in &mut moves {
                    if *candidate == source {
                        *candidate = scratch;
                    }
                }
            }
        }

        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::JUMP);
        instruction.branch(self.block_label(target.block)?);

        self.encode(instruction, &[])
    }

    /// Emit one exact register range move.
    fn emit_move(
        &mut self,
        source: bytecode::RegisterSpan,
        destination: bytecode::RegisterSpan,
        ty: bytecode::ValueType,
    ) -> Result<(), EmitError> {
        if source.word_count != ty.word_count() || destination.word_count != ty.word_count() {
            return Err(self.invalid_input("block argument width"));
        }
        let opcode = if source.word_count == 1 {
            bytecode::Opcode::MOVE
        } else {
            bytecode::Opcode::MOVE_RANGE
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        if source.word_count == 1 {
            instruction.register(source.start);
        } else {
            instruction.span(source);
        }

        self.encode(instruction, &[(destination, ty)])
    }

    /// Return one reusable exact-type scratch range.
    fn scratch(&mut self, ty: bytecode::ValueType) -> Result<bytecode::RegisterSpan, EmitError> {
        if let Some((_, registers)) = self.scratches.iter().find(|(other, _)| *other == ty) {
            return Ok(*registers);
        }

        // append one reusable scratch range for this exact value type
        let registers = self.append_registers(ty)?;
        self.scratches.push((ty, registers));

        Ok(registers)
    }

    /// Append one typed range to the physical register partition.
    fn append_registers(
        &mut self,
        ty: bytecode::ValueType,
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        let start = self.builder.register_count();
        let registers = bytecode::RegisterSpan::new(bytecode::RegisterId(start), ty.word_count());
        self.builder
            .reserve(registers)
            .map_err(|error| self.bytecode_error(error))?;

        Ok(registers)
    }

    /// Return a direct block label or defer one argument transfer.
    fn edge_label(
        &mut self,
        terminator: &mir::Terminator,
        target: &mir::BlockTarget,
    ) -> Result<bytecode::Label, EmitError> {
        if target.arguments(&self.optimized.tree).is_empty() {
            return self.block_label(target.block);
        }

        let parameters = terminator
            .successor_parameters(&self.optimized.tree, target.block)
            .iter()
            .map(|parameter| parameter.value)
            .collect();
        let label = bytecode::Label(self.next_label);
        self.next_label += 1;
        self.stubs.push(Stub::Transfer {
            label,
            target: target.clone(),
            parameters,
        });

        Ok(label)
    }

    /// Return direct result destinations at one successor.
    fn successor_definitions(
        &self,
        terminator: &mir::Terminator,
        target: &mir::BlockTarget,
    ) -> Result<Vec<(bytecode::RegisterSpan, bytecode::ValueType)>, EmitError> {
        let block = self.optimized.tree.get(target.block);
        let count = terminator.successor_result_count(target.block);
        if block.parameters.len() < count {
            return Err(self.invalid_input("missing successor result parameter"));
        }

        block.parameters[..count]
            .iter()
            .map(|parameter| self.definition(parameter.value))
            .collect()
    }

    /// Return an explicit unwind edge or one shared propagation block.
    fn unwind_label(
        &mut self,
        terminator: &mir::Terminator,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<bytecode::Label, EmitError> {
        if let Some(unwind) = unwind {
            return self.edge_label(terminator, unwind);
        }

        // reuse the function's one propagated unwind block
        if let Some(label) = self.stubs.iter().find_map(|stub| match stub {
            Stub::Unwind { label } => Some(*label),
            Stub::Transfer { .. } => None,
        }) {
            return Ok(label);
        }

        let label = bytecode::Label(self.next_label);
        self.next_label += 1;
        self.stubs.push(Stub::Unwind { label });

        Ok(label)
    }

    /// Emit one instruction without destinations.
    fn emit_empty(&mut self, opcode: bytecode::Opcode) -> Result<(), EmitError> {
        self.encode(bytecode::InstructionBuilder::new(opcode), &[])
    }

    /// Emit one physical bytecode instruction.
    fn encode(
        &mut self,
        instruction: bytecode::InstructionBuilder,
        definitions: &[(bytecode::RegisterSpan, bytecode::ValueType)],
    ) -> Result<(), EmitError> {
        self.encode_at(instruction, definitions).map(|_| ())
    }

    /// Emit one physical bytecode instruction and return its byte offset.
    fn encode_at(
        &mut self,
        instruction: bytecode::InstructionBuilder,
        definitions: &[(bytecode::RegisterSpan, bytecode::ValueType)],
    ) -> Result<bytecode::CodeOffset, EmitError> {
        let ranges = definitions
            .iter()
            .map(|(registers, _)| *registers)
            .collect::<Vec<_>>();
        self.builder
            .emit(instruction, &ranges)
            .map_err(|error| self.bytecode_error(error))
    }

    /// Define one physical bytecode label.
    fn define(&mut self, label: bytecode::Label) -> Result<(), EmitError> {
        self.builder
            .define(label)
            .map_err(|error| self.bytecode_error(error))
    }

    /// Return one MIR value's fixed register range.
    fn register(&self, value: mir::Value) -> Result<bytecode::RegisterSpan, EmitError> {
        self.ranges
            .get(value.0 as usize)
            .copied()
            .flatten()
            .ok_or_else(|| self.invalid_input("missing value register"))
    }

    /// Return one MIR value's physical registers and bytecode type.
    fn definition(
        &self,
        value: mir::Value,
    ) -> Result<(bytecode::RegisterSpan, bytecode::ValueType), EmitError> {
        Ok((self.register(value)?, self.register_type(value)?))
    }

    /// Return one MIR local's permanent register range.
    fn local(&self, local: mir::LocalId) -> Result<bytecode::RegisterSpan, EmitError> {
        self.locals
            .get(&local)
            .copied()
            .ok_or_else(|| self.invalid_input("missing local register"))
    }

    /// Return one single-word MIR value register.
    fn word(&self, value: mir::Value) -> Result<bytecode::RegisterId, EmitError> {
        let range = self.register(value)?;
        if range.word_count != 1 {
            return Err(self.invalid_input("expected one-register value"));
        }

        Ok(range.start)
    }

    /// Return one MIR value's bytecode representation.
    fn register_type(&self, value: mir::Value) -> Result<bytecode::ValueType, EmitError> {
        let ty = self.value_type(value)?;

        self.types.register_type(ty)
    }

    /// Return one MIR value's type.
    fn value_type(&self, value: mir::Value) -> Result<mir::TypeId, EmitError> {
        self.function
            .value_type(value)
            .ok_or_else(|| self.invalid_input("missing value type"))
    }

    /// Return one MIR block's dense bytecode label.
    fn block_label(&self, block: mir::BlockId) -> Result<bytecode::Label, EmitError> {
        self.blocks
            .get(&block)
            .copied()
            .ok_or_else(|| self.invalid_input("missing block label"))
    }

    /// Build one bytecode assembly diagnostic.
    fn bytecode_error(&self, error: bytecode::Error) -> EmitError {
        self.invalid_input(&error.to_string())
    }

    /// Build one invalid bytecode input diagnostic.
    fn invalid_input(&self, message: &str) -> EmitError {
        Self::invalid(self.module, message)
    }

    /// Build one unsupported bytecode emission diagnostic.
    fn unsupported(&self, message: &str) -> EmitError {
        EmitError::UnsupportedConstruct {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }

    /// Build one invalid bytecode input diagnostic without emitter state.
    fn invalid(module: ModuleId, message: &str) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: module.into(),
            module,
            message: message.to_owned(),
        }
    }
}
