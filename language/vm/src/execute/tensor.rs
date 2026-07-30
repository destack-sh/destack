use std::ptr;

use bytecode::{
    ConvertMode, ElementOperation, IndexReduceOperation, Instruction, Operands, ReduceOperation,
    RegisterId, RegisterSpan, Scalar, TensorOperand, TensorOperation, TieBreak,
};
use destack_bytecode as bytecode;
use destack_heap::{AllocationShape, HeapEdge, Payload};
use destack_mir as mir;
use destack_program::{
    AllocationSite, AllocationSiteId, LayoutId, LayoutShape, MemoryAccess, Runtime, ScalarFormat,
    TensorDimension, TensorLayout, TensorSharding, TensorViewLayout, TypeId, Word,
};
use mir::{TensorDimensionOrder, TensorFormat, TraceMap};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

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
    /// Resolve one dense owning tensor.
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
    /// Execute one tensor operation through the direct CPU engine.
    pub(crate) fn execute_tensor(
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
            TensorOperation::Contract
            | TensorOperation::Gather
            | TensorOperation::Scatter
            | TensorOperation::Convolution => {
                return Err(Error::unsupported_opcode(instruction.opcode().code()));
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
        let mut inputs = operands.tensors()?;
        let operator = operands.u16()?;
        let operator =
            ElementOperation::from_code(operator).ok_or_else(|| self.invalid_instruction())?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let expects_comparison = tensor_operation == TensorOperation::Compare;
        let first_input = inputs.next().ok_or_else(|| self.invalid_instruction())?;
        let first = self.tensor(first_input)?;
        let source_scalar = first.scalar;
        let (float_operation, integer_operation, input_count, is_comparison) =
            if let Some(operation) = operator.float_operation() {
                if !source_scalar.is_float() {
                    return Err(self.invalid_instruction());
                }

                (
                    Some(operation),
                    None,
                    operation.input_count(),
                    operation.is_comparison(),
                )
            } else if let Some(operation) = operator.integer_operation() {
                if !source_scalar.is_integer() || operation.is_overflowing() {
                    return Err(self.invalid_instruction());
                }

                (
                    None,
                    Some(operation),
                    operation.input_count(),
                    operation.is_comparison(),
                )
            } else {
                return Err(self.invalid_instruction());
            };
        if inputs.len() + 1 != input_count || is_comparison != expects_comparison {
            return Err(self.invalid_instruction());
        }
        let mut tensors = [first; 3];
        for (index, input) in inputs.enumerate() {
            let tensor = self.tensor(input)?;
            if tensor.scalar != source_scalar {
                return Err(self.invalid_instruction());
            }
            tensors[index + 1] = tensor;
        }
        for tensor in tensors.iter().take(input_count) {
            if !first.matches_dimensions(tensor) {
                return Err(self.invalid_instruction());
            }
        }
        if expects_comparison {
            if allocation.scalar != Scalar::Boolean {
                return Err(self.invalid_instruction());
            }
        } else if allocation.scalar != source_scalar {
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
            let addend = values[2];
            let value = if let Some(operation) = float_operation {
                self.float_value(operation, source_scalar, left, right, addend)?
            } else if let Some(operation) = integer_operation {
                self.integer_value(operation, source_scalar, left, right)?
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
        let mut inputs = operands.tensors()?;
        let allocation = self.tensor_allocation(&mut operands)?;
        let condition = self.tensor(inputs.next().ok_or_else(|| self.invalid_instruction())?)?;
        let left = self.tensor(inputs.next().ok_or_else(|| self.invalid_instruction())?)?;
        let right = self.tensor(inputs.next().ok_or_else(|| self.invalid_instruction())?)?;
        if condition.scalar != Scalar::Boolean
            || left.scalar != allocation.scalar
            || right.scalar != allocation.scalar
            || inputs.next().is_some()
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
        let mut tensors = operands.tensors()?;
        let target = self.tensor(tensors.next().ok_or_else(|| self.invalid_instruction())?)?;
        let source = self.tensor(tensors.next().ok_or_else(|| self.invalid_instruction())?)?;
        if target.scalar != source.scalar
            || tensors.next().is_some()
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
            let value = self.convert_scalar(value, source.scalar, allocation.scalar, mode)?;

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
            let value = self.reduce_value(
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
                let reduced =
                    self.reduce_value(comparison, source.scalar, selected_value, value)?;
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
            .machine
            .program
            .layout_by_id(layout)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::TensorView(layout) = layout.shape else {
            return Err(self.invalid_instruction());
        };
        let source = self.tensor(source)?;
        let scalar = self.tensor_scalar(layout.element)?;
        let rank = self
            .machine
            .program
            .tensor_dimensions(layout.dimensions)
            .len();
        if source.scalar != scalar
            || offsets.len() != rank
            || sizes.len() != rank
            || strides.len() != rank
        {
            return Err(self.invalid_instruction());
        }
        let expected = self.machine.program.tensor_dimensions(layout.dimensions);
        self.match_tensor_dimensions(expected, sizes.iter().copied())?;
        let base = self.activation.memory.address(source.edge);
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
            .machine
            .program
            .layout_by_id(layout)
            .ok_or_else(|| self.invalid_instruction())?;

        match layout.shape {
            LayoutShape::Tensor(layout) => self.owning_tensor(input.registers, layout),
            LayoutShape::TensorView(layout) => self.tensor_view(input.registers, layout),
            _ => Err(self.invalid_instruction()),
        }
    }

    /// Resolve one owning tensor register.
    fn owning_tensor(&self, registers: RegisterSpan, layout: TensorLayout) -> Result<Tensor> {
        if registers.word_count != 1 || !matches!(layout.sharding, TensorSharding::Unsharded) {
            return Err(Error::unsupported_tensor_sharding());
        }
        let scalar = self.tensor_scalar(layout.element)?;
        let rank = self
            .machine
            .program
            .tensor_dimensions(layout.dimensions)
            .len();
        let edge = self.read_edge(registers.start, layout.space)?;
        let base = self.activation.memory.address(edge);
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
        let scalar = self.tensor_scalar(layout.element)?;
        let rank = self
            .machine
            .program
            .tensor_dimensions(layout.dimensions)
            .len();
        if registers.word_count as usize != 2 + rank * 2 {
            return Err(self.invalid_instruction());
        }
        let space = layout
            .reference
            .heap_space()
            .ok_or_else(|| self.invalid_instruction())?;
        let edge = self.read_edge(registers.start, space)?;
        let byte_offset = self.read(registers.start.0 + 1).as_u64() as usize;
        let frame = self.frame();
        let dimension_register = frame.register(registers.start.0 + 2);
        let dimension_address = self
            .machine
            .stack
            .address(dimension_register * Word::BYTE_LEN);
        let stride_register = frame.register(registers.start.0 + 2 + rank as u16);
        let stride_address = self.machine.stack.address(stride_register * Word::BYTE_LEN);
        let expected = self.machine.program.tensor_dimensions(layout.dimensions);
        self.match_tensor_dimensions(expected, TensorDimensions::new(dimension_address, rank))?;
        let address = self
            .activation
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
            .machine
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
        if layout.space != site.space {
            return Err(self.invalid_instruction());
        }
        let expected_dimensions = self.machine.program.tensor_dimensions(layout.dimensions);
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
        let plan = self.activation.memory.plan_allocation(site.space, &shape);
        let edge = self
            .activation
            .memory
            .allocate(
                site.space,
                plan,
                Payload::Uninit,
                self.machine.program.trace_view(),
            )
            .map_err(Error::heap)?;
        self.write(target.0, Word::from_bits(edge.bits() as u64));

        // write the shape header before exposing element storage
        let base = self.activation.memory.address(edge);
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
            .machine
            .program
            .sites()
            .allocation_by_id(self.machine.program.sections(), site_id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let layout = self
            .machine
            .program
            .layout_by_id(site.layout)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::Tensor(layout) = layout.shape else {
            return Err(self.invalid_instruction());
        };
        let scalar = self.tensor_scalar(layout.element)?;

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
        self.machine
            .program
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
        let dimensions = self.machine.program.tensor_dimensions(layout.dimensions);
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
