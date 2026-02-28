use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::string::StringInterner;
use super::{ExternalFn, ExternalFnPtr, ExternalHandler, GlobalStorage, StringRef};
use crate::diagnostic::Error;
use crate::memory::{HeapBorrow, HeapHandle, HeapReadBorrow, RawPointer, SharedHeap, Value};
use crate::options::IsolateOptions;

// isolate id generator for continuation validation
static ISOLATE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Core isolate state shared across execution engines.
pub(crate) struct IsolateState {
    /// Unique id used to validate continuation ownership.
    pub(crate) isolate_id: u64,
    /// The MIR tree being executed.
    pub(crate) tree: mir::NodeTree,
    /// String pool for names.
    pub(crate) strings: ImmutableStringPool,
    /// Shared heap storage for managed and raw allocations.
    pub(crate) heap: SharedHeap,
    /// String interner for literal storage.
    pub(crate) string_interner: StringInterner,
    /// Global variable storage.
    pub(crate) globals: GlobalStorage,
    /// External function handlers.
    pub(crate) externals: HashMap<String, ExternalFn>,
    /// Cached external handlers by function id.
    pub(crate) externals_by_id: Vec<Option<ExternalFnPtr>>,
    /// Lookup table for function ids by name.
    pub(crate) function_name_map: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Lookup table for vtables keyed by vtable globals.
    pub(crate) vtable_by_global: HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId>,
    /// Configuration options for this isolate.
    pub(crate) options: IsolateOptions,
}

impl IsolateState {
    /// Create isolate state with the given options.
    pub(crate) fn new(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> Self {
        Self::new_with_heap_store(tree, strings, options, SharedHeap::default())
    }

    /// Create isolate state with explicit heap store.
    pub(crate) fn new_with_heap_store(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
        heap: SharedHeap,
    ) -> Self {
        let function_name_map = build_function_name_map(&tree, &strings);
        let vtable_by_global = build_vtable_map(&tree);

        // assign a unique isolate id
        let isolate_id = ISOLATE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            isolate_id,
            tree,
            strings,
            heap,
            string_interner: StringInterner::new(),
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            externals_by_id: Vec::new(),
            function_name_map,
            vtable_by_global,
            options,
        }
    }

    /// Borrow the heap store exclusively.
    pub(crate) fn heap_borrow(&self) -> HeapBorrow<'_> {
        self.heap.borrow()
    }

    /// Borrow the heap store for read-only access.
    pub(crate) fn heap_borrow_read(&self) -> HeapReadBorrow<'_> {
        self.heap.borrow_read()
    }

    /// Resolve a vtable id for a vtable global.
    pub(crate) fn vtable_for_global(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<mir::VtableId> {
        self.vtable_by_global.get(&global).copied()
    }

    /// Register a VM binding handler.
    pub(crate) fn register_vm_binding(
        &mut self,
        name: &str,
        handler: impl ExternalHandler + 'static,
    ) {
        self.externals.insert(name.to_string(), Box::new(handler));

        // cache handler pointer for direct id lookup
        if let Some(func_id) = self.function_name_map.get(name).copied() {
            let index = func_id.id as usize;
            if self.externals_by_id.len() <= index {
                self.externals_by_id.resize(index + 1, None);
            }
            if let Some(handler) = self.externals.get(name) {
                self.externals_by_id[index] = Some(ExternalFnPtr::from(handler.as_ref()));
            }
        }
    }

    /// Intern a string literal and return its managed value.
    pub(crate) fn intern_string_literal(&mut self, value: &str) -> Value {
        // delegate to the string interner
        let heap = &self.heap;
        let string_interner = &mut self.string_interner;
        let mut heap = heap.borrow();
        let heap = &mut *heap;
        string_interner.intern_string_literal(&mut heap.managed, &mut heap.raw, value)
    }

    /// Read a UTF-8 string value from the heap.
    pub(crate) fn string_value(&self, value: Value) -> Result<String, Error> {
        // delegate to the string interner
        let heap = self.heap_borrow();
        self.string_interner
            .string_value(&heap.managed, &heap.raw, value)
    }

    /// Read a UTF-8 string view from the heap.
    pub(crate) fn string_value_ref(&self, value: Value) -> Result<StringRef<'_>, Error> {
        // borrow the heap for read-only access
        let heap = self.heap.borrow_read();

        // resolve the borrowed string view
        let view = self
            .string_interner
            .string_value_view(&heap.managed, &heap.raw, value)?;

        Ok(StringRef::new(heap, view.ptr, view.len))
    }

    /// Read a UTF-8 string from a managed handle.
    pub(crate) fn string_value_for_handle(&self, handle: HeapHandle) -> Result<String, Error> {
        // delegate to the string interner
        let heap = self.heap_borrow();
        self.string_interner
            .string_value_for_handle(&heap.managed, &heap.raw, handle)
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        let handle = self.heap_borrow().managed.allocate_with_values(values);
        Value::aggregate(handle)
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        let handle = self.heap_borrow().managed.allocate_pair(first, second);
        Value::aggregate(handle)
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_single(&mut self, value: Value) -> Value {
        let handle = self.heap_borrow().managed.allocate_single(value);
        Value::aggregate(handle)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub(crate) fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.heap_borrow().raw.allocate_with_values(values)
    }

    /// Allocate a raw heap cell with byte storage and return its pointer.
    pub(crate) fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        self.heap_borrow().raw.allocate_with_bytes(bytes)
    }

    /// Collect string literal handles as GC roots.
    pub(crate) fn collect_string_roots(&self, roots: &mut Vec<HeapHandle>) {
        // delegate to the string interner
        self.string_interner.collect_roots(roots);
    }

    /// Sweep raw string payloads for freed managed string headers.
    pub(crate) fn sweep_string_buffers(&mut self) {
        // delegate to the string interner
        let heap = &self.heap;
        let string_interner = &mut self.string_interner;
        let mut heap = heap.borrow();
        let heap = &mut *heap;
        string_interner.sweep_buffers(&heap.managed, &mut heap.raw);
    }
}

impl fmt::Debug for IsolateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let heap = self.heap_borrow();

        f.debug_struct("IsolateState")
            .field("managed_heap", &heap.managed)
            .field("raw_heap", &heap.raw)
            .field("string_interner", &self.string_interner)
            .field("globals", &format!("<{} globals>", self.globals.len()))
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

/// Build the function name lookup table.
fn build_function_name_map(
    tree: &mir::NodeTree,
    strings: &ImmutableStringPool,
) -> HashMap<String, mir::LocalNodeId<mir::Function>> {
    let mut map = HashMap::new();
    for (id, func) in tree.iter_nodes::<mir::Function>() {
        let name = strings.get(func.name).to_string();
        map.entry(name).or_insert(id);
    }
    map
}

/// Build a lookup table from vtable globals to vtable ids.
fn build_vtable_map(tree: &mir::NodeTree) -> HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId> {
    let mut map = HashMap::new();
    for (table_id, table) in tree.type_table.iter_vtables() {
        let mir::VtableStorage::Global(global) = table.storage;
        map.insert(global, table_id);
    }
    map
}
