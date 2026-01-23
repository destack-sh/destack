#![allow(elided_lifetimes_in_paths)]

use std::ptr::NonNull;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::{
    HeapHandle, LocalPointer, RawPointer, ReferenceAddressSpace, ReferenceMeta, StackPointer,
    Value, ValueTag,
};

use super::decode::{
    ArgumentRange, ConstValue, ControlFlow, CopyRange, INVALID_FUNCTION_INDEX, INVALID_VALUE_ID,
    ThreadedFunction, ThreadedInstruction, ThreadedInstructionData, ThreadedState,
    UNKNOWN_SLOT_COUNT, is_invalid_value, operator,
};
use super::execute::call::copy_values_with_plan;
use super::execute::instruction;
use super::state::{Frame, resize_and_clear_stack};
use crate::telemetry::stat_inc;

// dispatch slot indices
const VTABLE_FIELD_INDEX: u32 = 0;
const INTERFACE_ITAB_FIELD_INDEX: u32 = 1;

// helper macro: do work, then become next handler
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        $state.maybe_profile_instruction(&$block[$pc]);
        let next_pc = $pc + 1;
        become ($block[next_pc].handler)($state, $block, next_pc)
    }};
}

/// Build a global id from a raw value.
#[inline]
fn global_id(raw: u32) -> mir::LocalNodeId<mir::Global> {
    mir::LocalNodeId::new(raw)
}

/// Build a type id from a raw value.
#[inline]
fn type_id(raw: u32) -> mir::LocalNodeId<mir::Type> {
    mir::LocalNodeId::new(raw)
}

/// Collect argument values into a smallvec.
#[inline]
fn collect_values(
    state: &mut ThreadedState<'_, '_>,
    arguments: ArgumentRange,
) -> SmallVec<[Value; 16]> {
    // load argument slice
    let argument_slice = state.argument_slice(arguments);
    let mut args = SmallVec::with_capacity(argument_slice.len());

    // resolve argument values
    for arg in argument_slice {
        let value = state.get(*arg);
        args.push(value);
    }

    // return argument values
    args
}

/// Format a reference kind label for diagnostics.
fn reference_label(reference: ReferenceMeta) -> String {
    match reference.kind() {
        Some(kind) => {
            let address_space = reference.address_space();
            if matches!(address_space, ReferenceAddressSpace::Generic) {
                format!("{kind:?}")
            } else {
                format!("{kind:?} addrspace({})", address_space.label())
            }
        }
        None => "unknown".to_string(),
    }
}

/// Validate reference kind against the pointer storage.
fn check_reference_kind(
    state: &ThreadedState<'_, '_>,
    reference: ReferenceMeta,
    pointer: Value,
) -> Result<(), Error> {
    if !state
        .interpreter
        .isolate
        .options
        .checks
        .enforce_reference_kinds
    {
        return Ok(());
    }

    let Some(kind) = reference.kind() else {
        return Ok(());
    };

    let is_managed = pointer.tag() == ValueTag::ManagedReference;
    match kind {
        mir::ReferenceKind::Managed if !is_managed => Err(Error::InvalidReferenceKind {
            reference: reference_label(reference),
            actual: format!("{pointer:?}"),
        }),
        mir::ReferenceKind::Owned | mir::ReferenceKind::Borrowed | mir::ReferenceKind::Raw
            if is_managed =>
        {
            Err(Error::InvalidReferenceKind {
                reference: reference_label(reference),
                actual: format!("{pointer:?}"),
            })
        }
        _ => Ok(()),
    }?;

    check_reference_address_space(state, reference, pointer)
}

/// Map a runtime value to an unsigned index.
fn value_to_u64(value: Value) -> Result<u64, Error> {
    // decode integer values
    match value.tag() {
        ValueTag::Int => {
            let raw = value.raw_data() as i64;
            if raw < 0 {
                return Err(Error::TypeMismatch {
                    expected: "non-negative integer".to_string(),
                    actual: format!("{value:?}"),
                });
            }
            Ok(raw as u64)
        }
        ValueTag::UInt => Ok(value.raw_data()),
        _ => Err(Error::TypeMismatch {
            expected: "integer".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Map a runtime value to a usize index.
fn value_to_usize(value: Value) -> Result<usize, Error> {
    // convert to u64 first
    let index = value_to_u64(value)?;
    usize::try_from(index).map_err(|_| Error::TypeMismatch {
        expected: "usize index".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Load aggregate slots for an aggregate value.
fn aggregate_slots<'a>(
    state: &'a ThreadedState<'_, '_>,
    value: Value,
) -> Result<&'a [Value], Error> {
    // require aggregate payload
    state
        .interpreter
        .get_aggregate_slots(&value)
        .ok_or_else(|| Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{value:?}"),
        })
}

/// Tensor layout information for flattened storage.
#[derive(Debug, Clone)]
struct TensorLayoutInfo {
    /// The static tensor shape.
    shape: Vec<u64>,
    /// The per-dimension strides in element units.
    strides: Vec<u64>,
    /// The total storage length in element slots.
    storage_len: usize,
}

/// Convert tensor dimensions to a static shape.
fn static_shape(shape: &[mir::TensorDimension]) -> Result<Vec<u64>, Error> {
    // reject dynamic shapes for the interpreter
    let mut dims = Vec::with_capacity(shape.len());
    for dim in shape {
        match dim {
            mir::TensorDimension::Static(value) => dims.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic shape".to_string(),
                });
            }
        }
    }

    Ok(dims)
}

/// Convert tensor strides to a static list.
fn static_strides(strides: &[mir::TensorDimension]) -> Result<Vec<u64>, Error> {
    // reject dynamic strides for the interpreter
    let mut values = Vec::with_capacity(strides.len());
    for dim in strides {
        match dim {
            mir::TensorDimension::Static(value) => values.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic stride".to_string(),
                });
            }
        }
    }

    Ok(values)
}

/// Compute row-major strides for a shape.
fn row_major_strides(shape: &[u64]) -> Vec<u64> {
    // compute row-major strides
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (i, dim) in shape.iter().enumerate().rev() {
        strides[i] = stride;
        stride = stride.saturating_mul(*dim);
    }
    strides
}

/// Compute column-major strides for a shape.
fn column_major_strides(shape: &[u64]) -> Vec<u64> {
    // compute column-major strides
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (i, dim) in shape.iter().enumerate() {
        strides[i] = stride;
        stride = stride.saturating_mul(*dim);
    }
    strides
}

/// Compute the storage length for a shape and stride list.
fn tensor_storage_len(shape: &[u64], strides: &[u64]) -> Result<usize, Error> {
    // empty shape stores a single scalar
    if shape.is_empty() {
        return Ok(1);
    }

    // zero-sized shapes have zero elements
    if shape.contains(&0) {
        return Ok(0);
    }

    // compute max linear index
    let mut max_index = 0u64;
    for (dim, stride) in shape.iter().zip(strides.iter()) {
        let count = dim.saturating_sub(1);
        max_index = max_index.saturating_add(count.saturating_mul(*stride));
    }
    let len = max_index.saturating_add(1);
    usize::try_from(len).map_err(|_| Error::TypeMismatch {
        expected: "tensor storage length".to_string(),
        actual: len.to_string(),
    })
}

/// Resolve tensor layout information from a tensor type.
fn tensor_layout_info(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<TensorLayoutInfo, Error> {
    // resolve tensor shape and layout
    let (shape, layout) = match tree.get(ty) {
        mir::Type::Tensor { shape, layout, .. } => (shape, layout),
        mir::Type::TensorReference { shape, layout, .. } => (shape, layout),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    // compute static shape
    let shape = static_shape(shape)?;

    // compute strides
    let strides = match layout {
        mir::TensorLayout::RowMajor => row_major_strides(&shape),
        mir::TensorLayout::ColumnMajor => column_major_strides(&shape),
        mir::TensorLayout::Strided { strides } => static_strides(strides)?,
    };

    // compute storage length
    let storage_len = tensor_storage_len(&shape, &strides)?;

    Ok(TensorLayoutInfo {
        shape,
        strides,
        storage_len,
    })
}

/// Compute the linear index for a multi-dimensional index.
fn tensor_linear_index(indices: &[u64], shape: &[u64], strides: &[u64]) -> Result<usize, Error> {
    // validate index length
    if indices.len() != shape.len() || shape.len() != strides.len() {
        return Err(Error::TypeMismatch {
            expected: "tensor index rank".to_string(),
            actual: format!(
                "indices={}, shape={}, strides={}",
                indices.len(),
                shape.len(),
                strides.len()
            ),
        });
    }

    // compute linear index
    let mut offset = 0u64;
    for ((index, dim), stride) in indices.iter().zip(shape.iter()).zip(strides.iter()) {
        if *index >= *dim {
            return Err(Error::IndexOutOfBounds {
                index: *index,
                length: *dim,
            });
        }
        offset = offset.saturating_add(index.saturating_mul(*stride));
    }

    usize::try_from(offset).map_err(|_| Error::TypeMismatch {
        expected: "tensor index".to_string(),
        actual: offset.to_string(),
    })
}

/// Iterate over all indices in a tensor shape.
fn for_each_index<F: FnMut(&[u64])>(shape: &[u64], mut f: F) {
    // handle scalar or empty shapes
    if shape.is_empty() {
        f(&[]);
        return;
    }
    if shape.contains(&0) {
        return;
    }

    // initialize index vector
    let mut index = vec![0u64; shape.len()];
    loop {
        f(&index);

        // increment the odometer
        let mut dim = shape.len();
        while dim > 0 {
            dim -= 1;
            index[dim] += 1;
            if index[dim] < shape[dim] {
                break;
            }
            index[dim] = 0;
            if dim == 0 {
                return;
            }
        }
    }
}

/// Offset a pointer by an element index.
fn offset_pointer(value: Value, offset: usize, length: usize) -> Result<Value, Error> {
    // validate bounds
    if offset >= length {
        return Err(Error::IndexOutOfBounds {
            index: offset as u64,
            length: length as u64,
        });
    }

    // preserve reference metadata
    let reference = value.reference_meta();

    match value.tag() {
        ValueTag::ManagedReference => {
            let Some(handle) = value.as_heap_handle() else {
                return Err(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                });
            };
            let base = handle.slot_index();
            let slot = base.saturating_add(offset);
            let slot = u32::try_from(slot).map_err(|_| Error::InvalidPointerType {
                actual: format!("{value:?}"),
            })?;
            let handle = HeapHandle::with_slot(handle.id(), slot);
            Ok(Value::managed_reference_with_meta(handle, reference))
        }
        ValueTag::RawPointer => {
            let Some(pointer) = value.as_raw_pointer() else {
                return Err(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                });
            };
            let base = pointer.slot_index();
            let slot = base.saturating_add(offset);
            let slot = u32::try_from(slot).map_err(|_| Error::InvalidPointerType {
                actual: format!("{value:?}"),
            })?;
            let pointer = RawPointer::with_slot(pointer.id(), slot);
            Ok(Value::raw_pointer_with_meta(pointer, reference))
        }
        ValueTag::StackPointer => {
            let Some(pointer) = value.as_stack_pointer() else {
                return Err(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                });
            };
            let slot = pointer.slot_offset.saturating_add(offset);
            let pointer = StackPointer::with_offset(pointer.frame_idx, pointer.slot, slot);
            Ok(Value::stack_pointer_with_meta(pointer, reference))
        }
        ValueTag::LocalPointer => {
            let Some(pointer) = value.as_local_pointer() else {
                return Err(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                });
            };
            let slot = pointer.slot_offset.saturating_add(offset);
            let pointer = LocalPointer::with_offset(pointer.frame_idx, pointer.local, slot);
            Ok(Value::local_pointer_with_meta(pointer, reference))
        }
        ValueTag::GlobalPointer => {
            let Some(pointer) = value.as_global_pointer() else {
                return Err(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                });
            };
            let slot = pointer.slot_offset.saturating_add(offset);
            Ok(Value::global_pointer_with_meta(pointer.id, slot, reference))
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{value:?}"),
        }),
    }
}

/// Reduction operators for vector/tensor reductions.
#[derive(Clone, Copy, Debug)]
enum ReduceOperator {
    Add,
    Multiply,
    Min,
    Max,
    And,
    Or,
    Xor,
}

impl From<mir::VectorReduceOperator> for ReduceOperator {
    fn from(value: mir::VectorReduceOperator) -> Self {
        match value {
            mir::VectorReduceOperator::Add => ReduceOperator::Add,
            mir::VectorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::VectorReduceOperator::Min => ReduceOperator::Min,
            mir::VectorReduceOperator::Max => ReduceOperator::Max,
            mir::VectorReduceOperator::And => ReduceOperator::And,
            mir::VectorReduceOperator::Or => ReduceOperator::Or,
            mir::VectorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

impl From<mir::TensorReduceOperator> for ReduceOperator {
    fn from(value: mir::TensorReduceOperator) -> Self {
        match value {
            mir::TensorReduceOperator::Add => ReduceOperator::Add,
            mir::TensorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::TensorReduceOperator::Min => ReduceOperator::Min,
            mir::TensorReduceOperator::Max => ReduceOperator::Max,
            mir::TensorReduceOperator::And => ReduceOperator::And,
            mir::TensorReduceOperator::Or => ReduceOperator::Or,
            mir::TensorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

/// Apply a reduction operator to two values.
fn apply_reduce_operator(op: ReduceOperator, a: Value, b: Value) -> Result<Value, Error> {
    let (a_tag, b_tag) = (a.tag(), b.tag());
    match op {
        ReduceOperator::Add => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.wrapping_add(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.wrapping_add(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av + bv))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av + bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible add operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Multiply => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.wrapping_mul(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.wrapping_mul(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av * bv))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av * bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible mul operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Min => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.min(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.min(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av.min(bv)))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av.min(bv)))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible min operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Max => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.max(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.max(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av.max(bv)))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av.max(bv)))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible max operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::And => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av & bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av & bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                Ok(Value::bool(a.raw_data() != 0 && b.raw_data() != 0))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible and operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Or => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av | bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av | bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                Ok(Value::bool(a.raw_data() != 0 || b.raw_data() != 0))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible or operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Xor => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av ^ bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av ^ bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                let av = a.raw_data() != 0;
                let bv = b.raw_data() != 0;
                Ok(Value::bool(av ^ bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible xor operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
    }
}

/// Validate reference address space against the pointer storage.
fn check_reference_address_space(
    state: &ThreadedState<'_, '_>,
    reference: ReferenceMeta,
    pointer: Value,
) -> Result<(), Error> {
    if !state
        .interpreter
        .isolate
        .options
        .checks
        .enforce_reference_kinds
    {
        return Ok(());
    }

    let address_space = reference.address_space();
    if matches!(address_space, ReferenceAddressSpace::Generic) {
        return Ok(());
    }

    if !address_space.is_supported_by_vm() {
        return Err(Error::UnsupportedAddressSpace {
            address_space: address_space.label().to_string(),
        });
    }

    let actual_space = match pointer.tag() {
        ValueTag::StackPointer => ReferenceAddressSpace::Stack,
        ValueTag::LocalPointer => ReferenceAddressSpace::Stack,
        ValueTag::GlobalPointer => ReferenceAddressSpace::Global,
        ValueTag::ManagedReference | ValueTag::RawPointer => ReferenceAddressSpace::Heap,
        _ => {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            });
        }
    };

    let is_match = match address_space {
        ReferenceAddressSpace::Stack => matches!(actual_space, ReferenceAddressSpace::Stack),
        ReferenceAddressSpace::Global | ReferenceAddressSpace::Constant => {
            matches!(actual_space, ReferenceAddressSpace::Global)
        }
        ReferenceAddressSpace::Heap => matches!(actual_space, ReferenceAddressSpace::Heap),
        ReferenceAddressSpace::Generic => true,
        ReferenceAddressSpace::Shared
        | ReferenceAddressSpace::Local
        | ReferenceAddressSpace::Target => false,
    };

    if !is_match {
        return Err(Error::InvalidAddressSpace {
            expected: address_space.label().to_string(),
            actual: actual_space.label().to_string(),
        });
    }

    Ok(())
}

/// Validate reference mutability for stores.
fn check_reference_mutability(
    state: &ThreadedState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    if !state
        .interpreter
        .isolate
        .options
        .checks
        .enforce_reference_mutability
    {
        return Ok(());
    }

    let Some(mutability) = reference.mutability() else {
        return Ok(());
    };

    if matches!(mutability, mir::Mutability::Immutable) {
        return Err(Error::ImmutableReferenceWrite {
            reference: reference_label(reference),
        });
    }

    Ok(())
}

mod aggregate;
mod call;
mod cast;
mod control;
mod dispatch;
mod intrinsic;
mod memory;
mod tensor;
mod vector;

pub(crate) use aggregate::*;
pub(crate) use call::*;
pub(crate) use cast::*;
pub(crate) use control::*;
pub(crate) use dispatch::*;
pub(crate) use intrinsic::*;
pub(crate) use memory::*;
pub(crate) use tensor::*;
pub(crate) use vector::*;
