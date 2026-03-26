use destack_engine as engine;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{Executable, FunctionTable};
use crate::interpreter::Frame;
use crate::snapshot::ContinuationImage;
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;
use crate::telemetry::Statistics;
use destack_heap::{ManagedReference, Value, ValueTag};

/// Resume state captured at a yield terminator.
#[derive(Debug, Clone)]
pub(crate) struct YieldState {
    /// Frame index to resume execution in.
    pub frame_index: usize,
    /// The semantic resume point for this yield.
    pub resume_point: engine::ResumePointId,
}

/// Continuation snapshot captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The isolate id used to validate the continuation.
    pub(crate) isolate_id: u64,
    /// The call stack for the suspended execution.
    pub(crate) call_stack: Vec<Frame>,
    /// The SSA value stack for the suspended execution.
    pub(crate) value_stack: Vec<Value>,
    /// The local variable stack for the suspended execution.
    pub(crate) local_stack: Vec<Value>,
    /// The resume state captured at the yield point.
    pub(crate) yield_state: YieldState,
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
        let call_stack = self.call_stack.iter().map(Frame::clone_for_fork).collect();
        let value_stack = self.value_stack.clone();
        let local_stack = self.local_stack.clone();
        let yield_state = self.yield_state.clone();
        let statistics = self.statistics.clone();
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.clone();

        Self {
            isolate_id: self.isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }

    /// Capture one immutable continuation image.
    pub fn image(&self, executable: &Executable) -> ContinuationImage {
        // capture the current stack state
        let frames = self
            .call_stack
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| {
                capture_continuation_frame(
                    executable,
                    frame,
                    &self.value_stack,
                    &self.local_stack,
                    frame_index,
                    &self.yield_state,
                )
            })
            .collect();

        ContinuationImage {
            isolate_id: self.isolate_id,
            frames,
            stats: (&self.statistics).into(),
        }
    }

    /// Collect managed heap roots referenced by this continuation.
    pub fn collect_roots(&self, executable: &Executable, roots: &mut Vec<ManagedReference>) {
        // collect roots from captured frames
        for frame in &self.call_stack {
            frame.collect_roots(executable, &self.value_stack, &self.local_stack, roots);
        }
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        executable: &Executable,
        functions: &FunctionTable,
    ) -> RuntimeResult<Self> {
        // reject empty continuations
        if image.frames.is_empty() {
            return Err(RuntimeError::new(Error::InvalidContinuation));
        }

        // rebuild the captured stack state
        let mut call_stack = Vec::with_capacity(image.frames.len());
        let mut value_stack = Vec::new();
        let mut local_stack = Vec::new();
        for frame in &image.frames {
            let frame = restore_frame_image(
                frame,
                executable,
                functions,
                &mut value_stack,
                &mut local_stack,
            )?;
            call_stack.push(frame);
        }

        // rebuild the yield metadata from the innermost frame image
        let yield_state = YieldState {
            frame_index: image.frames.len() - 1,
            resume_point: image.frames.last().unwrap().resume_point,
        };

        Ok(Self {
            isolate_id: image.isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics: image.stats.clone().into(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        })
    }
}

/// Capture one durable frame from one live frame.
fn capture_continuation_frame(
    executable: &Executable,
    frame: &Frame,
    value_stack: &[Value],
    local_stack: &[Value],
    frame_index: usize,
    yield_state: &YieldState,
) -> engine::FrameImage {
    let layout = executable
        .frame_layout_by_id(frame.frame_layout)
        .unwrap_or_else(|| panic!("missing frame layout for id: {:?}", frame.frame_layout));

    let resume_point = if frame_index == yield_state.frame_index {
        yield_state.resume_point
    } else {
        executable
            .resume_point_for_position(frame.function, frame.current_block, frame.resume_pc as u32)
            .unwrap_or_else(|| {
                panic!(
                    "missing generic resume point for frame position: {:?} {:?} {}",
                    frame.function, frame.current_block, frame.resume_pc
                )
            })
    };

    // validate the fixed-slot materialization contract for this resume point
    let safepoint = executable
        .safepoint_for_resume_point(resume_point)
        .unwrap_or_else(|| panic!("missing safepoint for resume point: {resume_point:?}"));
    let safepoint = executable
        .safepoint(safepoint)
        .unwrap_or_else(|| panic!("missing safepoint entry for id: {safepoint:?}"));
    let materialization_map = safepoint.materialization_map.unwrap_or_else(|| {
        panic!(
            "missing materialization map for safepoint: {:?}",
            safepoint.id
        )
    });
    let materialization_map = executable
        .materialization_map(materialization_map)
        .unwrap_or_else(|| {
            panic!("missing materialization map entry for id: {materialization_map:?}")
        });

    debug_assert_eq!(
        materialization_map.frames.len(),
        1,
        "vm safepoints should materialize one frame today"
    );
    debug_assert_eq!(
        materialization_map.frames[0].resume_point, resume_point,
        "vm safepoint materialization should target the captured resume point"
    );
    debug_assert_eq!(
        materialization_map.frames[0].frame_layout, frame.frame_layout,
        "vm safepoint materialization should target the captured frame layout"
    );
    let materialization_frame = &materialization_map.frames[0];

    let value_slice = &value_stack[frame.value_base..frame.value_base + frame.value_count];
    let local_slice = &local_stack[frame.local_base..frame.local_base + frame.local_count];
    let slots = capture_materialized_slots(
        layout,
        materialization_frame,
        frame,
        value_slice,
        local_slice,
    );

    debug_assert!(
        !frame.has_live_stack_allocations(),
        "stack-local allocations should not cross suspension"
    );

    engine::FrameImage {
        frame_layout: frame.frame_layout,
        resume_point,
        transfer: frame.transfer.clone(),
        slots,
    }
}

/// Restore one live frame from one logical frame image.
fn restore_frame_image(
    image: &engine::FrameImage,
    executable: &Executable,
    functions: &FunctionTable,
    value_stack: &mut Vec<Value>,
    local_stack: &mut Vec<Value>,
) -> RuntimeResult<Frame> {
    let layout = executable
        .frame_layout_by_id(image.frame_layout)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let resume_point = executable
        .resume_point(image.resume_point)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    // validate the fixed-slot materialization contract for this frame image
    let safepoint = executable
        .safepoint_for_resume_point(image.resume_point)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let safepoint = executable
        .safepoint(safepoint)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let materialization_map = safepoint
        .materialization_map
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let materialization_map = executable
        .materialization_map(materialization_map)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    if resume_point.frame_layout != image.frame_layout || resume_point.function != layout.function {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    if materialization_map.frames.len() != 1 {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    let materialization_frame = &materialization_map.frames[0];
    if materialization_frame.frame_layout != image.frame_layout
        || materialization_frame.resume_point != image.resume_point
    {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    if image.slots.len() != layout.slots.len() {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    for (slot, materialization_slot) in image.slots.iter().zip(&materialization_frame.slots) {
        if !slot_matches_materialization(slot, &materialization_slot.value) {
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

    let value_base = value_stack.len();
    let value_slice =
        &image.slots[layout.value_slots.start as usize..layout.value_slots.end as usize];
    for slot in value_slice {
        value_stack.push(restore_slot_value(slot));
    }

    let local_base = local_stack.len();
    let local_slice =
        &image.slots[layout.local_slots.start as usize..layout.local_slots.end as usize];
    for slot in local_slice {
        local_stack.push(restore_slot_value(slot));
    }

    let environment = if let Some(slot) = layout.environment_slot {
        let slot = image
            .slots
            .get(slot as usize)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
        restore_slot_value(slot)
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
        value_base,
        value_slice.len(),
        local_base,
        local_slice.len(),
        environment,
    );

    frame.current_block = resume_point.block;
    frame.block_index = block_index;
    frame.block_ptr = std::ptr::NonNull::from(block);
    frame.resume_pc = resume_point.instruction_offset as usize;
    frame.transfer = image.transfer.clone();
    frame.stack_values = Vec::new();

    Ok(frame)
}

/// Capture one logical slot value from one runtime value.
fn capture_slot_value(value: Value) -> engine::FrameValue {
    match value.tag() {
        ValueTag::Void => engine::FrameValue::Void,
        ValueTag::Bool => engine::FrameValue::Bool(value.as_bool().unwrap()),
        ValueTag::Int => {
            let (value, width) = value.as_int_with_width().unwrap();
            engine::FrameValue::Int { value, width }
        }
        ValueTag::UInt => {
            let (value, width) = value.as_uint_with_width().unwrap();
            engine::FrameValue::UInt { value, width }
        }
        ValueTag::Float32 => engine::FrameValue::Float32 {
            bits: value.as_float32().unwrap().to_bits(),
        },
        ValueTag::Float64 => engine::FrameValue::Float64 {
            bits: value.as_float64().unwrap().to_bits(),
        },
        ValueTag::Char => engine::FrameValue::Char(value.as_char().unwrap()),
        ValueTag::ManagedReference => engine::FrameValue::ManagedReference {
            reference: value.as_managed_reference().unwrap(),
            meta: value.reference_meta(),
        },
        ValueTag::RawPointer => engine::FrameValue::RawPointer {
            pointer: value.as_raw_pointer().unwrap(),
            meta: value.reference_meta(),
        },
        ValueTag::SharedPointer => engine::FrameValue::SharedPointer {
            pointer: value.as_shared_pointer().unwrap(),
            meta: value.reference_meta(),
        },
        ValueTag::StackPointer | ValueTag::LocalPointer => {
            panic!("frame-local pointers must not cross durable suspension")
        }
        ValueTag::GlobalPointer => {
            let pointer = value.as_global_pointer().unwrap();
            let slot_offset = u32::try_from(pointer.slot_offset)
                .unwrap_or_else(|_| panic!("global pointer offset exceeds u32"));

            engine::FrameValue::GlobalPointer {
                pointer: engine::GlobalPointer {
                    global: pointer.id,
                    slot_offset,
                },
                meta: value.reference_meta(),
            }
        }
        ValueTag::FunctionPointer => {
            engine::FrameValue::Function(value.as_function_pointer().unwrap())
        }
        ValueTag::Aggregate => engine::FrameValue::Aggregate(value.as_managed_reference().unwrap()),
        ValueTag::String => engine::FrameValue::String(value.as_managed_reference().unwrap()),
    }
}

/// Restore one runtime value from one logical slot value.
fn restore_slot_value(value: &engine::FrameValue) -> Value {
    match value {
        engine::FrameValue::Undefined => Value::VOID,
        engine::FrameValue::Void => Value::VOID,
        engine::FrameValue::Bool(value) => Value::bool(*value),
        engine::FrameValue::Int { value, width } => Value::int(*value, *width),
        engine::FrameValue::UInt { value, width } => Value::uint(*value, *width),
        engine::FrameValue::Float32 { bits } => Value::float32(f32::from_bits(*bits)),
        engine::FrameValue::Float64 { bits } => Value::float64(f64::from_bits(*bits)),
        engine::FrameValue::Char(value) => Value::char(*value),
        engine::FrameValue::ManagedReference { reference, meta } => {
            Value::managed_reference(*reference).with_reference_meta(*meta)
        }
        engine::FrameValue::RawPointer { pointer, meta } => {
            Value::raw_pointer(*pointer).with_reference_meta(*meta)
        }
        engine::FrameValue::SharedPointer { pointer, meta } => {
            Value::shared_pointer(*pointer).with_reference_meta(*meta)
        }
        engine::FrameValue::GlobalPointer { pointer, meta } => {
            let slot_offset = usize::try_from(pointer.slot_offset)
                .unwrap_or_else(|_| panic!("global pointer offset exceeds usize"));
            Value::global_pointer_with_offset(pointer.global, slot_offset)
                .with_reference_meta(*meta)
        }
        engine::FrameValue::Function(function) => Value::function_pointer(*function),
        engine::FrameValue::Aggregate(handle) => Value::aggregate(*handle),
        engine::FrameValue::String(handle) => Value::string(*handle),
    }
}

/// Capture one frame image using one materialization frame.
fn capture_materialized_slots(
    layout: &engine::FrameLayout,
    materialization_frame: &engine::MaterializationFrame,
    frame: &Frame,
    value_slice: &[Value],
    local_slice: &[Value],
) -> Vec<engine::FrameValue> {
    let mut slots = Vec::with_capacity(layout.slots.len());

    for materialization_slot in &materialization_frame.slots {
        let slot = match &materialization_slot.value {
            engine::MaterializationValue::FrameSlot(slot_index) => {
                let value = frame
                    .slot_value(value_slice, local_slice, *slot_index)
                    .unwrap_or_else(|| {
                        panic!(
                            "missing frame slot during continuation capture: {:?} {slot_index}",
                            frame.function
                        )
                    });
                capture_slot_value(value)
            }
            engine::MaterializationValue::Undefined => engine::FrameValue::Undefined,
            engine::MaterializationValue::Location(location) => {
                panic!("vm should not materialize native location: {location:?}")
            }
        };

        slots.push(slot);
    }

    slots
}

/// Return whether one captured slot is valid for one materialization recipe.
fn slot_matches_materialization(
    slot: &engine::FrameValue,
    materialization: &engine::MaterializationValue,
) -> bool {
    match materialization {
        engine::MaterializationValue::Undefined => matches!(slot, engine::FrameValue::Undefined),
        engine::MaterializationValue::FrameSlot(_) | engine::MaterializationValue::Location(_) => {
            !matches!(slot, engine::FrameValue::Undefined)
        }
    }
}
