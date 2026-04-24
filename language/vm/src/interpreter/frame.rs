use std::collections::BTreeMap;
use std::ptr::NonNull;

use destack_core::CowBuffer;
use destack_mir::ReferenceMap;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::module::{Block, Function, FunctionTable, Module, repr_type};
use crate::snapshot::FrameImage;
use crate::{RootSink, Value};
use destack_heap::{Heap, HeapReference, SharedHeapReference};

use super::StackAllocation;

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The logical frame layout for this activation.
    pub(crate) frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// Pointer to the lowered function for fast dispatch.
    pub(crate) function_ptr: NonNull<Function>,
    /// Pointer to the current block.
    pub(crate) block_ptr: NonNull<Block>,
    /// The entry block of the function.
    pub(crate) entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub(crate) current_block: mir::LocalNodeId<mir::Block>,
    /// Current block index.
    pub(crate) block_index: usize,
    /// Module counter within the current block.
    pub(crate) resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub(crate) transfer: Option<engine::ControlTransfer>,
    /// Count of SSA values in this frame.
    pub(crate) value_count: usize,
    /// Count of local variables in this frame.
    pub(crate) local_count: usize,
    /// Frame-owned slot storage.
    slots: CowBuffer<Value>,
    /// Stack-allocated byte buffers, freed when the frame pops.
    pub(crate) stack_allocations: Vec<Option<StackAllocation>>,
    /// Callable environment pointer for this frame.
    pub(crate) environment: Value,
}

// the raw function and block pointers always point into immutable module storage
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
        entry_block: mir::LocalNodeId<mir::Block>,
        block_index: usize,
        value_count: usize,
        local_count: usize,
        environment: Value,
    ) -> Self {
        let slot_count = value_count + local_count;

        // assemble frame state
        Self {
            frame_layout,
            function,
            function_ptr,
            block_ptr,
            entry_block,
            current_block: entry_block,
            block_index,
            resume_pc: 0,
            transfer: None,
            value_count,
            local_count,
            slots: CowBuffer::from_vec(vec![Value::VOID; slot_count]),
            stack_allocations: Vec::new(),
            environment,
        }
    }

    /// Borrow all frame slots mutably.
    #[inline]
    pub(crate) fn slots_mut(&mut self) -> &mut [Value] {
        self.slots.make_mut().as_mut_slice()
    }

    /// Return one raw mutable pointer to the frame slots.
    #[inline]
    pub(crate) fn slots_mut_ptr(&mut self) -> *mut Value {
        self.slots.make_mut().as_mut_ptr()
    }

    /// Reset the frame slot storage for one new layout shape.
    pub(crate) fn reset_slots(&mut self, value_count: usize, local_count: usize) {
        let slot_count = value_count + local_count;

        // resize storage first
        let slots = self.slots.make_mut();
        slots.resize(slot_count, Value::VOID);

        // clear all live slots
        slots.fill(Value::VOID);

        // store the new frame shape
        self.value_count = value_count;
        self.local_count = local_count;
    }

    /// Get a value from this frame.
    #[inline]
    pub fn get_value(&self, value: mir::Value) -> RuntimeResult<Value> {
        // forward to value lookup
        self.get_value_or_error(value).map_err(RuntimeError::new)
    }

    /// Get a value from this frame without call stack context.
    #[inline]
    pub fn get_value_or_error(&self, value: mir::Value) -> Result<Value, Error> {
        // compute value index
        let index = value.0 as usize;

        // reject out of bounds values
        if index >= self.value_count {
            return Err(Error::UndefinedValue { value });
        }

        // read value slot
        self.slots
            .as_slice()
            .get(index)
            .copied()
            .ok_or(Error::UndefinedValue { value })
    }

    /// Set a value in this frame.
    #[inline]
    pub fn set_value(&mut self, value: mir::Value, val: Value) {
        // compute value index
        let index = value.0 as usize;

        // validate bounds in debug builds
        debug_assert!(
            index < self.value_count,
            "ssa value out of bounds: {value:?}"
        );

        // write value slot
        self.slots.make_mut()[index] = val;
    }

    /// Get a local variable.
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> RuntimeResult<Value> {
        // forward to local lookup
        self.get_local_or_error(local).map_err(RuntimeError::new)
    }

    /// Get a local variable without call stack context.
    #[inline]
    pub fn get_local_or_error(&self, local: mir::LocalNodeId<mir::Local>) -> Result<Value, Error> {
        // compute local index
        let index = local.id as usize;

        // reject out of bounds locals
        if index >= self.local_count {
            return Err(Error::UndefinedLocal { local });
        }

        // read local slot
        self.slots
            .as_slice()
            .get(self.value_count + index)
            .copied()
            .ok_or(Error::UndefinedLocal { local })
    }

    /// Set a local variable.
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, value: Value) {
        // compute local index
        let index = local.id as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");

        // write local slot
        self.slots.make_mut()[self.value_count + index] = value;
    }

    /// Check if a value is defined in this frame.
    #[inline]
    pub fn has_value(&self, value: mir::Value) -> bool {
        // check value bounds
        let index = value.0 as usize;
        index < self.value_count
    }

    /// Clear all values (but keep locals).
    pub fn clear_values(&mut self) {
        // clear value slots
        self.slots.make_mut()[..self.value_count].fill(Value::VOID);
    }

    /// Allocate a new stack allocation, returning its slot index.
    pub(crate) fn allocate_stack_allocation(&mut self, allocation: StackAllocation) -> usize {
        let slot = self.stack_allocations.len();
        self.stack_allocations.push(Some(allocation));
        slot
    }

    /// Retire one stack allocation by slot index.
    pub(crate) fn retire_stack_allocation(&mut self, slot: usize) -> bool {
        let Some(buffer) = self.stack_allocations.get_mut(slot) else {
            return false;
        };

        buffer.take().is_some()
    }

    /// Return whether this frame still owns any live stack allocation.
    pub fn has_live_stack_allocations(&self) -> bool {
        self.stack_allocations.iter().any(Option::is_some)
    }

    /// Return one logical frame slot value.
    pub fn slot_value(&self, slot: u32) -> Option<Value> {
        if (slot as usize) < self.value_count {
            return self.slots.as_slice().get(slot as usize).copied();
        }

        let local_start = self.value_count as u32;
        if let Some(index) = slot.checked_sub(local_start)
            && (index as usize) < self.local_count
        {
            return self
                .slots
                .as_slice()
                .get(self.value_count + index as usize)
                .copied();
        }

        let environment_slot = local_start + self.local_count as u32;
        if slot == environment_slot {
            return Some(self.environment);
        }

        None
    }

    /// Get a stack allocation by slot index.
    #[inline]
    pub(crate) fn stack_allocation(&self, slot: usize) -> Option<&StackAllocation> {
        self.stack_allocations.get(slot).and_then(Option::as_ref)
    }

    /// Get a mutable reference to a stack allocation by slot index.
    #[inline]
    pub(crate) fn stack_allocation_mut(&mut self, slot: usize) -> Option<&mut StackAllocation> {
        self.stack_allocations
            .get_mut(slot)
            .and_then(Option::as_mut)
    }

    /// Visit all heap references from this frame for GC roots.
    pub(crate) fn visit_roots(
        &self,
        module: &Module,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout =
            module
                .frame_layout(self.function)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing frame layout for frame: {:?}", self.function),
                })?;

        // slice value and local ranges
        let slots = self.slots.as_slice();
        let value_slice = &slots[..self.value_count];
        let local_slice = &slots[self.value_count..self.value_count + self.local_count];

        // value slots
        for (index, value) in value_slice.iter().enumerate() {
            let slot = layout.value_slots.start + index as u32;
            if Self::slot_holds_direct_heap_roots(module, layout, slot)? {
                if let Some(reference) = value.as_heap_reference() {
                    roots.push_heap(reference);
                }

                if let Some(reference) = value.as_shared_heap_reference() {
                    roots.push_shared(reference);
                }
            }
        }

        // local slots
        for (index, value) in local_slice.iter().enumerate() {
            let slot = layout.local_slots.start + index as u32;
            if Self::slot_holds_direct_heap_roots(module, layout, slot)? {
                if let Some(reference) = value.as_heap_reference() {
                    roots.push_heap(reference);
                }

                if let Some(reference) = value.as_shared_heap_reference() {
                    roots.push_shared(reference);
                }
            }
        }

        // callable environment
        if let Some(slot) = layout.environment_slot
            && Self::slot_holds_direct_heap_roots(module, layout, slot)?
        {
            if let Some(reference) = self.environment.as_heap_reference() {
                roots.push_heap(reference);
            }

            if let Some(reference) = self.environment.as_shared_heap_reference() {
                roots.push_shared(reference);
            }
        }

        // dynamic stack allocations
        for allocation in self.stack_allocations.iter().flatten() {
            Self::visit_storage_roots(
                module,
                allocation.storage_type(),
                allocation.bytes(),
                roots,
            )?;
        }

        Ok(())
    }

    /// Collect every local heap reference reachable from this frame.
    pub(crate) fn collect_escape_heap_references(
        &self,
        module: &Module,
        references: &mut Vec<HeapReference>,
    ) -> Result<(), Error> {
        // slot values
        for value in self.slots.as_slice() {
            collect_value_reference(*value, references)?;
        }

        // callable environment
        collect_value_reference(self.environment, references)?;

        // stack allocations
        for allocation in self.stack_allocations.iter().flatten() {
            Self::collect_storage_heap_references(
                module,
                allocation.storage_type(),
                allocation.bytes(),
                references,
            )?;
        }

        Ok(())
    }

    /// Rewrite every local heap reference in this frame through one stable mapping.
    pub(crate) fn rewrite_escape_heap_references(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> Result<(), Error> {
        // slot values
        for value in self.slots.make_mut() {
            rewrite_value_reference(value, references)?;
        }

        // callable environment
        rewrite_value_reference(&mut self.environment, references)?;

        // stack allocations
        for allocation in self.stack_allocations.iter_mut().flatten() {
            Self::rewrite_storage_references(
                module,
                allocation.storage_type(),
                allocation.bytes_mut(),
                references,
            )?;
        }

        let layout = module
            .frame_layout_by_id(self.frame_layout)
            .ok_or(Error::InvalidInstruction)?;

        for slot_index in layout.value_slots.clone().chain(layout.local_slots.clone()) {
            let slot = layout.slot(slot_index).ok_or(Error::InvalidInstruction)?;
            let Some(value) = self.slots.as_slice().get(slot_index as usize).copied() else {
                return Err(Error::InvalidInstruction);
            };

            Self::rewrite_heap_payload_references(module, heap, slot.ty, value, references)?;
        }

        if let Some(slot_index) = layout.environment_slot {
            let slot = layout.slot(slot_index).ok_or(Error::InvalidInstruction)?;

            Self::rewrite_heap_payload_references(
                module,
                heap,
                slot.ty,
                self.environment,
                references,
            )?;
        }

        Ok(())
    }

    /// Rewrite heap payload references for one aggregate slot value.
    fn rewrite_heap_payload_references(
        module: &Module,
        heap: &mut Heap,
        ty: mir::LocalNodeId<mir::Type>,
        value: Value,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> Result<(), Error> {
        let layout = module.layout(ty).ok_or(Error::InvalidInstruction)?;
        if layout.is_scalar() {
            return Ok(());
        }

        let Some(reference) = value.as_heap_reference() else {
            return Ok(());
        };
        if reference.is_null() {
            return Ok(());
        }

        let mut bytes = heap.read_heap_bytes(reference)?;
        Self::rewrite_storage_references(module, ty, &mut bytes, references)?;
        heap.write_heap_bytes(reference, 0, &bytes)?;
        heap.write_barrier(reference, 0, bytes.len())?;

        Ok(())
    }

    /// Return whether one logical frame slot carries one direct heap root value.
    fn slot_holds_direct_heap_roots(
        module: &Module,
        layout: &engine::FrameLayout,
        slot: u32,
    ) -> Result<bool, Error> {
        let slot = layout.slot(slot).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing frame slot for root scan: slot={slot}"),
        })?;
        let slot_layout = module
            .layout(slot.ty)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing frame slot layout for root scan: slot={slot:?}"),
            })?;
        if !slot_layout.is_scalar() {
            return Ok(true);
        }

        let repr_ty = repr_type(&module.tree, slot.ty);

        Ok(matches!(
            module.tree.get(repr_ty),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local | mir::AddressSpace::Shared,
                ..
            } | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local | mir::AddressSpace::Shared,
                ..
            } | mir::Type::Callable { .. }
        ))
    }

    /// Visit heap roots from one storage image.
    pub(crate) fn visit_storage_roots(
        module: &Module,
        storage_type: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout = module
            .layout(storage_type)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing layout for storage root scan: storage_type={storage_type:?}",
                ),
            })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "storage root byte length mismatch: storage_type={storage_type:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        Self::collect_type_roots(module, storage_type, bytes, 0, roots)
    }

    /// Collect every local heap reference stored in one typed byte range.
    pub(crate) fn collect_storage_heap_references(
        module: &Module,
        storage_type: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        references: &mut Vec<HeapReference>,
    ) -> Result<(), Error> {
        let layout = module
            .layout(storage_type)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing layout for storage escape scan: storage_type={storage_type:?}",
                ),
            })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "storage escape scan byte length mismatch: storage_type={storage_type:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        Self::collect_local_type_references(module, storage_type, bytes, 0, references)
    }

    /// Visit heap roots from one typed byte range.
    fn collect_type_roots(
        module: &Module,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        base_offset: usize,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout = module.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing layout for stack root scan: type={ty:?}"),
        })?;
        let pointer_bytes = module.tree.pointer_bytes() as usize;

        // local heap roots
        for offset in local_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack root local offset overflow: type={ty:?}"),
                }
            })?;
            let bits = Self::decode_reference_bits(bytes, offset, pointer_bytes)?;

            if bits != 0 {
                roots.push_heap(HeapReference::from_bits(bits as usize));
            }
        }

        // shared heap roots
        for offset in shared_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack root shared offset overflow: type={ty:?}"),
                }
            })?;
            let bits = Self::decode_reference_bits(bytes, offset, pointer_bytes)?;

            if bits != 0 {
                roots.push_shared(SharedHeapReference::from_bits(bits as usize));
            }
        }

        Ok(())
    }

    /// Collect every local heap reference in one typed byte range.
    fn collect_local_type_references(
        module: &Module,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        base_offset: usize,
        references: &mut Vec<HeapReference>,
    ) -> Result<(), Error> {
        let layout = module.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing layout for stack escape scan: type={ty:?}"),
        })?;
        let pointer_bytes = module.tree.pointer_bytes() as usize;

        // local heap roots
        for offset in local_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack escape local offset overflow: type={ty:?}"),
                }
            })?;
            let bits = Self::decode_reference_bits(bytes, offset, pointer_bytes)?;

            if bits != 0 {
                references.push(HeapReference::from_bits(bits as usize));
            }
        }

        Ok(())
    }

    /// Rewrite every local heap reference in one typed byte range through one stable mapping.
    fn rewrite_type_references(
        module: &Module,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        base_offset: usize,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> Result<(), Error> {
        let layout = module.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing layout for stack stabilization: type={ty:?}"),
        })?;
        let pointer_bytes = module.tree.pointer_bytes() as usize;

        // local heap roots
        for offset in local_reference_offsets(&layout.reference_map)? {
            let offset = base_offset.checked_add(offset as usize).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!("stack stabilization local offset overflow: type={ty:?}"),
                }
            })?;
            let bits = Self::decode_reference_bits(bytes, offset, pointer_bytes)?;

            if bits != 0 {
                let reference = HeapReference::from_bits(bits as usize);
                let reference = references.get(&reference).copied().unwrap_or(reference);

                Self::encode_reference_bits(bytes, offset, pointer_bytes, reference.bits() as u64)?;
            }
        }

        Ok(())
    }

    /// Rewrite every local heap reference stored in one typed byte range through one stable mapping.
    pub(crate) fn rewrite_storage_references(
        module: &Module,
        storage_type: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> Result<(), Error> {
        let layout = module
            .layout(storage_type)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing layout for storage stabilization: storage_type={storage_type:?}",
                ),
            })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "storage stabilization byte length mismatch: storage_type={storage_type:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        Self::rewrite_type_references(module, storage_type, bytes, 0, references)
    }

    /// Decode one pointer-sized reference payload from one typed byte range.
    fn decode_reference_bits(bytes: &[u8], start: usize, width: usize) -> Result<u64, Error> {
        let end = start
            .checked_add(width)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "stack root scan byte range overflow: start={start}, width={width}",
                ),
            })?;
        let byte_range = bytes.get(start..end).ok_or_else(|| Error::InvariantViolation {
            context: format!(
                "stack root scan byte range out of bounds: start={start}, width={width}, len={}",
                bytes.len(),
            ),
        })?;

        match width {
            4 => {
                let mut raw = [0u8; 4];
                raw.copy_from_slice(byte_range);

                Ok(u32::from_le_bytes(raw) as u64)
            }
            8 => {
                let mut raw = [0u8; 8];
                raw.copy_from_slice(byte_range);

                Ok(u64::from_le_bytes(raw))
            }
            _ => Err(Error::InvariantViolation {
                context: format!("unsupported stack root reference width: {width}"),
            }),
        }
    }

    /// Encode one pointer-sized reference payload into one typed byte range.
    fn encode_reference_bits(
        bytes: &mut [u8],
        start: usize,
        width: usize,
        bits: u64,
    ) -> Result<(), Error> {
        let bytes_len = bytes.len();
        let end = start
            .checked_add(width)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "stack stabilization byte range overflow: start={start}, width={width}",
                ),
            })?;
        let byte_range = bytes.get_mut(start..end).ok_or_else(|| Error::InvariantViolation {
            context: format!(
                "stack stabilization byte range out of bounds: start={start}, width={width}, len={bytes_len}",
            ),
        })?;

        match width {
            4 => {
                let bits = u32::try_from(bits).map_err(|_| Error::InvariantViolation {
                    context: format!("stack stabilization reference bits exceed uint32: {bits}"),
                })?;
                byte_range.copy_from_slice(&bits.to_le_bytes());

                Ok(())
            }
            8 => {
                byte_range.copy_from_slice(&bits.to_le_bytes());

                Ok(())
            }
            _ => Err(Error::InvariantViolation {
                context: format!("unsupported stack stabilization reference width: {width}"),
            }),
        }
    }

    /// Clone this frame for a forked continuation.
    pub(crate) fn clone_for_fork(&self) -> Self {
        // clone stack value buffers for the forked frame
        let stack_allocations = self.stack_allocations.to_vec();

        // assemble cloned frame
        Self {
            frame_layout: self.frame_layout,
            function: self.function,
            function_ptr: self.function_ptr,
            block_ptr: self.block_ptr,
            entry_block: self.entry_block,
            current_block: self.current_block,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            value_count: self.value_count,
            local_count: self.local_count,
            slots: self.slots.clone(),
            stack_allocations,
            environment: self.environment,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self) -> FrameImage {
        FrameImage {
            frame_layout: self.frame_layout,
            function: self.function,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            value_count: self.value_count,
            local_count: self.local_count,
            slots: self.slots.as_slice().to_vec(),
            stack_allocations: self.stack_allocations.clone(),
            environment: self.environment,
        }
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(image: &FrameImage, functions: &FunctionTable) -> RuntimeResult<Self> {
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
        let block = function_ref.blocks.get(image.block_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedBlock {
                block: mir::LocalNodeId::new(image.block_index as u32),
            })
        })?;

        // resolve the restored entry block
        let entry_block = function_ref
            .blocks
            .get(function_ref.entry as usize)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedBlock {
                    block: mir::LocalNodeId::new(function_ref.entry),
                })
            })?;

        Ok(Self {
            frame_layout: image.frame_layout,
            function: image.function,
            function_ptr,
            block_ptr: NonNull::from(block),
            entry_block: entry_block.mir_block,
            current_block: block.mir_block,
            block_index: image.block_index,
            resume_pc: image.resume_pc,
            transfer: image.transfer.clone(),
            value_count: image.value_count,
            local_count: image.local_count,
            slots: CowBuffer::from_vec(image.slots.clone()),
            stack_allocations: image.stack_allocations.clone(),
            environment: image.environment,
        })
    }
}

/// Stabilize one local heap reference embedded in one VM value.
pub(crate) fn stabilize_value(heap: &mut Heap, value: &mut Value) -> Result<(), Error> {
    if !value.is_heap_reference() {
        return Ok(());
    }

    let reference = value.as_heap_reference().ok_or(Error::InvalidInstruction)?;
    if reference.is_null() {
        return Ok(());
    }

    let meta = value.reference_meta();
    let reference = heap.stabilize_heap(reference).map_err(Error::from)?;
    *value = Value::heap_reference_with_meta(reference, meta);

    Ok(())
}

/// Collect one local heap reference embedded in one VM value.
fn collect_value_reference(value: Value, references: &mut Vec<HeapReference>) -> Result<(), Error> {
    if !value.is_heap_reference() {
        return Ok(());
    }

    let reference = value.as_heap_reference().ok_or(Error::InvalidInstruction)?;
    if reference.is_null() {
        return Ok(());
    }

    references.push(reference);

    Ok(())
}

/// Rewrite one local heap reference embedded in one VM value through one stable mapping.
fn rewrite_value_reference(
    value: &mut Value,
    references: &BTreeMap<HeapReference, HeapReference>,
) -> Result<(), Error> {
    if !value.is_heap_reference() {
        return Ok(());
    }

    let reference = value.as_heap_reference().ok_or(Error::InvalidInstruction)?;
    if reference.is_null() {
        return Ok(());
    }

    let meta = value.reference_meta();
    let reference = references.get(&reference).copied().unwrap_or(reference);
    *value = Value::heap_reference_with_meta(reference, meta);

    Ok(())
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

/// Expand repeated reference map offsets into absolute payload offsets.
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
