use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::string::StringInterner;
use super::{ExternalFn, ExternalFnPtr, ExternalHandler, GlobalStorage};
use crate::diagnostic::Error;
use crate::memory::{HeapHandle, ManagedHeap, RawHeap, RawPointer, Value};
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
    /// The managed heap (GC-tracked allocations).
    pub(crate) managed_heap: ManagedHeap,
    /// The raw heap (manually managed allocations).
    pub(crate) raw_heap: RawHeap,
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
        let function_name_map = build_function_name_map(&tree, &strings);

        // assign a unique isolate id
        let isolate_id = ISOLATE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            isolate_id,
            tree,
            strings,
            managed_heap: ManagedHeap::new(),
            raw_heap: RawHeap::new(),
            string_interner: StringInterner::new(),
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            externals_by_id: Vec::new(),
            function_name_map,
            options,
        }
    }

    /// Register an external function handler.
    pub(crate) fn register_external(
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
        self.string_interner.intern_string_literal(
            &mut self.managed_heap,
            &mut self.raw_heap,
            value,
        )
    }

    /// Read a UTF-8 string value from the heap.
    pub(crate) fn string_value(&self, value: Value) -> Result<String, Error> {
        // delegate to the string interner
        self.string_interner
            .string_value(&self.managed_heap, &self.raw_heap, value)
    }

    /// Read a UTF-8 string from a managed handle.
    pub(crate) fn string_value_for_handle(&self, handle: HeapHandle) -> Result<String, Error> {
        // delegate to the string interner
        self.string_interner
            .string_value_for_handle(&self.managed_heap, &self.raw_heap, handle)
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        let handle = self.managed_heap.allocate_with_values(values);
        Value::aggregate(handle)
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        let handle = self.managed_heap.allocate_pair(first, second);
        Value::aggregate(handle)
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    pub(crate) fn allocate_single(&mut self, value: Value) -> Value {
        let handle = self.managed_heap.allocate_single(value);
        Value::aggregate(handle)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub(crate) fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.raw_heap.allocate_with_values(values)
    }

    /// Collect string literal handles as GC roots.
    pub(crate) fn collect_string_roots(&self, roots: &mut Vec<HeapHandle>) {
        // delegate to the string interner
        self.string_interner.collect_roots(roots);
    }

    /// Sweep raw string payloads for freed managed string headers.
    pub(crate) fn sweep_string_buffers(&mut self) {
        // delegate to the string interner
        self.string_interner
            .sweep_buffers(&self.managed_heap, &mut self.raw_heap);
    }
}

impl fmt::Debug for IsolateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsolateState")
            .field("managed_heap", &self.managed_heap)
            .field("raw_heap", &self.raw_heap)
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
    // collect names into a lookup map
    let mut map = HashMap::new();
    for (id, func) in tree.iter_nodes::<mir::Function>() {
        let name = strings.get(func.name).to_string();
        map.entry(name).or_insert(id);
    }

    // return lookup map
    map
}
