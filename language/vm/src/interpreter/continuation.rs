use {destack_engine as engine, destack_mir as mir};

use super::Frame;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::execute::{
    frame_pointer_value, frame_pointer_value_with_meta, stack_pointer_value,
    stack_pointer_value_with_meta, static_pointer_value, static_pointer_value_with_meta,
};
use crate::interpreter::StackAllocation;
use crate::module::{FunctionTable, Module, repr_type};
use crate::snapshot::ContinuationImage;
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;
use crate::telemetry::Statistics;
use crate::{FramePointer, ReferenceMeta, RootVisitor, StackPointer, Value, ValueTag};
use destack_heap::Heap;

/// Continuation snapshot captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The isolate id used to validate the continuation.
    pub(crate) isolate_id: engine::IsolateId,
    /// The frame stack for the suspended execution.
    pub(crate) stack: Vec<Frame>,
    /// The frame index to resume execution in.
    pub(crate) resume_frame_index: usize,
    /// The resume point for this continuation.
    pub(crate) resume_point: engine::ResumePointId,
    /// The statistics captured for the suspended execution.
    pub(crate) statistics: Statistics,
    /// The instruction profile state for the suspended execution.
    #[cfg(feature = "stats")]
    pub(crate) instruction_profile: Option<InstructionProfile>,
}

impl Continuation {
    /// Clone this continuation for multi-shot resumption.
    pub fn clone_for_fork(&self) -> Self {
        // clone all stack state for the fork
        let stack = self.stack.iter().map(Frame::clone_for_fork).collect();
        let statistics = self.statistics.clone();
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.clone();

        Self {
            isolate_id: self.isolate_id,
            stack,
            resume_frame_index: self.resume_frame_index,
            resume_point: self.resume_point,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }

    /// Capture one immutable continuation image.
    pub fn image(&self, module: &Module) -> RuntimeResult<ContinuationImage> {
        // capture the current stack state
        let frames = self
            .stack
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| {
                capture_continuation_frame(
                    module,
                    frame,
                    frame_index,
                    self.resume_frame_index,
                    self.resume_point,
                )
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ContinuationImage {
            isolate_id: self.isolate_id,
            frames,
            stats: (&self.statistics).into(),
        })
    }

    /// Visit heap roots referenced by this continuation.
    pub(crate) fn visit_roots(
        &self,
        module: &Module,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        // collect roots from captured frames
        for frame in &self.stack {
            frame.visit_roots(module, roots)?;
        }

        Ok(())
    }

    /// Stabilize every local heap reference before this continuation escapes.
    pub(crate) fn stabilize(&mut self, module: &Module, heap: &mut Heap) -> RuntimeResult<()> {
        let mut references = Vec::new();

        for frame in &mut self.stack {
            frame
                .collect_escape_heap_references(module, &mut references)
                .map_err(RuntimeError::new)?;
        }

        references.sort_unstable();
        references.dedup();

        let mut replacements = std::collections::BTreeMap::new();

        for reference in references {
            let stabilized = heap
                .stabilize_heap(reference)
                .map_err(|error| RuntimeError::new(Error::from(error)))?;
            replacements.insert(reference, stabilized);
        }

        for frame in &mut self.stack {
            frame
                .rewrite_escape_heap_references(module, heap, &replacements)
                .map_err(RuntimeError::new)?;
        }

        Ok(())
    }

    /// Visit heap roots from one captured continuation image.
    pub(crate) fn visit_image_roots(
        image: &ContinuationImage,
        module: &Module,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        // captured frames
        for frame in &image.frames {
            visit_frame_image_roots(frame, module, roots)?;
        }

        Ok(())
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        module: &Module,
        functions: &FunctionTable,
    ) -> RuntimeResult<Self> {
        // reject empty continuations
        if image.frames.is_empty() {
            return Err(RuntimeError::new(Error::InvalidContinuation));
        }

        // rebuild the captured stack state
        let mut stack = Vec::with_capacity(image.frames.len());
        for frame in &image.frames {
            let frame = restore_frame_image(frame, module, functions, stack.len())?;
            stack.push(frame);
        }

        // rebuild the yield metadata from the innermost frame image
        let resume_frame_index = image.frames.len() - 1;
        let resume_point = image
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?
            .resume_point;

        Ok(Self {
            isolate_id: image.isolate_id,
            stack,
            resume_frame_index,
            resume_point,
            statistics: image.stats.clone().into(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        })
    }
}

/// Visit heap roots from one captured frame image.
fn visit_frame_image_roots(
    image: &engine::FrameImage,
    module: &Module,
    roots: &mut impl RootVisitor,
) -> Result<(), Error> {
    // captured slots
    for value in &image.slots {
        match value {
            engine::MaterializedValue::HeapReference(reference) => {
                roots.push_heap(*reference);
            }
            engine::MaterializedValue::SharedHeapReference(reference) => {
                roots.push_shared(*reference);
            }
            _ => {}
        }
    }

    // captured frame-owned storage
    for allocation in image.allocations.iter().flatten() {
        Frame::visit_storage_roots(module, allocation.storage_type, &allocation.bytes, roots)?;
    }

    Ok(())
}

/// Visit heap roots from one materialized boundary value.
pub(crate) fn visit_materialized_value_roots(
    value: &engine::MaterializedValue,
    _module: &Module,
    roots: &mut impl RootVisitor,
) -> Result<(), Error> {
    if let engine::MaterializedValue::HeapReference(reference) = value {
        roots.push_heap(*reference);
    }

    if let engine::MaterializedValue::SharedHeapReference(reference) = value {
        roots.push_shared(*reference);
    }

    Ok(())
}

/// Stabilize every local heap reference in one materialized boundary value.
pub(crate) fn stabilize_materialized_value(
    _module: &Module,
    heap: &mut Heap,
    value: &mut engine::MaterializedValue,
) -> RuntimeResult<()> {
    let engine::MaterializedValue::HeapReference(reference) = value else {
        return Ok(());
    };

    if !reference.is_null() {
        *reference = heap
            .stabilize_heap(*reference)
            .map_err(|error| RuntimeError::new(Error::from(error)))?;
    }

    Ok(())
}

/// Resolve the captured resume point and materialization frame for one suspended frame.
pub(crate) fn frame_capture_materialization<'a>(
    module: &'a Module,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<(engine::ResumePointId, &'a engine::MaterializationFrame)> {
    // resolve the captured resume point first
    let resume_point =
        captured_resume_point(module, frame, frame_index, resume_frame_index, resume_point)?;

    // resolve the corresponding materialization frame
    let materialization_frame = module.materialization_frame(resume_point).ok_or_else(|| {
        RuntimeError::new(Error::InvariantViolation {
            context: format!("missing materialization frame for resume point: {resume_point:?}"),
        })
    })?;

    // validate the current fixed-slot VM contract
    debug_assert_eq!(
        materialization_frame.resume_point, resume_point,
        "vm safepoint materialization should target the captured resume point"
    );
    debug_assert_eq!(
        materialization_frame.frame_layout, frame.frame_layout,
        "vm safepoint materialization should target the captured frame layout"
    );

    Ok((resume_point, materialization_frame))
}

/// Resolve the captured resume point for one suspended frame.
fn captured_resume_point(
    module: &Module,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<engine::ResumePointId> {
    // the yielded frame already carries the exact captured resume point
    if frame_index == resume_frame_index {
        return Ok(resume_point);
    }

    // older frames resume from their current lowered position
    module
        .resume_point_for_position(frame.function, frame.current_block, frame.resume_pc as u32)
        .ok_or_else(|| {
            RuntimeError::new(Error::InvariantViolation {
                context: format!(
                    "missing generic resume point for frame position: {:?} {:?} {}",
                    frame.function, frame.current_block, frame.resume_pc
                ),
            })
        })
}

/// Capture one durable frame from one live frame.
fn capture_continuation_frame(
    module: &Module,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<engine::FrameImage> {
    let layout = module
        .frame_layout_by_id(frame.frame_layout)
        .ok_or_else(|| {
            RuntimeError::new(Error::InvariantViolation {
                context: format!("missing frame layout for id: {:?}", frame.frame_layout),
            })
        })?;

    let (resume_point, materialization_frame) = frame_capture_materialization(
        module,
        frame,
        frame_index,
        resume_frame_index,
        resume_point,
    )?;

    let slots = capture_materialized_slots(layout, materialization_frame, frame, frame_index)?;

    Ok(engine::FrameImage {
        frame_layout: frame.frame_layout,
        resume_point,
        transfer: frame.transfer.clone(),
        slots,
        allocations: frame
            .stack_allocations
            .iter()
            .map(|allocation| {
                allocation
                    .as_ref()
                    .map(|allocation| engine::AllocationImage {
                        bytes: allocation.clone_bytes(),
                        storage_type: allocation.storage_type(),
                    })
            })
            .collect(),
    })
}

/// Restore one live frame from one logical frame image.
fn restore_frame_image(
    image: &engine::FrameImage,
    module: &Module,
    functions: &FunctionTable,
    frame_index: usize,
) -> RuntimeResult<Frame> {
    let layout = module
        .frame_layout_by_id(image.frame_layout)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let resume_point = module
        .resume_point(image.resume_point)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let materialization_frame = module
        .materialization_frame(image.resume_point)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    if resume_point.frame_layout != image.frame_layout || resume_point.function != layout.function {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    if materialization_frame.frame_layout != image.frame_layout
        || materialization_frame.resume_point != image.resume_point
    {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    if image.slots.len() != layout.slots.len() {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    for (slot, materialization) in image.slots.iter().zip(&materialization_frame.slots) {
        if !slot_matches_materialization(slot, materialization) {
            return Err(RuntimeError::new(Error::InvalidContinuation));
        }
    }

    let function_ptr = functions.get_ptr_for(layout.function).ok_or_else(|| {
        RuntimeError::new(Error::UndefinedFunction {
            function: layout.function,
        })
    })?;
    let function = unsafe { function_ptr.as_ref() };
    let entry_block_id = function.blocks[function.entry as usize].mir_block;

    let block_index = function
        .blocks
        .iter()
        .position(|block| block.mir_block == resume_point.block)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let block = function
        .blocks
        .get(block_index)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    // restore the environment slot
    let environment = if let Some(slot) = layout.environment_slot {
        restore_image_slot(module, layout, image, slot, frame_index)?
    } else {
        Value::VOID
    };

    let mut frame = Frame::new(
        image.frame_layout,
        layout.function,
        function_ptr,
        std::ptr::NonNull::from(block),
        entry_block_id,
        block_index,
        layout.value_slots.len(),
        layout.local_slots.len(),
        environment,
    );

    restore_slot_range(
        module,
        layout,
        image,
        layout.value_slots.clone(),
        frame_index,
        frame.slots_mut(),
    )?;

    restore_slot_range(
        module,
        layout,
        image,
        layout.local_slots.clone(),
        frame_index,
        frame.slots_mut(),
    )?;

    frame.current_block = resume_point.block;
    frame.block_index = block_index;
    frame.block_ptr = std::ptr::NonNull::from(block);
    frame.resume_pc = resume_point.instruction_offset as usize;
    frame.transfer = image.transfer.clone();
    frame.stack_allocations = image
        .allocations
        .iter()
        .map(|allocation| {
            allocation.as_ref().map(|allocation| {
                StackAllocation::from_bytes(allocation.bytes.clone(), allocation.storage_type)
            })
        })
        .collect();

    Ok(frame)
}

/// Restore one logical slot range into one live stack.
fn restore_slot_range(
    module: &Module,
    layout: &engine::FrameLayout,
    image: &engine::FrameImage,
    slots: std::ops::Range<u32>,
    frame_index: usize,
    frame_slots: &mut [Value],
) -> RuntimeResult<()> {
    for slot in slots {
        let value = restore_image_slot(module, layout, image, slot, frame_index)?;
        frame_slots[slot as usize] = value;
    }

    Ok(())
}

/// Restore one logical slot by layout index.
fn restore_image_slot(
    module: &Module,
    layout: &engine::FrameLayout,
    image: &engine::FrameImage,
    slot: u32,
    frame_index: usize,
) -> RuntimeResult<Value> {
    let value = image
        .slots
        .get(slot as usize)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let layout_slot = layout
        .slots
        .get(slot as usize)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    restore_slot_value(module, layout_slot.ty, value, frame_index)
}

/// Capture one logical slot value from one runtime value.
fn capture_slot_value(
    value: Value,
    frame_index: usize,
) -> RuntimeResult<engine::MaterializedValue> {
    Ok(match value.tag() {
        ValueTag::Void => engine::MaterializedValue::Void,
        ValueTag::Bool => engine::MaterializedValue::Bool(
            value
                .as_bool()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::Int => {
            let (value, width) = value
                .as_int_with_width()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
            engine::MaterializedValue::Int { value, width }
        }
        ValueTag::UInt => {
            let (value, width) = value
                .as_uint_with_width()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
            engine::MaterializedValue::UInt { value, width }
        }
        ValueTag::Float32 => engine::MaterializedValue::Float32 {
            bits: value
                .as_float32()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?
                .to_bits(),
        },
        ValueTag::Float64 => engine::MaterializedValue::Float64 {
            bits: value
                .as_float64()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?
                .to_bits(),
        },
        ValueTag::Char => engine::MaterializedValue::Char(
            value
                .as_char()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::HeapReference => engine::MaterializedValue::HeapReference(
            value
                .as_heap_reference()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::SharedHeapReference => engine::MaterializedValue::SharedHeapReference(
            value
                .as_shared_heap_reference()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::RawPointer => engine::MaterializedValue::RawPointer(
            value
                .as_raw_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::SharedRawPointer => engine::MaterializedValue::SharedRawPointer(
            value
                .as_shared_raw_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
        ValueTag::StackPointer => {
            let pointer = value
                .as_stack_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

            if pointer.frame_idx != frame_index {
                return Err(RuntimeError::new(Error::InvariantViolation {
                    context: format!(
                        "captured stack pointer escapes its frame: pointer_frame={}, frame_index={frame_index}",
                        pointer.frame_idx,
                    ),
                }));
            }

            let allocation = u32::try_from(pointer.slot).map_err(|_| {
                RuntimeError::new(Error::InvariantViolation {
                    context: "stack allocation index exceeds uint32".to_string(),
                })
            })?;
            let byte_offset = u32::try_from(pointer.byte_offset).map_err(|_| {
                RuntimeError::new(Error::InvariantViolation {
                    context: "stack allocation offset exceeds uint32".to_string(),
                })
            })?;

            engine::MaterializedValue::FrameAddress(engine::FrameAddress::Allocation {
                allocation,
                byte_offset,
            })
        }
        ValueTag::FramePointer => {
            let pointer = value
                .as_frame_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

            if pointer.frame_idx != frame_index {
                return Err(RuntimeError::new(Error::InvariantViolation {
                    context: format!(
                        "captured frame pointer escapes its frame: pointer_frame={}, frame_index={frame_index}",
                        pointer.frame_idx,
                    ),
                }));
            }

            let local = u32::try_from(pointer.slot).map_err(|_| {
                RuntimeError::new(Error::InvariantViolation {
                    context: "frame pointer index exceeds uint32".to_string(),
                })
            })?;
            let byte_offset = u32::try_from(pointer.byte_offset).map_err(|_| {
                RuntimeError::new(Error::InvariantViolation {
                    context: "frame pointer offset exceeds uint32".to_string(),
                })
            })?;

            engine::MaterializedValue::FrameAddress(engine::FrameAddress::Local {
                local,
                byte_offset,
            })
        }
        ValueTag::StaticPointer => {
            let pointer = value
                .as_static_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
            let byte_offset = u32::try_from(pointer.byte_offset).map_err(|_| {
                RuntimeError::new(Error::InvariantViolation {
                    context: "static pointer offset exceeds uint32".to_string(),
                })
            })?;

            engine::MaterializedValue::StaticAddress(engine::StaticAddress {
                global: pointer.id,
                byte_offset,
            })
        }
        ValueTag::FunctionPointer => engine::MaterializedValue::Function(
            value
                .as_function_pointer()
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?,
        ),
    })
}

/// Restore one runtime value from one logical slot value.
fn restore_slot_value(
    module: &Module,
    slot_type: mir::LocalNodeId<mir::Type>,
    value: &engine::MaterializedValue,
    frame_index: usize,
) -> RuntimeResult<Value> {
    let reference_meta = reference_meta_for_slot(module, slot_type)?;

    Ok(match value {
        engine::MaterializedValue::Undefined => Value::VOID,
        engine::MaterializedValue::Void => Value::VOID,
        engine::MaterializedValue::Bool(value) => Value::bool(*value),
        engine::MaterializedValue::Int { value, width } => Value::int(*value, *width),
        engine::MaterializedValue::UInt { value, width } => Value::uint(*value, *width),
        engine::MaterializedValue::Float32 { bits } => Value::float32(f32::from_bits(*bits)),
        engine::MaterializedValue::Float64 { bits } => Value::float64(f64::from_bits(*bits)),
        engine::MaterializedValue::Char(value) => Value::char(*value),
        engine::MaterializedValue::HeapReference(reference) => {
            if let Some(reference_meta) = reference_meta {
                Value::heap_reference_with_meta(*reference, reference_meta)
            } else {
                Value::heap_reference(*reference)
            }
        }
        engine::MaterializedValue::SharedHeapReference(reference) => {
            if let Some(reference_meta) = reference_meta {
                Value::shared_heap_reference_with_meta(*reference, reference_meta)
            } else {
                Value::shared_heap_reference(*reference)
            }
        }
        engine::MaterializedValue::RawPointer(pointer) => {
            if let Some(reference_meta) = reference_meta {
                Value::raw_pointer_with_meta(*pointer, reference_meta)
            } else {
                Value::raw_pointer(*pointer)
            }
        }
        engine::MaterializedValue::SharedRawPointer(pointer) => {
            if let Some(reference_meta) = reference_meta {
                Value::shared_raw_pointer_with_meta(*pointer, reference_meta)
            } else {
                Value::shared_raw_pointer(*pointer)
            }
        }
        engine::MaterializedValue::FrameAddress(engine::FrameAddress::Allocation {
            allocation,
            byte_offset,
        }) => {
            let pointer =
                StackPointer::with_offset(frame_index, *allocation as usize, *byte_offset as usize);

            if let Some(reference_meta) = reference_meta {
                stack_pointer_value_with_meta(pointer, reference_meta).map_err(RuntimeError::new)?
            } else {
                stack_pointer_value(pointer).map_err(RuntimeError::new)?
            }
        }
        engine::MaterializedValue::FrameAddress(engine::FrameAddress::Local {
            local,
            byte_offset,
        }) => {
            let pointer =
                FramePointer::with_offset(frame_index, *local as usize, *byte_offset as usize);

            if let Some(reference_meta) = reference_meta {
                frame_pointer_value_with_meta(pointer, reference_meta).map_err(RuntimeError::new)?
            } else {
                frame_pointer_value(pointer).map_err(RuntimeError::new)?
            }
        }
        engine::MaterializedValue::StaticAddress(pointer) => {
            let byte_offset = usize::try_from(pointer.byte_offset)
                .map_err(|_| RuntimeError::new(Error::InvalidContinuation))?;

            if let Some(reference_meta) = reference_meta {
                static_pointer_value_with_meta(pointer.global, byte_offset, reference_meta)
                    .map_err(RuntimeError::new)?
            } else {
                static_pointer_value(pointer.global, byte_offset).map_err(RuntimeError::new)?
            }
        }
        engine::MaterializedValue::Function(function) => Value::function_pointer(*function),
    })
}

/// Return the reference metadata implied by one logical slot type.
fn reference_meta_for_slot(
    module: &Module,
    slot_type: mir::LocalNodeId<mir::Type>,
) -> RuntimeResult<Option<ReferenceMeta>> {
    let repr_ty = repr_type(&module.tree, slot_type);

    Ok(match module.tree.get(repr_ty) {
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            is_nullable,
            ..
        }
        | mir::Type::TensorView {
            kind,
            address_space,
            mutability,
            is_nullable,
            ..
        } => Some(ReferenceMeta::new(
            *kind,
            address_space.clone(),
            *mutability,
            *is_nullable,
        )),

        // callables stay boxed in the vm without extra packed reference metadata
        mir::Type::Callable { .. } => None,
        _ => None,
    })
}

/// Capture one frame image using one materialization frame.
fn capture_materialized_slots(
    layout: &engine::FrameLayout,
    materialization_frame: &engine::MaterializationFrame,
    frame: &Frame,
    frame_index: usize,
) -> RuntimeResult<Vec<engine::MaterializedValue>> {
    let mut slots = Vec::with_capacity(layout.slots.len());

    for (slot_index, materialization) in materialization_frame.slots.iter().enumerate() {
        let slot = match materialization {
            engine::MaterializationValue::FrameSlot(layout_slot) => {
                let value = frame.slot_value(*layout_slot).ok_or_else(|| {
                    RuntimeError::new(Error::InvariantViolation {
                        context: format!(
                            "missing frame slot during continuation capture: {:?} {slot_index}",
                            frame.function,
                        ),
                    })
                })?;
                capture_slot_value(value, frame_index)?
            }
            engine::MaterializationValue::Undefined => engine::MaterializedValue::Undefined,
            engine::MaterializationValue::Location(location) => {
                return Err(RuntimeError::new(Error::InvariantViolation {
                    context: format!("vm should not materialize native location: {location:?}"),
                }));
            }
        };

        slots.push(slot);
    }

    Ok(slots)
}

/// Return whether one captured slot is valid for one materialization recipe.
fn slot_matches_materialization(
    slot: &engine::MaterializedValue,
    materialization: &engine::MaterializationValue,
) -> bool {
    match materialization {
        engine::MaterializationValue::Undefined => {
            matches!(slot, engine::MaterializedValue::Undefined)
        }
        engine::MaterializationValue::FrameSlot(_) | engine::MaterializationValue::Location(_) => {
            !matches!(slot, engine::MaterializedValue::Undefined)
        }
    }
}
