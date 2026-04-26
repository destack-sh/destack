use std::mem;
use std::ptr::NonNull;

use destack_mir::ReferenceMap;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::program::{Block, Function, FunctionTable, Program, repr_type};
use crate::snapshot::FrameImage;
use crate::{FramePointer, RootVisitor, Word};
use destack_heap::{HeapReference, HeapResult, RootSlot, SharedHeapReference};

/// Heap reference carried by one scalar type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScalarRoot {
    /// No heap root.
    None,
    /// Worker-local heap root.
    Local,
    /// World-shared heap root.
    Shared,
}

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The logical frame layout.
    pub(crate) frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// Pointer to the lowered function.
    pub(crate) function_ptr: NonNull<Function>,
    /// Pointer to the current block.
    pub(crate) block_ptr: NonNull<Block>,
    /// The current block being executed.
    pub(crate) current_block: mir::LocalNodeId<mir::Block>,
    /// Program counter within the current block.
    pub(crate) resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub(crate) transfer: Option<engine::ControlTransfer>,
    /// The byte offset in the interpreter stack arena.
    pub(crate) stack_offset: usize,
    /// The frame byte width.
    pub(crate) byte_len: usize,
    /// Pointer to the frame bytes in the interpreter stack arena.
    base: *mut u8,
}

// the raw function and block pointers always point into immutable program data
// that stays alive for the duration of the owning isolate
unsafe impl Send for Frame {}

impl Frame {
    /// Create a new frame for a function.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        frame_layout: engine::FrameLayoutId,
        function: mir::LocalNodeId<mir::Function>,
        function_ptr: NonNull<Function>,
        block_ptr: NonNull<Block>,
        current_block: mir::LocalNodeId<mir::Block>,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: *mut u8,
    ) -> Self {
        Self {
            frame_layout,
            function,
            function_ptr,
            block_ptr,
            current_block,
            resume_pc: 0,
            transfer: None,
            stack_offset,
            byte_len: layout.byte_len as usize,
            base,
        }
    }

    /// Repoint this frame after its backing stack arena has moved.
    pub(crate) fn remap_bytes(&mut self, stack_base: *mut u8) {
        self.base = unsafe { stack_base.add(self.stack_offset) };
    }

    /// Replace this frame's byte range.
    pub(crate) fn replace_bytes(&mut self, stack_offset: usize, byte_len: usize, base: *mut u8) {
        self.stack_offset = stack_offset;
        self.byte_len = byte_len;
        self.base = base;
    }

    /// Extend this frame's stack-owned byte range.
    pub(crate) fn extend_bytes_to(&mut self, stack_end: usize) {
        if stack_end > self.stack_offset {
            self.byte_len = stack_end - self.stack_offset;
        }
    }

    /// Return the native address of this frame's byte range.
    #[inline]
    pub(crate) fn base_address(&self) -> usize {
        self.base as usize
    }

    /// Return this frame's byte range.
    #[inline]
    pub(crate) fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.byte_len) }
    }

    /// Return this frame's byte range mutably.
    #[inline]
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.base, self.byte_len) }
    }

    /// Return a pointer to one frame region.
    #[inline(always)]
    pub(crate) fn region_address(&self, region: &engine::FrameRegion) -> usize {
        self.base_address() + region.offset as usize
    }

    /// Read one word from a region.
    #[inline(always)]
    pub(crate) fn read_word(&self, region: &engine::FrameRegion) -> Word {
        debug_assert!(region.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.region_address(region) % mem::align_of::<Word>()), 0);

        unsafe { std::ptr::read(self.region_address(region) as *const Word) }
    }

    /// Write one word into a region.
    #[inline(always)]
    pub(crate) fn write_word(&mut self, region: &engine::FrameRegion, value: Word) {
        debug_assert!(region.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.region_address(region) % mem::align_of::<Word>()), 0);

        unsafe {
            std::ptr::write(self.region_address(region) as *mut Word, value);
        }
    }

    /// Read one word or frame address from a region.
    #[inline(always)]
    pub(crate) fn read_operand(&self, region: &engine::FrameRegion) -> Word {
        if region.is_word {
            return self.read_word(region);
        }

        Word::frame_pointer(FramePointer::from_address(self.region_address(region)))
    }

    /// Borrow one region byte range.
    pub(crate) fn region_bytes(&self, region: &engine::FrameRegion) -> &[u8] {
        let start = region.offset as usize;
        let end = start + region.byte_len as usize;

        &self.bytes()[start..end]
    }

    /// Borrow one region byte range mutably.
    pub(crate) fn region_bytes_mut(&mut self, region: &engine::FrameRegion) -> &mut [u8] {
        let start = region.offset as usize;
        let end = start + region.byte_len as usize;

        &mut self.bytes_mut()[start..end]
    }

    /// Get a value from this frame.
    #[inline]
    pub fn get_value(
        &self,
        layout: &engine::FrameLayout,
        value: mir::Value,
    ) -> RuntimeResult<Word> {
        self.get_value_or_error(layout, value)
            .map_err(RuntimeError::new)
    }

    /// Get a value from this frame without call stack context.
    #[inline]
    pub fn get_value_or_error(
        &self,
        layout: &engine::FrameLayout,
        value: mir::Value,
    ) -> Result<Word, Error> {
        let region = layout
            .value_index(value.0)
            .ok_or(Error::UndefinedValue { value })?;

        Ok(self.read_operand(region))
    }

    /// Write one word into a scalar SSA value.
    #[inline]
    pub(crate) fn write_value_word(
        &mut self,
        layout: &engine::FrameLayout,
        value: mir::Value,
        word: Word,
    ) -> Result<(), Error> {
        let region = layout
            .value_index(value.0)
            .ok_or(Error::UndefinedValue { value })?;
        if !region.is_word {
            return Err(Error::TypeMismatch {
                expected: "word value".to_string(),
                actual: format!("frame-backed value: {value:?}"),
            });
        }

        self.write_word(region, word);

        Ok(())
    }

    /// Check if a value is defined in this frame.
    #[inline]
    pub fn has_value(&self, layout: &engine::FrameLayout, value: mir::Value) -> bool {
        layout.value_index(value.0).is_some()
    }

    /// Get a local variable.
    pub fn get_local(
        &self,
        layout: &engine::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> RuntimeResult<Word> {
        self.get_local_or_error(layout, local)
            .map_err(RuntimeError::new)
    }

    /// Get a local variable without call stack context.
    #[inline]
    pub fn get_local_or_error(
        &self,
        layout: &engine::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<Word, Error> {
        let region = layout
            .local_index(local.id)
            .ok_or(Error::UndefinedLocal { local })?;

        Ok(self.read_operand(region))
    }

    /// Return the address of one local value.
    pub(crate) fn local_address(
        &self,
        layout: &engine::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<usize, Error> {
        let region = layout
            .local_index(local.id)
            .ok_or(Error::UndefinedLocal { local })?;

        Ok(self.region_address(region))
    }

    /// Return the callable environment for this frame.
    pub(crate) fn environment(&self, layout: &engine::FrameLayout) -> Result<Word, Error> {
        let Some(region) = layout.environment.as_ref() else {
            return Ok(Word::VOID);
        };

        Ok(self.read_word(region))
    }

    /// Store the callable environment for this frame.
    pub(crate) fn set_environment(&mut self, layout: &engine::FrameLayout, value: Word) {
        if let Some(region) = layout.environment.as_ref() {
            self.write_word(region, value);
        }
    }

    /// Clear all values (but keep locals).
    pub fn clear_values(&mut self, layout: &engine::FrameLayout) {
        for region in &layout.values {
            self.region_bytes_mut(region).fill(0);
        }
    }

    /// Return whether this frame owns one stack byte range.
    pub(crate) fn owns_stack_range(&self, address: usize, byte_len: usize) -> bool {
        let start = self.base_address();
        let end = match address.checked_add(byte_len) {
            Some(end) => end,
            None => return false,
        };
        let stack_end = start.saturating_add(self.byte_len);

        start <= address && end <= stack_end
    }

    /// Visit all heap references from this frame for GC roots.
    pub(crate) fn visit_roots(
        &self,
        program: &Program,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        let layout =
            program
                .frame_layout(self.function)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing frame layout for frame: {:?}", self.function),
                })?;

        // ssa values
        for region in &layout.values {
            self.visit_region_roots(program, region, roots)?;
        }

        // locals
        for region in &layout.locals {
            self.visit_region_roots(program, region, roots)?;
        }

        // callable environment
        if let Some(region) = layout.environment.as_ref() {
            self.visit_region_roots(program, region, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots in this frame.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout =
            program
                .frame_layout(self.function)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing frame layout for frame: {:?}", self.function),
                })?;

        // ssa values
        for region in &layout.values {
            self.visit_region_root_slots(program, region, visit)?;
        }

        // locals
        for region in &layout.locals {
            self.visit_region_root_slots(program, region, visit)?;
        }

        // callable environment
        if let Some(region) = layout.environment.as_ref() {
            self.visit_region_root_slots(program, region, visit)?;
        }

        Ok(())
    }

    /// Visit one heap root stored in one typed value.
    pub(crate) fn visit_value_root(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
        value: Word,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        Self::visit_scalar_root(Self::type_scalar_root(program, ty)?, value, roots)
    }

    /// Visit one scalar root after its reference domain is known.
    fn visit_scalar_root(
        root: ScalarRoot,
        value: Word,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        match root {
            ScalarRoot::None => {}
            ScalarRoot::Local => {
                let reference = HeapReference::from_bits(value.bits() as usize);
                if !reference.is_null() {
                    roots.push_heap(reference);
                }
            }
            ScalarRoot::Shared => {
                let reference = SharedHeapReference::from_bits(value.bits() as usize);
                if !reference.is_null() {
                    roots.push_shared(reference);
                }
            }
        }
        Ok(())
    }

    /// Visit heap roots stored in one frame region.
    fn visit_region_roots(
        &self,
        program: &Program,
        region: &engine::FrameRegion,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        let layout = program
            .layout_for_id(region.ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing frame region layout for root scan: type={:?}",
                    region.ty
                ),
            })?;
        if !layout.is_scalar() {
            return Self::visit_byte_roots(
                program,
                program.type_for_id(region.ty),
                self.region_bytes(region),
                roots,
            );
        }

        Self::visit_value_root(
            program,
            program.type_for_id(region.ty),
            self.read_word(region),
            roots,
        )
    }

    /// Visit local root slots stored in one frame region.
    fn visit_region_root_slots(
        &mut self,
        program: &Program,
        region: &engine::FrameRegion,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = program
            .layout_for_id(region.ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing frame region layout for heap roots: type={:?}",
                    region.ty
                ),
            })?;
        if !layout.is_scalar() {
            return Self::visit_byte_root_slots(
                program,
                program.type_for_id(region.ty),
                self.region_bytes_mut(region),
                visit,
            );
        }

        if Self::type_scalar_root(program, program.type_for_id(region.ty))? == ScalarRoot::Local {
            visit(RootSlot::Bytes(self.region_bytes_mut(region))).map_err(Error::from)?;
        }

        Ok(())
    }

    /// Return the heap reference domain carried by one scalar type.
    fn type_scalar_root(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<ScalarRoot, Error> {
        let layout = program
            .layout(ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing scalar layout for root scan: type={ty:?}"),
            })?;
        if !layout.is_scalar() {
            return Ok(ScalarRoot::None);
        }

        let repr_ty = repr_type(&program.tree, ty);

        match program.tree.get(repr_ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local,
                ..
            }
            | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local,
                ..
            }
            | mir::Type::Callable { .. } => Ok(ScalarRoot::Local),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Shared,
                ..
            }
            | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Shared,
                ..
            } => Ok(ScalarRoot::Shared),
            _ => Ok(ScalarRoot::None),
        }
    }

    /// Visit heap roots from one typed byte range.
    pub(crate) fn visit_byte_roots(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        let layout = program
            .layout(ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing layout for byte root scan: type={ty:?}"),
            })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "byte root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        Self::collect_type_roots(program, ty, bytes, 0, roots)
    }

    /// Visit local root slots from one typed byte range.
    pub(crate) fn visit_byte_root_slots(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = program
            .layout(ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing layout for byte heap roots: type={ty:?}"),
            })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "byte heap root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        Self::visit_type_root_slots(program, ty, bytes, 0, visit)
    }

    /// Visit heap roots from one typed byte range.
    fn collect_type_roots(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        base_offset: usize,
        roots: &mut impl RootVisitor,
    ) -> Result<(), Error> {
        let layout = program
            .layout(ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing layout for stack root scan: type={ty:?}"),
            })?;
        let pointer_bytes = program.tree.pointer_bytes() as usize;

        // local heap roots
        for offset in local_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack root local offset overflow: type={ty:?}"),
                }
            })?;
            let window = Self::reference_window(bytes, offset, pointer_bytes, "stack root scan")?;
            let reference = HeapReference::read_from_bytes(window).map_err(Error::from)?;

            if !reference.is_null() {
                roots.push_heap(reference);
            }
        }

        // shared heap roots
        for offset in shared_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack root shared offset overflow: type={ty:?}"),
                }
            })?;
            let window = Self::reference_window(bytes, offset, pointer_bytes, "stack root scan")?;
            let reference = SharedHeapReference::read_from_bytes(window).map_err(Error::from)?;

            if !reference.is_null() {
                roots.push_shared(reference);
            }
        }

        Ok(())
    }

    /// Visit local root slots from one typed byte range.
    fn visit_type_root_slots(
        program: &Program,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        base_offset: usize,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = program
            .layout(ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing layout for stack heap roots: type={ty:?}"),
            })?;
        let pointer_bytes = program.tree.pointer_bytes() as usize;

        // local heap roots
        for offset in local_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack heap root offset overflow: type={ty:?}"),
                }
            })?;
            let end = offset.checked_add(pointer_bytes).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!(
                        "stack heap root byte range overflow: offset={offset}, width={pointer_bytes}",
                    ),
                }
            })?;
            let bytes_len = bytes.len();
            let Some(slot) = bytes.get_mut(offset..end) else {
                return Err(Error::InvariantViolation {
                    context: format!(
                        "stack heap root byte range out of bounds: offset={offset}, width={pointer_bytes}, len={bytes_len}",
                    ),
                });
            };

            visit(RootSlot::Bytes(slot)).map_err(Error::from)?;
        }

        Ok(())
    }

    /// Return one immutable reference byte window.
    fn reference_window<'a>(
        bytes: &'a [u8],
        start: usize,
        width: usize,
        context: &'static str,
    ) -> Result<&'a [u8], Error> {
        if width != HeapReference::BYTE_LEN {
            return Err(Error::InvariantViolation {
                context: format!("{context} unsupported reference width: {width}"),
            });
        }

        let end = start
            .checked_add(width)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("{context} byte range overflow: start={start}, width={width}"),
            })?;

        bytes
            .get(start..end)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "{context} byte range out of bounds: start={start}, width={width}, len={}",
                    bytes.len(),
                ),
            })
    }

    /// Clone this frame for a forked continuation.
    pub(crate) fn clone_for_fork(&self) -> Self {
        Self {
            frame_layout: self.frame_layout,
            function: self.function,
            function_ptr: self.function_ptr,
            block_ptr: self.block_ptr,
            current_block: self.current_block,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            stack_offset: self.stack_offset,
            byte_len: self.byte_len,
            base: self.base,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self) -> FrameImage {
        FrameImage {
            frame_layout: self.frame_layout,
            function: self.function,
            current_block: self.current_block,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            bytes: self.bytes().to_vec(),
        }
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(
        image: &FrameImage,
        functions: &FunctionTable,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: *mut u8,
    ) -> RuntimeResult<Self> {
        // resolve the lowered function for this frame
        let function_index = functions.index_for(image.function).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        let function_ptr = functions.get_ptr_by_index(function_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        // resolve the current block pointer from the lowered function
        let function_ref = unsafe { function_ptr.as_ref() };
        let block = function_ref
            .blocks
            .iter()
            .find(|block| block.mir_block == image.current_block)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedBlock {
                    block: image.current_block,
                })
            })?;

        Ok(Self {
            frame_layout: image.frame_layout,
            function: image.function,
            function_ptr,
            block_ptr: NonNull::from(block),
            current_block: block.mir_block,
            resume_pc: image.resume_pc,
            transfer: image.transfer.clone(),
            stack_offset,
            byte_len: layout.byte_len as usize,
            base,
        })
    }
}

/// Return all local heap reference offsets in one map.
fn local_reference_offsets(reference_map: &ReferenceMap) -> Result<Vec<u32>, Error> {
    match reference_map {
        ReferenceMap::None => Ok(Vec::new()),
        ReferenceMap::Reference { local_offsets, .. } => Ok(local_offsets.to_vec()),
        ReferenceMap::RepeatedReference {
            count,
            stride,
            local_offsets,
            ..
        } => repeated_reference_offsets(*count, *stride, local_offsets),
    }
}

/// Return all shared heap reference offsets in one map.
fn shared_reference_offsets(reference_map: &ReferenceMap) -> Result<Vec<u32>, Error> {
    match reference_map {
        ReferenceMap::None => Ok(Vec::new()),
        ReferenceMap::Reference { shared_offsets, .. } => Ok(shared_offsets.to_vec()),
        ReferenceMap::RepeatedReference {
            count,
            stride,
            shared_offsets,
            ..
        } => repeated_reference_offsets(*count, *stride, shared_offsets),
    }
}

/// Expand repeated reference map offsets into absolute byte offsets.
fn repeated_reference_offsets(count: u32, stride: u32, offsets: &[u32]) -> Result<Vec<u32>, Error> {
    let mut result = Vec::with_capacity(count as usize * offsets.len());

    // expand each repeated element
    for index in 0..count {
        let base = index
            .checked_mul(stride)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "repeated reference-map base offset overflow: index={index}, stride={stride}",
                ),
            })?;

        for offset in offsets {
            let offset = base
                .checked_add(*offset)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!(
                        "repeated reference-map offset overflow: base={base}, offset={offset}",
                    ),
                })?;

            result.push(offset);
        }
    }

    Ok(result)
}
