use std::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, Ordering, fence};

use destack_bytecode::{
    AtomicAccess, AtomicOperation, AtomicOrder, CodeOffset, CompareExchangeAccess, FenceAccess,
    Instruction, Scalar,
};
use destack_program::{Continuation, MemoryAccess, Outcome, Word};

use crate::diagnostic::Result;
use crate::machine::{Activation, Frame};

impl Activation<'_, '_> {
    /// Execute one typed atomic memory operation.
    pub(crate) fn execute_atomic<const WATCH: bool>(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        if !operation.supports(scalar) {
            return Err(self.invalid_instruction());
        }

        let needs_range = WATCH
            && self
                .watch_points
                .is_some_and(|points| points.requires_memory_range());
        let address = if needs_range {
            Some(self.atomic_address(instruction, operation, scalar)?)
        } else {
            None
        };

        // execute through the exact machine width
        match scalar.bit_width() {
            8 => self.execute_atomic_word::<AtomicU8>(instruction, operation, scalar)?,
            16 => self.execute_atomic_word::<AtomicU16>(instruction, operation, scalar)?,
            32 => self.execute_atomic_word::<AtomicU32>(instruction, operation, scalar)?,
            64 => self.execute_atomic_word::<AtomicU64>(instruction, operation, scalar)?,
            _ => unreachable!("atomic scalars occupy one bytecode word"),
        }

        // report the completed access only in the observed loop
        let access = match operation {
            AtomicOperation::Load => MemoryAccess::Read,
            AtomicOperation::Store => MemoryAccess::Write,
            _ => MemoryAccess::ReadWrite,
        };
        if WATCH {
            self.watch_after(frame, instruction_offset, access, address)
        } else {
            Ok(None)
        }
    }

    /// Execute one atomic fence.
    pub(crate) fn execute_atomic_fence(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let access = operands
            .u32()
            .ok()
            .and_then(FenceAccess::from_bits)
            .ok_or_else(|| self.invalid_instruction())?;
        if access.order == AtomicOrder::Relaxed || access.storage.0 == 0 {
            return Err(self.invalid_instruction());
        }

        fence(Self::atomic_order(access.order));

        Ok(())
    }

    /// Execute one atomic operation through its exact machine width.
    fn execute_atomic_word<A: AtomicWord>(
        &mut self,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> Result<()> {
        match operation {
            AtomicOperation::Load => self.execute_atomic_load::<A>(instruction, scalar),
            AtomicOperation::Store => self.execute_atomic_store::<A>(instruction),
            operation if operation.is_compare_exchange() => {
                self.execute_atomic_compare_exchange::<A>(instruction, operation, scalar)
            }
            operation => self.execute_atomic_update::<A>(instruction, operation, scalar),
        }
    }

    /// Execute one exact-width atomic load.
    #[inline(always)]
    fn execute_atomic_load<A: AtomicWord>(
        &mut self,
        instruction: Instruction<'_>,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let pointer = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let access = operands
            .u16()
            .ok()
            .and_then(AtomicAccess::from_bits)
            .filter(|access| AtomicOperation::Load.accepts(access.order))
            .ok_or_else(|| self.invalid_instruction())?;

        // SAFETY: atomic bytecode requires a live naturally aligned atomic address
        let atomic = unsafe { A::from_address(self.read(pointer.0).bits() as usize) };
        let value = atomic.load(Self::atomic_order(access.order));

        self.write(target.0, Word::from_bits(scalar.encode(value)));

        Ok(())
    }

    /// Execute one exact-width atomic store.
    #[inline(always)]
    fn execute_atomic_store<A: AtomicWord>(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let pointer = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let value = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let access = operands
            .u16()
            .ok()
            .and_then(AtomicAccess::from_bits)
            .filter(|access| AtomicOperation::Store.accepts(access.order))
            .ok_or_else(|| self.invalid_instruction())?;

        // SAFETY: atomic bytecode requires a live naturally aligned atomic address
        let atomic = unsafe { A::from_address(self.read(pointer.0).bits() as usize) };
        atomic.store(self.read(value.0).bits(), Self::atomic_order(access.order));

        Ok(())
    }

    /// Execute one exact-width atomic compare exchange.
    #[inline(always)]
    fn execute_atomic_compare_exchange<A: AtomicWord>(
        &mut self,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let status = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let pointer = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let expected = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let replacement = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let access = operands
            .u16()
            .ok()
            .and_then(CompareExchangeAccess::from_bits)
            .filter(|access| access.success.permits_failure(access.failure))
            .ok_or_else(|| self.invalid_instruction())?;

        // SAFETY: atomic bytecode requires a live naturally aligned atomic address
        let atomic = unsafe { A::from_address(self.read(pointer.0).bits() as usize) };
        let result = atomic.compare_exchange(
            self.read(expected.0).bits(),
            self.read(replacement.0).bits(),
            Self::atomic_order(access.success),
            Self::atomic_order(access.failure),
            operation == AtomicOperation::CompareExchangeWeak,
        );
        let (value, did_exchange) = match result {
            Ok(value) => (value, true),
            Err(value) => (value, false),
        };

        self.write(target.0, Word::from_bits(scalar.encode(value)));
        self.write(status.0, Word::boolean(did_exchange));

        Ok(())
    }

    /// Execute one exact-width atomic exchange or update.
    #[inline(always)]
    fn execute_atomic_update<A: AtomicWord>(
        &mut self,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let pointer = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let value = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let access = operands
            .u16()
            .ok()
            .and_then(AtomicAccess::from_bits)
            .ok_or_else(|| self.invalid_instruction())?;

        // SAFETY: atomic bytecode requires a live naturally aligned atomic address
        let atomic = unsafe { A::from_address(self.read(pointer.0).bits() as usize) };
        let value = self.read(value.0).bits();
        let order = Self::atomic_order(access.order);
        let previous = if operation == AtomicOperation::Exchange {
            atomic.exchange(value, order)
        } else {
            Self::atomic_update(atomic, operation, scalar, value, order)
        };

        self.write(target.0, Word::from_bits(scalar.encode(previous)));

        Ok(())
    }

    /// Apply one typed atomic read-modify-write operation.
    fn atomic_update<A: AtomicWord>(
        atomic: &A,
        operation: AtomicOperation,
        scalar: Scalar,
        operand: u64,
        order: Ordering,
    ) -> u64 {
        let failure = Self::atomic_failure_order(order);
        let mut current = atomic.load(failure);

        loop {
            let next = Self::atomic_value(operation, scalar, current, operand);
            match atomic.compare_exchange(current, next, order, failure, true) {
                Ok(_) => return current,
                Err(observed) => current = observed,
            }
        }
    }

    /// Compute one typed atomic read-modify-write result.
    fn atomic_value(operation: AtomicOperation, scalar: Scalar, left: u64, right: u64) -> u64 {
        if scalar.is_float() {
            return Self::atomic_float(operation, scalar, left, right);
        }

        let width = scalar.bit_width();
        let mask = if width == u64::BITS as u8 {
            u64::MAX
        } else {
            (1_u64 << width) - 1
        };
        let value = match operation {
            AtomicOperation::FetchAdd => left.wrapping_add(right),
            AtomicOperation::FetchSubtract => left.wrapping_sub(right),
            AtomicOperation::FetchAnd => left & right,
            AtomicOperation::FetchOr => left | right,
            AtomicOperation::FetchXor => left ^ right,
            AtomicOperation::FetchMinimum if scalar.is_signed_integer() => {
                let (Some(left), Some(right)) = (scalar.integer(left), scalar.integer(right))
                else {
                    unreachable!("integer atomic opcodes carry integer scalars");
                };

                left.min(right) as u64
            }
            AtomicOperation::FetchMinimum => left.min(right),
            AtomicOperation::FetchMaximum if scalar.is_signed_integer() => {
                let (Some(left), Some(right)) = (scalar.integer(left), scalar.integer(right))
                else {
                    unreachable!("integer atomic opcodes carry integer scalars");
                };

                left.max(right) as u64
            }
            AtomicOperation::FetchMaximum => left.max(right),
            _ => unreachable!("atomic update dispatch selects one read-modify-write operation"),
        };

        value & mask
    }

    /// Compute one floating-point atomic read-modify-write result.
    fn atomic_float(operation: AtomicOperation, scalar: Scalar, left: u64, right: u64) -> u64 {
        let (Some(left), Some(right)) = (scalar.float(left), scalar.float(right)) else {
            unreachable!("floating-point atomic opcodes carry floating-point scalars");
        };
        let value = match operation {
            AtomicOperation::FetchAdd => left + right,
            AtomicOperation::FetchSubtract => left - right,
            AtomicOperation::FetchMinimum if left.is_nan() => left,
            AtomicOperation::FetchMinimum if right.is_nan() => right,
            AtomicOperation::FetchMinimum => left.min(right),
            AtomicOperation::FetchMaximum if left.is_nan() => left,
            AtomicOperation::FetchMaximum if right.is_nan() => right,
            AtomicOperation::FetchMaximum => left.max(right),
            _ => unreachable!("floating-point atomics support arithmetic and extrema"),
        };

        let Some(bits) = scalar.float_bits(value) else {
            unreachable!("floating-point atomic opcodes carry floating-point scalars");
        };

        bits
    }

    /// Convert one bytecode memory order to the host atomic order.
    const fn atomic_order(order: AtomicOrder) -> Ordering {
        match order {
            AtomicOrder::Relaxed => Ordering::Relaxed,
            AtomicOrder::Acquire => Ordering::Acquire,
            AtomicOrder::Release => Ordering::Release,
            AtomicOrder::AcquireRelease => Ordering::AcqRel,
            AtomicOrder::SequentiallyConsistent => Ordering::SeqCst,
        }
    }

    /// Return the strongest legal failure order for one atomic update.
    fn atomic_failure_order(order: Ordering) -> Ordering {
        match order {
            Ordering::Relaxed | Ordering::Release => Ordering::Relaxed,
            Ordering::Acquire | Ordering::AcqRel => Ordering::Acquire,
            Ordering::SeqCst => Ordering::SeqCst,
            _ => unreachable!("host atomics expose the standard memory orders"),
        }
    }
}

/// One exact-width host atomic word.
trait AtomicWord {
    /// Resolve one naturally aligned native pointer.
    unsafe fn from_address<'a>(address: usize) -> &'a Self;

    /// Load one word.
    fn load(&self, order: Ordering) -> u64;

    /// Store one word.
    fn store(&self, value: u64, order: Ordering);

    /// Exchange one word.
    fn exchange(&self, value: u64, order: Ordering) -> u64;

    /// Compare and exchange one word.
    fn compare_exchange(
        &self,
        current: u64,
        new: u64,
        success: Ordering,
        failure: Ordering,
        is_weak: bool,
    ) -> std::result::Result<u64, u64>;
}

macro_rules! implement_atomic_word {
    ($atomic:ty, $integer:ty) => {
        impl AtomicWord for $atomic {
            unsafe fn from_address<'a>(address: usize) -> &'a Self {
                // SAFETY: the caller provides one live naturally aligned atomic address
                unsafe { &*(address as *const Self) }
            }

            fn load(&self, order: Ordering) -> u64 {
                <$atomic>::load(self, order) as u64
            }

            fn store(&self, value: u64, order: Ordering) {
                <$atomic>::store(self, value as $integer, order);
            }

            fn exchange(&self, value: u64, order: Ordering) -> u64 {
                <$atomic>::swap(self, value as $integer, order) as u64
            }

            fn compare_exchange(
                &self,
                current: u64,
                new: u64,
                success: Ordering,
                failure: Ordering,
                is_weak: bool,
            ) -> std::result::Result<u64, u64> {
                let result = if is_weak {
                    <$atomic>::compare_exchange_weak(
                        self,
                        current as $integer,
                        new as $integer,
                        success,
                        failure,
                    )
                } else {
                    <$atomic>::compare_exchange(
                        self,
                        current as $integer,
                        new as $integer,
                        success,
                        failure,
                    )
                };

                result
                    .map(|value| value as u64)
                    .map_err(|value| value as u64)
            }
        }
    };
}

implement_atomic_word!(AtomicU8, u8);
implement_atomic_word!(AtomicU16, u16);
implement_atomic_word!(AtomicU32, u32);
implement_atomic_word!(AtomicU64, u64);
