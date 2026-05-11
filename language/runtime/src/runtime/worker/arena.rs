#![allow(clippy::missing_const_for_thread_local)]

use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem::{align_of, needs_drop, size_of};
use std::ptr;

use crate::diagnostic::RuntimeResult;
use crate::platform::NativeArray;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};

/// Native ABI scratch page width.
const NATIVE_CALL_PAGE_BYTES: usize = 4 * 1024;
/// Number of fixed width pages reserved in one native ABI scratch block.
const PAGES_PER_NATIVE_CALL_BLOCK: usize = 64;

thread_local! {
    /// TLS storage for native ABI references returned by bindings.
    static CURRENT_NATIVE_CALL_ARENA: NativeCallArena = const { NativeCallArena::new() };
}

/// Per call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same native thread.
#[derive(Debug)]
pub struct NativeCallArena {
    /// The page-backed scratch arena for this native thread.
    pages: RefCell<NativeCallPages>,
    /// Drop records for in-place values stored inside the scratch pages.
    drops: RefCell<Vec<NativeCallDrop>>,
}

impl Default for NativeCallArena {
    /// Create one empty native call arena.
    fn default() -> Self {
        Self::new()
    }
}

impl NativeCallArena {
    /// Create an empty native call arena.
    pub const fn new() -> Self {
        Self {
            pages: RefCell::new(NativeCallPages::new(NATIVE_CALL_PAGE_BYTES)),
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
        fill: impl FnOnce(&mut NativeCallBuilder<T>) -> RuntimeResult<()>,
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

        // copy the final payload into native ABI scratch storage
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
        fill: impl FnOnce(&mut NativeCallBuilder<T>) -> RuntimeResult<()>,
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

        // copy the final payload into native ABI scratch storage
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
        fill: impl FnOnce(&mut NativeCallBuilder<NativeStringRef>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeStringSlice> {
        let mut builder = self.allocate_builder::<NativeStringRef>(capacity);

        fill(&mut builder)?;

        let values = self.finish_slice_builder(builder, "string slice");

        Ok(NativeStringSlice {
            data: values.data.cast_const(),
            len: values.len,
        })
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
    fn allocate_builder<T>(&self, capacity: usize) -> NativeCallBuilder<T> {
        if capacity == 0 {
            return NativeCallBuilder::new(ptr::NonNull::<T>::dangling().as_ptr(), 0);
        }

        let byte_len = size_of::<T>()
            .checked_mul(capacity)
            .unwrap_or_else(|| panic!("native call allocation overflow for {capacity} items"));
        let data = self.allocate_bytes(byte_len, align_of::<T>()).cast::<T>();

        NativeCallBuilder::new(data, capacity)
    }

    /// Finish one slice builder and return one native slice.
    fn finish_slice_builder<T: 'static>(
        &self,
        builder: NativeCallBuilder<T>,
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
        builder: NativeCallBuilder<T>,
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

        self.drops.borrow_mut().push(NativeCallDrop {
            data,
            len,
            drop: drop_stored_values::<T>,
        });
    }

    /// Allocate one aligned byte range from the page-backed native call arena.
    fn allocate_bytes(&self, byte_len: usize, align: usize) -> *mut u8 {
        self.pages.borrow_mut().allocate(byte_len, align)
    }
}

/// One direct writer into one native ABI scratch range.
#[derive(Debug)]
pub struct NativeCallBuilder<T> {
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

impl<T> NativeCallBuilder<T> {
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
            panic!("native call builder capacity exceeded");
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
            panic!("native call builder capacity exceeded");
        }

        self.len = len;

        self
    }
}

impl<T> Drop for NativeCallBuilder<T> {
    /// Drop any written elements when the builder exits early.
    fn drop(&mut self) {
        if self.is_finished || !needs_drop::<T>() || self.len == 0 {
            return;
        }

        let values = ptr::slice_from_raw_parts_mut(self.data, self.len);
        unsafe { ptr::drop_in_place(values) };
    }
}

/// One resettable page-backed arena for native ABI scratch storage.
#[derive(Debug)]
struct NativeCallPages {
    /// The fixed byte width for one underlying page.
    page_bytes: usize,
    /// The owned page blocks.
    blocks: Vec<NativeCallBlock>,
}

impl NativeCallPages {
    /// Create one empty native call arena.
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
        let mut block = NativeCallBlock::new(block_bytes);
        let Some(ptr) = block.allocate(byte_len, align) else {
            handle_alloc_error(Layout::new::<u8>());
        };
        self.blocks.push(block);

        ptr
    }

    /// Reset reusable blocks for the next native call.
    fn reset(&mut self) {
        for block in &mut self.blocks {
            block.reset();
        }
    }

    /// Return one block size large enough for the given allocation.
    fn block_bytes_for(&self, byte_len: usize, align: usize) -> usize {
        let default_block_bytes = self.page_bytes * PAGES_PER_NATIVE_CALL_BLOCK;
        let required_bytes = align_offset(byte_len, align);

        default_block_bytes.max(round_up_to_page(required_bytes, self.page_bytes))
    }
}

/// One page-backed native call arena block.
#[derive(Debug)]
struct NativeCallBlock {
    /// The owned page bytes for this block.
    data: *mut u8,
    /// The total byte length of this block.
    byte_len: usize,
    /// The number of bytes already reserved from this block.
    used: usize,
}

impl NativeCallBlock {
    /// Create one empty block with the given byte length.
    fn new(byte_len: usize) -> Self {
        Self {
            data: allocate_native_call_block_bytes(byte_len, NATIVE_CALL_PAGE_BYTES),
            byte_len,
            used: 0,
        }
    }

    /// Reserve one aligned byte range from this block.
    fn allocate(&mut self, byte_len: usize, align: usize) -> Option<*mut u8> {
        let start = align_offset(self.used, align);
        let end = start + byte_len;
        if end > self.byte_len {
            return None;
        }

        let ptr = unsafe { self.data.add(start) };
        self.used = end;

        Some(ptr)
    }

    /// Reset the used cursor for the next native call.
    fn reset(&mut self) {
        self.used = 0;
    }
}

impl Drop for NativeCallBlock {
    /// Release the owned block pages.
    fn drop(&mut self) {
        free_native_call_block_bytes(self.data, self.byte_len, NATIVE_CALL_PAGE_BYTES);
    }
}

/// One drop record for values stored inside the native call arena.
#[derive(Debug)]
struct NativeCallDrop {
    /// Pointer to the first stored value.
    data: *mut (),
    /// The number of initialized values.
    len: usize,
    /// Drop glue for one contiguous value range.
    drop: unsafe fn(*mut (), usize),
}

impl NativeCallDrop {
    /// Drop one stored value range.
    unsafe fn release(self) {
        unsafe { (self.drop)(self.data, self.len) };
    }
}

/// Borrow the current native call arena.
pub(crate) fn with_native_call_arena<T>(f: impl FnOnce(&NativeCallArena) -> T) -> T {
    CURRENT_NATIVE_CALL_ARENA.with(f)
}

/// Round one offset up to the given alignment.
fn align_offset(offset: usize, align: usize) -> usize {
    if align <= 1 {
        return offset;
    }

    let mask = align - 1;

    (offset + mask) & !mask
}

/// Round one byte length up to the next whole page.
fn round_up_to_page(byte_len: usize, page_bytes: usize) -> usize {
    byte_len.div_ceil(page_bytes) * page_bytes
}

/// Allocate zeroed page aligned bytes for native ABI scratch storage.
fn allocate_native_call_block_bytes(byte_len: usize, page_bytes: usize) -> *mut u8 {
    let layout = native_call_block_layout(byte_len, page_bytes);
    let data = unsafe { alloc_zeroed(layout) };

    if data.is_null() {
        handle_alloc_error(layout);
    }

    data
}

/// Free page aligned bytes allocated by the native ABI scratch arena.
fn free_native_call_block_bytes(data: *mut u8, byte_len: usize, page_bytes: usize) {
    if data.is_null() {
        return;
    }

    let layout = native_call_block_layout(byte_len, page_bytes);
    unsafe { dealloc(data, layout) };
}

/// Return one allocation layout for a native ABI scratch block.
fn native_call_block_layout(byte_len: usize, page_bytes: usize) -> Layout {
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
