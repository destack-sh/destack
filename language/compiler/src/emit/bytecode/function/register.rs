use destack_core::FxIndexMap;

use destack_artifact::MirOptimized;
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::emit::bytecode::TypeEmitter;
use crate::{EmitError, ObjectEmitter};

use super::{ArgumentRegisters, FunctionEmitter};

/// Bytecode register assignments for one MIR function.
pub(crate) struct RegisterAllocation {
    /// Complete physical register word count.
    pub(crate) register_count: u16,
    /// Register ranges keyed by MIR value identity.
    pub(crate) values: Vec<Option<bytecode::RegisterSpan>>,
    /// Permanent register ranges keyed by MIR local identity.
    pub(crate) locals: FxIndexMap<mir::LocalId, bytecode::RegisterSpan>,
}

/// Assign reusable bytecode register ranges to MIR values.
pub(crate) struct RegisterAllocator<'a> {
    /// Module receiving diagnostics.
    module: ModuleId,
    /// MIR tree containing the function body.
    tree: &'a mir::Tree,
    /// MIR function being allocated.
    function: &'a mir::Function,
    /// Object-local bytecode type projection.
    types: &'a TypeEmitter<'a>,
    /// Common object operation order.
    object: &'a ObjectEmitter,
    /// Cached liveness of the optimized MIR function.
    liveness: &'a mir::LivenessTable,
    /// Bytecode value types keyed by MIR value.
    value_types: Vec<Option<bytecode::ValueType>>,
    /// Assigned register ranges keyed by MIR value.
    ranges: Vec<Option<bytecode::RegisterSpan>>,
    /// Leading calling-convention words unavailable to MIR values.
    reserved_word_count: u16,
    /// Highest occupied register word.
    register_count: u32,
}

/// One MIR value's conservative live interval.
#[derive(Debug, Clone, Copy)]
struct Interval {
    /// MIR value occupying the interval.
    value: mir::Value,
    /// First logical operation that needs the value.
    start: u32,
    /// Last logical operation that needs the value.
    end: u32,
    /// The bytecode value type determining the physical width.
    ty: bytecode::ValueType,
}

/// One register range occupied during linear scan.
#[derive(Debug, Clone, Copy)]
struct Active {
    /// Last logical operation that needs the range.
    end: u32,
    /// Physical register range occupied by the value.
    registers: bytecode::RegisterSpan,
}

impl<'a> RegisterAllocator<'a> {
    /// Create one register allocator.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        function: &'a mir::Function,
        types: &'a TypeEmitter<'a>,
        object: &'a ObjectEmitter,
        liveness: &'a mir::LivenessTable,
    ) -> Result<Self, EmitError> {
        // project every MIR value into its fixed bytecode representation
        let body = function
            .body
            .as_ref()
            .ok_or_else(|| ObjectEmitter::internal(module, "missing body"))?;
        let value_types = body
            .value_types()
            .iter()
            .copied()
            .map(|ty| ty.map(|ty| types.register_type(ty)).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        let value_count = value_types.len();

        // reserve the hidden environment word for the complete function lifetime
        let environment = function
            .environment
            .map(|ty| types.register_type(ty))
            .transpose()?;
        let reserved_word_count = match environment {
            Some(ty) => ty.word_count(),
            None => 0,
        };
        if reserved_word_count > 1 {
            return Err(ObjectEmitter::internal(
                module,
                "environment exceeds one register word",
            ));
        }

        Ok(Self {
            module,
            tree: &optimized.tree,
            function,
            types,
            object,
            liveness,
            value_types,
            ranges: vec![None; value_count],
            reserved_word_count,
            register_count: u32::from(reserved_word_count),
        })
    }

    /// Build compact reusable register ranges.
    pub(crate) fn build(mut self) -> Result<RegisterAllocation, EmitError> {
        let intervals = self.collect_intervals()?;
        self.allocate_parameters()?;
        self.allocate_intervals(intervals)?;
        let locals = self.allocate_locals()?;

        let register_count = u16::try_from(self.register_count).map_err(|_| self.unsupported())?;

        Ok(RegisterAllocation {
            register_count,
            values: self.ranges,
            locals,
        })
    }

    /// Assign one permanent register range to every MIR local.
    fn allocate_locals(
        &mut self,
    ) -> Result<FxIndexMap<mir::LocalId, bytecode::RegisterSpan>, EmitError> {
        let mut locals =
            FxIndexMap::with_capacity_and_hasher(self.function.locals().len(), Default::default());

        // append locals after reusable SSA ranges so their addresses remain stable
        for &local_id in self.function.locals() {
            let local = self.tree.get(local_id);
            let ty = self.types.register_type(local.ty)?;
            let registers = self.append_range(ty.word_count())?;
            locals.insert(local_id, registers);
        }

        Ok(locals)
    }

    /// Collect conservative live intervals in function layout order.
    fn collect_intervals(&self) -> Result<Vec<Interval>, EmitError> {
        // allocate one interval per SSA value
        let mut intervals = vec![None; self.value_types.len()];
        let mut point = 0;

        // reserve the leading register window for function parameters
        for parameter in &self.function.parameters {
            self.touch(&mut intervals, parameter.value, point)?;
        }

        let mut blocks = self.function.blocks().to_vec();
        self.object.order_blocks(&mut blocks);
        for block_id in blocks {
            let block = self.tree.get(block_id);

            // retain incoming values and block parameters at block entry
            for value in self.liveness.value_live_in(block_id) {
                self.touch(&mut intervals, value, point)?;
            }
            for parameter in &block.parameters {
                self.touch(&mut intervals, parameter.value, point)?;
            }
            point += 1;

            // retain each instruction's sources and destination together
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                for value in instruction.uses() {
                    self.touch(&mut intervals, value, point)?;
                }
                if let Some(arguments) = instruction.argument_slice() {
                    for &value in self.tree.get_values(arguments) {
                        self.touch(&mut intervals, value, point)?;
                    }
                }
                if let Some(value) = instruction.destination() {
                    self.touch(&mut intervals, value, point)?;
                }
                point += 1;
            }

            // retain outgoing values and terminator operands at block exit
            let terminator = self.tree.get(block.terminator);
            for value in terminator.uses(self.tree) {
                self.touch(&mut intervals, value, point)?;
            }
            for value in self.liveness.value_live_out(block_id) {
                self.touch(&mut intervals, value, point)?;
            }
            point += 1;
        }

        Ok(intervals.into_iter().flatten().collect())
    }

    /// Extend one value's interval through one logical operation.
    fn touch(
        &self,
        intervals: &mut [Option<Interval>],
        value: mir::Value,
        point: u32,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(value)?;
        let interval = intervals
            .get_mut(value.0 as usize)
            .ok_or_else(|| ObjectEmitter::internal(self.module, "missing value"))?;

        if let Some(interval) = interval {
            interval.end = point;
        } else {
            *interval = Some(Interval {
                value,
                start: point,
                end: point,
                ty,
            });
        }

        Ok(())
    }

    /// Place function parameters into one contiguous leading register window.
    fn allocate_parameters(&mut self) -> Result<(), EmitError> {
        // place explicit parameters after the environment reserved during construction
        for parameter in &self.function.parameters {
            let ty = self.register_type(parameter.value)?;
            let registers = self.append_range(ty.word_count())?;
            self.ranges[parameter.value.0 as usize] = Some(registers);
        }

        Ok(())
    }

    /// Place each interval into the lowest available contiguous register range.
    fn allocate_intervals(&mut self, mut intervals: Vec<Interval>) -> Result<(), EmitError> {
        // process parameter intervals first at a shared entry point
        intervals.sort_by_key(|interval| {
            let is_parameter = self.ranges[interval.value.0 as usize].is_some();

            (interval.start, !is_parameter)
        });
        let mut active = Vec::new();

        // reuse the lowest inactive contiguous word range
        for interval in intervals {
            active.retain(|allocation: &Active| allocation.end >= interval.start);

            let index = interval.value.0 as usize;
            let registers = if let Some(registers) = self.ranges[index] {
                registers
            } else {
                self.allocate_range(&active, interval.ty.word_count())?
            };
            self.ranges[index] = Some(registers);
            active.push(Active {
                end: interval.end,
                registers,
            });
        }

        Ok(())
    }

    /// Return the first contiguous word range not occupied by an active interval.
    fn allocate_range(
        &mut self,
        active: &[Active],
        word_count: u16,
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        let mut start = self.reserved_word_count;

        // scan physical words in order for the first complete gap
        while u32::from(start) + u32::from(word_count) <= self.register_count {
            let candidate = bytecode::RegisterSpan::new(bytecode::RegisterId(start), word_count);
            let overlap = active
                .iter()
                .filter(|allocation| allocation.registers.intersects(candidate))
                .map(|allocation| allocation.registers.end())
                .max();
            if let Some(next) = overlap {
                start = u16::try_from(next).map_err(|_| self.unsupported())?;
            } else {
                return Ok(candidate);
            }
        }

        self.append_range(word_count)
    }

    /// Append one physical register range.
    fn append_range(&mut self, word_count: u16) -> Result<bytecode::RegisterSpan, EmitError> {
        // reject a physical word count that cannot fit the function header
        let end = self.register_count + u32::from(word_count);
        if end > u32::from(u16::MAX) {
            return Err(self.unsupported());
        }

        // append the range to the dense physical register partition
        let start = u16::try_from(self.register_count).map_err(|_| self.unsupported())?;
        let registers = bytecode::RegisterSpan::new(bytecode::RegisterId(start), word_count);
        self.register_count = end;

        Ok(registers)
    }

    /// Return one MIR value's bytecode type.
    fn register_type(&self, value: mir::Value) -> Result<bytecode::ValueType, EmitError> {
        self.value_types
            .get(value.0 as usize)
            .copied()
            .flatten()
            .ok_or_else(|| ObjectEmitter::internal(self.module, "missing value type"))
    }

    /// Build one unsupported register file diagnostic.
    fn unsupported(&self) -> EmitError {
        EmitError::UnsupportedType {
            anchor: self.module.into(),
            module: self.module,
        }
    }
}

impl<'a> FunctionEmitter<'a> {
    /// Move scattered values into one reusable contiguous call register partition.
    pub(in crate::emit::bytecode::function) fn emit_arguments(
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

            Self::pack(&registers).ok_or_else(|| self.internal("outgoing call registers"))?
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
    pub(in crate::emit::bytecode::function) fn pack(
        ranges: &[bytecode::RegisterSpan],
    ) -> Option<bytecode::RegisterSpan> {
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

    /// Emit one exact register range move.
    pub(in crate::emit::bytecode::function) fn emit_move(
        &mut self,
        source: bytecode::RegisterSpan,
        destination: bytecode::RegisterSpan,
        ty: bytecode::ValueType,
    ) -> Result<(), EmitError> {
        if source.word_count != ty.word_count() || destination.word_count != ty.word_count() {
            return Err(self.internal("block argument width"));
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

        self.encode(instruction, &[destination])
    }

    /// Reserve one scratch range until the current operation ends.
    pub(in crate::emit::bytecode::function) fn scratch(
        &mut self,
        ty: bytecode::ValueType,
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        // select an unused range of the requested type or allocate one
        let available = self.scratches[self.scratch_count..]
            .iter()
            .position(|(other, _)| *other == ty);
        let index = match available {
            Some(index) => self.scratch_count + index,
            None => {
                let registers = self.append_registers(ty)?;
                let index = self.scratches.len();
                self.scratches.push((ty, registers));

                index
            }
        };

        // keep reserved ranges before the remaining reusable ranges
        self.scratches.swap(self.scratch_count, index);
        let registers = self.scratches[self.scratch_count].1;
        self.scratch_count += 1;

        Ok(registers)
    }

    /// Append one typed range to the physical register partition.
    pub(in crate::emit::bytecode::function) fn append_registers(
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

    /// Return one MIR value's fixed register range.
    pub(in crate::emit::bytecode::function) fn register(
        &self,
        value: mir::Value,
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        self.values
            .get(value.0 as usize)
            .copied()
            .flatten()
            .ok_or_else(|| self.internal("missing value register"))
    }

    /// Return one MIR local's permanent register range.
    pub(in crate::emit::bytecode::function) fn local(
        &self,
        local: mir::LocalId,
    ) -> Result<bytecode::RegisterSpan, EmitError> {
        self.locals
            .get(&local)
            .copied()
            .ok_or_else(|| self.internal("missing local register"))
    }

    /// Return one single-word MIR value register.
    pub(in crate::emit::bytecode::function) fn word(
        &self,
        value: mir::Value,
    ) -> Result<bytecode::RegisterId, EmitError> {
        let range = self.register(value)?;
        if range.word_count != 1 {
            return Err(self.internal("expected one-register value"));
        }

        Ok(range.start)
    }

    /// Return one MIR value's bytecode representation.
    pub(in crate::emit::bytecode::function) fn register_type(
        &self,
        value: mir::Value,
    ) -> Result<bytecode::ValueType, EmitError> {
        let ty = self.value_type(value)?;

        self.types.register_type(ty)
    }

    /// Return one MIR value's type.
    pub(in crate::emit::bytecode::function) fn value_type(
        &self,
        value: mir::Value,
    ) -> Result<mir::TypeId, EmitError> {
        self.function
            .value_type(value)
            .ok_or_else(|| self.internal("missing value type"))
    }
}
