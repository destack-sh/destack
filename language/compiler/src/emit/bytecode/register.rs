use std::collections::HashMap;

use destack_artifact::MirOptimized;
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::EmitError;

use super::TypeEmitter;

/// Bytecode register assignments for one MIR function.
pub(crate) struct RegisterAllocation {
    /// Register ranges keyed by MIR value identity.
    pub(crate) values: Vec<Option<bytecode::RegisterSpan>>,
    /// Permanent register ranges keyed by MIR local identity.
    pub(crate) locals: HashMap<mir::LocalId, bytecode::RegisterSpan>,
    /// MIR liveness used to derive canonical frame states.
    pub(crate) liveness: mir::FunctionLiveness,
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
    /// CFG-correct MIR value liveness.
    liveness: mir::FunctionLiveness,
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
    ) -> Result<Self, EmitError> {
        // project every MIR value into its fixed bytecode representation
        let body = function
            .body
            .as_ref()
            .ok_or_else(|| Self::invalid(module, "missing body"))?;
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
            return Err(Self::invalid(
                module,
                "environment exceeds one register word",
            ));
        }

        Ok(Self {
            module,
            tree: &optimized.tree,
            function,
            types,
            liveness: mir::FunctionLiveness::build(function, &optimized.tree),
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

        Ok(RegisterAllocation {
            values: self.ranges,
            locals,
            liveness: self.liveness,
        })
    }

    /// Assign one permanent register range to every MIR local.
    fn allocate_locals(
        &mut self,
    ) -> Result<HashMap<mir::LocalId, bytecode::RegisterSpan>, EmitError> {
        let mut locals = HashMap::with_capacity(self.function.locals().len());

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
        let mut intervals = vec![None; self.value_types.len()];
        let mut point = 0;

        // function parameters enter together through the leading register window
        for parameter in &self.function.parameters {
            self.touch(&mut intervals, parameter.value, point)?;
        }

        for &block_id in self.function.blocks() {
            let block = self.tree.get(block_id);

            // block entry retains incoming values and receives block parameters
            for &value in self.liveness.value_live_in(block_id) {
                self.touch(&mut intervals, value, point)?;
            }
            for parameter in &block.parameters {
                self.touch(&mut intervals, parameter.value, point)?;
            }
            point += 1;

            // each instruction retains its simultaneous sources and destination
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

            // block exit retains outgoing values and terminator operands
            let terminator = self.tree.get(block.terminator);
            for value in terminator.uses(self.tree) {
                self.touch(&mut intervals, value, point)?;
            }
            for &value in self.liveness.value_live_out(block_id) {
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
            .ok_or_else(|| Self::invalid(self.module, "missing value"))?;

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
        // reserve the hidden closure environment before explicit parameters
        if let Some(environment) = self.function.environment {
            let ty = self.types.register_type(environment)?;
            self.append_range(ty.word_count())?;
        }

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
            .ok_or_else(|| Self::invalid(self.module, "missing value type"))
    }

    /// Build one unsupported register file diagnostic.
    fn unsupported(&self) -> EmitError {
        EmitError::UnsupportedType {
            anchor: self.module.into(),
            module: self.module,
        }
    }

    /// Build one invalid MIR diagnostic.
    fn invalid(module: ModuleId, message: &str) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: module.into(),
            module,
            message: message.to_owned(),
        }
    }
}
