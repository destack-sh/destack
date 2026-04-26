#![allow(clippy::missing_const_for_thread_local)]

use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::{align_of, needs_drop, size_of};
use std::ptr;

use serde::{Deserialize, Serialize};

use super::call::BindingCallContext;
use super::worker::Worker;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Session;
use crate::platform::NativeArray;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::runtime::bindings::BindingAffinity;
use crate::runtime::scheduler::{EventLoop, MicrotaskId, TaskId};
use crate::runtime::world::WorldRef;

thread_local! {
    /// TLS slot for the current runtime execution context.
    static CURRENT_WORKER_CONTEXT: Cell<CurrentWorkerContext> =
        const { Cell::new(CurrentWorkerContext::empty()) };
    /// TLS slot for the current binding call context.
    static CURRENT_BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> =
        const { Cell::new(ptr::null()) };
    /// TLS storage for native ABI references returned by bindings.
    static CURRENT_BINDING_CALL_ARENA: BindingCallArena = const { BindingCallArena::new() };
    /// TLS slot for the currently running task or microtask.
    static CURRENT_RUNNABLE_SCOPE: Cell<RunnableScope> =
        const { Cell::new(RunnableScope::empty()) };
}

/// Stable identifier for one execution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionContextId(pub u64);

impl ExecutionContextId {
    /// Build one identifier from an opaque hash payload.
    pub const fn from_hash(hash: u64) -> Self {
        Self(hash)
    }
}

/// Runtime execution context for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Stable execution context identifier.
    pub id: ExecutionContextId,
    /// Whether the current execution context is the process main context.
    pub is_process_main: bool,
}

impl ExecutionContext {
    /// Build one execution context payload.
    pub const fn new(id: ExecutionContextId, is_process_main: bool) -> Self {
        Self {
            id,
            is_process_main,
        }
    }
}

/// Current runtime execution context for VM callback bridging.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CurrentWorkerContext {
    /// Worker pointer for callback dispatch.
    pub worker: *const Worker,
    /// Event loop pointer for callback dispatch.
    pub event_loop: *const EventLoop,
    /// Host pointer for callback dispatch.
    pub host: *const Session,
    /// World pointer for replay, time, random, and policy.
    pub world: *const WorldRef,
    /// Execution context identifier for callback dispatch.
    pub execution_context_id: ExecutionContextId,
    /// Whether this execution scope runs on the process main context.
    pub is_process_main: bool,
}

impl CurrentWorkerContext {
    /// Return one empty runtime execution context.
    pub(crate) const fn empty() -> Self {
        Self {
            worker: ptr::null(),
            event_loop: ptr::null(),
            host: ptr::null(),
            world: ptr::null(),
            execution_context_id: ExecutionContextId(0),
            is_process_main: false,
        }
    }

    /// Return whether this execution context is available.
    pub(crate) const fn is_empty(self) -> bool {
        self.worker.is_null()
            || self.event_loop.is_null()
            || self.host.is_null()
            || self.world.is_null()
    }
}

/// Guard that restores the previous current-worker execution context.
#[derive(Debug)]
pub(crate) struct CurrentWorkerContextGuard {
    /// Previous current-worker execution context.
    previous: CurrentWorkerContext,
}

impl Drop for CurrentWorkerContextGuard {
    /// Restore the previous current-worker execution context.
    fn drop(&mut self) {
        CURRENT_WORKER_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Currently running task or microtask.
#[derive(Debug, Clone, Copy)]
pub struct RunnableScope {
    /// Current task identifier, if any.
    task_id: Option<TaskId>,
    /// Current microtask identifier, if any.
    microtask_id: Option<MicrotaskId>,
    /// Current nested microtask execution depth.
    microtask_depth: usize,
}

impl RunnableScope {
    /// Create an empty runnable scope.
    pub const fn empty() -> Self {
        Self {
            task_id: None,
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a task runnable scope.
    pub const fn for_task(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a microtask runnable scope.
    pub const fn for_microtask(microtask_id: MicrotaskId, depth: usize) -> Self {
        Self {
            task_id: None,
            microtask_id: Some(microtask_id),
            microtask_depth: depth,
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(self) -> Option<TaskId> {
        self.task_id
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(self) -> Option<MicrotaskId> {
        self.microtask_id
    }

    /// Return the current microtask nesting depth.
    pub const fn microtask_depth(self) -> usize {
        self.microtask_depth
    }
}

/// Guard that restores the previous runnable scope.
#[derive(Debug)]
pub struct RunnableScopeGuard {
    /// Previous runnable scope.
    previous: RunnableScope,
}

impl Drop for RunnableScopeGuard {
    /// Restore the previous runnable scope.
    fn drop(&mut self) {
        CURRENT_RUNNABLE_SCOPE.with(|slot| slot.set(self.previous));
    }
}

/// Guard that restores the previous TLS binding call context.
#[derive(Debug)]
pub struct BindingCallGuard {
    /// Previous TLS context pointer.
    previous: *const BindingCallContext,
}

impl Drop for BindingCallGuard {
    /// Restore the previous binding call context.
    fn drop(&mut self) {
        CURRENT_BINDING_CALL_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// The number of fixed-width pages reserved in one call-arena block.
const PAGES_PER_CALL_BLOCK: usize = 64;
/// The native binding-call scratch page width.
const CALL_ARENA_PAGE_BYTES: usize = 4 * 1024;

/// Per-call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same native thread.
#[derive(Debug)]
pub struct BindingCallArena {
    /// The page-backed scratch arena for this native thread.
    pages: RefCell<CallPageArena>,
    /// Drop records for in-place values stored inside the scratch pages.
    drops: RefCell<Vec<BindingCallDrop>>,
}

impl Default for BindingCallArena {
    /// Create one empty binding call arena.
    fn default() -> Self {
        Self::new()
    }
}

/// One page-backed call arena block.
#[derive(Debug)]
struct CallBlock {
    /// The owned page bytes for this block.
    data: *mut u8,
    /// The total byte length of this block.
    byte_len: usize,
    /// The number of bytes already reserved from this block.
    used: usize,
}

impl CallBlock {
    /// Create one empty block with the given byte length.
    fn new(byte_len: usize) -> Self {
        Self {
            data: allocate_call_block_bytes(byte_len, CALL_ARENA_PAGE_BYTES),
            byte_len,
            used: 0,
        }
    }

    /// Reserve one aligned byte range from this block.
    fn allocate(&mut self, byte_len: usize, align: usize) -> Option<*mut u8> {
        let start = align_offset(self.used, align);
        let end = start.checked_add(byte_len)?;
        if end > self.byte_len {
            return None;
        }

        let ptr = unsafe { self.data.add(start) };
        self.used = end;

        Some(ptr)
    }

    /// Reset the used cursor for the next binding call.
    fn reset(&mut self) {
        self.used = 0;
    }
}

impl Drop for CallBlock {
    /// Release the owned block pages.
    fn drop(&mut self) {
        free_call_block_bytes(self.data, self.byte_len, CALL_ARENA_PAGE_BYTES);
    }
}

/// One resettable page-backed arena for binding call scratch storage.
#[derive(Debug)]
struct CallPageArena {
    /// The fixed byte width for one underlying page.
    page_bytes: usize,
    /// The owned page blocks.
    blocks: Vec<CallBlock>,
}

impl CallPageArena {
    /// Create one empty call arena.
    const fn new(page_bytes: usize) -> Self {
        Self {
            page_bytes,
            blocks: Vec::new(),
        }
    }

    /// Reserve one aligned byte range.
    fn allocate(&mut self, byte_len: usize, align: usize) -> *mut u8 {
        let byte_len = byte_len.max(1);
        let align = align.max(1);

        for block in self.blocks.iter_mut().rev() {
            if let Some(ptr) = block.allocate(byte_len, align) {
                return ptr;
            }
        }

        let block_bytes = self.block_bytes_for(byte_len, align);
        let mut block = CallBlock::new(block_bytes);
        let Some(ptr) = block.allocate(byte_len, align) else {
            handle_alloc_error(Layout::new::<u8>());
        };
        self.blocks.push(block);

        ptr
    }

    /// Reset reusable blocks for the next binding call.
    fn reset(&mut self) {
        for block in &mut self.blocks {
            block.reset();
        }
    }

    /// Return one block size large enough for the given allocation.
    fn block_bytes_for(&self, byte_len: usize, align: usize) -> usize {
        let default_block_bytes = self.page_bytes * PAGES_PER_CALL_BLOCK;
        let required_bytes = align_offset(byte_len, align);

        default_block_bytes.max(round_up_to_page(required_bytes, self.page_bytes))
    }
}

/// One drop record for values stored inside the call arena.
#[derive(Debug)]
struct BindingCallDrop {
    /// Pointer to the first stored value.
    data: *mut (),
    /// The number of initialized values.
    len: usize,
    /// Drop glue for one contiguous value range.
    drop: unsafe fn(*mut (), usize),
}

impl BindingCallDrop {
    /// Drop one stored value range.
    unsafe fn release(self) {
        unsafe { (self.drop)(self.data, self.len) };
    }
}

/// One direct writer into one call-arena allocation range.
#[derive(Debug)]
pub struct BindingCallBuilder<T> {
    /// Pointer to the first element slot.
    data: *mut T,
    /// The number of initialized elements.
    len: usize,
    /// The maximum number of elements this builder may write.
    capacity: usize,
    /// Whether the builder has already transferred ownership out.
    is_finished: bool,
    /// Marker for the element type.
    marker: PhantomData<T>,
}

impl<T> BindingCallBuilder<T> {
    /// Create one empty builder over the given allocation range.
    fn new(data: *mut T, capacity: usize) -> Self {
        Self {
            data,
            len: 0,
            capacity,
            is_finished: false,
            marker: PhantomData,
        }
    }

    /// Append one element into the reserved range.
    pub fn push(&mut self, value: T) {
        if self.len >= self.capacity {
            panic!("binding call builder capacity exceeded");
        }

        unsafe { self.data.add(self.len).write(value) };
        self.len += 1;
    }

    /// Finish the builder and return the initialized element count.
    fn finish(mut self) -> (*mut T, usize) {
        self.is_finished = true;
        (self.data, self.len)
    }

    /// Finish the builder after one direct bulk initialization.
    fn finish_initialized(mut self, len: usize) -> Self {
        if len > self.capacity {
            panic!("binding call builder capacity exceeded");
        }

        self.len = len;
        self
    }
}

impl<T> Drop for BindingCallBuilder<T> {
    /// Drop any written elements when the builder exits early.
    fn drop(&mut self) {
        if self.is_finished || !needs_drop::<T>() || self.len == 0 {
            return;
        }

        let values = ptr::slice_from_raw_parts_mut(self.data, self.len);
        unsafe { ptr::drop_in_place(values) };
    }
}

/// Round one offset up to the given alignment.
fn align_offset(offset: usize, align: usize) -> usize {
    if align <= 1 {
        return offset;
    }

    let mask = align - 1;
    offset.saturating_add(mask) & !mask
}

/// Round one byte length up to the next whole page.
fn round_up_to_page(byte_len: usize, page_bytes: usize) -> usize {
    byte_len.div_ceil(page_bytes).saturating_mul(page_bytes)
}

/// Allocate zeroed page-aligned bytes for native binding-call scratch storage.
fn allocate_call_block_bytes(byte_len: usize, page_bytes: usize) -> *mut u8 {
    let layout = call_block_layout(byte_len, page_bytes);
    let data = unsafe { alloc_zeroed(layout) };

    if data.is_null() {
        handle_alloc_error(layout);
    }

    data
}

/// Free page-aligned bytes allocated by the native binding-call scratch arena.
fn free_call_block_bytes(data: *mut u8, byte_len: usize, page_bytes: usize) {
    if data.is_null() {
        return;
    }

    let layout = call_block_layout(byte_len, page_bytes);
    unsafe { dealloc(data, layout) };
}

/// Return one allocation layout for a native binding-call scratch block.
fn call_block_layout(byte_len: usize, page_bytes: usize) -> Layout {
    let byte_len = byte_len.max(1);

    match Layout::from_size_align(byte_len, page_bytes) {
        Ok(layout) => layout,
        Err(_) => handle_alloc_error(Layout::new::<u8>()),
    }
}

/// Drop one contiguous arena-stored value range.
unsafe fn drop_stored_values<T>(data: *mut (), len: usize) {
    let values = ptr::slice_from_raw_parts_mut(data.cast::<T>(), len);
    unsafe { ptr::drop_in_place(values) };
}

/// Narrow one ABI length to `u32`.
fn abi_len_u32(len: usize, label: &str) -> u32 {
    u32::try_from(len).unwrap_or_else(|_| panic!("native ABI {label} length exceeds u32: {len}"))
}

impl BindingCallArena {
    /// Create an empty call arena.
    pub const fn new() -> Self {
        Self {
            pages: RefCell::new(CallPageArena::new(CALL_ARENA_PAGE_BYTES)),
            drops: RefCell::new(Vec::new()),
        }
    }

    /// Clear all stored references.
    pub fn clear(&self) {
        let mut drops = self.drops.borrow_mut();

        // drop arena-stored values in reverse insertion order
        while let Some(drop) = drops.pop() {
            unsafe { drop.release() };
        }

        self.pages.borrow_mut().reset();
    }

    /// Store a string and return a native string reference.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        self.store_string_bytes(value.as_bytes())
    }

    /// Store one owned string and return a native string reference.
    pub fn store_string_owned(&self, value: String) -> NativeStringRef {
        let bytes = value.into_bytes();
        let reference = self.store_string_bytes(&bytes);
        drop(bytes);
        reference
    }

    /// Store one string byte slice and return a native string reference.
    fn store_string_bytes(&self, bytes: &[u8]) -> NativeStringRef {
        let len = bytes.len();
        let data = self.allocate_bytes(len, align_of::<u8>());
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), data, len) };

        NativeStringRef {
            data: data.cast(),
            len: abi_len_u32(len, "string"),
        }
    }

    /// Store an optional string and return a native string reference.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        match value {
            Some(value) => self.store_string(value),
            None => NativeStringRef {
                data: ptr::null(),
                len: 0,
            },
        }
    }

    /// Store a slice and return a native slice reference.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        self.store_slice_from_iter(values)
    }

    /// Build and store one slice with exact expected capacity.
    pub fn store_slice_with<T: 'static>(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut BindingCallBuilder<T>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeSlice<T>> {
        let mut builder = self.allocate_builder::<T>(capacity);

        fill(&mut builder)?;

        Ok(self.finish_slice_builder(builder, "slice"))
    }

    /// Copy a slice and return a native slice reference.
    pub fn store_slice_copy<T: Copy + 'static>(&self, values: &[T]) -> NativeSlice<T> {
        let builder = self.allocate_builder::<T>(values.len());
        let data = builder.data;
        let len = values.len();

        // copy the final payload into call-arena storage
        if len != 0 {
            unsafe { ptr::copy_nonoverlapping(values.as_ptr(), data, len) };
        }

        self.finish_slice_builder(builder.finish_initialized(len), "slice")
    }

    /// Store a slice and return a native array reference.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        self.store_array_from_iter(values)
    }

    /// Build and store one array with exact expected capacity.
    pub fn store_array_with<T: 'static>(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut BindingCallBuilder<T>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeArray<T>> {
        let mut builder = self.allocate_builder::<T>(capacity);

        fill(&mut builder)?;

        Ok(self.finish_array_builder(builder, "array"))
    }

    /// Copy a slice and return a native array reference.
    pub fn store_array_copy<T: Copy + 'static>(&self, values: &[T]) -> NativeArray<T> {
        let builder = self.allocate_builder::<T>(values.len());
        let data = builder.data;
        let len = values.len();

        // copy the final payload into call-arena storage
        if len != 0 {
            unsafe { ptr::copy_nonoverlapping(values.as_ptr(), data, len) };
        }

        self.finish_array_builder(builder.finish_initialized(len), "array")
    }

    /// Store one zeroed byte slice and return a native slice reference.
    pub fn store_zeroed_byte_slice(&self, len: usize) -> NativeSlice<u8> {
        let builder = self.allocate_builder::<u8>(len);
        let data = builder.data;

        // zero the final payload in place
        if len != 0 {
            unsafe { ptr::write_bytes(data, 0, len) };
        }

        self.finish_slice_builder(builder.finish_initialized(len), "slice")
    }

    /// Store one zeroed byte array and return a native array reference.
    pub fn store_zeroed_byte_array(&self, len: usize) -> NativeArray<u8> {
        let builder = self.allocate_builder::<u8>(len);
        let data = builder.data;

        // zero the final payload in place
        if len != 0 {
            unsafe { ptr::write_bytes(data, 0, len) };
        }

        self.finish_array_builder(builder.finish_initialized(len), "array")
    }

    /// Store a string slice and return a native string slice.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        let values = self.store_slice(values);

        NativeStringSlice {
            data: values.data.cast_const(),
            len: values.len,
        }
    }

    /// Build and store one string slice with exact expected capacity.
    pub fn store_string_slice_with(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut BindingCallBuilder<NativeStringRef>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeStringSlice> {
        let mut builder = self.allocate_builder::<NativeStringRef>(capacity);

        fill(&mut builder)?;

        let values = self.finish_slice_builder(builder, "string slice");

        Ok(NativeStringSlice {
            data: values.data.cast_const(),
            len: values.len,
        })
    }

    /// Store one slice from an owned iterator.
    fn store_slice_from_iter<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        let mut builder = self.allocate_builder::<T>(values.len());

        for value in values {
            builder.push(value);
        }

        self.finish_slice_builder(builder, "slice")
    }

    /// Store one array from an owned iterator.
    fn store_array_from_iter<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        let mut builder = self.allocate_builder::<T>(values.len());

        for value in values {
            builder.push(value);
        }

        self.finish_array_builder(builder, "array")
    }

    /// Allocate one uninitialized builder for the given element capacity.
    fn allocate_builder<T>(&self, capacity: usize) -> BindingCallBuilder<T> {
        if capacity == 0 {
            return BindingCallBuilder::new(ptr::NonNull::<T>::dangling().as_ptr(), 0);
        }

        let byte_len = size_of::<T>()
            .checked_mul(capacity)
            .unwrap_or_else(|| panic!("binding call allocation overflow for {capacity} items"));
        let data = self.allocate_bytes(byte_len, align_of::<T>()).cast::<T>();

        BindingCallBuilder::new(data, capacity)
    }

    /// Finish one slice builder and return one native slice.
    fn finish_slice_builder<T: 'static>(
        &self,
        builder: BindingCallBuilder<T>,
        label: &str,
    ) -> NativeSlice<T> {
        let (data, len) = builder.finish();
        self.register_drop::<T>(data.cast(), len);

        NativeSlice {
            data,
            len: abi_len_u32(len, label),
        }
    }

    /// Finish one array builder and return one native array.
    fn finish_array_builder<T: 'static>(
        &self,
        builder: BindingCallBuilder<T>,
        label: &str,
    ) -> NativeArray<T> {
        let (data, len) = builder.finish();
        self.register_drop::<T>(data.cast(), len);

        let len = abi_len_u32(len, label);

        NativeArray {
            data,
            len,
            capacity: len,
        }
    }

    /// Register one drop range when the element type needs drop glue.
    fn register_drop<T: 'static>(&self, data: *mut (), len: usize) {
        if !needs_drop::<T>() || len == 0 {
            return;
        }

        self.drops.borrow_mut().push(BindingCallDrop {
            data,
            len,
            drop: drop_stored_values::<T>,
        });
    }

    /// Allocate one aligned byte range from the page-backed call arena.
    fn allocate_bytes(&self, byte_len: usize, align: usize) -> *mut u8 {
        self.pages.borrow_mut().allocate(byte_len, align)
    }
}

/// Return the stable metadata name for one binding-affinity class.
pub const fn binding_affinity_name(affinity: BindingAffinity) -> &'static str {
    match affinity {
        BindingAffinity::Any => "any",
        BindingAffinity::EventLoop => "eventLoop",
        BindingAffinity::Owner => "owner",
        BindingAffinity::ProcessMain => "processMain",
    }
}

/// Enter one current-worker execution context for VM callback dispatch.
pub(crate) fn enter_current_worker_context(
    worker: *const Worker,
    event_loop: *const EventLoop,
    host: *const Session,
    world: *const WorldRef,
    is_process_main: bool,
) -> CurrentWorkerContextGuard {
    let event_loop = unsafe { &*event_loop };
    let execution_context_id = event_loop.execution_context_id();
    let next = CurrentWorkerContext {
        worker,
        event_loop: event_loop as *const EventLoop,
        host,
        world,
        execution_context_id,
        is_process_main,
    };
    let previous = CURRENT_WORKER_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(next);
        previous
    });

    CurrentWorkerContextGuard { previous }
}

/// Return the current-worker execution context when available.
pub(crate) fn current_worker_context() -> Option<CurrentWorkerContext> {
    CURRENT_WORKER_CONTEXT.with(|slot| {
        let context = slot.get();
        if context.is_empty() {
            return None;
        }

        Some(context)
    })
}

/// Enter the scope for the currently running task or microtask.
#[inline]
pub(crate) fn enter_runnable_scope(scope: RunnableScope) -> RunnableScopeGuard {
    let previous = CURRENT_RUNNABLE_SCOPE.with(|slot| {
        let previous = slot.get();
        slot.set(scope);
        previous
    });

    RunnableScopeGuard { previous }
}

/// Return the currently running task or microtask.
#[inline]
pub(crate) fn current_runnable_scope() -> RunnableScope {
    CURRENT_RUNNABLE_SCOPE.with(|slot| slot.get())
}

/// Enter a binding call context for native bindings.
#[inline]
pub fn enter_binding_call_context(context: &BindingCallContext) -> BindingCallGuard {
    let previous = CURRENT_BINDING_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const BindingCallContext);
        previous
    });

    BindingCallGuard { previous }
}

/// Access the current binding call context for native bindings.
#[inline]
pub fn with_binding_call_context<T>(
    f: impl FnOnce(&BindingCallContext) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let context = CURRENT_BINDING_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::BindingCallContextMissing.boxed());
    }

    let context = unsafe { &*context };
    context.clear_values();
    f(context)
}

/// Borrow the current binding call arena.
pub(crate) fn with_binding_call_arena<T>(f: impl FnOnce(&BindingCallArena) -> T) -> T {
    CURRENT_BINDING_CALL_ARENA.with(f)
}
