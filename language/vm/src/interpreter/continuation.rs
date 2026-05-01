use destack_engine as engine;

use super::{Frame, Stack};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::options::IsolateOptions;
use crate::program::{FunctionTable, Program};
use crate::snapshot::ContinuationImage;
use crate::{RootSink, Word};
use destack_heap::{HeapResult, RootSlot};

/// Continuation snapshot captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The engine id used to validate the continuation.
    pub(crate) isolate_id: engine::EngineId,
    /// Page-backed stack bytes captured with this continuation.
    pub(crate) stack: Stack,
    /// The frame stack for the suspended execution.
    pub(crate) frames: Vec<Frame>,
    /// The frame index to resume execution in.
    pub(crate) resume_frame_index: usize,
    /// The resume point for this continuation.
    pub(crate) resume_point: engine::ResumePointId,
}

impl Continuation {
    /// Clone this continuation for multi-shot resumption.
    pub fn clone_for_fork(&self) -> RuntimeResult<Self> {
        let stack = self.stack.fork()?;
        let mut frames: Vec<_> = self.frames.iter().map(Frame::clone_for_fork).collect();

        // point cloned frames at the forked stack bytes
        for frame in &mut frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len)
                .map_err(|_| RuntimeError::new(Error::InvalidContinuation))?;
            frame.replace_bytes(frame.stack_offset, frame.byte_len, base);
        }

        Ok(Self {
            isolate_id: self.isolate_id,
            stack,
            frames,
            resume_frame_index: self.resume_frame_index,
            resume_point: self.resume_point,
        })
    }

    /// Capture one immutable continuation image.
    pub fn image(&self, program: &Program) -> RuntimeResult<ContinuationImage> {
        let frames = self
            .frames
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| {
                capture_continuation_frame(
                    program,
                    frame,
                    frame_index,
                    self.resume_frame_index,
                    self.resume_point,
                )
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ContinuationImage {
            engine_id: self.isolate_id,
            frames,
        })
    }

    /// Visit heap roots referenced by this continuation.
    pub(crate) fn visit_roots(
        &self,
        program: &Program,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        for (frame_index, frame) in self.frames.iter().enumerate() {
            let materialization = continuation_frame_materialization(
                program,
                frame,
                frame_index,
                self.resume_frame_index,
                self.resume_point,
            )?;

            frame.visit_materialized_roots(program, materialization, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots referenced by this continuation.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame_index in 0..self.frames.len() {
            let materialization = {
                let frame = &self.frames[frame_index];
                continuation_frame_materialization(
                    program,
                    frame,
                    frame_index,
                    self.resume_frame_index,
                    self.resume_point,
                )?
            };

            self.frames[frame_index].visit_materialized_root_slots(
                program,
                materialization,
                visit,
            )?;
        }

        Ok(())
    }

    /// Visit heap roots from one captured continuation image.
    pub(crate) fn visit_image_roots(
        image: &ContinuationImage,
        program: &Program,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        for frame in &image.frames {
            visit_frame_image_roots(frame, program, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots from one captured continuation image.
    pub(crate) fn visit_image_root_slots(
        image: &mut ContinuationImage,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame in &mut image.frames {
            visit_frame_image_root_slots(frame, program, visit)?;
        }

        Ok(())
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        program: &Program,
        functions: &FunctionTable,
        options: &IsolateOptions,
    ) -> RuntimeResult<Self> {
        if image.frames.is_empty() {
            return Err(RuntimeError::new(Error::InvalidContinuation));
        }

        let mut stack = Stack::reserve(options.limits.max_stack_bytes)?;
        let mut frames = Vec::with_capacity(image.frames.len());

        for frame_image in &image.frames {
            let layout = program
                .frame_layout_by_id(frame_image.frame_layout)
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
            if frame_image.bytes.len() != layout.byte_len as usize {
                return Err(RuntimeError::new(Error::InvalidContinuation));
            }

            let base = stack.allocate(frame_image.bytes.len(), Word::BYTE_LEN)?;
            stack.write(base, &frame_image.bytes)?;
            let frame_base = stack.address(base, frame_image.bytes.len())?;
            let frame = restore_frame_image(frame_image, program, functions, base, frame_base)?;
            frames.push(frame);
        }

        let resume_frame_index = image.frames.len() - 1;
        let resume_point = image
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?
            .resume_point;

        Ok(Self {
            isolate_id: image.engine_id,
            stack,
            frames,
            resume_frame_index,
            resume_point,
        })
    }
}

/// Resolve one continuation frame materialization recipe.
fn continuation_frame_materialization<'a>(
    program: &'a Program,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> Result<&'a engine::MaterializationFrame, Error> {
    let (_resume_point, materialization) = frame_capture_materialization(
        program,
        frame,
        frame_index,
        resume_frame_index,
        resume_point,
    )
    .map_err(|error| error.error)?;

    Ok(materialization)
}

/// Visit heap roots from one captured frame image.
fn visit_frame_image_roots(
    image: &engine::FrameImage,
    program: &Program,
    roots: &mut impl RootSink,
) -> Result<(), Error> {
    let layout = program
        .frame_layout_by_id(image.frame_layout)
        .ok_or(Error::InvalidContinuation)?;
    if image.bytes.len() != layout.byte_len as usize {
        return Err(Error::InvalidContinuation);
    }

    let materialization = program
        .materialization_frame(image.resume_point)
        .ok_or(Error::InvalidContinuation)?;
    visit_materialized_image_roots(image, program, layout, materialization, roots)?;

    Ok(())
}

/// Visit mutable local root slots from one captured frame image.
fn visit_frame_image_root_slots(
    image: &mut engine::FrameImage,
    program: &Program,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let layout = program
        .frame_layout_by_id(image.frame_layout)
        .ok_or(Error::InvalidContinuation)?;
    if image.bytes.len() != layout.byte_len as usize {
        return Err(Error::InvalidContinuation);
    }

    let materialization = program
        .materialization_frame(image.resume_point)
        .ok_or(Error::InvalidContinuation)?;
    visit_materialized_image_root_slots(image, program, layout, materialization, visit)?;

    Ok(())
}

/// Visit heap roots from materialized frame image regions.
fn visit_materialized_image_roots(
    image: &engine::FrameImage,
    program: &Program,
    layout: &engine::FrameLayout,
    materialization: &engine::MaterializationFrame,
    roots: &mut impl RootSink,
) -> Result<(), Error> {
    let mut visited = Vec::new();

    // scan each live source region once
    for value in &materialization.regions {
        let engine::MaterializationValue::FrameRegion(region) = value else {
            continue;
        };
        if visited.contains(region) {
            continue;
        }
        visited.push(*region);

        let region = layout.region(*region).ok_or(Error::InvalidContinuation)?;
        visit_region_image_roots(image, program, region, roots)?;
    }

    Ok(())
}

/// Visit mutable local root slots from materialized frame image regions.
fn visit_materialized_image_root_slots(
    image: &mut engine::FrameImage,
    program: &Program,
    layout: &engine::FrameLayout,
    materialization: &engine::MaterializationFrame,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let mut visited = Vec::new();

    // visit each live source region once
    for value in &materialization.regions {
        let engine::MaterializationValue::FrameRegion(region) = value else {
            continue;
        };
        if visited.contains(region) {
            continue;
        }
        visited.push(*region);

        let region = layout.region(*region).ok_or(Error::InvalidContinuation)?;
        visit_region_image_root_slots(image, program, region, visit)?;
    }

    Ok(())
}

/// Visit heap roots from one frame image region.
fn visit_region_image_roots(
    image: &engine::FrameImage,
    program: &Program,
    region: &engine::FrameRegion,
    roots: &mut impl RootSink,
) -> Result<(), Error> {
    let layout = program
        .layout_for_id(region.ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!(
                "missing image frame layout for root scan: type={:?}",
                region.ty
            ),
        })?;
    let start = region.offset as usize;
    let end = start + region.byte_len as usize;
    let bytes = &image.bytes[start..end];

    if !layout.is_word() {
        return Frame::visit_byte_roots(program, program.type_for_id(region.ty), bytes, roots);
    }

    let word = read_image_word(bytes)?;
    Frame::visit_value_root(program, program.type_for_id(region.ty), word, roots)
}

/// Visit mutable local root slots from one frame image region.
fn visit_region_image_root_slots(
    image: &mut engine::FrameImage,
    program: &Program,
    region: &engine::FrameRegion,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let start = region.offset as usize;
    let end = start + region.byte_len as usize;
    let bytes = &mut image.bytes[start..end];

    Frame::visit_byte_root_slots(program, program.type_for_id(region.ty), bytes, visit)
}

/// Resolve the captured resume point and materialization frame for one suspended frame.
pub(crate) fn frame_capture_materialization<'a>(
    program: &'a Program,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<(engine::ResumePointId, &'a engine::MaterializationFrame)> {
    let resume_point = captured_resume_point(
        program,
        frame,
        frame_index,
        resume_frame_index,
        resume_point,
    )?;

    let materialization_frame = program.materialization_frame(resume_point).ok_or_else(|| {
        RuntimeError::new(Error::InvariantViolation {
            context: format!("missing materialization frame for resume point: {resume_point:?}"),
        })
    })?;

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
    program: &Program,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<engine::ResumePointId> {
    if frame_index == resume_frame_index {
        return Ok(resume_point);
    }

    program
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
    program: &Program,
    frame: &Frame,
    frame_index: usize,
    resume_frame_index: usize,
    resume_point: engine::ResumePointId,
) -> RuntimeResult<engine::FrameImage> {
    let (resume_point, _materialization_frame) = frame_capture_materialization(
        program,
        frame,
        frame_index,
        resume_frame_index,
        resume_point,
    )?;

    Ok(engine::FrameImage {
        frame_layout: frame.frame_layout,
        resume_point,
        transfer: frame.transfer.clone(),
        bytes: frame.bytes().to_vec(),
    })
}

/// Restore one live frame from one logical frame image.
fn restore_frame_image(
    image: &engine::FrameImage,
    program: &Program,
    functions: &FunctionTable,
    stack_offset: usize,
    frame_base: *mut u8,
) -> RuntimeResult<Frame> {
    let layout = program
        .frame_layout_by_id(image.frame_layout)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let resume_point = program
        .resume_point(image.resume_point)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    if resume_point.frame_layout != image.frame_layout || resume_point.function != layout.function {
        return Err(RuntimeError::new(Error::InvalidContinuation));
    }

    let function_id = program.function_for_id(layout.function);
    let block_id = program.block_for_id(resume_point.block);
    let function_ptr = functions.pointer_for(function_id).ok_or_else(|| {
        RuntimeError::new(Error::UndefinedFunction {
            function: function_id,
        })
    })?;
    let function = unsafe { function_ptr.as_ref() };

    let block_index = function
        .blocks
        .iter()
        .position(|block| block.mir_block == block_id)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
    let block = function
        .blocks
        .get(block_index)
        .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

    let mut frame = Frame::new(
        image.frame_layout,
        function_id,
        function_ptr,
        std::ptr::NonNull::from(block),
        block_id,
        layout,
        stack_offset,
        frame_base,
    );
    frame.current_block = block_id;
    frame.block_ptr = std::ptr::NonNull::from(block);
    frame.resume_pc = resume_point.instruction_offset as usize;
    frame.transfer = image.transfer.clone();

    Ok(frame)
}

/// Read one word from captured frame bytes.
fn read_image_word(bytes: &[u8]) -> Result<Word, Error> {
    if bytes.len() < Word::BYTE_LEN {
        return Err(Error::InvalidContinuation);
    }

    let mut raw = [0u8; Word::BYTE_LEN];
    raw.copy_from_slice(&bytes[..Word::BYTE_LEN]);

    Ok(Word::from_bits(u64::from_le_bytes(raw)))
}
