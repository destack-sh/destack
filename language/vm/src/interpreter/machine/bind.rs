use destack_heap::Value;
use destack_mir as mir;
use smallvec::SmallVec;

use super::super::state::Frame;
use crate::executable::{ArgumentRange, CopyPair, CopyRange, INVALID_VALUE_ID};

/// The contiguous copy length where a small loop beats bulk copy setup.
const CONTIGUOUS_COPY_THRESHOLD: usize = 8;

/// Copy argument values from one frame into another.
pub(crate) fn copy_values_between_frames(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) {
    // resolve the argument and parameter slices
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    // copy contiguous ranges directly when both sides line up
    if let (Some((src_start, src_len)), Some((dest_start, dest_len))) =
        (arguments.contiguous_range(), params.contiguous_range())
    {
        // require equal contiguous lengths before using bulk copy
        if src_len == dest_len {
            let src_index = source_frame.value_base + src_start as usize;
            let dest_index = dest_frame.value_base + dest_start as usize;

            debug_assert!(
                (src_start as usize) + src_len <= source_frame.value_count,
                "ssa value out of bounds: {src_start}"
            );
            debug_assert!(
                (dest_start as usize) + src_len <= dest_frame.value_count,
                "ssa value out of bounds: {dest_start}"
            );

            let values_ptr = values.as_mut_ptr();
            unsafe {
                if std::ptr::eq(source_frame, dest_frame) {
                    std::ptr::copy(
                        values_ptr.add(src_index),
                        values_ptr.add(dest_index),
                        src_len,
                    );
                } else {
                    std::ptr::copy_nonoverlapping(
                        values_ptr.add(src_index),
                        values_ptr.add(dest_index),
                        src_len,
                    );
                }
            }
            return;
        }
    }

    // use a raw pointer for the general path
    let values_ptr = values.as_mut_ptr();

    // fill the destination parameters from the provided arguments
    for (index, param) in param_slice.iter().enumerate() {
        let value = if let Some(argument) = argument_slice.get(index) {
            let arg_index = source_frame.value_base + argument.0 as usize;
            debug_assert!(
                (argument.0 as usize) < source_frame.value_count,
                "ssa value out of bounds: {argument:?}"
            );

            unsafe { *values_ptr.add(arg_index) }
        } else {
            Value::VOID
        };

        let dest_index = dest_frame.value_base + param.0 as usize;
        debug_assert!(
            (param.0 as usize) < dest_frame.value_count,
            "ssa value out of bounds: {param:?}"
        );

        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}

/// Copy values between frames using one precomputed plan.
pub(crate) fn copy_values_with_plan(
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) {
    // copy contiguous plans directly when they line up
    if let Some((src_start, dest_start, len)) = copies.contiguous_plan() {
        let src_index = source_frame.value_base + src_start as usize;
        let dest_index = dest_frame.value_base + dest_start as usize;

        debug_assert!(
            (src_start as usize) + len <= source_frame.value_count,
            "ssa value out of bounds: {src_start}"
        );
        debug_assert!(
            (dest_start as usize) + len <= dest_frame.value_count,
            "ssa value out of bounds: {dest_start}"
        );

        let values_ptr = values.as_mut_ptr();
        unsafe {
            if std::ptr::eq(source_frame, dest_frame) {
                std::ptr::copy(values_ptr.add(src_index), values_ptr.add(dest_index), len);
            } else {
                std::ptr::copy_nonoverlapping(
                    values_ptr.add(src_index),
                    values_ptr.add(dest_index),
                    len,
                );
            }
        }
        return;
    }

    // apply each logical copy pair
    let values_ptr = values.as_mut_ptr();
    let pairs = copies.slice(copy_pool);

    for pair in pairs {
        let dest_index = dest_frame.value_base + pair.dest as usize;
        debug_assert!(
            (pair.dest as usize) < dest_frame.value_count,
            "ssa value out of bounds: {}",
            pair.dest
        );

        let value = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_index = source_frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < source_frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );

            unsafe { *values_ptr.add(src_index) }
        };

        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}

/// Collect argument values from one frame range.
pub(crate) fn collect_argument_values_range(
    values: &[Value],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> SmallVec<[Value; 16]> {
    // copy contiguous argument ranges directly into the output buffer
    if let Some((start, len)) = arguments.contiguous_range() {
        debug_assert!(
            (start as usize) + len <= frame.value_count,
            "ssa value out of bounds for contiguous args"
        );

        let mut arguments = SmallVec::with_capacity(len);

        if len <= CONTIGUOUS_COPY_THRESHOLD {
            let start_index = frame.value_base + start as usize;
            let values_ptr = values.as_ptr();

            for offset in 0..len {
                unsafe {
                    arguments.push(*values_ptr.add(start_index + offset));
                }
            }

            return arguments;
        }

        unsafe {
            arguments.set_len(len);
            let src_index = frame.value_base + start as usize;
            std::ptr::copy_nonoverlapping(
                values.as_ptr().add(src_index),
                arguments.as_mut_ptr(),
                len,
            );
        }

        return arguments;
    }

    // gather sparse arguments one by one
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());

    for argument in argument_slice {
        let index = frame.value_base + argument.0 as usize;
        debug_assert!(
            (argument.0 as usize) < frame.value_count,
            "ssa value out of bounds: {argument:?}"
        );

        let value = unsafe { *values.get_unchecked(index) };
        collected_arguments.push(value);
    }

    collected_arguments
}

/// Collect argument values from one logical copy plan.
pub(crate) fn collect_argument_values_from_copies(
    values: &[Value],
    frame: &Frame,
    copy_pool: &[CopyPair],
    copies: CopyRange,
) -> SmallVec<[Value; 16]> {
    // copy contiguous argument plans directly into the output buffer
    if let Some((src_start, _dest_start, len)) = copies.contiguous_plan() {
        debug_assert!(
            (src_start as usize) + len <= frame.value_count,
            "ssa value out of bounds for contiguous args"
        );

        let mut arguments = SmallVec::with_capacity(len);

        if len <= CONTIGUOUS_COPY_THRESHOLD {
            let start_index = frame.value_base + src_start as usize;
            let values_ptr = values.as_ptr();

            for offset in 0..len {
                unsafe {
                    arguments.push(*values_ptr.add(start_index + offset));
                }
            }

            return arguments;
        }

        unsafe {
            arguments.set_len(len);
            let src_index = frame.value_base + src_start as usize;
            std::ptr::copy_nonoverlapping(
                values.as_ptr().add(src_index),
                arguments.as_mut_ptr(),
                len,
            );
        }

        return arguments;
    }

    // gather sparse argument values one by one
    let pairs = copies.slice(copy_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    let values_ptr = values.as_ptr();

    for pair in pairs {
        let value = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_index = frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );

            unsafe { *values_ptr.add(src_index) }
        };

        arguments.push(value);
    }

    arguments
}

/// Bind argument values to parameter slots in one frame.
pub(crate) fn bind_parameters_from_values(
    values: &mut [Value],
    frame: &Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[Value],
) {
    // resolve the parameter slice first
    let param_slice = params.slice(param_pool);

    // copy contiguous parameter ranges directly when possible
    if let Some((dest_start, len)) = params.contiguous_range() {
        // require one argument per contiguous destination slot
        if len == arguments.len() {
            let dest_index = frame.value_base + dest_start as usize;
            debug_assert!(
                (dest_start as usize) + len <= frame.value_count,
                "ssa value out of bounds: {dest_start}"
            );

            let values_ptr = values.as_mut_ptr();
            unsafe {
                std::ptr::copy_nonoverlapping(arguments.as_ptr(), values_ptr.add(dest_index), len);
            }
            return;
        }
    }

    // fill the destination parameters from the provided arguments
    let values_ptr = values.as_mut_ptr();

    for (index, param) in param_slice.iter().enumerate() {
        let value = arguments.get(index).copied().unwrap_or(Value::VOID);
        let dest_index = frame.value_base + param.0 as usize;
        debug_assert!(
            (param.0 as usize) < frame.value_count,
            "ssa value out of bounds: {param:?}"
        );

        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}
