use std::collections::HashMap;
use std::fmt;

use destack_core::ImmutableStringPool;
use destack_mir as mir;

use super::FunctionTable;

/// Immutable runnable lowering and metadata shared across isolates.
pub struct Executable {
    /// The MIR tree executed by this executable.
    pub(crate) tree: mir::NodeTree,
    /// The immutable string pool for this executable.
    pub(crate) strings: ImmutableStringPool,
    /// Lookup table for function ids by name.
    pub(crate) function_id_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Lookup table for vtables keyed by vtable globals.
    pub(crate) vtable_id_by_global: HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId>,
    /// Lowered function bodies for the current interpreter backend.
    pub(crate) functions: FunctionTable,
}

impl Executable {
    /// Build one executable from one MIR tree and immutable string pool.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        let function_id_by_name = build_function_id_by_name(&tree, &strings);
        let vtable_id_by_global = build_vtable_id_by_global(&tree);
        let functions = FunctionTable::new(&tree);

        Self {
            tree,
            strings,
            function_id_by_name,
            vtable_id_by_global,
            functions,
        }
    }
}

impl fmt::Debug for Executable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executable")
            .field(
                "functions",
                &format!("<{} functions>", self.function_id_by_name.len()),
            )
            .finish_non_exhaustive()
    }
}

/// Build the function name lookup table.
fn build_function_id_by_name(
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
fn build_vtable_id_by_global(
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Global>, mir::VtableId> {
    let mut map = HashMap::new();
    for (table_id, table) in tree.dispatch_table.iter_vtables() {
        let mir::VtableStorage::Global(global) = table.storage;
        map.insert(global, table_id);
    }
    map
}
