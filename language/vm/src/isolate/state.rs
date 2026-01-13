use std::collections::HashMap;
use std::fmt;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::GlobalStorage;
use crate::diagnostic::Error;
use crate::memory::{HeapHandle, ManagedHeap, RawHeap, RawPointer, Value};
use crate::options::IsolateOptions;

/// External function type.
pub type ExternalFn = Box<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;
/// Cached external handler pointer.
pub(crate) type ExternalFnPtr = NonNull<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;

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
    /// Interned string literals mapped to heap handles.
    pub(crate) string_literals: HashMap<String, HeapHandle>,
    /// Raw heap buffers for string payloads.
    pub(crate) string_buffers: HashMap<HeapHandle, RawPointer>,
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
            string_literals: HashMap::new(),
            string_buffers: HashMap::new(),
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            externals_by_id: Vec::new(),
            function_name_map,
            options,
        }
    }
}

impl fmt::Debug for IsolateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsolateState")
            .field("managed_heap", &self.managed_heap)
            .field("raw_heap", &self.raw_heap)
            .field(
                "string_literals",
                &format!("<{} literals>", self.string_literals.len()),
            )
            .field(
                "string_buffers",
                &format!("<{} buffers>", self.string_buffers.len()),
            )
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
