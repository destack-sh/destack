use std::ptr;

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{vaddq_s32, vld1q_s32, vst1q_s32};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128i, _mm_add_epi32, _mm_loadu_si128, _mm_storeu_si128};

use tspp_bytecode::{
    ConvertMode, FloatOperation, Instruction, IntegerOperation, Operands, ReduceOperation,
    RegisterSpan, Scalar, VectorOperation, VectorType,
};
use tspp_program::{Runtime, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

use super::arithmetic::Arithmetic;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one packed vector operation.
    pub(crate) fn execute_vector(
        &mut self,
        instruction: Instruction<'_>,
        operation: VectorOperation,
    ) -> Result<()> {
        match operation {
            VectorOperation::Splat => self.execute_vector_splat(instruction),
            VectorOperation::Insert => self.execute_vector_insert(instruction),
            VectorOperation::Extract => self.execute_vector_extract(instruction),
            VectorOperation::Shuffle => self.execute_vector_shuffle(instruction),
            VectorOperation::Element => self.execute_vector_element(instruction, false),
            VectorOperation::Compare => self.execute_vector_element(instruction, true),
            VectorOperation::Select => self.execute_vector_select(instruction),
            VectorOperation::Reduce => self.execute_vector_reduce(instruction),
            VectorOperation::Load => self.execute_vector_load(instruction),
            VectorOperation::Store => self.execute_vector_store(instruction),
            VectorOperation::Convert => self.execute_vector_convert(instruction),
        }
    }

    /// Return the touched native byte range for one vector memory operation.
    pub(crate) fn vector_address(
        &self,
        instruction: Instruction<'_>,
        operation: VectorOperation,
    ) -> Result<(usize, usize)> {
        let mut operands = self.operands(instruction);
        let pointer = if operation == VectorOperation::Load {
            let _target = operands.span()?;

            operands.register()?
        } else {
            operands.register()?
        };
        if operation == VectorOperation::Store {
            let _source = operands.span()?;
        }
        let vector = operands.vector_type()?;
        let address = self.read(pointer.0).bits() as usize;

        Ok((address, Self::vector_byte_len(vector)))
    }

    /// Broadcast one scalar across every lane.
    fn execute_vector_splat(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let value = operands.register()?;
        let vector = operands.vector_type()?;
        let value = self.read(value.0).bits();
        self.clear_vector(target);

        for lane in 0..vector.lane_count {
            self.write_vector_lane(target, vector, lane, value);
        }

        Ok(())
    }

    /// Replace one vector lane.
    fn execute_vector_insert(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let source = operands.span()?;
        let index = operands.register()?;
        let value = operands.register()?;
        let vector = operands.vector_type()?;
        let index = self.read(index.0).bits();
        if index >= u64::from(vector.lane_count) {
            return Err(Error::trap(Trap::Bounds));
        }
        self.move_range(source, target);
        self.write_vector_lane(target, vector, index as u16, self.read(value.0).bits());

        Ok(())
    }

    /// Extract one vector lane.
    fn execute_vector_extract(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.span()?;
        let index = operands.register()?;
        let vector = operands.vector_type()?;
        let index = self.read(index.0).bits();
        if index >= u64::from(vector.lane_count) {
            return Err(Error::trap(Trap::Bounds));
        }
        let value = self.read_vector_lane(source, vector, index as u16);

        self.write(target.0, Word::from_bits(vector.scalar.encode(value)));

        Ok(())
    }

    /// Select lanes from two vectors.
    fn execute_vector_shuffle(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let input_count = operands.u16()?;
        if input_count != 2 {
            return Err(self.invalid_instruction());
        }
        let left = operands.register()?;
        let right = operands.register()?;
        let lane_count = operands.u16()?;
        if lane_count == 0 {
            return Err(self.invalid_instruction());
        }
        let lanes = operands;
        let mut vector_operand = lanes;
        for _ in 0..lane_count {
            vector_operand.u16()?;
        }
        let vector = vector_operand.vector_type()?;
        if lane_count != vector.lane_count {
            return Err(self.invalid_instruction());
        }
        self.clear_vector(target);

        let mut lanes = lanes;
        for target_lane in 0..lane_count {
            let source_lane = lanes.u16()?;
            if source_lane >= lane_count * 2 {
                return Err(self.invalid_instruction());
            }
            let (source, source_lane) = if source_lane < lane_count {
                (left, source_lane)
            } else {
                (right, source_lane - lane_count)
            };
            let source = RegisterSpan::new(source, vector.word_count());
            let value = self.read_vector_lane(source, vector, source_lane);
            self.write_vector_lane(target, vector, target_lane, value);
        }

        Ok(())
    }

    /// Apply one scalar operation independently to every lane.
    fn execute_vector_element(
        &mut self,
        instruction: Instruction<'_>,
        is_comparison: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let input_count = operands.u16()?;
        if input_count == 0 || input_count > 3 {
            return Err(self.invalid_instruction());
        }
        let inputs = operands;
        for _ in 0..input_count {
            operands.register()?;
        }
        let vector = operands.vector_type()?;
        let operator = operands.u16()? as u8;
        if vector.scalar.is_float() {
            let operation =
                FloatOperation::from_code(operator).ok_or_else(|| self.invalid_instruction())?;
            if operation.returns_boolean() != is_comparison
                || (is_comparison && operation.input_count() != 2)
                || input_count != operation.input_count() as u16
            {
                return Err(self.invalid_instruction());
            }

            self.execute_float_vector_lanes(target, inputs, input_count, vector, operation)
        } else {
            let operation =
                IntegerOperation::from_code(operator).ok_or_else(|| self.invalid_instruction())?;
            if operation.is_overflowing()
                || operation.returns_boolean() != is_comparison
                || (is_comparison && operation.input_count() != 2)
                || input_count != operation.input_count() as u16
            {
                return Err(self.invalid_instruction());
            }

            self.execute_integer_vector_lanes(target, inputs, input_count, vector, operation)
        }
    }

    /// Execute one floating-point operation across packed vector lanes.
    fn execute_float_vector_lanes(
        &mut self,
        target: RegisterSpan,
        inputs: Operands<'_, false>,
        input_count: u16,
        vector: VectorType,
        operation: FloatOperation,
    ) -> Result<()> {
        let scalar = operation
            .result_scalar(vector.scalar)
            .ok_or_else(|| self.invalid_instruction())?;
        let result = VectorType::new(scalar, vector.lane_count);
        if target.word_count != result.word_count() {
            return Err(self.invalid_instruction());
        }
        self.clear_vector(target);

        // apply the decoded operation to every packed lane
        for lane in 0..vector.lane_count {
            let mut inputs = inputs;
            let left = self.vector_input(&mut inputs, vector, lane)?;
            let right = if input_count >= 2 {
                Some(self.vector_input(&mut inputs, vector, lane)?)
            } else {
                None
            };
            let third = if input_count == 3 {
                Some(self.vector_input(&mut inputs, vector, lane)?)
            } else {
                None
            };
            let value = Arithmetic::float(operation, vector.scalar, left, right, third)?;
            self.write_vector_lane(target, result, lane, value.bits());
        }

        Ok(())
    }

    /// Execute one integer operation across packed vector lanes.
    fn execute_integer_vector_lanes(
        &mut self,
        target: RegisterSpan,
        inputs: Operands<'_, false>,
        input_count: u16,
        vector: VectorType,
        operation: IntegerOperation,
    ) -> Result<()> {
        // execute the common packed 128-bit add directly
        if operation == IntegerOperation::Add
            && vector.lane_count == 4
            && matches!(vector.scalar, Scalar::Int32 | Scalar::Uint32)
        {
            let mut inputs = inputs;
            let left = inputs.register()?;
            let right = inputs.register()?;
            let left = RegisterSpan::new(left, vector.word_count());
            let right = RegisterSpan::new(right, vector.word_count());
            self.add_i32x4(target, left, right);

            return Ok(());
        }

        let scalar = operation
            .result_scalar(vector.scalar)
            .ok_or_else(|| self.invalid_instruction())?;
        let result = VectorType::new(scalar, vector.lane_count);
        if target.word_count != result.word_count() {
            return Err(self.invalid_instruction());
        }
        self.clear_vector(target);

        // apply the decoded operation to every packed lane
        for lane in 0..vector.lane_count {
            let mut inputs = inputs;
            let left = self.vector_input(&mut inputs, vector, lane)?;
            let right = if input_count >= 2 {
                Some(self.vector_input(&mut inputs, vector, lane)?)
            } else {
                None
            };
            let third = if input_count == 3 {
                Some(self.vector_input(&mut inputs, vector, lane)?)
            } else {
                None
            };
            let value = Arithmetic::integer(operation, vector.scalar, left, right, third)?;
            self.write_vector_lane(target, result, lane, value.bits());
        }

        Ok(())
    }

    /// Convert every vector lane into one target scalar representation.
    fn execute_vector_convert(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let source = operands.span()?;
        let source_type = operands.vector_type()?;
        let target_type = operands.vector_type()?;
        let mode = operands.u16()? as u8;
        let mode = ConvertMode::from_code(mode).ok_or_else(|| self.invalid_instruction())?;
        if source_type.lane_count != target_type.lane_count {
            return Err(self.invalid_instruction());
        }
        self.clear_vector(target);

        // convert each packed lane through the shared scalar conversion
        for lane in 0..source_type.lane_count {
            let value = self.read_vector_lane(source, source_type, lane);
            let value = Word::from_bits(source_type.scalar.encode(value));
            let value = Arithmetic::convert(value, source_type.scalar, target_type.scalar, mode)?;
            self.write_vector_lane(target, target_type, lane, value.bits());
        }

        Ok(())
    }

    /// Select each lane through one packed boolean mask.
    fn execute_vector_select(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let input_count = operands.u16()?;
        if input_count != 3 {
            return Err(self.invalid_instruction());
        }
        let condition = operands.register()?;
        let left = operands.register()?;
        let right = operands.register()?;
        let vector = operands.vector_type()?;
        let condition = RegisterSpan::new(condition, vector.mask().word_count());
        let left = RegisterSpan::new(left, vector.word_count());
        let right = RegisterSpan::new(right, vector.word_count());
        self.clear_vector(target);

        for lane in 0..vector.lane_count {
            let source = if self.read_vector_lane(condition, vector.mask(), lane) != 0 {
                left
            } else {
                right
            };
            let value = self.read_vector_lane(source, vector, lane);
            self.write_vector_lane(target, vector, lane, value);
        }

        Ok(())
    }

    /// Reduce every vector lane into one scalar.
    fn execute_vector_reduce(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.span()?;
        let vector = operands.vector_type()?;
        let operation = operands.u16()? as u8;
        let operation =
            ReduceOperation::from_code(operation).ok_or_else(|| self.invalid_instruction())?;
        if vector.lane_count == 0 {
            return Err(self.invalid_instruction());
        }
        let lane = self.read_vector_lane(source, vector, 0);
        let mut value = Word::from_bits(vector.scalar.encode(lane));

        for lane in 1..vector.lane_count {
            let right = self.read_vector_lane(source, vector, lane);
            let right = Word::from_bits(vector.scalar.encode(right));
            value = Arithmetic::reduce(operation, vector.scalar, value, right)?;
        }
        self.write(target.0, value);

        Ok(())
    }

    /// Load one packed vector from a native pointer.
    fn execute_vector_load(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let source = operands.register()?;
        let vector = operands.vector_type()?;
        let target = self.vector_pointer(target);
        let source = self.read(source.0).bits() as *const u8;

        // SAFETY: raw pointer operations require one live range of the encoded vector width
        unsafe {
            ptr::copy_nonoverlapping(source, target, Self::vector_byte_len(vector));
        }

        Ok(())
    }

    /// Store one packed vector through a native pointer.
    fn execute_vector_store(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.span()?;
        let vector = operands.vector_type()?;
        let target = self.read(target.0).bits() as *mut u8;
        let source = self.vector_pointer(source).cast_const();

        // SAFETY: raw pointer operations require one live range of the encoded vector width
        unsafe {
            ptr::copy_nonoverlapping(source, target, Self::vector_byte_len(vector));
        }

        Ok(())
    }

    /// Read one vector input lane from an encoded register list.
    fn vector_input(
        &self,
        operands: &mut Operands<'_, false>,
        vector: VectorType,
        lane: u16,
    ) -> Result<Word> {
        let register = operands.register()?;
        let range = RegisterSpan::new(register, vector.word_count());
        let value = self.read_vector_lane(range, vector, lane);

        Ok(Word::from_bits(vector.scalar.encode(value)))
    }
}

impl Arithmetic {
    /// Reduce two scalar lanes with one associative operation.
    pub(super) fn reduce(
        operation: ReduceOperation,
        scalar: Scalar,
        left: Word,
        right: Word,
    ) -> Result<Word> {
        if scalar.is_float() {
            let operation = match operation {
                ReduceOperation::Add => FloatOperation::Add,
                ReduceOperation::Multiply => FloatOperation::Multiply,
                ReduceOperation::Minimum => FloatOperation::Minimum,
                ReduceOperation::Maximum => FloatOperation::Maximum,
                _ => return Err(Error::invalid_instruction()),
            };

            Self::float(operation, scalar, left, Some(right), None)
        } else {
            let operation = match operation {
                ReduceOperation::Add => IntegerOperation::Add,
                ReduceOperation::Multiply => IntegerOperation::Multiply,
                ReduceOperation::Minimum => {
                    let comparison =
                        Self::integer(IntegerOperation::LessThan, scalar, left, Some(right), None)?;
                    let is_less = comparison.bits() != 0;

                    return Ok(if is_less { left } else { right });
                }
                ReduceOperation::Maximum => {
                    let comparison = Self::integer(
                        IntegerOperation::GreaterThan,
                        scalar,
                        left,
                        Some(right),
                        None,
                    )?;
                    let is_greater = comparison.bits() != 0;

                    return Ok(if is_greater { left } else { right });
                }
                ReduceOperation::And => IntegerOperation::And,
                ReduceOperation::Or => IntegerOperation::Or,
                ReduceOperation::Xor => IntegerOperation::Xor,
            };

            Self::integer(operation, scalar, left, Some(right), None)
        }
    }
}

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Read one packed lane from a contiguous register range.
    #[inline(always)]
    fn read_vector_lane(&self, range: RegisterSpan, vector: VectorType, lane: u16) -> u64 {
        let bit_width = u32::from(vector.scalar.bit_width());
        let bit_offset = u32::from(lane) * bit_width;
        let word = range.start.0 + (bit_offset / u64::BITS) as u16;
        let shift = bit_offset % u64::BITS;
        let mask = Self::lane_mask(vector.scalar);

        (self.read(word).bits() >> shift) & mask
    }

    /// Write one packed lane into a contiguous register range.
    #[inline(always)]
    fn write_vector_lane(
        &mut self,
        range: RegisterSpan,
        vector: VectorType,
        lane: u16,
        value: u64,
    ) {
        let bit_width = u32::from(vector.scalar.bit_width());
        let bit_offset = u32::from(lane) * bit_width;
        let word = range.start.0 + (bit_offset / u64::BITS) as u16;
        let shift = bit_offset % u64::BITS;
        let mask = Self::lane_mask(vector.scalar);
        let bits = self.read(word).bits();
        let bits = (bits & !(mask << shift)) | ((value & mask) << shift);

        self.write(word, Word::from_bits(bits));
    }

    /// Clear one vector result range before packed lane writes.
    fn clear_vector(&mut self, range: RegisterSpan) {
        for word in 0..range.word_count {
            self.write(range.start.0 + word, Word::ZERO);
        }
    }

    /// Add four packed 32-bit integer lanes on AArch64.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    fn add_i32x4(&mut self, target: RegisterSpan, left: RegisterSpan, right: RegisterSpan) {
        let target = self.vector_pointer(target).cast::<i32>();
        let left = self.vector_pointer(left).cast::<i32>();
        let right = self.vector_pointer(right).cast::<i32>();

        // SAFETY: every range names two live register words containing four 32-bit lanes
        unsafe {
            let left = vld1q_s32(left);
            let right = vld1q_s32(right);
            vst1q_s32(target, vaddq_s32(left, right));
        }
    }

    /// Add four packed 32-bit integer lanes on x86-64.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn add_i32x4(&mut self, target: RegisterSpan, left: RegisterSpan, right: RegisterSpan) {
        let target = self.vector_pointer(target).cast::<__m128i>();
        let left = self.vector_pointer(left).cast::<__m128i>();
        let right = self.vector_pointer(right).cast::<__m128i>();

        // SAFETY: every range names two live register words containing four 32-bit lanes
        unsafe {
            let left = _mm_loadu_si128(left);
            let right = _mm_loadu_si128(right);
            _mm_storeu_si128(target, _mm_add_epi32(left, right));
        }
    }

    /// Add four packed 32-bit integer lanes without target SIMD.
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    #[inline(always)]
    fn add_i32x4(&mut self, target: RegisterSpan, left: RegisterSpan, right: RegisterSpan) {
        self.clear_vector(target);
        let vector = VectorType::new(Scalar::Uint32, 4);

        // preserve wrapping lane arithmetic on scalar targets
        for lane in 0..vector.lane_count {
            let left = self.read_vector_lane(left, vector, lane);
            let right = self.read_vector_lane(right, vector, lane);
            self.write_vector_lane(target, vector, lane, left.wrapping_add(right));
        }
    }

    /// Return the native address of one vector register range.
    fn vector_pointer(&self, range: RegisterSpan) -> *mut u8 {
        let frame = self.frame();
        let byte_offset = frame.range(range) * Word::BYTE_LEN;

        self.fiber.stack.address(byte_offset) as *mut u8
    }

    /// Return one vector lane bit mask.
    const fn lane_mask(scalar: Scalar) -> u64 {
        let bit_width = scalar.bit_width();
        if bit_width == u64::BITS as u8 {
            u64::MAX
        } else {
            (1_u64 << bit_width) - 1
        }
    }

    /// Return one vector's exact packed byte width.
    const fn vector_byte_len(vector: VectorType) -> usize {
        let bit_len = vector.scalar.bit_width() as usize * vector.lane_count as usize;

        bit_len.div_ceil(u8::BITS as usize)
    }
}
