use tspp_core::FxIndexMap;

use tspp_artifact::MirOptimized;
use tspp_bytecode as bytecode;
use tspp_mir as mir;
use tspp_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::super::TypeEmitter;
use super::FrameEmission;
use super::register::{RegisterAllocation, RegisterAllocator};

/// Emit one MIR function into relocatable bytecode.
#[derive(Debug)]
pub(crate) struct FunctionEmitter<'a> {
    /// Optimized MIR being emitted.
    pub(super) optimized: &'a MirOptimized,
    /// Object-local type identities.
    pub(super) types: &'a TypeEmitter<'a>,
    /// Object-local identity assignments.
    pub(super) object: &'a ObjectEmitter,
    /// Module owning the function.
    pub(super) module: ModuleId,
    /// MIR function declaration.
    pub(super) function: &'a mir::Function,
    /// Object-local function identity.
    pub(super) function_id: mir::FunctionId,
    /// Physical instruction builder.
    pub(super) builder: bytecode::FunctionBuilder,
    /// Fixed registers keyed by MIR value identity.
    pub(super) values: Vec<Option<bytecode::RegisterSpan>>,
    /// Permanent register ranges keyed by MIR local identity.
    pub(super) locals: FxIndexMap<mir::LocalId, bytecode::RegisterSpan>,
    /// Reusable exact-type scratch ranges.
    pub(super) scratches: Vec<(bytecode::ValueType, bytecode::RegisterSpan)>,
    /// Scratch ranges reserved by the current operation.
    pub(super) scratch_count: usize,
    /// Reusable contiguous outgoing call registers.
    pub(super) arguments: Vec<ArgumentRegisters>,
    /// Frame states in canonical operation order.
    pub(super) frames: Vec<FrameEmission>,
    /// Branch labels keyed by MIR block identity.
    pub(super) blocks: FxIndexMap<mir::BlockId, bytecode::Label>,
    /// Deferred blocks emitted after the main blocks.
    pub(super) stubs: Vec<Stub>,
    /// Next dense branch label.
    pub(super) next_label: u32,
}

/// One bytecode function emission and its object sections.
#[derive(Debug)]
pub(crate) struct FunctionEmission {
    /// The encoded function body and object-relative references.
    pub(crate) body: bytecode::FunctionBody,
    /// Logical and physical frame states in canonical operation order.
    pub(crate) frames: Vec<FrameEmission>,
}

/// One deferred bytecode block.
#[derive(Debug)]
pub(super) enum Stub {
    /// One parallel block argument transfer.
    Transfer {
        /// Branch label entering this transfer.
        label: bytecode::Label,
        /// MIR edge target and arguments.
        target: mir::BlockTarget,
        /// Target parameters receiving the explicit edge arguments.
        parameters: Vec<mir::Value>,
    },
    /// One unreachable fallback for an exhaustive transfer.
    Unreachable {
        /// Branch label entering this fallback.
        label: bytecode::Label,
    },
}

/// One reusable outgoing call register partition.
#[derive(Debug)]
pub(super) struct ArgumentRegisters {
    /// Logical argument types in call order.
    pub(super) types: Vec<bytecode::ValueType>,
    /// Complete contiguous outgoing register range.
    pub(super) range: bytecode::RegisterSpan,
}

impl ArgumentRegisters {
    /// Return whether this partition has one exact argument type sequence.
    pub(super) fn matches(&self, types: &[bytecode::ValueType]) -> bool {
        self.types == types
    }
}

impl<'a> FunctionEmitter<'a> {
    /// Start one operation, releasing its scratch registers.
    fn begin_operation(&mut self) {
        self.scratch_count = 0;
        self.builder.begin_operation();
    }

    /// Create one function emitter and assign fixed register ranges.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
        types: &'a TypeEmitter<'a>,
        function_id: mir::FunctionId,
        function: &'a mir::Function,
        liveness: &mir::LivenessTable,
    ) -> Result<Self, EmitError> {
        // require a defined function body
        let body = function
            .body
            .as_ref()
            .ok_or_else(|| ObjectEmitter::internal(module, "missing body"))?;
        let blocks = body
            .blocks()
            .iter()
            .enumerate()
            .map(|(index, block)| (*block, bytecode::Label(index as u32)))
            .collect();
        let registers =
            RegisterAllocator::new(module, optimized, function, types, object, liveness)?
                .build()?;
        let RegisterAllocation {
            register_count,
            values,
            locals,
        } = registers;
        let mut builder = bytecode::FunctionBuilder::new();

        // reserve the complete fixed register partition, including the hidden environment
        if register_count != 0 {
            let registers = bytecode::RegisterSpan::new(bytecode::RegisterId(0), register_count);
            builder
                .reserve(registers)
                .map_err(|error| ObjectEmitter::internal(module, &error.to_string()))?;
        }

        Ok(Self {
            optimized,
            types,
            object,
            module,
            function,
            function_id,
            builder,
            values,
            locals,
            scratches: Vec::new(),
            scratch_count: 0,
            arguments: Vec::new(),
            frames: Vec::new(),
            blocks,
            stubs: Vec::new(),
            next_label: body.blocks().len() as u32,
        })
    }

    /// Emit the complete function definition.
    pub(crate) fn emit(mut self) -> Result<FunctionEmission, EmitError> {
        // read the function body and entry parameters
        let body = self
            .function
            .body
            .as_ref()
            .ok_or_else(|| self.internal("missing body"))?;

        // emit blocks in the operation order shared by object metadata
        let mut blocks = body.blocks().to_vec();
        self.object.order_blocks(&mut blocks);
        for block_id in blocks {
            let label = self.block_label(block_id)?;
            self.define(label)?;
            let block = self.optimized.tree.get(block_id);

            for instruction_id in &block.instructions {
                let instruction = self.optimized.tree.get(*instruction_id);
                self.begin_operation();
                self.emit_instruction(*instruction_id, instruction)?;
            }

            self.begin_operation();
            self.emit_terminator(block_id, self.optimized.tree.get(block.terminator))?;
        }

        // emit deferred transfer and fallback blocks in label order
        for stub in std::mem::take(&mut self.stubs) {
            self.begin_operation();
            match stub {
                Stub::Transfer {
                    label,
                    target,
                    parameters,
                } => {
                    self.define(label)?;
                    self.emit_transfer(&target, &parameters)?;
                }
                Stub::Unreachable { label } => {
                    self.define(label)?;
                    self.emit_empty(bytecode::Opcode::UNREACHABLE)?;
                }
            }
        }

        // project selected logical states into this function's physical registers
        for state in self
            .object
            .frames()
            .iter()
            .filter(|state| state.point.function() == self.function_id)
        {
            let frame = self.frame(state)?;
            self.frames.push(frame);
        }

        let module = self.module;
        let body = self
            .builder
            .build()
            .map_err(|error| ObjectEmitter::internal(module, &error.to_string()))?;

        Ok(FunctionEmission {
            body,
            frames: self.frames,
        })
    }

    /// Emit one instruction without destinations.
    pub(super) fn emit_empty(&mut self, opcode: bytecode::Opcode) -> Result<(), EmitError> {
        self.encode(bytecode::InstructionBuilder::new(opcode), &[])
    }

    /// Emit one physical bytecode instruction.
    pub(super) fn encode(
        &mut self,
        instruction: bytecode::InstructionBuilder,
        destinations: &[bytecode::RegisterSpan],
    ) -> Result<(), EmitError> {
        self.encode_at(instruction, destinations).map(|_| ())
    }

    /// Emit one physical bytecode instruction and return its byte offset.
    pub(super) fn encode_at(
        &mut self,
        instruction: bytecode::InstructionBuilder,
        destinations: &[bytecode::RegisterSpan],
    ) -> Result<bytecode::CodeOffset, EmitError> {
        self.builder
            .emit(instruction, destinations)
            .map_err(|error| self.bytecode_error(error))
    }

    /// Define one physical bytecode label.
    pub(super) fn define(&mut self, label: bytecode::Label) -> Result<(), EmitError> {
        self.builder
            .define(label)
            .map_err(|error| self.bytecode_error(error))
    }

    /// Return one MIR block's dense bytecode label.
    pub(super) fn block_label(&self, block: mir::BlockId) -> Result<bytecode::Label, EmitError> {
        self.blocks
            .get(&block)
            .copied()
            .ok_or_else(|| self.internal("missing block label"))
    }

    /// Return one bytecode numeric conversion policy.
    pub(super) const fn convert_mode(mode: mir::ConvertMode) -> bytecode::ConvertMode {
        match mode {
            mir::ConvertMode::Exact => bytecode::ConvertMode::Exact,
            mir::ConvertMode::RoundTiesEven => bytecode::ConvertMode::RoundTiesEven,
            mir::ConvertMode::RoundTowardZero => bytecode::ConvertMode::RoundTowardZero,
            mir::ConvertMode::RoundFloor => bytecode::ConvertMode::RoundFloor,
            mir::ConvertMode::RoundCeil => bytecode::ConvertMode::RoundCeil,
            mir::ConvertMode::Saturate => bytecode::ConvertMode::Saturate,
        }
    }

    /// Build one bytecode assembly diagnostic.
    pub(super) fn bytecode_error(&self, error: bytecode::Error) -> EmitError {
        self.internal(&error.to_string())
    }

    /// Build one internal bytecode emission diagnostic.
    pub(super) fn internal(&self, message: &str) -> EmitError {
        ObjectEmitter::internal(self.module, message)
    }
}
