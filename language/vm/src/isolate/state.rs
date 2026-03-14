use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::ImmutableStringPool;
use destack_mir as mir;

use super::string::StringInterner;
use super::{ExternalFn, ExternalFnPtr, ExternalHandler, GlobalStorage, StringRef};
use crate::diagnostic::Error;
use crate::snapshot::IsolateImage;
use destack_heap::{Heap, ManagedReference, RawPointer, Value};

// isolate id generator for continuation validation
static ISOLATE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Core isolate state shared across execution engines.
pub(crate) struct IsolateState {
    /// Unique id used to validate continuation ownership.
    pub(crate) isolate_id: u64,
    /// Immutable isolate image metadata.
    pub(crate) image: Arc<IsolateImage>,
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
}

impl IsolateState {
    /// Create isolate state from one shared immutable image.
    pub(crate) fn new(image: Arc<IsolateImage>) -> Self {
        let function_name_map = build_function_name_map(&image.tree, &image.strings);
        let vtable_by_global = build_vtable_map(&image.tree);

        // reuse the captured isolate id when present
        let isolate_id = if image.isolate_id == 0 {
            ISOLATE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        } else {
            image.isolate_id
        };

        Self {
            isolate_id,
            image,
            string_interner: StringInterner::new(),
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            externals_by_id: Vec::new(),
            function_name_map,
            vtable_by_global,
        }
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
    pub(crate) fn intern_string_literal(&mut self, heap: &mut Heap, value: &str) -> Value {
        // delegate to the string interner
        self.string_interner.intern_string_literal(heap, value)
    }

    /// Intern a UTF-8 string literal and return its managed value.
    pub(crate) fn try_intern_string_literal(
        &mut self,
        heap: &mut Heap,
        value: &str,
    ) -> Result<Value, Error> {
        // delegate to the string interner
        self.string_interner.try_intern_string_literal(heap, value)
    }

    /// Read a UTF-8 string value from the heap.
    pub(crate) fn string_value(&self, heap: &Heap, value: Value) -> Result<String, Error> {
        // delegate to the string interner
        self.string_interner.string_value(heap, value)
    }

    /// Read a UTF-8 string view from the heap.
    pub(crate) fn string_value_ref<'a>(
        &'a self,
        heap: &'a Heap,
        value: Value,
    ) -> Result<StringRef<'a>, Error> {
        // resolve the borrowed string view
        let view = self.string_interner.string_value_view(heap, value)?;

        Ok(StringRef::new(heap, view.ptr, view.len))
    }

    /// Read a UTF-8 string from a managed handle.
    pub(crate) fn string_value_for_handle(
        &self,
        heap: &Heap,
        handle: ManagedReference,
    ) -> Result<String, Error> {
        self.string_interner.string_value_for_handle(heap, handle)
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub(crate) fn allocate_aggregate(&mut self, heap: &mut Heap, values: Vec<Value>) -> Value {
        self.try_allocate_aggregate(heap, values)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_pair(&mut self, heap: &mut Heap, first: Value, second: Value) -> Value {
        self.try_allocate_pair(heap, first, second)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_single(&mut self, heap: &mut Heap, value: Value) -> Value {
        self.try_allocate_single(heap, value)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Allocate an aggregate on the heap and return it as a value.
    pub(crate) fn try_allocate_aggregate(
        &mut self,
        heap: &mut Heap,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        let handle = heap.allocate_packed_values(values).map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate a 2-element aggregate on the heap.
    pub(crate) fn try_allocate_pair(
        &mut self,
        heap: &mut Heap,
        first: Value,
        second: Value,
    ) -> Result<Value, Error> {
        let handle = heap
            .allocate_packed_pair(first, second)
            .map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate a 1-element aggregate on the heap.
    pub(crate) fn try_allocate_single(
        &mut self,
        heap: &mut Heap,
        value: Value,
    ) -> Result<Value, Error> {
        let handle = heap.allocate_packed_single(value).map_err(Error::from)?;
        Ok(Value::aggregate(handle))
    }

    /// Allocate one raw byte allocation and return its pointer.
    pub(crate) fn try_allocate_raw_bytes(
        &mut self,
        heap: &mut Heap,
        bytes: &[u8],
    ) -> Result<RawPointer, Error> {
        heap.allocate_raw_bytes(bytes).map_err(Error::from)
    }

    /// Collect string literal handles as GC roots.
    pub(crate) fn collect_string_roots(&self, roots: &mut Vec<ManagedReference>) {
        // delegate to the string interner
        self.string_interner.collect_roots(roots);
    }

    /// Sweep raw string payloads for freed managed string headers.
    pub(crate) fn sweep_string_buffers(&mut self, heap: &mut Heap) {
        // delegate to the string interner
        self.string_interner.sweep_buffers(heap);
    }
}

impl fmt::Debug for IsolateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsolateState")
            .field("string_interner", &self.string_interner)
            .field("globals", &format!("<{} globals>", self.globals.len()))
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.image.options)
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
    for (table_id, table) in tree.dispatch_table.iter_vtables() {
        let mir::VtableStorage::Global(global) = table.storage;
        map.insert(global, table_id);
    }
    map
}
