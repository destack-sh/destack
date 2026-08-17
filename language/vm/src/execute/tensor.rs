use std::ptr;

use bytecode::{
    ConvertMode, ElementOperation, IndexReduceOperation, Instruction, Operands, ReduceOperation,
    RegisterId, RegisterSpan, Scalar, ScatterOperation, TensorOperand, TensorOperation, TieBreak,
};
use destack_bytecode as bytecode;
use destack_heap::{AllocationShape, HeapEdge, HeapReference, Payload, SharedHeapReference};
use destack_mir as mir;
use destack_program::{
    AllocationSite, AllocationSiteId, LayoutId, LayoutShape, Memory, MemoryAccess, Profile,
    Program, Runtime, ScalarFormat, TensorDimension, TensorLayout, TensorSharding,
    TensorViewLayout, TypeId, Word,
};
use mir::{TensorDimensionOrder, TensorFormat, TraceMap};

use crate::diagnostic::{Error, ExecutionResult, Result, Trap};
use crate::machine::Activation;

use super::arithmetic::Arithmetic;

/// State required to execute one tensor instruction.
struct Execution<'program, 'memory, 'registers, 'profile> {
    /// Linked metadata used by tensor operands.
    program: &'program Program,
    /// Program memory used by tensor storage.
    memory: Memory<'memory>,
    /// Canonical word registers used by the instruction.
    registers: &'registers mut [Word],
    /// Optional allocation profile.
    profile: Option<&'profile mut Profile>,
    /// Allocation completed by this instruction when present.
    allocation: Option<Allocation>,
}

/// One allocation completed by a tensor instruction.
#[derive(Clone, Copy)]
struct Allocation {
    /// The executed allocation site.
    site_id: AllocationSiteId,
    /// The allocated heap edge.
    edge: HeapEdge,
    /// The allocated byte length.
    byte_len: usize,
}

/// One tensor resolved against live execution memory.
#[derive(Clone, Copy)]
struct Tensor {
    /// The stable allocation edge.
    edge: HeapEdge,
    /// The first scalar byte.
    address: usize,
    /// The first 64-bit dimension in live memory.
    dimension_address: usize,
    /// The tensor rank.
    rank: usize,
    /// The physical tensor shape.
    shape: TensorShape,
    /// The scalar representation.
    scalar: Scalar,
    /// The number of logical elements.
    element_count: usize,
}

/// One linked allocation selected by a tensor result operand.
#[derive(Clone, Copy)]
struct TensorAllocation {
    /// The allocation site used for profiling.
    site_id: AllocationSiteId,
    /// The linked allocation operation.
    site: AllocationSite,
    /// The physical tensor layout.
    layout: TensorLayout,
    /// The tensor element representation.
    scalar: Scalar,
}

/// The physical addressing rule for one resolved tensor.
#[derive(Clone, Copy)]
enum TensorShape {
    /// Dense storage in one canonical dimension order.
    Dense(TensorFormat),
    /// Explicit byte strides in live memory.
    Strided(usize),
}

/// Dimensions read directly from live tensor storage.
#[derive(Clone, Copy)]
struct TensorDimensions {
    /// The next 64-bit dimension.
    address: usize,
    /// The number of unread dimensions.
    remaining: usize,
}

impl Tensor {
    /// Resolve one dense tensor handle.
    fn dense(
        edge: HeapEdge,
        address: usize,
        dimensions: usize,
        rank: usize,
        scalar: Scalar,
        format: TensorFormat,
    ) -> Option<Self> {
        let dimensions = TensorDimensions::new(dimensions, rank);
        let element_count = Self::count_elements(dimensions)?;

        Some(Self {
            edge,
            address,
            dimension_address: dimensions.address,
            rank,
            shape: TensorShape::Dense(format),
            scalar,
            element_count,
        })
    }

    /// Resolve one explicitly strided tensor view.
    fn strided(
        edge: HeapEdge,
        address: usize,
        dimensions: usize,
        strides: usize,
        rank: usize,
        scalar: Scalar,
    ) -> Option<Self> {
        let dimensions = TensorDimensions::new(dimensions, rank);
        let element_count = Self::count_elements(dimensions)?;

        Some(Self {
            edge,
            address,
            dimension_address: dimensions.address,
            rank,
            shape: TensorShape::Strided(strides),
            scalar,
            element_count,
        })
    }

    /// Return the number of logical elements.
    const fn element_count(&self) -> usize {
        self.element_count
    }

    /// Return the checked element count for one logical shape.
    fn count_elements(mut dimensions: impl Iterator<Item = usize>) -> Option<usize> {
        dimensions.try_fold(1usize, |count, dimension| count.checked_mul(dimension))
    }

    /// Return the tensor rank.
    const fn rank(&self) -> usize {
        self.rank
    }

    /// Return this tensor's dimensions without copying them.
    const fn dimensions(&self) -> TensorDimensions {
        TensorDimensions::new(self.dimension_address, self.rank)
    }

    /// Return one logical dimension.
    fn dimension(&self, axis: usize) -> Option<usize> {
        self.dimensions().nth(axis)
    }

    /// Return whether another tensor has the same logical dimensions.
    fn matches_dimensions(&self, other: &Self) -> bool {
        self.rank == other.rank && self.dimensions().eq(other.dimensions())
    }

    /// Return the address selected by one logical coordinate.
    fn element_address(&self, coordinate: &[usize]) -> Option<usize> {
        if coordinate.len() != self.rank {
            return None;
        }
        let mut address = self.address;

        // accumulate each checked axis displacement
        for (axis, index) in coordinate.iter().copied().enumerate() {
            let dimension = self.dimension(axis)?;
            if index >= dimension {
                return None;
            }
            let stride = self.stride(axis)?;
            address = address.checked_add(index.checked_mul(stride)?)?;
        }

        Some(address)
    }

    /// Return the address selected by one dense row-major element index.
    fn linear_address(&self, mut index: usize) -> Option<usize> {
        if index >= self.element_count() {
            return None;
        }
        let byte_len = self.scalar.bit_width() as usize / u8::BITS as usize;

        match self.shape {
            // dense row-major elements are physically contiguous
            TensorShape::Dense(TensorFormat::Dense {
                order: TensorDimensionOrder::RowMajor,
            }) => self.address.checked_add(index.checked_mul(byte_len)?),
            // derive each column-major stride once while peeling row-major coordinates
            TensorShape::Dense(TensorFormat::Dense {
                order: TensorDimensionOrder::ColumnMajor,
            }) => {
                let mut address = self.address;
                let mut row_stride = self.element_count;
                let mut column_stride = byte_len;

                // map logical row-major order into column-major storage
                for dimension in self.dimensions() {
                    if dimension == 0 {
                        return None;
                    }
                    row_stride /= dimension;
                    let coordinate = index / row_stride;
                    index %= row_stride;
                    address = address.checked_add(coordinate.checked_mul(column_stride)?)?;
                    column_stride = column_stride.checked_mul(dimension)?;
                }

                Some(address)
            }
            // explicit views carry one byte stride per logical dimension
            TensorShape::Strided(strides) => {
                let mut address = self.address;
                let mut dimensions = self.dimensions();
                let mut strides = TensorDimensions::new(strides, self.rank);

                // map logical row-major order into explicit storage strides
                while let (Some(dimension), Some(stride)) =
                    (dimensions.next_back(), strides.next_back())
                {
                    if dimension == 0 {
                        return None;
                    }
                    let coordinate = index % dimension;
                    address = address.checked_add(coordinate.checked_mul(stride)?)?;
                    index /= dimension;
                }

                Some(address)
            }
        }
    }

    /// Return the byte span containing every reachable element.
    fn byte_span(&self) -> Option<usize> {
        if self.element_count == 0 {
            return Some(0);
        }
        let byte_len = self.scalar.bit_width() as usize / u8::BITS as usize;
        let mut last = 0usize;

        // accumulate the final reachable element in every dimension
        for axis in 0..self.rank {
            let index = self.dimension(axis)?.checked_sub(1)?;
            let offset = index.checked_mul(self.stride(axis)?)?;
            last = last.checked_add(offset)?;
        }

        last.checked_add(byte_len)
    }

    /// Return one physical byte stride.
    fn stride(&self, axis: usize) -> Option<usize> {
        if axis >= self.rank {
            return None;
        }

        match self.shape {
            TensorShape::Strided(address) => TensorDimensions::new(address, self.rank).nth(axis),
            TensorShape::Dense(TensorFormat::Dense {
                order: TensorDimensionOrder::RowMajor,
            }) => {
                let byte_len = self.scalar.bit_width() as usize / u8::BITS as usize;

                self.dimensions()
                    .skip(axis + 1)
                    .try_fold(byte_len, usize::checked_mul)
            }
            TensorShape::Dense(TensorFormat::Dense {
                order: TensorDimensionOrder::ColumnMajor,
            }) => {
                let byte_len = self.scalar.bit_width() as usize / u8::BITS as usize;

                self.dimensions()
                    .take(axis)
                    .try_fold(byte_len, usize::checked_mul)
            }
        }
    }
}

impl TensorDimensions {
    /// Create one iterator over live 64-bit dimension words.
    const fn new(address: usize, rank: usize) -> Self {
        Self {
            address,
            remaining: rank,
        }
    }
}

impl Iterator for TensorDimensions {
    type Item = usize;

    /// Read the next dimension.
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        // SAFETY: resolved tensor shapes point at aligned live words for their complete rank
        let dimension = unsafe { ptr::read(self.address as *const Word) }.as_u64() as usize;
        self.address += Word::BYTE_LEN;
        self.remaining -= 1;

        Some(dimension)
    }

    /// Return the exact remaining dimension count.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for TensorDimensions {}

impl DoubleEndedIterator for TensorDimensions {
    /// Read the final remaining dimension.
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let address = self.address + self.remaining * Word::BYTE_LEN;

        // SAFETY: resolved tensor shapes point at aligned live words for their complete rank
        let dimension = unsafe { ptr::read(address as *const Word) }.as_u64() as usize;

        Some(dimension)
    }
}

/// One logical tensor coordinate.
struct Coordinate {
    /// The index for each tensor axis.
    values: Vec<usize>,
}

impl Coordinate {
    /// Create one zero coordinate for a tensor rank.
    fn zero(rank: usize) -> Self {
        Self {
            values: vec![0; rank],
        }
    }

    /// Set this coordinate from one dense row-major index.
    fn set<Dimensions>(&mut self, mut index: usize, dimensions: Dimensions) -> Option<()>
    where
        Dimensions: Clone + DoubleEndedIterator<Item = usize> + ExactSizeIterator,
    {
        let element_count = Tensor::count_elements(dimensions.clone())?;
        if index >= element_count || self.values.len() != dimensions.len() {
            return None;
        }

        // peel dimensions from the innermost axis outward
        for (axis, dimension) in dimensions.rev().enumerate() {
            if dimension == 0 {
                return None;
            }
            let axis = self.values.len() - axis - 1;
            self.values[axis] = index % dimension;
            index /= dimension;
        }

        Some(())
    }
}

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one tensor instruction over the active register window.
    pub(crate) fn execute_tensor<const OBSERVE: bool>(
        &mut self,
        instruction: Instruction<'_>,
        operation: TensorOperation,
    ) -> ExecutionResult<Option<(MemoryAccess, (usize, usize))>, R::Error> {
        let frame = self.frame();
        let program = &self.machine.program;
        let memory = self.activation.memory.reborrow();
        let registers = self.cursor.registers(frame.register_count);
        let profile = self.profile.as_deref_mut();
        let mut execution = Execution {
            program,
            memory,
            registers,
            profile,
            allocation: None,
        };
        let access = execution.execute(instruction, operation)?;
        let allocation = execution.allocation;

        if OBSERVE && let Some(allocation) = allocation {
            self.observe_allocation(allocation.site_id, allocation.edge, allocation.byte_len)?;
        }

        Ok(access)
    }
}

impl Execution<'_, '_, '_, '_> {
    /// Read operands from one linked tensor instruction.
    #[inline(always)]
    fn operands<'code>(&self, instruction: Instruction<'code>) -> Operands<'code, false> {
        // SAFETY: Program linking establishes each tensor opcode's exact operand layout
        unsafe { instruction.operands_unchecked() }
    }

    /// Decode one immediate axis list.
    fn axes(operands: &mut Operands<'_, false>) -> Result<Vec<usize>> {
        let axes = operands.u16s()?.map(usize::from).collect();

        Ok(axes)
    }

    /// Return whether one axis list contains distinct in-range axes.
    fn single_axes_are_unique(rank: usize, axes: &[usize]) -> bool {
        axes.iter()
            .enumerate()
            .all(|(index, axis)| *axis < rank && !axes[..index].contains(axis))
    }

    /// Return whether two axis lists are distinct, disjoint, and in range.
    fn axes_are_unique(rank: usize, first: &[usize], second: &[usize]) -> bool {
        Self::single_axes_are_unique(rank, first)
            && Self::single_axes_are_unique(rank, second)
            && first.iter().all(|axis| !second.contains(axis))
    }

    /// Return axes not claimed by either selected list.
    fn free_axes(rank: usize, first: &[usize], second: &[usize]) -> Vec<usize> {
        (0..rank)
            .filter(|axis| !first.contains(axis) && !second.contains(axis))
            .collect()
    }

    /// Return dimensions selected from one tensor.
    fn dimensions_for_axes(&self, tensor: &Tensor, axes: &[usize]) -> Result<Vec<usize>> {
        axes.iter()
            .map(|axis| {
                tensor
                    .dimension(*axis)
                    .ok_or_else(|| self.invalid_instruction())
            })
            .collect()
    }

    /// Return the number of index-vector components.
    fn index_vector_len(indices: &Tensor, axis: usize) -> Result<usize> {
        if axis == indices.rank() {
            Ok(1)
        } else {
            indices
                .dimension(axis)
                .ok_or_else(Error::invalid_instruction)
        }
    }

    /// Return index tensor axes outside the index-vector dimension.
    fn index_batch_axes(rank: usize, vector_axis: usize) -> Result<Vec<usize>> {
        if vector_axis > rank {
            return Err(Error::invalid_instruction());
        }
        let axes = (0..rank).filter(|axis| *axis != vector_axis).collect();

        Ok(axes)
    }

    /// Read one complete gather or scatter start index vector.
    fn gather_starts(
        &self,
        indices: &Tensor,
        batch: &[usize],
        vector_axis: usize,
        mapped_axes: &[usize],
    ) -> Result<Vec<i128>> {
        let batch_axes = Self::index_batch_axes(indices.rank(), vector_axis)?;
        if batch.len() != batch_axes.len()
            || mapped_axes.len() != Self::index_vector_len(indices, vector_axis)?
        {
            return Err(self.invalid_instruction());
        }
        let mut coordinate = vec![0; indices.rank()];
        for (&axis, &index) in batch_axes.iter().zip(batch) {
            coordinate[axis] = index;
        }
        let mut starts = Vec::with_capacity(mapped_axes.len());

        // load each explicit or implicit index-vector component
        for component in 0..mapped_axes.len() {
            if vector_axis < indices.rank() {
                coordinate[vector_axis] = component;
            }
            let address = indices
                .element_address(&coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let value = self.load(address, indices.scalar);
            let value = indices
                .scalar
                .integer(value.bits())
                .ok_or_else(|| self.invalid_instruction())?;
            starts.push(value);
        }

        Ok(starts)
    }

    /// Read one canonical input word.
    #[inline(always)]
    fn read(&self, register: u16) -> Word {
        self.registers[register as usize]
    }

    /// Write one canonical result word.
    #[inline(always)]
    fn write(&mut self, register: u16, value: Word) {
        self.registers[register as usize] = value;
    }

    /// Return one canonical register's native address.
    fn register_address(&self, register: u16) -> usize {
        self.registers.as_ptr().wrapping_add(register as usize) as usize
    }

    /// Return one malformed tensor instruction failure.
    #[inline(never)]
    fn invalid_instruction(&self) -> Error {
        Error::invalid_instruction()
    }

    /// Read one stable heap edge from a canonical register.
    fn read_edge(&self, register: RegisterId, space: mir::Space) -> Result<HeapEdge> {
        let bits = self.read(register.0).bits() as usize;

        match space {
            mir::Space::Local => Ok(HeapEdge::Local(HeapReference::from_bits(bits))),
            mir::Space::Shared => Ok(HeapEdge::Shared(SharedHeapReference::from_bits(bits))),
        }
    }

    /// Load one scalar from a resolved native address.
    fn load(&self, address: usize, scalar: Scalar) -> Word {
        // SAFETY: resolved tensor storage remains live for the complete operation
        let bits = unsafe {
            match scalar.bit_width() {
                8 => u64::from(ptr::read_unaligned(address as *const u8)),
                16 => u64::from(ptr::read_unaligned(address as *const u16)),
                32 => u64::from(ptr::read_unaligned(address as *const u32)),
                64 => ptr::read_unaligned(address as *const u64),
                _ => unreachable!("tensor scalars occupy one bytecode word"),
            }
        };

        Word::from_bits(scalar.encode(bits))
    }

    /// Store one scalar at a resolved native address.
    fn store(&self, address: usize, scalar: Scalar, value: Word) {
        let bits = value.bits();

        // SAFETY: resolved mutable tensor storage remains live for the complete operation
        unsafe {
            match scalar.bit_width() {
                8 => ptr::write_unaligned(address as *mut u8, bits as u8),
                16 => ptr::write_unaligned(address as *mut u16, bits as u16),
                32 => ptr::write_unaligned(address as *mut u32, bits as u32),
                64 => ptr::write_unaligned(address as *mut u64, bits),
                _ => unreachable!("tensor scalars occupy one bytecode word"),
            }
        }
    }

    /// Execute one tensor operation through the direct CPU engine.
    fn execute(
        &mut self,
        instruction: Instruction<'_>,
        operation: TensorOperation,
    ) -> Result<Option<(MemoryAccess, (usize, usize))>> {
        let access = match operation {
            TensorOperation::Load => self.execute_tensor_load(instruction).map(Some),
            TensorOperation::Store => self.execute_tensor_store(instruction).map(Some),
            TensorOperation::Fill => self.execute_tensor_fill(instruction).map(Some),
            TensorOperation::Copy => self.execute_tensor_copy(instruction).map(Some),
            TensorOperation::Element | TensorOperation::Compare => {
                self.execute_tensor_element(instruction, operation)?;

                Ok(None)
            }
            TensorOperation::Select => {
                self.execute_tensor_select(instruction)?;

                Ok(None)
            }
            TensorOperation::Transpose => {
                self.execute_tensor_transpose(instruction)?;

                Ok(None)
            }
            TensorOperation::Reshape => {
                self.execute_tensor_reshape(instruction)?;

                Ok(None)
            }
            TensorOperation::Broadcast => {
                self.execute_tensor_broadcast(instruction)?;

                Ok(None)
            }
            TensorOperation::Slice => {
                self.execute_tensor_slice(instruction)?;

                Ok(None)
            }
            TensorOperation::Pad => {
                self.execute_tensor_pad(instruction)?;

                Ok(None)
            }
            TensorOperation::Concat => {
                self.execute_tensor_concat(instruction)?;

                Ok(None)
            }
            TensorOperation::Splat => {
                self.execute_tensor_splat(instruction)?;

                Ok(None)
            }
            TensorOperation::Convert => {
                self.execute_tensor_convert(instruction)?;

                Ok(None)
            }
            TensorOperation::Bitcast => {
                self.execute_tensor_bitcast(instruction)?;

                Ok(None)
            }
            TensorOperation::Reduce => {
                self.execute_tensor_reduce(instruction)?;

                Ok(None)
            }
            TensorOperation::IndexReduce => {
                self.execute_tensor_index_reduce(instruction)?;

                Ok(None)
            }
            TensorOperation::Extract => {
                self.execute_tensor_extract(instruction)?;

                Ok(None)
            }
            TensorOperation::View => {
                self.execute_tensor_view(instruction)?;

                Ok(None)
            }
            TensorOperation::Contract => {
                self.execute_tensor_contract(instruction)?;

                Ok(None)
            }
            TensorOperation::Gather => {
                self.execute_tensor_gather(instruction)?;

                Ok(None)
            }
            TensorOperation::Scatter => {
                self.execute_tensor_scatter(instruction)?;

                Ok(None)
            }
            TensorOperation::Convolution => {
                self.execute_tensor_convolution(instruction)?;

                Ok(None)
            }
        }?;

        Ok(access)
    }

    /// Execute one elementwise scalar tensor operation.
    fn execute_tensor_element(
        &mut self,
        instruction: Instruction<'_>,
        tensor_operation: TensorOperation,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let expects_comparison = tensor_operation == TensorOperation::Compare;
        let mut inputs = [None; 3];
        let input_count = if expects_comparison {
            inputs[0] = Some(operands.tensor()?);
            inputs[1] = Some(operands.tensor()?);

            2
        } else {
            let encoded = operands.tensors()?;
            if encoded.is_empty() || encoded.len() > inputs.len() {
                return Err(self.invalid_instruction());
            }
            let input_count = encoded.len();
            for (index, input) in encoded.enumerate() {
                inputs[index] = Some(input);
            }

            input_count
        };
        let operator = operands.u16()?;
        let operator =
            ElementOperation::from_code(operator).ok_or_else(|| self.invalid_instruction())?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let first_input = inputs[0].ok_or_else(|| self.invalid_instruction())?;
        let first = self.tensor(first_input)?;
        let source_scalar = first.scalar;
        let (float_operation, integer_operation, expected_input_count, returns_boolean) =
            if let Some(operation) = operator.float_operation() {
                if !source_scalar.is_float() {
                    return Err(self.invalid_instruction());
                }

                (
                    Some(operation),
                    None,
                    operation.input_count(),
                    operation.returns_boolean(),
                )
            } else if let Some(operation) = operator.integer_operation() {
                if !source_scalar.is_integer() || operation.is_overflowing() {
                    return Err(self.invalid_instruction());
                }

                (
                    None,
                    Some(operation),
                    operation.input_count(),
                    operation.returns_boolean(),
                )
            } else {
                return Err(self.invalid_instruction());
            };
        if input_count != expected_input_count || returns_boolean != expects_comparison {
            return Err(self.invalid_instruction());
        }
        let mut tensors = [first; 3];
        for (index, input) in inputs.into_iter().take(input_count).enumerate() {
            let input = input.ok_or_else(|| self.invalid_instruction())?;
            let tensor = self.tensor(input)?;
            if tensor.scalar != source_scalar {
                return Err(self.invalid_instruction());
            }
            tensors[index] = tensor;
        }
        for tensor in tensors.iter().take(input_count) {
            if !first.matches_dimensions(tensor) {
                return Err(self.invalid_instruction());
            }
        }
        let result_scalar = operator
            .result_scalar(source_scalar)
            .ok_or_else(|| self.invalid_instruction())?;
        if allocation.scalar != result_scalar {
            return Err(self.invalid_instruction());
        }
        let result = self.allocate_tensor(target, allocation, first.dimensions())?;

        // execute one scalar operation per logical element
        for index in 0..result.element_count() {
            let mut values = [None; 3];
            for (input, tensor) in tensors.iter().take(input_count).enumerate() {
                let address = tensor
                    .linear_address(index)
                    .ok_or_else(|| Error::trap(Trap::Bounds))?;
                values[input] = Some(self.load(address, source_scalar));
            }
            let left = values[0].ok_or_else(|| self.invalid_instruction())?;
            let right = values[1];
            let third = values[2];
            let value = if let Some(operation) = float_operation {
                Arithmetic::float(operation, source_scalar, left, right, third)?
            } else if let Some(operation) = integer_operation {
                Arithmetic::integer(operation, source_scalar, left, right, third)?
            } else {
                return Err(self.invalid_instruction());
            };
            let address = result
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one elementwise tensor selection.
    fn execute_tensor_select(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let condition = operands.tensor()?;
        let left = operands.tensor()?;
        let right = operands.tensor()?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let condition = self.tensor(condition)?;
        let left = self.tensor(left)?;
        let right = self.tensor(right)?;
        if condition.scalar != Scalar::Boolean
            || left.scalar != allocation.scalar
            || right.scalar != allocation.scalar
            || !condition.matches_dimensions(&left)
            || !condition.matches_dimensions(&right)
        {
            return Err(self.invalid_instruction());
        }
        let result = self.allocate_tensor(target, allocation, condition.dimensions())?;

        // select the source tensor independently for every element
        for index in 0..result.element_count() {
            let condition = condition
                .linear_address(index)
                .map(|address| self.load(address, Scalar::Boolean).as_boolean())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let source = if condition { &left } else { &right };
            let source = source
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target = result
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target,
                allocation.scalar,
                self.load(source, allocation.scalar),
            );
        }

        Ok(())
    }

    /// Execute one tensor splat.
    fn execute_tensor_splat(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let value = operands.register()?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let dimensions = self.fixed_tensor_dimensions(allocation.layout)?;
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let value = self.read(value.0);

        // fill dense and non-default physical orders through logical addresses
        for index in 0..result.element_count() {
            let address = result
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one tensor scalar extraction.
    fn execute_tensor_extract(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let indices = self.tensor_indices(&mut operands)?;
        let source = self.tensor(source)?;
        let address = source
            .element_address(&indices)
            .ok_or_else(|| Error::trap(Trap::Bounds))?;

        self.write(target.0, self.load(address, source.scalar));

        Ok(())
    }

    /// Execute one scalar tensor-view load.
    fn execute_tensor_load(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<(MemoryAccess, (usize, usize))> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let view = operands.tensor()?;
        let indices = self.tensor_indices(&mut operands)?;
        let view = self.tensor(view)?;
        if !matches!(view.shape, TensorShape::Strided(_)) {
            return Err(self.invalid_instruction());
        }
        let address = view
            .element_address(&indices)
            .ok_or_else(|| Error::trap(Trap::Bounds))?;
        let byte_len = view.scalar.bit_width() as usize / u8::BITS as usize;

        self.write(target.0, self.load(address, view.scalar));

        Ok((MemoryAccess::Read, (address, byte_len)))
    }

    /// Execute one scalar tensor-view store.
    fn execute_tensor_store(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<(MemoryAccess, (usize, usize))> {
        let mut operands = self.operands(instruction);
        let view = operands.tensor()?;
        let indices = self.tensor_indices(&mut operands)?;
        let value = operands.register()?;
        let view = self.tensor(view)?;
        if !matches!(view.shape, TensorShape::Strided(_)) {
            return Err(self.invalid_instruction());
        }
        let address = view
            .element_address(&indices)
            .ok_or_else(|| Error::trap(Trap::Bounds))?;
        let byte_len = view.scalar.bit_width() as usize / u8::BITS as usize;

        self.store(address, view.scalar, self.read(value.0));

        Ok((MemoryAccess::Write, (address, byte_len)))
    }

    /// Execute one tensor-view fill.
    fn execute_tensor_fill(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<(MemoryAccess, (usize, usize))> {
        let mut operands = self.operands(instruction);
        let view = operands.tensor()?;
        let value = operands.register()?;
        let view = self.tensor(view)?;
        if !matches!(view.shape, TensorShape::Strided(_)) {
            return Err(self.invalid_instruction());
        }
        let value = self.read(value.0);

        // fill each logical view element without assuming contiguity
        for index in 0..view.element_count() {
            let address = view
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(address, view.scalar, value);
        }

        let byte_span = view.byte_span().ok_or_else(|| self.invalid_instruction())?;

        Ok((MemoryAccess::Write, (view.address, byte_span)))
    }

    /// Execute one tensor-view copy.
    fn execute_tensor_copy(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<(MemoryAccess, (usize, usize))> {
        let mut operands = self.operands(instruction);
        let target = operands.tensor()?;
        let source = operands.tensor()?;
        let target = self.tensor(target)?;
        let source = self.tensor(source)?;
        if target.scalar != source.scalar
            || !matches!(target.shape, TensorShape::Strided(_))
            || !target.matches_dimensions(&source)
        {
            return Err(self.invalid_instruction());
        }

        // copy through logical coordinates so arbitrary strides remain valid
        for index in 0..target.element_count() {
            let source_address = source
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = target
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(
                target_address,
                target.scalar,
                self.load(source_address, source.scalar),
            );
        }

        let byte_span = target
            .byte_span()
            .ok_or_else(|| self.invalid_instruction())?;

        Ok((MemoryAccess::Write, (target.address, byte_span)))
    }

    /// Execute one tensor reshape by preserving logical element order.
    fn execute_tensor_reshape(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let dimensions = self.tensor_indices(&mut operands)?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar {
            return Err(self.invalid_instruction());
        }
        let element_count = Tensor::count_elements(dimensions.iter().copied())
            .ok_or_else(|| self.invalid_instruction())?;
        if element_count != source.element_count() {
            return Err(self.invalid_instruction());
        }
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        self.copy_tensor(&result, &source)
    }

    /// Execute one tensor transpose.
    fn execute_tensor_transpose(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let permutation = operands.u16s()?.collect::<Vec<_>>();
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar || permutation.len() != source.rank() {
            return Err(self.invalid_instruction());
        }
        let mut seen = vec![false; permutation.len()];
        for axis in &permutation {
            let Some(seen) = seen.get_mut(*axis as usize) else {
                return Err(self.invalid_instruction());
            };
            if *seen {
                return Err(self.invalid_instruction());
            }
            *seen = true;
        }
        let dimensions = permutation
            .iter()
            .map(|axis| source.dimension(*axis as usize))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| self.invalid_instruction())?;
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        let mut coordinate = Coordinate::zero(result.rank());

        // map each result coordinate back through the axis permutation
        for index in 0..result.element_count() {
            coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut source_coordinate = vec![0; source.rank()];
            for (result_axis, source_axis) in permutation.iter().copied().enumerate() {
                source_coordinate[source_axis as usize] = coordinate.values[result_axis];
            }
            let source_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = result
                .element_address(&coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target_address,
                allocation.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }

    /// Execute one tensor broadcast.
    fn execute_tensor_broadcast(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let axes = operands.u16s()?.collect::<Vec<_>>();
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar || axes.len() != source.rank() {
            return Err(self.invalid_instruction());
        }
        let dimensions = self.broadcast_dimensions(allocation.layout, &source, &axes)?;
        let mut seen = vec![false; dimensions.len()];
        for axis in &axes {
            let Some(seen) = seen.get_mut(*axis as usize) else {
                return Err(self.invalid_instruction());
            };
            if *seen {
                return Err(self.invalid_instruction());
            }
            *seen = true;
        }
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        let mut coordinate = Coordinate::zero(result.rank());

        // project selected result axes into the source coordinate
        for index in 0..result.element_count() {
            coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let source_coordinate = axes
                .iter()
                .map(|axis| coordinate.values.get(*axis as usize).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| self.invalid_instruction())?;
            let source_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = result
                .element_address(&coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target_address,
                allocation.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }

    /// Execute one tensor slice.
    fn execute_tensor_slice(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let offsets = self.tensor_indices(&mut operands)?;
        let sizes = self.tensor_indices(&mut operands)?;
        let strides = self.tensor_indices(&mut operands)?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar
            || offsets.len() != source.rank()
            || sizes.len() != source.rank()
            || strides.len() != source.rank()
        {
            return Err(self.invalid_instruction());
        }
        for axis in 0..source.rank() {
            if strides[axis] == 0 {
                return Err(self.invalid_instruction());
            }
            let last = if sizes[axis] == 0 {
                offsets[axis]
            } else {
                let index = sizes[axis] - 1;
                index
                    .checked_mul(strides[axis])
                    .and_then(|span| offsets[axis].checked_add(span))
                    .ok_or_else(|| self.invalid_instruction())?
            };
            let source_dimension = source
                .dimension(axis)
                .ok_or_else(|| self.invalid_instruction())?;
            if (sizes[axis] == 0 && offsets[axis] > source_dimension)
                || (sizes[axis] > 0 && last >= source_dimension)
            {
                return Err(Error::trap(Trap::Bounds));
            }
        }
        let result = self.allocate_tensor(target, allocation, sizes.iter().copied())?;

        let mut coordinate = Coordinate::zero(result.rank());

        // map each sliced coordinate through its offset and stride
        for index in 0..result.element_count() {
            coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let source_coordinate = coordinate
                .values
                .iter()
                .zip(&offsets)
                .zip(&strides)
                .map(|((index, offset), stride)| offset + index * stride)
                .collect::<Vec<_>>();
            let source_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = result
                .element_address(&coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target_address,
                allocation.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }

    /// Execute one tensor padding operation.
    fn execute_tensor_pad(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let padding = operands.register()?;
        let low = self.tensor_indices(&mut operands)?;
        let high = self.tensor_indices(&mut operands)?;
        let interior = self.tensor_indices(&mut operands)?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar {
            return Err(self.invalid_instruction());
        }
        if low.len() != source.rank()
            || high.len() != source.rank()
            || interior.len() != source.rank()
        {
            return Err(self.invalid_instruction());
        }
        let dimensions = source
            .dimensions()
            .zip(&low)
            .zip(&high)
            .zip(&interior)
            .map(|(((dimension, low), high), interior)| {
                let gaps = if dimension == 0 { 0 } else { dimension - 1 };
                low.checked_add(*high)?
                    .checked_add(dimension)?
                    .checked_add(gaps.checked_mul(*interior)?)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| self.invalid_instruction())?;
        let spacings = interior
            .iter()
            .map(|spacing| spacing.checked_add(1))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| self.invalid_instruction())?;
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let padding = self.read(padding.0);

        let mut coordinate = Coordinate::zero(result.rank());

        // distinguish inserted padding coordinates from source coordinates
        for index in 0..result.element_count() {
            coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut source_coordinate = Vec::with_capacity(source.rank());
            let mut is_padding = false;
            for axis in 0..source.rank() {
                let index = coordinate.values[axis];
                if index < low[axis] {
                    is_padding = true;
                    break;
                }
                let index = index - low[axis];
                let spacing = spacings[axis];
                let dimension = source
                    .dimension(axis)
                    .ok_or_else(|| self.invalid_instruction())?;
                if !index.is_multiple_of(spacing) || index / spacing >= dimension {
                    is_padding = true;
                    break;
                }
                source_coordinate.push(index / spacing);
            }
            let value = if is_padding {
                padding
            } else {
                let address = source
                    .element_address(&source_coordinate)
                    .ok_or_else(|| Error::trap(Trap::Bounds))?;

                self.load(address, source.scalar)
            };
            let address = result
                .element_address(&coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one tensor concatenation.
    fn execute_tensor_concat(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let inputs = operands.tensors()?;
        let axis = operands.u16()? as usize;
        let allocation = self.tensor_allocation(&mut operands)?;
        if inputs.is_empty() {
            return Err(self.invalid_instruction());
        }
        let inputs = inputs
            .into_iter()
            .map(|input| self.tensor(input))
            .collect::<Result<Vec<_>>>()?;
        if inputs.iter().any(|input| input.scalar != allocation.scalar) {
            return Err(self.invalid_instruction());
        }
        let mut dimensions = inputs[0].dimensions().collect::<Vec<_>>();
        if axis >= dimensions.len() {
            return Err(self.invalid_instruction());
        }
        if inputs.iter().any(|input| {
            input.rank() != dimensions.len()
                || input
                    .dimensions()
                    .zip(&dimensions)
                    .enumerate()
                    .any(|(current, (left, right))| current != axis && left != *right)
        }) {
            return Err(self.invalid_instruction());
        }
        dimensions[axis] = inputs
            .iter()
            .try_fold(0usize, |size, input| {
                size.checked_add(input.dimension(axis)?)
            })
            .ok_or_else(|| self.invalid_instruction())?;
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        let mut coordinate = Coordinate::zero(result.rank());

        // select one source tensor from the concatenated axis interval
        for index in 0..result.element_count() {
            coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut axis_index = coordinate.values[axis];
            let mut source = None;

            // select the source interval containing this concatenated axis index
            for input in &inputs {
                let dimension = input
                    .dimension(axis)
                    .ok_or_else(|| self.invalid_instruction())?;
                if axis_index < dimension {
                    source = Some(input);
                    break;
                }
                axis_index -= dimension;
            }
            let source = source.ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut source_coordinate = coordinate.values.clone();
            source_coordinate[axis] = axis_index;
            let source_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = result
                .element_address(&coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target_address,
                allocation.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }

    /// Execute one tensor element conversion.
    fn execute_tensor_convert(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let mode = operands.u16()? as u8;
        let mode = ConvertMode::from_code(mode).ok_or_else(|| self.invalid_instruction())?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        let result = self.allocate_tensor(target, allocation, source.dimensions())?;

        // convert each logical element through the scalar conversion path
        for index in 0..result.element_count() {
            let source_address = source
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = result
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let value = self.load(source_address, source.scalar);
            let value = Arithmetic::convert(value, source.scalar, allocation.scalar, mode)?;

            self.store(target_address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one exact tensor storage bitcast.
    fn execute_tensor_bitcast(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        let dimensions = self.fixed_tensor_dimensions(allocation.layout)?;
        let target_element_count = Tensor::count_elements(dimensions.iter().copied())
            .ok_or_else(|| self.invalid_instruction())?;
        let source_byte_len = source
            .element_count()
            .checked_mul(source.scalar.bit_width() as usize / u8::BITS as usize)
            .ok_or_else(|| self.invalid_instruction())?;
        let target_byte_len = target_element_count
            .checked_mul(allocation.scalar.bit_width() as usize / u8::BITS as usize)
            .ok_or_else(|| self.invalid_instruction())?;
        if source_byte_len != target_byte_len {
            return Err(self.invalid_instruction());
        }
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        // SAFETY: both tensor allocations own non-overlapping payloads of the same byte length
        unsafe {
            ptr::copy_nonoverlapping(
                source.address as *const u8,
                result.address as *mut u8,
                source_byte_len,
            );
        }

        Ok(())
    }

    /// Execute one tensor reduction over selected axes.
    fn execute_tensor_reduce(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let initial = operands.register()?;
        let operation = operands.u16()? as u8;
        let operation =
            ReduceOperation::from_code(operation).ok_or_else(|| self.invalid_instruction())?;
        let axes = operands
            .u16s()?
            .into_iter()
            .map(usize::from)
            .collect::<Vec<_>>();
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if source.scalar != allocation.scalar {
            return Err(self.invalid_instruction());
        }
        let mut reduced = vec![false; source.rank()];
        for axis in axes {
            let Some(is_reduced) = reduced.get_mut(axis) else {
                return Err(self.invalid_instruction());
            };
            if *is_reduced {
                return Err(self.invalid_instruction());
            }
            *is_reduced = true;
        }
        let dimensions = source
            .dimensions()
            .enumerate()
            .filter_map(|(axis, dimension)| (!reduced[axis]).then_some(dimension))
            .collect::<Vec<_>>();
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let initial = self.read(initial.0);

        let mut source_coordinate = Coordinate::zero(source.rank());
        let mut result_coordinate = Vec::with_capacity(result.rank());

        // initialize every result, including reductions over an empty dimension
        for index in 0..result.element_count() {
            let address = result
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(address, allocation.scalar, initial);
        }

        // reduce source coordinates into each result coordinate
        for source_index in 0..source.element_count() {
            source_coordinate
                .set(source_index, source.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            result_coordinate.clear();
            result_coordinate.extend(
                source_coordinate
                    .values
                    .iter()
                    .enumerate()
                    .filter_map(|(axis, index)| (!reduced[axis]).then_some(*index)),
            );
            let result_address = result
                .element_address(&result_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let source_address = source
                .element_address(&source_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let current = self.load(result_address, allocation.scalar);
            let value = Arithmetic::reduce(
                operation,
                allocation.scalar,
                current,
                self.load(source_address, source.scalar),
            )?;

            self.store(result_address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one tensor index reduction.
    fn execute_tensor_index_reduce(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.tensor()?;
        let operation = operands.u16()? as u8;
        let operation =
            IndexReduceOperation::from_code(operation).ok_or_else(|| self.invalid_instruction())?;
        let axis = operands.u16()? as usize;
        let tie = operands.u16()? as u8;
        let tie = TieBreak::from_code(tie).ok_or_else(|| self.invalid_instruction())?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let source = self.tensor(source)?;
        if axis >= source.rank()
            || source.dimension(axis) == Some(0)
            || !allocation.scalar.is_integer()
        {
            return Err(self.invalid_instruction());
        }
        let dimensions = source
            .dimensions()
            .enumerate()
            .filter_map(|(current, dimension)| (current != axis).then_some(dimension))
            .collect::<Vec<_>>();
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;

        let mut result_coordinate = Coordinate::zero(result.rank());

        // select one extremum index for every result coordinate
        for index in 0..result.element_count() {
            result_coordinate
                .set(index, result.dimensions())
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut source_coordinate = result_coordinate.values.clone();
            source_coordinate.insert(axis, 0);
            let first_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let mut selected = 0usize;
            let mut selected_value = self.load(first_address, source.scalar);

            let dimension = source
                .dimension(axis)
                .ok_or_else(|| self.invalid_instruction())?;
            for candidate in 1..dimension {
                source_coordinate[axis] = candidate;
                let address = source
                    .element_address(&source_coordinate)
                    .ok_or_else(|| Error::trap(Trap::Bounds))?;
                let value = self.load(address, source.scalar);
                let comparison = match operation {
                    IndexReduceOperation::Minimum => ReduceOperation::Minimum,
                    IndexReduceOperation::Maximum => ReduceOperation::Maximum,
                };
                let reduced = Arithmetic::reduce(comparison, source.scalar, selected_value, value)?;
                let replaces =
                    reduced == value && (selected_value != value || tie == TieBreak::Last);
                if replaces {
                    selected = candidate;
                    selected_value = value;
                }
            }
            let address = result
                .element_address(&result_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            let selected = Word::from_bits(allocation.scalar.encode(selected as u64));
            self.store(address, allocation.scalar, selected);
        }

        Ok(())
    }

    /// Execute one generalized tensor contraction.
    fn execute_tensor_contract(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let left = self.tensor(operands.tensor()?)?;
        let right = self.tensor(operands.tensor()?)?;
        let left_batch = Self::axes(&mut operands)?;
        let right_batch = Self::axes(&mut operands)?;
        let left_contract = Self::axes(&mut operands)?;
        let right_contract = Self::axes(&mut operands)?;
        let allocation = self.tensor_allocation(&mut operands)?;
        if left.scalar != right.scalar
            || left.scalar != allocation.scalar
            || left.scalar == Scalar::Boolean
            || left_batch.len() != right_batch.len()
            || left_contract.len() != right_contract.len()
            || !Self::axes_are_unique(left.rank(), &left_batch, &left_contract)
            || !Self::axes_are_unique(right.rank(), &right_batch, &right_contract)
        {
            return Err(self.invalid_instruction());
        }
        for (&left_axis, &right_axis) in left_batch.iter().zip(&right_batch) {
            if left.dimension(left_axis) != right.dimension(right_axis) {
                return Err(self.invalid_instruction());
            }
        }
        for (&left_axis, &right_axis) in left_contract.iter().zip(&right_contract) {
            if left.dimension(left_axis) != right.dimension(right_axis) {
                return Err(self.invalid_instruction());
            }
        }

        // derive the stable dot-general result dimension order
        let left_free = Self::free_axes(left.rank(), &left_batch, &left_contract);
        let right_free = Self::free_axes(right.rank(), &right_batch, &right_contract);
        let mut dimensions = self.dimensions_for_axes(&left, &left_batch)?;
        dimensions.extend(self.dimensions_for_axes(&left, &left_free)?);
        dimensions.extend(self.dimensions_for_axes(&right, &right_free)?);
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let contract_dimensions = self.dimensions_for_axes(&left, &left_contract)?;
        let contract_count = Tensor::count_elements(contract_dimensions.iter().copied())
            .ok_or_else(|| self.invalid_instruction())?;
        let mut result_coordinate = Coordinate::zero(result.rank());
        let mut contract_coordinate = Coordinate::zero(left_contract.len());

        // accumulate every contraction domain into one result element
        for result_index in 0..result.element_count() {
            result_coordinate
                .set(result_index, dimensions.iter().copied())
                .ok_or_else(|| self.invalid_instruction())?;
            let mut left_coordinate = vec![0; left.rank()];
            let mut right_coordinate = vec![0; right.rank()];
            let mut output_axis = 0;
            for (&left_axis, &right_axis) in left_batch.iter().zip(&right_batch) {
                let value = result_coordinate.values[output_axis];
                left_coordinate[left_axis] = value;
                right_coordinate[right_axis] = value;
                output_axis += 1;
            }
            for &axis in &left_free {
                left_coordinate[axis] = result_coordinate.values[output_axis];
                output_axis += 1;
            }
            for &axis in &right_free {
                right_coordinate[axis] = result_coordinate.values[output_axis];
                output_axis += 1;
            }
            let mut value = Word::from_bits(allocation.scalar.encode(0));
            for contract_index in 0..contract_count {
                contract_coordinate
                    .set(contract_index, contract_dimensions.iter().copied())
                    .ok_or_else(|| self.invalid_instruction())?;
                for (index, (&left_axis, &right_axis)) in
                    left_contract.iter().zip(&right_contract).enumerate()
                {
                    left_coordinate[left_axis] = contract_coordinate.values[index];
                    right_coordinate[right_axis] = contract_coordinate.values[index];
                }
                let left_address = left
                    .element_address(&left_coordinate)
                    .ok_or_else(|| Error::trap(Trap::Bounds))?;
                let right_address = right
                    .element_address(&right_coordinate)
                    .ok_or_else(|| Error::trap(Trap::Bounds))?;
                value = Arithmetic::product_sum(
                    allocation.scalar,
                    value,
                    self.load(left_address, left.scalar),
                    self.load(right_address, right.scalar),
                )?;
            }
            let result_address = result
                .element_address(&result_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(result_address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one StableHLO-style tensor gather.
    fn execute_tensor_gather(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = self.tensor(operands.tensor()?)?;
        let indices = self.tensor(operands.tensor()?)?;
        let offset_axes = Self::axes(&mut operands)?;
        let collapsed_axes = Self::axes(&mut operands)?;
        let start_axes = Self::axes(&mut operands)?;
        let index_vector_axis = operands.u16()? as usize;
        let slice_sizes = operands
            .u64s()?
            .map(|size| size as usize)
            .collect::<Vec<_>>();
        let allocation = self.tensor_allocation(&mut operands)?;
        if allocation.scalar != source.scalar
            || !indices.scalar.is_integer()
            || slice_sizes.len() != source.rank()
            || start_axes.len() != Self::index_vector_len(&indices, index_vector_axis)?
            || !Self::single_axes_are_unique(source.rank(), &collapsed_axes)
            || !Self::single_axes_are_unique(source.rank(), &start_axes)
        {
            return Err(self.invalid_instruction());
        }
        let window_axes = (0..source.rank())
            .filter(|axis| !collapsed_axes.contains(axis))
            .collect::<Vec<_>>();
        if window_axes.len() != offset_axes.len() {
            return Err(self.invalid_instruction());
        }
        for (axis, &size) in slice_sizes.iter().enumerate() {
            let dimension = source
                .dimension(axis)
                .ok_or_else(|| self.invalid_instruction())?;
            if size == 0 || size > dimension || collapsed_axes.contains(&axis) && size != 1 {
                return Err(self.invalid_instruction());
            }
        }

        // interleave index batch dimensions and slice offsets in result order
        let index_axes = Self::index_batch_axes(indices.rank(), index_vector_axis)?;
        let result_rank = index_axes.len() + offset_axes.len();
        if !Self::single_axes_are_unique(result_rank, &offset_axes) {
            return Err(self.invalid_instruction());
        }
        let batch_output_axes = (0..result_rank)
            .filter(|axis| !offset_axes.contains(axis))
            .collect::<Vec<_>>();
        let mut dimensions = vec![0; result_rank];
        for (index, &axis) in offset_axes.iter().enumerate() {
            dimensions[axis] = slice_sizes[window_axes[index]];
        }
        for (index, &axis) in batch_output_axes.iter().enumerate() {
            dimensions[axis] = indices
                .dimension(index_axes[index])
                .ok_or_else(|| self.invalid_instruction())?;
        }
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let mut result_coordinate = Coordinate::zero(result_rank);

        // resolve one clamped source window for every result coordinate
        for linear in 0..result.element_count() {
            result_coordinate
                .set(linear, dimensions.iter().copied())
                .ok_or_else(|| self.invalid_instruction())?;
            let index_batch = batch_output_axes
                .iter()
                .map(|&axis| result_coordinate.values[axis])
                .collect::<Vec<_>>();
            let starts =
                self.gather_starts(&indices, &index_batch, index_vector_axis, &start_axes)?;
            let mut source_coordinate = vec![0; source.rank()];
            for (component, &axis) in start_axes.iter().enumerate() {
                let dimension = source
                    .dimension(axis)
                    .ok_or_else(|| self.invalid_instruction())?;
                let maximum = dimension - slice_sizes[axis];
                source_coordinate[axis] = starts[component].clamp(0, maximum as i128) as usize;
            }
            for (index, &axis) in window_axes.iter().enumerate() {
                source_coordinate[axis] += result_coordinate.values[offset_axes[index]];
            }
            let source_address = source
                .element_address(&source_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let result_address = result
                .element_address(&result_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(
                result_address,
                allocation.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }

    /// Execute one StableHLO-style tensor scatter.
    fn execute_tensor_scatter(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = self.tensor(operands.tensor()?)?;
        let indices = self.tensor(operands.tensor()?)?;
        let updates = self.tensor(operands.tensor()?)?;
        let update_window_axes = Self::axes(&mut operands)?;
        let inserted_axes = Self::axes(&mut operands)?;
        let scatter_axes = Self::axes(&mut operands)?;
        let index_vector_axis = operands.u16()? as usize;
        let operation = operands.u16()? as u8;
        let operation =
            ScatterOperation::from_code(operation).ok_or_else(|| self.invalid_instruction())?;
        let allocation = self.tensor_allocation(&mut operands)?;
        if source.scalar != updates.scalar
            || source.scalar != allocation.scalar
            || !indices.scalar.is_integer()
            || scatter_axes.len() != Self::index_vector_len(&indices, index_vector_axis)?
            || !Self::single_axes_are_unique(source.rank(), &inserted_axes)
            || !Self::single_axes_are_unique(source.rank(), &scatter_axes)
            || !Self::single_axes_are_unique(updates.rank(), &update_window_axes)
        {
            return Err(self.invalid_instruction());
        }
        let operand_window_axes = (0..source.rank())
            .filter(|axis| !inserted_axes.contains(axis))
            .collect::<Vec<_>>();
        let update_batch_axes = (0..updates.rank())
            .filter(|axis| !update_window_axes.contains(axis))
            .collect::<Vec<_>>();
        let index_batch_axes = Self::index_batch_axes(indices.rank(), index_vector_axis)?;
        if operand_window_axes.len() != update_window_axes.len()
            || update_batch_axes.len() != index_batch_axes.len()
        {
            return Err(self.invalid_instruction());
        }
        for (&update_axis, &operand_axis) in update_window_axes.iter().zip(&operand_window_axes) {
            if updates.dimension(update_axis) != source.dimension(operand_axis) {
                return Err(self.invalid_instruction());
            }
        }
        for (&update_axis, &index_axis) in update_batch_axes.iter().zip(&index_batch_axes) {
            if updates.dimension(update_axis) != indices.dimension(index_axis) {
                return Err(self.invalid_instruction());
            }
        }
        let result = self.allocate_tensor(target, allocation, source.dimensions())?;

        // initialize the result with the complete source tensor
        for linear in 0..source.element_count() {
            let source_address = source
                .linear_address(linear)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let result_address = result
                .linear_address(linear)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(
                result_address,
                result.scalar,
                self.load(source_address, source.scalar),
            );
        }

        // apply each in-bounds update in logical update order
        let update_dimensions = updates.dimensions().collect::<Vec<_>>();
        let mut update_coordinate = Coordinate::zero(updates.rank());
        for linear in 0..updates.element_count() {
            update_coordinate
                .set(linear, update_dimensions.iter().copied())
                .ok_or_else(|| self.invalid_instruction())?;
            let index_batch = update_batch_axes
                .iter()
                .map(|&axis| update_coordinate.values[axis])
                .collect::<Vec<_>>();
            let starts =
                self.gather_starts(&indices, &index_batch, index_vector_axis, &scatter_axes)?;
            let mut result_coordinate = vec![0; result.rank()];
            for (component, &axis) in scatter_axes.iter().enumerate() {
                let start = starts[component];
                if start < 0 {
                    result_coordinate[axis] = usize::MAX;
                } else {
                    result_coordinate[axis] = start as usize;
                }
            }
            for (&update_axis, &operand_axis) in update_window_axes.iter().zip(&operand_window_axes)
            {
                result_coordinate[operand_axis] = result_coordinate[operand_axis]
                    .saturating_add(update_coordinate.values[update_axis]);
            }
            if result_coordinate
                .iter()
                .enumerate()
                .any(|(axis, &index)| result.dimension(axis).is_none_or(|extent| index >= extent))
            {
                continue;
            }
            let update_address = updates
                .element_address(&update_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let result_address = result
                .element_address(&result_coordinate)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let update = self.load(update_address, updates.scalar);
            let value = if operation == ScatterOperation::Replace {
                update
            } else {
                let current = self.load(result_address, result.scalar);
                Arithmetic::scatter(operation, result.scalar, current, update)?
            };
            self.store(result_address, result.scalar, value);
        }

        Ok(())
    }

    /// Execute one generalized tensor convolution.
    fn execute_tensor_convolution(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let input = self.tensor(operands.tensor()?)?;
        let kernel = self.tensor(operands.tensor()?)?;
        let input_batch_axis = operands.u16()? as usize;
        let input_feature_axis = operands.u16()? as usize;
        let input_spatial_axes = Self::axes(&mut operands)?;
        let kernel_input_axis = operands.u16()? as usize;
        let kernel_output_axis = operands.u16()? as usize;
        let kernel_spatial_axes = Self::axes(&mut operands)?;
        let output_batch_axis = operands.u16()? as usize;
        let output_feature_axis = operands.u16()? as usize;
        let output_spatial_axes = Self::axes(&mut operands)?;
        let strides = operands
            .u64s()?
            .map(|value| value as usize)
            .collect::<Vec<_>>();
        let padding_low = operands
            .u64s()?
            .map(|value| value as usize)
            .collect::<Vec<_>>();
        let padding_high = operands
            .u64s()?
            .map(|value| value as usize)
            .collect::<Vec<_>>();
        let input_dilation = operands
            .u64s()?
            .map(|value| value as usize)
            .collect::<Vec<_>>();
        let kernel_dilation = operands
            .u64s()?
            .map(|value| value as usize)
            .collect::<Vec<_>>();
        let reversal = operands.u16s()?.map(|value| value != 0).collect::<Vec<_>>();
        let feature_groups = operands.u32()? as usize;
        let batch_groups = operands.u32()? as usize;
        let allocation = self.tensor_allocation(&mut operands)?;
        let spatial_rank = input
            .rank()
            .checked_sub(2)
            .ok_or_else(|| self.invalid_instruction())?;
        let input_axes = [
            vec![input_batch_axis, input_feature_axis],
            input_spatial_axes.clone(),
        ]
        .concat();
        let kernel_axes = [
            vec![kernel_input_axis, kernel_output_axis],
            kernel_spatial_axes.clone(),
        ]
        .concat();
        let output_axes = [
            vec![output_batch_axis, output_feature_axis],
            output_spatial_axes.clone(),
        ]
        .concat();
        let parameter_lengths = [
            input_spatial_axes.len(),
            kernel_spatial_axes.len(),
            output_spatial_axes.len(),
            strides.len(),
            padding_low.len(),
            padding_high.len(),
            input_dilation.len(),
            kernel_dilation.len(),
            reversal.len(),
        ];
        if input.rank() != kernel.rank()
            || input.scalar != kernel.scalar
            || input.scalar != allocation.scalar
            || input.scalar == Scalar::Boolean
            || parameter_lengths
                .iter()
                .any(|length| *length != spatial_rank)
            || !Self::single_axes_are_unique(input.rank(), &input_axes)
            || !Self::single_axes_are_unique(kernel.rank(), &kernel_axes)
            || !Self::single_axes_are_unique(input.rank(), &output_axes)
            || feature_groups == 0
            || batch_groups == 0
            || feature_groups > 1 && batch_groups > 1
            || strides.contains(&0)
            || input_dilation.contains(&0)
            || kernel_dilation.contains(&0)
        {
            return Err(self.invalid_instruction());
        }
        let input_batches = input
            .dimension(input_batch_axis)
            .ok_or_else(|| self.invalid_instruction())?;
        let input_features = input
            .dimension(input_feature_axis)
            .ok_or_else(|| self.invalid_instruction())?;
        let kernel_input_features = kernel
            .dimension(kernel_input_axis)
            .ok_or_else(|| self.invalid_instruction())?;
        let kernel_output_features = kernel
            .dimension(kernel_output_axis)
            .ok_or_else(|| self.invalid_instruction())?;
        if !input_batches.is_multiple_of(batch_groups)
            || !input_features.is_multiple_of(feature_groups)
            || kernel_input_features != input_features / feature_groups
            || !kernel_output_features.is_multiple_of(feature_groups)
            || !kernel_output_features.is_multiple_of(batch_groups)
        {
            return Err(self.invalid_instruction());
        }

        // derive the exact output shape from the encoded window
        let mut dimensions = vec![0; input.rank()];
        dimensions[output_batch_axis] = input_batches / batch_groups;
        dimensions[output_feature_axis] = kernel_output_features;
        let mut kernel_dimensions = Vec::with_capacity(spatial_rank);
        for spatial in 0..spatial_rank {
            let input_extent = input
                .dimension(input_spatial_axes[spatial])
                .ok_or_else(|| self.invalid_instruction())?;
            let kernel_extent = kernel
                .dimension(kernel_spatial_axes[spatial])
                .ok_or_else(|| self.invalid_instruction())?;
            let dilated_input = input_extent
                .saturating_sub(1)
                .checked_mul(input_dilation[spatial])
                .and_then(|extent| extent.checked_add(usize::from(input_extent != 0)))
                .ok_or_else(|| self.invalid_instruction())?;
            let dilated_kernel = kernel_extent
                .saturating_sub(1)
                .checked_mul(kernel_dilation[spatial])
                .and_then(|extent| extent.checked_add(usize::from(kernel_extent != 0)))
                .ok_or_else(|| self.invalid_instruction())?;
            let padded_input = dilated_input
                .checked_add(padding_low[spatial])
                .and_then(|extent| extent.checked_add(padding_high[spatial]))
                .ok_or_else(|| self.invalid_instruction())?;
            let output_extent = if padded_input < dilated_kernel {
                0
            } else {
                (padded_input - dilated_kernel) / strides[spatial] + 1
            };
            dimensions[output_spatial_axes[spatial]] = output_extent;
            kernel_dimensions.push(kernel_extent);
        }
        let result = self.allocate_tensor(target, allocation, dimensions.iter().copied())?;
        let kernel_spatial_count = Tensor::count_elements(kernel_dimensions.iter().copied())
            .ok_or_else(|| self.invalid_instruction())?;
        let batch_extent = dimensions[output_batch_axis];
        let batch_feature_extent = kernel_output_features / batch_groups;
        let feature_extent = kernel_output_features / feature_groups;
        let mut output_coordinate = Coordinate::zero(result.rank());
        let mut kernel_coordinate = Coordinate::zero(spatial_rank);

        // evaluate each output window directly against the input and kernel tensors
        for output_linear in 0..result.element_count() {
            output_coordinate
                .set(output_linear, dimensions.iter().copied())
                .ok_or_else(|| self.invalid_instruction())?;
            let output_batch = output_coordinate.values[output_batch_axis];
            let output_feature = output_coordinate.values[output_feature_axis];
            let batch_group = output_feature / batch_feature_extent;
            let feature_group = output_feature / feature_extent;
            let input_batch = output_batch + batch_group * batch_extent;
            let mut value = Word::from_bits(allocation.scalar.encode(0));
            for kernel_linear in 0..kernel_spatial_count {
                kernel_coordinate
                    .set(kernel_linear, kernel_dimensions.iter().copied())
                    .ok_or_else(|| self.invalid_instruction())?;
                let mut input_coordinate = vec![0; input.rank()];
                let mut kernel_element = vec![0; kernel.rank()];
                input_coordinate[input_batch_axis] = input_batch;
                kernel_element[kernel_output_axis] = output_feature;
                let mut is_in_bounds = true;
                for spatial in 0..spatial_rank {
                    let window_index = kernel_coordinate.values[spatial];
                    let dilated = output_coordinate.values[output_spatial_axes[spatial]]
                        .checked_mul(strides[spatial])
                        .and_then(|offset| {
                            window_index
                                .checked_mul(kernel_dilation[spatial])
                                .and_then(|window| offset.checked_add(window))
                        })
                        .ok_or_else(|| self.invalid_instruction())?;
                    if dilated < padding_low[spatial] {
                        is_in_bounds = false;
                        break;
                    }
                    let dilated = dilated - padding_low[spatial];
                    if !dilated.is_multiple_of(input_dilation[spatial]) {
                        is_in_bounds = false;
                        break;
                    }
                    let input_index = dilated / input_dilation[spatial];
                    let input_extent = input
                        .dimension(input_spatial_axes[spatial])
                        .ok_or_else(|| self.invalid_instruction())?;
                    if input_index >= input_extent {
                        is_in_bounds = false;
                        break;
                    }
                    input_coordinate[input_spatial_axes[spatial]] = input_index;
                    kernel_element[kernel_spatial_axes[spatial]] = if reversal[spatial] {
                        kernel_dimensions[spatial] - window_index - 1
                    } else {
                        window_index
                    };
                }
                if !is_in_bounds {
                    continue;
                }
                for kernel_feature in 0..kernel_input_features {
                    input_coordinate[input_feature_axis] =
                        feature_group * kernel_input_features + kernel_feature;
                    kernel_element[kernel_input_axis] = kernel_feature;
                    let input_address = input
                        .element_address(&input_coordinate)
                        .ok_or_else(|| Error::trap(Trap::Bounds))?;
                    let kernel_address = kernel
                        .element_address(&kernel_element)
                        .ok_or_else(|| Error::trap(Trap::Bounds))?;
                    value = Arithmetic::product_sum(
                        allocation.scalar,
                        value,
                        self.load(input_address, input.scalar),
                        self.load(kernel_address, kernel.scalar),
                    )?;
                }
            }
            let result_address = result
                .element_address(&output_coordinate.values)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            self.store(result_address, allocation.scalar, value);
        }

        Ok(())
    }

    /// Execute one derived tensor view.
    fn execute_tensor_view(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let source = operands.tensor()?;
        let offsets = self.tensor_indices(&mut operands)?;
        let sizes = self.tensor_indices(&mut operands)?;
        let strides = self.tensor_indices(&mut operands)?;
        let layout = operands.u32()?;
        let layout = LayoutId::from_raw(layout).ok_or_else(|| self.invalid_instruction())?;
        let layout = self
            .program
            .layout_by_id(layout)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::TensorView(layout) = layout.shape else {
            return Err(self.invalid_instruction());
        };
        let source = self.tensor(source)?;
        let scalar = self.tensor_scalar(layout.reference.pointee)?;
        let rank = self.program.tensor_dimensions(layout.dimensions).len();
        if source.scalar != scalar
            || offsets.len() != rank
            || sizes.len() != rank
            || strides.len() != rank
        {
            return Err(self.invalid_instruction());
        }
        let expected = self.program.tensor_dimensions(layout.dimensions);
        self.match_tensor_dimensions(expected, sizes.iter().copied())?;
        let base = self.memory.address(source.edge);
        let mut byte_offset = source
            .address
            .checked_sub(base)
            .ok_or_else(|| self.invalid_instruction())?;
        let mut result_strides = Vec::with_capacity(rank);
        for axis in 0..rank {
            let source_dimension = source
                .dimension(axis)
                .ok_or_else(|| self.invalid_instruction())?;
            let source_stride = source
                .stride(axis)
                .ok_or_else(|| self.invalid_instruction())?;
            let is_out_of_bounds = if sizes[axis] == 0 {
                offsets[axis] > source_dimension
            } else {
                offsets[axis] >= source_dimension
            };
            if strides[axis] == 0 || is_out_of_bounds {
                return Err(Error::trap(Trap::Bounds));
            }
            let offset = offsets[axis]
                .checked_mul(source_stride)
                .ok_or_else(|| self.invalid_instruction())?;
            byte_offset = byte_offset
                .checked_add(offset)
                .ok_or_else(|| self.invalid_instruction())?;
            let stride = source_stride
                .checked_mul(strides[axis])
                .ok_or_else(|| self.invalid_instruction())?;
            result_strides.push(stride);

            if sizes[axis] > 0 {
                let index = sizes[axis] - 1;
                let last = index
                    .checked_mul(strides[axis])
                    .and_then(|index| offsets[axis].checked_add(index))
                    .ok_or_else(|| self.invalid_instruction())?;
                if last >= source_dimension {
                    return Err(Error::trap(Trap::Bounds));
                }
            }
        }
        let expected_words = 2 + rank * 2;
        if target.word_count as usize != expected_words {
            return Err(self.invalid_instruction());
        }

        // materialize the canonical edge, offset, dimensions, and strides
        self.write(target.start.0, Word::from_bits(source.edge.bits() as u64));
        self.write(target.start.0 + 1, Word::uint64(byte_offset as u64));
        for axis in 0..rank {
            self.write(
                target.start.0 + 2 + axis as u16,
                Word::uint64(sizes[axis] as u64),
            );
            self.write(
                target.start.0 + 2 + rank as u16 + axis as u16,
                Word::uint64(result_strides[axis] as u64),
            );
        }

        Ok(())
    }

    /// Resolve one tensor operand against its program layout.
    fn tensor(&self, input: TensorOperand) -> Result<Tensor> {
        let layout =
            LayoutId::from_raw(input.layout.0).ok_or_else(|| self.invalid_instruction())?;
        let layout = self
            .program
            .layout_by_id(layout)
            .ok_or_else(|| self.invalid_instruction())?;

        match layout.shape {
            LayoutShape::Tensor(layout) => self.tensor_handle(input.registers, layout),
            LayoutShape::TensorView(layout) => self.tensor_view(input.registers, layout),
            _ => Err(self.invalid_instruction()),
        }
    }

    /// Resolve one tensor handle register.
    fn tensor_handle(&self, registers: RegisterSpan, layout: TensorLayout) -> Result<Tensor> {
        if registers.word_count != 1 || !matches!(layout.sharding, TensorSharding::Unsharded) {
            return Err(Error::unsupported_tensor_sharding());
        }
        let scalar = self.tensor_scalar(layout.reference.pointee)?;
        let rank = self.program.tensor_dimensions(layout.dimensions).len();
        let Some(mir::Storage::Heap(space)) = layout.reference.storage() else {
            return Err(self.invalid_instruction());
        };
        let edge = self.read_edge(registers.start, space)?;
        let base = self.memory.address(edge);
        let header_byte_len = rank
            .checked_mul(Word::BYTE_LEN)
            .ok_or_else(|| self.invalid_instruction())?;
        let address = base
            .checked_add(header_byte_len)
            .ok_or_else(|| self.invalid_instruction())?;
        Tensor::dense(edge, address, base, rank, scalar, layout.format)
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Resolve one canonical tensor view descriptor.
    fn tensor_view(&self, registers: RegisterSpan, layout: TensorViewLayout) -> Result<Tensor> {
        if !matches!(layout.sharding, TensorSharding::Unsharded) {
            return Err(Error::unsupported_tensor_sharding());
        }
        let scalar = self.tensor_scalar(layout.reference.pointee)?;
        let rank = self.program.tensor_dimensions(layout.dimensions).len();
        if registers.word_count as usize != 2 + rank * 2 {
            return Err(self.invalid_instruction());
        }
        let Some(mir::Storage::Heap(space)) = layout.reference.storage() else {
            return Err(self.invalid_instruction());
        };
        let edge = self.read_edge(registers.start, space)?;
        let byte_offset = self.read(registers.start.0 + 1).as_u64() as usize;
        let dimension_address = self.register_address(registers.start.0 + 2);
        let stride_address = self.register_address(registers.start.0 + 2 + rank as u16);
        let expected = self.program.tensor_dimensions(layout.dimensions);
        self.match_tensor_dimensions(expected, TensorDimensions::new(dimension_address, rank))?;
        let address = self
            .memory
            .address(edge)
            .checked_add(byte_offset)
            .ok_or_else(|| self.invalid_instruction())?;

        Tensor::strided(
            edge,
            address,
            dimension_address,
            stride_address,
            rank,
            scalar,
        )
        .ok_or_else(|| self.invalid_instruction())
    }

    /// Resolve one tensor element scalar.
    fn tensor_scalar(&self, element: TypeId) -> Result<Scalar> {
        let format = self
            .program
            .scalar_format(element)
            .ok_or_else(|| self.invalid_instruction())?;
        let scalar = match format {
            ScalarFormat::Int {
                width: 8,
                is_signed: 1,
            } => Scalar::Int8,
            ScalarFormat::Int {
                width: 8,
                is_signed: 0,
            } => Scalar::Uint8,
            ScalarFormat::Int {
                width: 16,
                is_signed: 1,
            } => Scalar::Int16,
            ScalarFormat::Int {
                width: 16,
                is_signed: 0,
            } => Scalar::Uint16,
            ScalarFormat::Int {
                width: 32,
                is_signed: 1,
            } => Scalar::Int32,
            ScalarFormat::Int {
                width: 32,
                is_signed: 0,
            } => Scalar::Uint32,
            ScalarFormat::Int {
                width: 64,
                is_signed: 1,
            } => Scalar::Int64,
            ScalarFormat::Int {
                width: 64,
                is_signed: 0,
            } => Scalar::Uint64,
            ScalarFormat::Float {
                format: mir::FloatType::Float16,
            } => Scalar::Float16,
            ScalarFormat::Float {
                format: mir::FloatType::Bfloat16,
            } => Scalar::Bfloat16,
            ScalarFormat::Float {
                format: mir::FloatType::Float32,
            } => Scalar::Float32,
            ScalarFormat::Float {
                format: mir::FloatType::Float64,
            } => Scalar::Float64,
            ScalarFormat::Boolean => Scalar::Boolean,
            ScalarFormat::Character | ScalarFormat::Int { .. } => {
                return Err(self.invalid_instruction());
            }
        };

        Ok(scalar)
    }

    /// Allocate one tensor payload and publish its stable edge.
    fn allocate_tensor<Dimensions>(
        &mut self,
        target: RegisterId,
        allocation: TensorAllocation,
        dimensions: Dimensions,
    ) -> Result<Tensor>
    where
        Dimensions: Clone + ExactSizeIterator<Item = usize>,
    {
        let TensorAllocation {
            site_id,
            site,
            layout,
            scalar,
        } = allocation;
        if !matches!(layout.sharding, TensorSharding::Unsharded) {
            return Err(Error::unsupported_tensor_sharding());
        }
        if !matches!(
            layout.reference.kind(),
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Unique)
        ) {
            return Err(self.invalid_instruction());
        }
        if layout.reference.storage() != Some(mir::Storage::Heap(site.space)) {
            return Err(self.invalid_instruction());
        }
        let expected_dimensions = self.program.tensor_dimensions(layout.dimensions);
        if expected_dimensions.len() != dimensions.len()
            || expected_dimensions
                .iter()
                .zip(dimensions.clone())
                .any(|(expected, actual)| {
                    expected
                        .fixed_extent()
                        .is_some_and(|expected| expected as usize != actual)
                })
        {
            return Err(self.invalid_instruction());
        }
        let rank = expected_dimensions.len();
        let element_byte_len = scalar.bit_width() as usize / u8::BITS as usize;
        let element_count = Tensor::count_elements(dimensions.clone());
        let element_count = element_count.ok_or_else(|| self.invalid_instruction())?;
        let header_byte_len = dimensions.len() * Word::BYTE_LEN;
        let byte_len = element_count
            .checked_mul(element_byte_len)
            .and_then(|byte_len| byte_len.checked_add(header_byte_len))
            .ok_or_else(|| self.invalid_instruction())?;
        let shape = AllocationShape::new(byte_len, Word::BYTE_LEN, None, TraceMap::empty());
        let plan = self.memory.plan_allocation(site.space, &shape);
        let edge = self
            .memory
            .allocate(site.space, plan, Payload::Uninit, self.program.trace_view())
            .map_err(Error::heap)?;
        self.write(target.0, Word::from_bits(edge.bits() as u64));

        // write the shape header before exposing element storage
        let base = self.memory.address(edge);
        for (axis, dimension) in dimensions.enumerate() {
            self.store(
                base + axis * Word::BYTE_LEN,
                Scalar::Uint64,
                Word::uint64(dimension as u64),
            );
        }
        if let Some(profile) = self.profile.as_deref_mut() {
            profile.record_allocation(site_id, plan.byte_len);
        }
        self.allocation = Some(Allocation {
            site_id,
            edge,
            byte_len: plan.byte_len(),
        });
        let address = base
            .checked_add(header_byte_len)
            .ok_or_else(|| self.invalid_instruction())?;
        Tensor::dense(edge, address, base, rank, scalar, layout.format)
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Resolve one linked tensor allocation.
    fn tensor_allocation(&self, operands: &mut Operands<'_, false>) -> Result<TensorAllocation> {
        let site_id = operands.u32()?;
        let site_id = AllocationSiteId(site_id);
        let site = self
            .program
            .sites()
            .allocation_by_id(self.program.sections(), site_id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let layout = self
            .program
            .layout_by_id(site.layout)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::Tensor(layout) = layout.shape else {
            return Err(self.invalid_instruction());
        };
        let scalar = self.tensor_scalar(layout.reference.pointee)?;

        Ok(TensorAllocation {
            site_id,
            site,
            layout,
            scalar,
        })
    }

    /// Read one encoded tensor index register list.
    fn tensor_indices(&self, operands: &mut Operands<'_, false>) -> Result<Vec<usize>> {
        let registers = operands.registers()?;

        Ok(registers
            .into_iter()
            .map(|register| self.read(register.0).as_u64() as usize)
            .collect())
    }

    /// Match one live tensor shape against its program dimensions.
    fn match_tensor_dimensions<Dimensions>(
        &self,
        expected: &[TensorDimension],
        dimensions: Dimensions,
    ) -> Result<()>
    where
        Dimensions: ExactSizeIterator<Item = usize>,
    {
        if expected.len() != dimensions.len()
            || expected.iter().zip(dimensions).any(|(expected, actual)| {
                expected
                    .fixed_extent()
                    .is_some_and(|expected| expected as usize != actual)
            })
        {
            return Err(self.invalid_instruction());
        }

        Ok(())
    }

    /// Return one tensor type's fully static dimensions.
    fn fixed_tensor_dimensions(&self, layout: TensorLayout) -> Result<Vec<usize>> {
        self.program
            .tensor_dimensions(layout.dimensions)
            .iter()
            .map(|dimension| {
                dimension
                    .fixed_extent()
                    .map(|extent| extent as usize)
                    .ok_or_else(|| self.invalid_instruction())
            })
            .collect()
    }

    /// Resolve result dimensions for one broadcast operation.
    fn broadcast_dimensions(
        &self,
        layout: TensorLayout,
        source: &Tensor,
        axes: &[u16],
    ) -> Result<Vec<usize>> {
        let dimensions = self.program.tensor_dimensions(layout.dimensions);
        let mut result = Vec::with_capacity(dimensions.len());

        // source axes provide dynamic extents, other axes must be fixed
        for (axis, dimension) in dimensions.iter().copied().enumerate() {
            let source_axis = axes.iter().position(|mapped| *mapped as usize == axis);
            let extent = match (dimension.fixed_extent(), source_axis) {
                (Some(extent), Some(source_axis))
                    if Some(extent as usize) != source.dimension(source_axis) =>
                {
                    return Err(self.invalid_instruction());
                }
                (Some(extent), _) => extent as usize,
                (None, Some(source_axis)) => source
                    .dimension(source_axis)
                    .ok_or_else(|| self.invalid_instruction())?,
                (None, None) => return Err(self.invalid_instruction()),
            };
            result.push(extent);
        }

        Ok(result)
    }

    /// Copy all logical elements between shape-compatible tensors.
    fn copy_tensor(&self, target: &Tensor, source: &Tensor) -> Result<()> {
        if target.element_count() != source.element_count() || target.scalar != source.scalar {
            return Err(self.invalid_instruction());
        }

        for index in 0..target.element_count() {
            let source_address = source
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;
            let target_address = target
                .linear_address(index)
                .ok_or_else(|| Error::trap(Trap::Bounds))?;

            self.store(
                target_address,
                target.scalar,
                self.load(source_address, source.scalar),
            );
        }

        Ok(())
    }
}
